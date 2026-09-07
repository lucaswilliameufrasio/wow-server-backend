use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
};
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode};
use serde_json::{Value, json};
use tower::ServiceExt;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::auth::*;
use crate::error::*;
use crate::handlers::*;
use crate::models::*;
use crate::repos::*;

// ---------------------------------------------------------------------------
// Mock AccountRepo
// ---------------------------------------------------------------------------

struct MockAccountRepo {
    accounts_by_name: Mutex<HashMap<String, SignInAccountRow>>,
    accounts_by_id: Mutex<HashMap<u64, AuthAccountRow>>,
    gm_levels: Mutex<HashMap<u64, u8>>,
}

impl MockAccountRepo {
    fn new() -> Self {
        Self {
            accounts_by_name: Mutex::new(HashMap::new()),
            accounts_by_id: Mutex::new(HashMap::new()),
            gm_levels: Mutex::new(HashMap::new()),
        }
    }

    fn add_account(&self, row: SignInAccountRow) {
        let name = row.username.clone();
        let id = row.id;
        let locked = row.locked;
        let auth_name = name.clone();
        self.accounts_by_name.lock().unwrap().insert(name, row);
        self.accounts_by_id.lock().unwrap().insert(
            id,
            AuthAccountRow {
                username: auth_name,
                locked,
            },
        );
    }

    fn set_gm_level(&self, account_id: u64, level: u8) {
        self.gm_levels.lock().unwrap().insert(account_id, level);
    }
}

#[async_trait]
impl AccountRepo for MockAccountRepo {
    async fn exists(&self, username: &str) -> Result<bool, ApiError> {
        Ok(self.accounts_by_name.lock().unwrap().contains_key(username))
    }

    async fn create(
        &self,
        username: &str,
        salt: &[u8],
        verifier: &[u8],
        email: Option<&str>,
        _ip: &str,
    ) -> Result<u64, ApiError> {
        let id = (self.accounts_by_name.lock().unwrap().len() + 1) as u64;
        self.accounts_by_name.lock().unwrap().insert(
            username.to_string(),
            SignInAccountRow {
                id,
                username: username.to_string(),
                email: email.map(|s| s.to_string()),
                salt: salt.to_vec(),
                verifier: verifier.to_vec(),
                locked: false,
            },
        );
        self.accounts_by_id.lock().unwrap().insert(
            id,
            AuthAccountRow {
                username: username.to_string(),
                locked: false,
            },
        );
        Ok(id)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<SignInAccountRow>, ApiError> {
        Ok(self.accounts_by_name.lock().unwrap().get(username).cloned())
    }

    async fn increment_failed_logins(&self, _account_id: u64) -> Result<(), ApiError> {
        Ok(())
    }

    async fn record_successful_login(&self, _account_id: u64, _ip: &str) -> Result<(), ApiError> {
        Ok(())
    }

    async fn get_gm_level(&self, account_id: u64) -> u8 {
        self.gm_levels
            .lock()
            .unwrap()
            .get(&account_id)
            .copied()
            .unwrap_or(0)
    }

    async fn find_auth(&self, account_id: u64) -> Result<Option<AuthAccountRow>, ApiError> {
        Ok(self
            .accounts_by_id
            .lock()
            .unwrap()
            .get(&account_id)
            .cloned())
    }

    async fn find_username(&self, account_id: u64) -> Result<Option<String>, ApiError> {
        Ok(self
            .accounts_by_id
            .lock()
            .unwrap()
            .get(&account_id)
            .map(|r| r.username.clone()))
    }

    async fn update_lock(&self, account_id: u64, locked: bool) -> Result<bool, ApiError> {
        let mut by_id = self.accounts_by_id.lock().unwrap();
        if let Some(row) = by_id.get_mut(&account_id) {
            row.locked = locked;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn login_is_locked(&self, _account_id: u64) -> bool {
        false
    }

    fn record_login_failure(&self, _account_id: u64) {}

    fn reset_login_lockout(&self, _account_id: u64) {}

    async fn search_players(
        &self,
        search: Option<&str>,
        _online: Option<bool>,
        limit: u32,
        cursor: Option<u64>,
    ) -> Result<(Vec<AdminPlayerSummary>, Option<u64>), ApiError> {
        let by_name = self.accounts_by_name.lock().unwrap();
        let mut filtered: Vec<AdminPlayerSummary> = by_name
            .iter()
            .filter(|(name, row)| {
                let matches_search = search
                    .map(|s| name.contains(&s.to_uppercase()))
                    .unwrap_or(true);
                let matches_cursor = cursor.map(|c| row.id > c).unwrap_or(true);
                matches_search && matches_cursor
            })
            .map(|(_, row)| AdminPlayerSummary {
                id: row.id,
                username: row.username.clone(),
                email: row.email.clone(),
                joined_unix: None,
                last_login_unix: None,
                last_ip: None,
                locked: row.locked,
                online: false,
                gm_level: 0,
                character_count: 0,
            })
            .collect();
        filtered.sort_by_key(|p| p.id);

        let limit = limit as usize;
        let next_cursor = if filtered.len() > limit {
            Some(filtered[limit - 1].id)
        } else {
            None
        };
        filtered.truncate(limit);
        Ok((filtered, next_cursor))
    }

    async fn find_online_players(&self) -> Result<Vec<OnlinePlayerSummary>, ApiError> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Mock CharacterRepo
// ---------------------------------------------------------------------------

struct MockCharacterRepo {
    chars: Mutex<HashMap<u64, Vec<CharacterSummary>>>,
    owners: Mutex<HashMap<u64, u64>>,
    locations: Mutex<HashMap<u64, CharacterLocationResponse>>,
}

impl MockCharacterRepo {
    fn new() -> Self {
        Self {
            chars: Mutex::new(HashMap::new()),
            owners: Mutex::new(HashMap::new()),
            locations: Mutex::new(HashMap::new()),
        }
    }

    fn add_character(
        &self,
        account_id: u64,
        guid: u64,
        name: &str,
        location: Option<CharacterLocationResponse>,
    ) {
        let mut chars = self.chars.lock().unwrap();
        let list = chars.entry(account_id).or_default();
        list.push(CharacterSummary {
            guid,
            name: name.to_string(),
            race: 1,
            class_id: 1,
            gender: 0,
            level: 80,
            map: 0,
            zone: 0,
            online: false,
            money: 0,
        });
        self.owners.lock().unwrap().insert(guid, account_id);
        if let Some(loc) = location {
            self.locations.lock().unwrap().insert(guid, loc);
        }
    }
}

#[async_trait]
impl CharacterRepo for MockCharacterRepo {
    async fn list_for_account(&self, account_id: u64) -> Result<Vec<CharacterSummary>, ApiError> {
        Ok(self
            .chars
            .lock()
            .unwrap()
            .get(&account_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn find_owner(&self, guid: u64) -> Result<Option<u64>, ApiError> {
        Ok(self.owners.lock().unwrap().get(&guid).copied())
    }

    async fn find_location(
        &self,
        guid: u64,
    ) -> Result<Option<CharacterLocationResponse>, ApiError> {
        Ok(self.locations.lock().unwrap().get(&guid).cloned())
    }

    async fn find_locations_by_account(
        &self,
        account_id: u64,
    ) -> Result<Vec<CharacterLocationResponse>, ApiError> {
        let owners = self.owners.lock().unwrap();
        let locations = self.locations.lock().unwrap();
        let mut result = Vec::new();
        for (guid, owner) in owners.iter() {
            if *owner == account_id
                && let Some(loc) = locations.get(guid)
            {
                result.push(loc.clone());
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Mock ItemRepo
// ---------------------------------------------------------------------------

struct MockItemRepo {
    items: Mutex<Vec<ItemSummary>>,
}

impl MockItemRepo {
    fn new() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
        }
    }

    fn add_item(&self, item: ItemSummary) {
        self.items.lock().unwrap().push(item);
    }
}

#[async_trait]
impl ItemRepo for MockItemRepo {
    async fn search(&self, query: &ItemQuery) -> Result<(Vec<ItemSummary>, Option<u32>), ApiError> {
        let items = self.items.lock().unwrap();
        let mut filtered: Vec<ItemSummary> = items
            .iter()
            .filter(|item| {
                query
                    .search
                    .as_ref()
                    .map(|s| item.name.to_lowercase().contains(&s.to_lowercase()))
                    .unwrap_or(true)
            })
            .filter(|item| query.class.map(|c| item.class == c).unwrap_or(true))
            .filter(|item| query.cursor.map(|c| item.entry > c).unwrap_or(true))
            .cloned()
            .collect();
        filtered.sort_by_key(|i| i.entry);

        let limit = query.limit.unwrap_or(50).min(200) as usize;
        let next_cursor = if filtered.len() > limit {
            Some(filtered[limit - 1].entry)
        } else {
            None
        };
        filtered.truncate(limit);
        Ok((filtered, next_cursor))
    }

    async fn find_by_entry(&self, entry: u32) -> Result<Option<ItemSummary>, ApiError> {
        let items = self.items.lock().unwrap();
        Ok(items.iter().find(|i| i.entry == entry).cloned())
    }
}

// ---------------------------------------------------------------------------
// Mock RefreshTokenRepo
// ---------------------------------------------------------------------------

struct StoredToken {
    id: i64,
    account_id: u64,
    family_id: String,
    is_revoked: bool,
    is_expired: bool,
}

struct MockRefreshTokenRepo {
    tokens_by_hash: Mutex<HashMap<String, StoredToken>>,
    revoked_access: Mutex<HashSet<String>>,
    next_id: Mutex<i64>,
}

impl MockRefreshTokenRepo {
    fn new() -> Self {
        Self {
            tokens_by_hash: Mutex::new(HashMap::new()),
            revoked_access: Mutex::new(HashSet::new()),
            next_id: Mutex::new(1),
        }
    }

    fn add_valid_token(&self, token: &str, account_id: u64) -> String {
        let hash = hash_refresh_token(token);
        let mut id = self.next_id.lock().unwrap();
        let family_id = format!("fam_{}", *id);
        self.tokens_by_hash.lock().unwrap().insert(
            hash.clone(),
            StoredToken {
                id: *id,
                account_id,
                family_id,
                is_revoked: false,
                is_expired: false,
            },
        );
        *id += 1;
        hash
    }
}

#[async_trait]
impl RefreshTokenRepo for MockRefreshTokenRepo {
    async fn store(
        &self,
        account_id: u64,
        refresh_token: &str,
        family_id: Option<&str>,
        _parent_id: Option<i64>,
        _ip: &str,
        _ua: Option<&str>,
    ) -> Result<i64, ApiError> {
        let hash = hash_refresh_token(refresh_token);
        let mut id = self.next_id.lock().unwrap();
        let fam = family_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("fam_{}", *id));
        self.tokens_by_hash.lock().unwrap().insert(
            hash,
            StoredToken {
                id: *id,
                account_id,
                family_id: fam,
                is_revoked: false,
                is_expired: false,
            },
        );
        let current = *id;
        *id += 1;
        Ok(current)
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshTokenRow>, ApiError> {
        Ok(self
            .tokens_by_hash
            .lock()
            .unwrap()
            .get(token_hash)
            .map(|t| RefreshTokenRow {
                id: t.id,
                account_id: t.account_id,
                family_id: t.family_id.clone(),
                is_revoked: t.is_revoked,
                is_expired: t.is_expired,
            }))
    }

    async fn revoke_family(&self, family_id: &str, _reason: &str) -> Result<(), ApiError> {
        let mut tokens = self.tokens_by_hash.lock().unwrap();
        for token in tokens.values_mut() {
            if token.family_id == family_id {
                token.is_revoked = true;
            }
        }
        Ok(())
    }

    async fn revoke_for_account(
        &self,
        account_id: u64,
        refresh_token: &str,
        _reason: &str,
    ) -> Result<(), ApiError> {
        let hash = hash_refresh_token(refresh_token);
        let mut tokens = self.tokens_by_hash.lock().unwrap();
        if let Some(token) = tokens.get_mut(&hash)
            && token.account_id == account_id
        {
            token.is_revoked = true;
        }
        Ok(())
    }

    async fn mark_rotated(&self, old_id: i64, _new_id: i64) -> Result<(), ApiError> {
        let mut tokens = self.tokens_by_hash.lock().unwrap();
        for token in tokens.values_mut() {
            if token.id == old_id {
                token.is_revoked = true;
            }
        }
        Ok(())
    }

    async fn is_access_revoked(&self, jti: &str) -> Result<bool, ApiError> {
        Ok(self.revoked_access.lock().unwrap().contains(jti))
    }

    async fn revoke_access(
        &self,
        _account_id: u64,
        jti: &str,
        _reason: &str,
        _expires_minutes: i64,
    ) -> Result<(), ApiError> {
        self.revoked_access.lock().unwrap().insert(jti.to_string());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDFPcePi+rXcLSd
eFyhwyeFvMoOO1wdatMcJVG/3z31Vw1VdH3t7FQy3xUbCDoiePRQIz0hajTtNrL5
ANDrAVQXP18se+QYF5/mQx/Ib/thU70Q6TF9SsmshF50OyA6DekgsF8BO+TUgShU
4Z0ivN5vK4e7EuPCmSgnHzvpidgrPBdkd52kGci2yLZelovZCIGvHBV/RSGQTrJq
bakujbGz/dA/aaW3W+agNhl7Dd9vaxT4ooI9kDnL0KHzmVsnTX3S/q7M2WONWXG4
GZjmNgHcRehL9Gg3h8ApLJAWuQXlcRl93x86Ny3eZonq4twA9nnf8EU4vZXTCouJ
VYQbtFQtAgMBAAECggEATNtK5Ktvoe1f47Bf4ASMZPdwgGUu+qOCiYgdN7fEi9IU
9wSCgXBCSuRUdAkbpg0dnhBtJJUe9IQI7zAbOEd3PevKqSnJcn3aJ75mJxNDj+Qy
WuGTEDBRL3EQ4Rec8iIzgjJXgsKU4x1E2vZi5YNU5Vq/8+xjZZOKatwn21OEMSv0
/SYKnM1tQS/qqO6CYltojBwhDv6EWloUEEv3tRxNKCtcNNWcrrkcGbMmfr5GQqjW
70RY7H9BnZjG/850yqnMldfk72YIbsW6+cShFpCOlTjC+EZkBb9WnUhXPC64IaDo
spw2sPIRwl2cM9EM4inGr14ODifDTUGi7DMGMs01XQKBgQDnC6IwPkucGMqgbxHE
S5v6GGg47TjeMN0FeDE6TiWwlwc6p/Tlnfn+3wUkMcFhaMM7oc9AbZSgkybXpulu
j+4S0ia9qFC7MKNGKTMreHYMoyWoWfQ+qh3A7GoMXmwhVyqw7hZ9eSscV2OjyCJY
EVSxRWqw/f42BKOx5TlGur8Q0wKBgQDai3fPMJ00BOyS+Y/LH2AMxXDa4pchprHw
h19t35L6OLJAnt25OpikSmBXwn1FtUOtMH+CUp9cQ0aCjBGRiTjUneUtP2kIEdNf
4cm9AKo5EoR8Cvft2BWOO6hVvdWdulz17zMM/IQRFPocleIAMyGo6gex66J6gwRQ
3bj91Yfm/wKBgAju9Dh1UCsa8kq9wKwcWE2VQAJjeb1tmj4p5Y1hlCd9z3O/JsLy
FsZ6DRLXMaj4igP2P7M4CXUj+25/L6tsuUHVClZu+aAjQ0zlLutRXw8iB8S4pa7+
mOPqwDb2N6waWLY6nnf/hWE1J88fX+ST1vh7vKJXT8r65vFr8YkAk36tAoGBALpX
gOigwum/6RfIwtqm/fblwrxfyA1hXQeB5dSBdYj1HsgKrXNqiwxKfqtVogr166aY
W6B7YnYAxvY5CCHXpyVjHC3gi2XeDSUMGD+XeY0ARQafM5cRUA/evkGdg67hYLIy
Ko1AIjuOb1RAWFtjPagRJE6IZBmh7OQmqb2FfENxAoGAZGbIvleM0UR7d9AmBg3S
eqjHhoe4Te6MHJvRBHlGXEeNO8h73pIpeUjbyHftpOXzm5+EGzCj15r56GDdMCBY
nZ5aYoyMjG1tXUt2uFs7KoHOqXq5/MasmuMIEVOX6wmhdGjol6Y/HZrVnQ9CN2Me
1EKl2gU1bA8oiyC7ubUIy3k=
-----END PRIVATE KEY-----"#;

const TEST_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxT3Hj4vq13C0nXhcocMn
hbzKDjtcHWrTHCVRv9899VcNVXR97exUMt8VGwg6Inj0UCM9IWo07Tay+QDQ6wFU
Fz9fLHvkGBef5kMfyG/7YVO9EOkxfUrJrIRedDsgOg3pILBfATvk1IEoVOGdIrze
byuHuxLjwpkoJx876YnYKzwXZHedpBnItsi2XpaL2QiBrxwVf0UhkE6yam2pLo2x
s/3QP2mlt1vmoDYZew3fb2sU+KKCPZA5y9Ch85lbJ0190v6uzNljjVlxuBmY5jYB
3EXoS/RoN4fAKSyQFrkF5XEZfd8fOjct3maJ6uLcAPZ53/BFOL2V0wqLiVWEG7RU
LQIDAQAB
-----END PUBLIC KEY-----"#;

fn test_jwt_config() -> JwtConfig {
    let encoding_key = EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY.as_bytes())
        .unwrap_or_else(|e| panic!("invalid private key: {e:?}"));
    let decoding_key = DecodingKey::from_rsa_pem(TEST_PUBLIC_KEY.as_bytes())
        .unwrap_or_else(|e| panic!("invalid public key: {e:?}"));

    JwtConfig {
        encoding_key,
        decoding_key,
        issuer: "wow-backend-test".to_string(),
        audience: "wow-web-test".to_string(),
        expires_minutes: 15,
        refresh_expires_days: 30,
    }
}

fn test_state(
    accounts: MockAccountRepo,
    characters: MockCharacterRepo,
    items: MockItemRepo,
    refresh_tokens: MockRefreshTokenRepo,
) -> AppState {
    AppState {
        config: AppConfig {
            auth_db: "acore_auth".to_string(),
            characters_db: "acore_characters".to_string(),
            world_db: "acore_world".to_string(),
            srp6_core5_mode: false,
            rate_limit: false,
            debug_enabled: false,
        },
        jwt: test_jwt_config(),
        accounts: std::sync::Arc::new(accounts),
        characters: std::sync::Arc::new(characters),
        items: std::sync::Arc::new(items),
        refresh_tokens: std::sync::Arc::new(refresh_tokens),
        service_tokens: std::sync::Arc::new(MockServiceTokenRepo::new()),
        started_at: 0,
    }
}

fn default_test_state() -> AppState {
    test_state(
        MockAccountRepo::new(),
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body to collect")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("valid json")
}

fn issue_test_token(jwt: &JwtConfig, account_id: u64, username: &str, gm_level: u8) -> String {
    let policy = build_rbac_policy(gm_level);
    issue_jwt(jwt, account_id, username, gm_level, &policy).expect("token issue")
}

// ---------------------------------------------------------------------------
// Pure function tests (no DB needed, same as original)
// ---------------------------------------------------------------------------

#[test]
fn srp6_roundtrip_verifier_matches() {
    let (salt, verifier) = srp6_register("ADMIN", "StrongPassword123", false);

    assert!(srp6_verify(
        "ADMIN",
        "StrongPassword123",
        &salt,
        &verifier,
        false
    ));
    assert!(!srp6_verify(
        "ADMIN",
        "WrongPassword",
        &salt,
        &verifier,
        false
    ));
}

#[test]
fn issued_jwt_can_be_verified() {
    let jwt = test_jwt_config();
    let policy = build_rbac_policy(3);
    let token = issue_jwt(&jwt, 1, "ADMIN", 3, &policy).expect("token issue");

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[jwt.issuer.as_str()]);
    validation.set_audience(&[jwt.audience.as_str()]);

    let decoded =
        decode::<JwtClaims>(&token, &jwt.decoding_key, &validation).expect("token decode");

    assert_eq!(decoded.claims.sub, 1);
    assert!(!decoded.claims.jti.is_empty());
    assert!(
        decoded
            .claims
            .permissions
            .iter()
            .any(|p| p == "players:write")
    );
}

#[test]
fn normalize_username_validation_works() {
    assert_eq!(
        normalize_username("player123").expect("valid username"),
        "PLAYER123"
    );
    assert!(normalize_username("ab").is_err());
    assert!(normalize_username("invalid-user").is_err());
}

#[test]
fn password_validation_works() {
    assert!(validate_password("StrongPass123").is_ok());
    assert!(validate_password("short").is_err());
    assert!(validate_password(&"x".repeat(65)).is_err());
}

#[test]
fn rbac_policy_scopes_by_gm_level() {
    let player = build_rbac_policy(0);
    assert!(player.roles.iter().any(|r| r == "player"));
    assert!(!player.permissions.iter().any(|p| p == "players:read"));

    let moderator = build_rbac_policy(1);
    assert!(moderator.roles.iter().any(|r| r == "moderator"));
    assert!(moderator.permissions.iter().any(|p| p == "players:read"));
    assert!(
        moderator
            .permissions
            .iter()
            .any(|p| p == "characters:read:any")
    );

    let admin = build_rbac_policy(3);
    assert!(admin.roles.iter().any(|r| r == "admin"));
    assert!(admin.permissions.iter().any(|p| p == "players:write"));
}

#[test]
fn refresh_token_generation_and_hashing_is_sound() {
    let t1 = generate_refresh_token();
    let t2 = generate_refresh_token();

    assert_eq!(t1.len(), 128);
    assert_eq!(t2.len(), 128);
    assert_ne!(t1, t2);

    let h1 = hash_refresh_token(&t1);
    let h1_repeat = hash_refresh_token(&t1);
    let h2 = hash_refresh_token(&t2);

    assert_eq!(h1.len(), 64);
    assert_eq!(h1, h1_repeat);
    assert_ne!(h1, h2);
}

#[test]
fn parse_bearer_token_rejects_invalid_headers() {
    let mut headers = axum::http::HeaderMap::new();
    assert!(extract_bearer_token(&headers).is_err());

    headers.insert(AUTHORIZATION, "Basic abc".parse().expect("header"));
    assert!(extract_bearer_token(&headers).is_err());

    headers.insert(AUTHORIZATION, "Bearer".parse().expect("header"));
    assert!(extract_bearer_token(&headers).is_err());
}

#[tokio::test]
async fn api_error_without_extra_omits_extra_field() {
    let response = ApiError::bad_request("invalid", "INVALID_INPUT").into_response();
    let body = body_json(response).await;
    assert_eq!(body["message"], "invalid");
    assert_eq!(body["error_code"], "INVALID_INPUT");
    assert!(body.get("extra").is_none());
}

#[tokio::test]
async fn api_error_with_extra_serializes_extra_field() {
    let response = ApiError::bad_request("invalid", "INVALID_INPUT")
        .with_extra(json!({ "unknown_value": "abc" }))
        .into_response();
    let body = body_json(response).await;
    assert_eq!(body["message"], "invalid");
    assert_eq!(body["error_code"], "INVALID_INPUT");
    assert_eq!(body["extra"]["unknown_value"], "abc");
}

// ---------------------------------------------------------------------------
// Router-level tests (using mocks, covering routing + auth)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_check_returns_ok() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health-check")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("health-check response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["message"], "ok");
}

#[tokio::test]
async fn not_found_returns_custom_payload() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/does-not-exist")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("not-found response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "ROUTE_NOT_FOUND");
}

#[tokio::test]
async fn protected_route_rejects_missing_token() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/items")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "MISSING_AUTH");
}

#[tokio::test]
async fn protected_route_rejects_invalid_token() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/auth/me")
                .header(AUTHORIZATION, "Bearer invalid.token.value")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "AUTH_FAILED");
}

#[tokio::test]
async fn auth_refresh_requires_payload_token() {
    let app = build_router(default_test_state());
    let payload = json!({ "refresh_token": "" }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "INVALID_REFRESH_TOKEN");
}

#[tokio::test]
async fn versioned_auth_route_exists() {
    let app = build_router(default_test_state());
    let payload = json!({
        "username": "PLAYER001",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// New handler tests: Register
// ---------------------------------------------------------------------------

#[tokio::test]
async fn register_creates_account() {
    let state = default_test_state();
    let app = build_router(state);
    let payload = json!({
        "username": "newplayer",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert!(body["account_id"].as_u64().unwrap_or(0) > 0);
    assert_eq!(body["username"], "NEWPLAYER");
}

#[tokio::test]
async fn register_duplicate_username_returns_conflict() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "EXISTING".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let app = build_router(state);
    let payload = json!({
        "username": "EXISTING",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "USERNAME_ALREADY_EXISTS");
}

#[tokio::test]
async fn register_weak_password_returns_error() {
    let state = default_test_state();
    let app = build_router(state);
    let payload = json!({
        "username": "newplayer",
        "password": "short"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_invalid_username_returns_error() {
    let state = default_test_state();
    let app = build_router(state);
    let payload = json!({
        "username": "ab",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// New handler tests: Sign-in
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sign_in_success() {
    let accounts = MockAccountRepo::new();

    let (salt, verifier) = srp6_register("PLAYER001", "StrongPassword123", false);
    accounts.add_account(SignInAccountRow {
        id: 42,
        username: "PLAYER001".to_string(),
        email: Some("player@test.com".to_string()),
        salt,
        verifier,
        locked: false,
    });
    accounts.set_gm_level(42, 3);

    let tokens = MockRefreshTokenRepo::new();
    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        tokens,
    );
    let app = build_router(state);
    let payload = json!({
        "username": "PLAYER001",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["account_id"], 42);
    assert_eq!(body["username"], "PLAYER001");
    assert_eq!(body["gm_level"], 3);
    assert!(body["access_token"].as_str().unwrap().len() > 20);
    assert!(body["refresh_token"].as_str().unwrap().len() == 128);
    assert!(body["roles"].as_array().unwrap().contains(&json!("admin")));
}

#[tokio::test]
async fn sign_in_wrong_password_returns_unauthorized() {
    let accounts = MockAccountRepo::new();

    let (salt, verifier) = srp6_register("PLAYER001", "RealPassword123", false);
    accounts.add_account(SignInAccountRow {
        id: 42,
        username: "PLAYER001".to_string(),
        email: None,
        salt,
        verifier,
        locked: false,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let app = build_router(state);
    let payload = json!({
        "username": "PLAYER001",
        "password": "WrongPassword"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn sign_in_locked_account_returns_unauthorized() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 42,
        username: "LOCKED".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: true,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let app = build_router(state);
    let payload = json!({
        "username": "LOCKED",
        "password": "AnyPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "ACCOUNT_LOCKED");
}

#[tokio::test]
async fn sign_in_nonexistent_account_returns_unauthorized() {
    let state = default_test_state();
    let app = build_router(state);
    let payload = json!({
        "username": "GHOST",
        "password": "StrongPassword123"
    })
    .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "INVALID_CREDENTIALS");
}

// ---------------------------------------------------------------------------
// New handler tests: Auth Me
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_me_returns_profile() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: Some("a@b.com".to_string()),
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 1);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let jwt = test_jwt_config();
    let token = issue_test_token(&jwt, 1, "PLAYER001", 1);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/auth/me")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["account_id"], 1);
    assert_eq!(body["username"], "PLAYER001");
    assert_eq!(body["gm_level"], 1);
}

// ---------------------------------------------------------------------------
// New handler tests: Characters
// ---------------------------------------------------------------------------

#[tokio::test]
async fn characters_returns_list() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let chars = MockCharacterRepo::new();
    chars.add_character(1, 100, "CharA", None);
    chars.add_character(1, 101, "CharB", None);

    let state = test_state(
        accounts,
        chars,
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let jwt = test_jwt_config();
    let token = issue_test_token(&jwt, 1, "PLAYER001", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/characters")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["account_id"], 1);
    assert_eq!(body["characters"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn characters_empty_list() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let jwt = test_jwt_config();
    let token = issue_test_token(&jwt, 1, "PLAYER001", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/characters")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert!(body["characters"].as_array().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// New handler tests: Character Location
// ---------------------------------------------------------------------------

#[tokio::test]
async fn character_location_own() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let chars = MockCharacterRepo::new();
    chars.add_character(
        1,
        100,
        "CharA",
        Some(CharacterLocationResponse {
            guid: 100,
            name: "CharA".to_string(),
            map: 1,
            zone: 100,
            position_x: 1.0,
            position_y: 2.0,
            position_z: 3.0,
            orientation: 0.0,
            online: true,
        }),
    );

    let state = test_state(
        accounts,
        chars,
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER001", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/characters/100/location")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["guid"], 100);
    assert_eq!(body["name"], "CharA");
}

#[tokio::test]
async fn character_location_not_found() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER001", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/characters/999/location")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// New handler tests: Items
// ---------------------------------------------------------------------------

#[tokio::test]
async fn items_search_returns_results() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);

    let items = MockItemRepo::new();
    items.add_item(ItemSummary {
        entry: 100,
        name: "Sword of Testing".to_string(),
        quality: 4,
        item_level: 200,
        class: 2,
        subclass: 8,
        display_id: 1234,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        items,
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "ADMIN", 3);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/items?search=Sword")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn items_cursor_pagination_returns_next_page() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);

    let items = MockItemRepo::new();
    for entry in [100u32, 101, 102, 103] {
        items.add_item(ItemSummary {
            entry,
            name: format!("Item {entry}"),
            quality: 1,
            item_level: 10,
            class: 0,
            subclass: 0,
            display_id: 1,
        });
    }

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        items,
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "ADMIN", 3);
    let app = build_router(state);

    let page1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/items?limit=2")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");
    assert_eq!(page1.status(), StatusCode::OK);
    let body1 = body_json(page1).await;
    assert_eq!(body1["items"].as_array().unwrap().len(), 2);
    assert_eq!(body1["items"][0]["entry"], 100);
    assert_eq!(body1["items"][1]["entry"], 101);
    let cursor = body1["cursor"].as_u64().unwrap();
    assert_eq!(cursor, 101);

    let page2 = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/items?limit=2&cursor={cursor}"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");
    assert_eq!(page2.status(), StatusCode::OK);
    let body2 = body_json(page2).await;
    assert_eq!(body2["items"].as_array().unwrap().len(), 2);
    assert_eq!(body2["items"][0]["entry"], 102);
    assert_eq!(body2["items"][1]["entry"], 103);
    assert!(body2["cursor"].is_null(), "no more pages");
}

#[tokio::test]
async fn item_by_entry_found() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);

    let items = MockItemRepo::new();
    items.add_item(ItemSummary {
        entry: 100,
        name: "Sword of Testing".to_string(),
        quality: 4,
        item_level: 200,
        class: 2,
        subclass: 8,
        display_id: 1234,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        items,
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "ADMIN", 3);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/items/100")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["entry"], 100);
    assert_eq!(body["name"], "Sword of Testing");
}

#[tokio::test]
async fn item_by_entry_not_found() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "ADMIN", 3);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/items/999")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "ITEM_NOT_FOUND");
}

// ---------------------------------------------------------------------------
// New handler tests: Admin endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_lock_player() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);
    accounts.add_account(SignInAccountRow {
        id: 2,
        username: "TARGET".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "ADMIN", 3);
    let app = build_router(state);
    let payload = json!({ "locked": true }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/admin/players/2/lock")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["account_id"], 2);
    assert_eq!(body["locked"], true);
}

#[tokio::test]
async fn admin_lock_player_forbidden_without_permission() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 0);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER", 0);
    let app = build_router(state);
    let payload = json!({ "locked": true }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/admin/players/2/lock")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// New handler tests: Refresh token flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn refresh_token_valid_rotation() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 0);

    let tokens = MockRefreshTokenRepo::new();
    let old_token = "valid_refresh_token_1234567890abcdef";
    tokens.add_valid_token(old_token, 1);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        tokens,
    );
    let app = build_router(state);
    let payload = json!({ "refresh_token": old_token }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert!(body["access_token"].as_str().unwrap().len() > 20);
    assert_eq!(body["refresh_token"].as_str().unwrap().len(), 128);
    assert!(body["expires_in_seconds"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn refresh_token_revoked_returns_unauthorized() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let tokens = MockRefreshTokenRepo::new();
    let bad_token = "revoked_token_value";
    let _hash = tokens.add_valid_token(bad_token, 1);
    // Revoke the token
    tokens
        .revoke_for_account(1, bad_token, "test")
        .await
        .unwrap();

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        tokens,
    );
    let app = build_router(state);
    let payload = json!({ "refresh_token": bad_token }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// New handler tests: Logout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn logout_revokes_access_token() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER001".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER001", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/logout")
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({}).to_string()))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn logout_requires_auth_header() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/logout")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "MISSING_AUTH");
}

// ---------------------------------------------------------------------------
// New handler tests: Admin online players
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_online_players_requires_permission() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 0);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/admin/online-players")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// New handler tests: Admin players
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_players_forbidden_for_regular_player() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 0);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER", 0);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/admin/players")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_players_cursor_pagination_returns_next_page() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "PLAYER1".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.add_account(SignInAccountRow {
        id: 2,
        username: "PLAYER2".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.add_account(SignInAccountRow {
        id: 3,
        username: "PLAYER3".to_string(),
        email: None,
        salt: vec![0u8; 32],
        verifier: vec![0u8; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);

    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let token = issue_test_token(&test_jwt_config(), 1, "PLAYER1", 3);
    let app = build_router(state);

    let page1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/admin/players?limit=2")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");
    assert_eq!(page1.status(), StatusCode::OK);
    let body1 = body_json(page1).await;
    assert_eq!(body1["players"].as_array().unwrap().len(), 2);
    assert_eq!(body1["players"][0]["id"], 1);
    assert_eq!(body1["players"][1]["id"], 2);
    let cursor = body1["cursor"].as_u64().unwrap();
    assert_eq!(cursor, 2);

    let page2 = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/admin/players?limit=2&cursor={cursor}"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");
    assert_eq!(page2.status(), StatusCode::OK);
    let body2 = body_json(page2).await;
    assert_eq!(body2["players"].as_array().unwrap().len(), 1);
    assert_eq!(body2["players"][0]["id"], 3);
    assert!(body2["cursor"].is_null(), "no more pages");
}

// ---------------------------------------------------------------------------
// Tests for read_client_ip and read_user_agent
// ---------------------------------------------------------------------------

#[test]
fn read_client_ip_from_x_forwarded_for() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-forwarded-for", "192.168.1.1".parse().unwrap());
    assert_eq!(read_client_ip(&headers), "192.168.1.1");
}

#[test]
fn read_client_ip_falls_back_to_x_real_ip() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
    assert_eq!(read_client_ip(&headers), "10.0.0.1");
}

#[test]
fn read_client_ip_defaults_when_missing() {
    let headers = axum::http::HeaderMap::new();
    assert_eq!(read_client_ip(&headers), "0.0.0.0");
}

#[test]
fn read_user_agent_returns_none_when_missing() {
    let headers = axum::http::HeaderMap::new();
    assert!(read_user_agent(&headers).is_none());
}

#[test]
fn read_user_agent_returns_value() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("user-agent", "test-agent".parse().unwrap());
    assert_eq!(read_user_agent(&headers).as_deref(), Some("test-agent"));
}

// ---------------------------------------------------------------------------
// Integration tests with testcontainers (require Docker)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn integration_refresh_token_store_find_revoke() {
    use testcontainers::runners::AsyncRunner;

    let pg_image = testcontainers_modules::postgres::Postgres::default();
    let container: testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres> =
        match pg_image.start().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("testcontainers skipped (Docker unavailable?): {e}");
                return;
            }
        };

    let host = container.get_host().await.expect("container host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");
    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("pg pool");

    crate::run_app_migrations(&pool).await.expect("migrations");

    let jwt = test_jwt_config();
    let state = AppState {
        config: AppConfig {
            auth_db: String::new(),
            characters_db: String::new(),
            world_db: String::new(),
            srp6_core5_mode: false,
            rate_limit: false,
            debug_enabled: false,
        },
        jwt: jwt.clone(),
        accounts: Arc::new(MockAccountRepo::new()),
        characters: Arc::new(MockCharacterRepo::new()),
        items: Arc::new(MockItemRepo::new()),
        refresh_tokens: Arc::new(LiveRefreshTokenRepo::new(
            pool,
            jwt.refresh_expires_days,
            jwt.expires_minutes,
        )),
        service_tokens: Arc::new(MockServiceTokenRepo::new()),
        started_at: 0,
    };

    let refresh_token = generate_refresh_token();
    let token_id = state
        .refresh_tokens
        .store(
            999001,
            &refresh_token,
            None,
            None,
            "127.0.0.1",
            Some("integration-test"),
        )
        .await
        .expect("store");

    assert!(token_id > 0, "token id should be positive");

    let hash = hash_refresh_token(&refresh_token);
    let found = state
        .refresh_tokens
        .find_by_hash(&hash)
        .await
        .expect("find")
        .expect("token should exist");

    assert!(!found.is_revoked, "fresh token should not be revoked");
    assert!(!found.is_expired, "fresh token should not be expired");
    assert_eq!(found.account_id, 999001);

    state
        .refresh_tokens
        .revoke_for_account(999001, &refresh_token, "integration")
        .await
        .expect("revoke");

    let after = state
        .refresh_tokens
        .find_by_hash(&hash)
        .await
        .expect("find after")
        .expect("token should exist after revoke");

    assert!(after.is_revoked, "token should be revoked");

    drop(container);
}

#[tokio::test]
async fn integration_access_token_revocation_roundtrip() {
    use testcontainers::runners::AsyncRunner;

    let pg_image = testcontainers_modules::postgres::Postgres::default();
    let container: testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres> =
        match pg_image.start().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("testcontainers skipped (Docker unavailable?): {e}");
                return;
            }
        };

    let host = container.get_host().await.expect("container host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");
    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("pg pool");

    crate::run_app_migrations(&pool).await.expect("migrations");

    let jwt = test_jwt_config();
    let state = AppState {
        config: AppConfig {
            auth_db: String::new(),
            characters_db: String::new(),
            world_db: String::new(),
            srp6_core5_mode: false,
            rate_limit: false,
            debug_enabled: false,
        },
        jwt: jwt.clone(),
        accounts: Arc::new(MockAccountRepo::new()),
        characters: Arc::new(MockCharacterRepo::new()),
        items: Arc::new(MockItemRepo::new()),
        refresh_tokens: Arc::new(LiveRefreshTokenRepo::new(
            pool,
            jwt.refresh_expires_days,
            jwt.expires_minutes,
        )),
        service_tokens: Arc::new(MockServiceTokenRepo::new()),
        started_at: 0,
    };

    let jti = uuid::Uuid::new_v4().to_string();

    let not_revoked = state
        .refresh_tokens
        .is_access_revoked(&jti)
        .await
        .expect("check before");
    assert!(!not_revoked, "fresh jti should not be revoked");

    state
        .refresh_tokens
        .revoke_access(999002, &jti, "integration", 15)
        .await
        .expect("revoke");

    let now_revoked = state
        .refresh_tokens
        .is_access_revoked(&jti)
        .await
        .expect("check after");
    assert!(now_revoked, "jti should be revoked after revoke_access");

    drop(container);
}

// ---------------------------------------------------------------------------
// Mock ServiceTokenRepo
// ---------------------------------------------------------------------------

struct MockServiceTokenRepo {
    next_id: Mutex<i64>,
    tokens: Mutex<HashMap<i64, MockServiceToken>>,
}

struct MockServiceToken {
    hash: String,
    name: String,
    created_by: i64,
    expires_at_unix: Option<i64>,
    revoked: bool,
}

impl MockServiceTokenRepo {
    fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            tokens: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ServiceTokenRepo for MockServiceTokenRepo {
    async fn create(
        &self,
        token_hash: &str,
        name: &str,
        created_by: i64,
        expires_at_unix: Option<i64>,
    ) -> Result<i64, ApiError> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        self.tokens.lock().unwrap().insert(
            id,
            MockServiceToken {
                hash: token_hash.to_string(),
                name: name.to_string(),
                created_by,
                expires_at_unix,
                revoked: false,
            },
        );
        Ok(id)
    }

    async fn list(&self) -> Result<Vec<ServiceTokenRow>, ApiError> {
        let tokens = self.tokens.lock().unwrap();
        let mut rows: Vec<ServiceTokenRow> = tokens
            .iter()
            .map(|(id, t)| ServiceTokenRow {
                id: *id,
                name: t.name.clone(),
                created_by: t.created_by,
                created_at_unix: 0,
                expires_at_unix: t.expires_at_unix,
                revoked_at_unix: if t.revoked { Some(1) } else { None },
                last_used_at_unix: None,
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.id));
        Ok(rows)
    }

    async fn find_active_by_hash(&self, token_hash: &str) -> Result<Option<(i64, i64)>, ApiError> {
        let tokens = self.tokens.lock().unwrap();
        Ok(tokens
            .iter()
            .find(|(_, t)| t.hash == token_hash && !t.revoked)
            .map(|(id, t)| (*id, t.created_by)))
    }

    async fn revoke(&self, token_id: i64) -> Result<bool, ApiError> {
        let mut tokens = self.tokens.lock().unwrap();
        match tokens.get_mut(&token_id) {
            Some(t) if !t.revoked => {
                t.revoked = true;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn touch_last_used(&self, _token_id: i64) -> Result<(), ApiError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Service token endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_service_token_requires_admin() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 2,
        username: "PLAYER2".to_string(),
        email: None,
        salt: vec![0; 32],
        verifier: vec![0; 32],
        locked: false,
    });
    accounts.set_gm_level(2, 0);
    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let jwt = issue_test_token(&state.jwt, 2, "PLAYER2", 0);
    let app = build_router(state);
    let payload = json!({ "name": "mcp" }).to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/admin/service-tokens")
                .header(AUTHORIZATION, format!("Bearer {jwt}"))
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn service_token_full_lifecycle() {
    let accounts = MockAccountRepo::new();
    accounts.add_account(SignInAccountRow {
        id: 1,
        username: "ADMIN".to_string(),
        email: None,
        salt: vec![0; 32],
        verifier: vec![0; 32],
        locked: false,
    });
    accounts.set_gm_level(1, 3);
    let state = test_state(
        accounts,
        MockCharacterRepo::new(),
        MockItemRepo::new(),
        MockRefreshTokenRepo::new(),
    );
    let admin_jwt = issue_test_token(&state.jwt, 1, "ADMIN", 3);

    let app = build_router(state);

    let payload = json!({ "name": "mcp-local", "expires_in_days": 30 }).to_string();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/admin/service-tokens")
                .header(AUTHORIZATION, format!("Bearer {admin_jwt}"))
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request builds"),
        )
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let token = body["token"]
        .as_str()
        .expect("token in response")
        .to_string();
    assert!(token.starts_with("wowst_"));
    let token_id = body["id"].as_i64().expect("id in response");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/admin/players")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("authed response");

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/admin/service-tokens/{token_id}"))
                .header(AUTHORIZATION, format!("Bearer {admin_jwt}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("revoke response");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/admin/players")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("revoked token response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "INVALID_SERVICE_TOKEN");
}

#[tokio::test]
async fn unknown_service_token_is_rejected() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/admin/online-players")
                .header(AUTHORIZATION, "Bearer wowst_deadbeef")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(response).await;
    assert_eq!(body["error_code"], "INVALID_SERVICE_TOKEN");
}

#[tokio::test]
async fn metrics_router_serves_prometheus_on_dedicated_router() {
    let app = build_metrics_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("metrics response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn main_router_no_longer_serves_metrics() {
    let app = build_router(default_test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
