use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::HeaderMap;
use jsonwebtoken::{Algorithm, Header, encode, errors::ErrorKind};
use num_bigint::{BigInt, Sign};
use once_cell::sync::Lazy;
use rand::RngCore;
use serde_json::json;
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::error::*;
use crate::models::*;

// ---------------------------------------------------------------------------
// SRP6
// ---------------------------------------------------------------------------

static SRP6_N: Lazy<BigInt> = Lazy::new(|| {
    BigInt::from_bytes_be(
        Sign::Plus,
        &[
            0x89, 0x4B, 0x64, 0x5E, 0x89, 0xE1, 0x53, 0x5B, 0xBD, 0xAD, 0x5B, 0x8B, 0x29, 0x06,
            0x50, 0x53, 0x08, 0x01, 0xB1, 0x8E, 0xBF, 0xBF, 0x5E, 0x8F, 0xAB, 0x3C, 0x82, 0x87,
            0x2A, 0x3E, 0x9B, 0xB7,
        ],
    )
});

static SRP6_G: Lazy<BigInt> = Lazy::new(|| BigInt::from(7u8));

fn sha1_bytes(data: &[u8]) -> Vec<u8> {
    let mut h = Sha1::new();
    h.update(data);
    h.finalize().to_vec()
}

fn srp6_calculate_verifier(username: &str, password: &str, salt: &[u8], core5: bool) -> Vec<u8> {
    let up = format!("{}:{}", username.to_uppercase(), password.to_uppercase());
    let h1 = sha1_bytes(up.as_bytes());

    let mut h2_input = Vec::new();
    if core5 {
        h2_input.extend(salt.iter().rev());
    } else {
        h2_input.extend(salt);
    }
    h2_input.extend(h1);

    let h2 = sha1_bytes(&h2_input);
    let x = BigInt::from_bytes_le(Sign::Plus, &h2);

    let v = SRP6_G.modpow(&x, &SRP6_N);
    let mut out = v.to_bytes_le().1;
    out.resize(32, 0);

    if core5 {
        out.reverse();
    }

    out
}

pub fn srp6_register(username: &str, password: &str, core5: bool) -> (Vec<u8>, Vec<u8>) {
    let mut salt = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);

    let verifier = srp6_calculate_verifier(username, password, &salt, core5);
    (salt, verifier)
}

pub fn srp6_verify(
    username: &str,
    password: &str,
    salt: &[u8],
    verifier: &[u8],
    core5: bool,
) -> bool {
    let expected = srp6_calculate_verifier(username, password, salt, core5);
    constant_time_eq(&expected, verifier)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// JWT
// ---------------------------------------------------------------------------

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix time should be valid")
        .as_secs()
}

pub fn issue_jwt(
    jwt: &JwtConfig,
    account_id: u64,
    username: &str,
    gm_level: u8,
    policy: &RbacPolicy,
) -> Result<String, ApiError> {
    let now = unix_now();
    let claims = JwtClaims {
        sub: account_id,
        jti: Uuid::new_v4().to_string(),
        username: username.to_string(),
        roles: policy.roles.clone(),
        permissions: policy.permissions.clone(),
        gm_level,
        iss: jwt.issuer.clone(),
        aud: jwt.audience.clone(),
        iat: now,
        nbf: now,
        exp: now + jwt.expires_minutes * 60,
    };

    encode(&Header::new(Algorithm::RS256), &claims, &jwt.encoding_key)
        .map_err(|_| ApiError::internal("Failed to issue token", "TOKEN_ISSUE_FAILED"))
}

pub fn map_jwt_error_to_api(err: jsonwebtoken::errors::Error) -> ApiError {
    match err.kind() {
        ErrorKind::ExpiredSignature => ApiError::unauthorized("Token expired", "TOKEN_EXPIRED"),
        ErrorKind::InvalidToken
        | ErrorKind::InvalidIssuer
        | ErrorKind::InvalidAudience
        | ErrorKind::InvalidSignature
        | ErrorKind::ImmatureSignature => ApiError::unauthorized("Invalid token", "INVALID_TOKEN"),
        _ => ApiError::unauthorized("Authentication failed", "AUTH_FAILED").with_extra(json!({
            "jwt_error_kind": format!("{:?}", err.kind())
        })),
    }
}

// ---------------------------------------------------------------------------
// RBAC
// ---------------------------------------------------------------------------

pub fn build_rbac_policy(gm_level: u8) -> RbacPolicy {
    let mut roles = vec!["player".to_string()];
    let mut permissions: HashSet<String> = [
        "profile:read:self",
        "characters:read:self",
        "characters:location:self",
        "items:read",
    ]
    .iter()
    .map(|v| (*v).to_string())
    .collect();

    if gm_level >= 1 {
        roles.push("moderator".to_string());
        permissions.insert("players:read".to_string());
        permissions.insert("characters:read:any".to_string());
        permissions.insert("characters:location:any".to_string());
    }

    if gm_level >= 3 {
        roles.push("admin".to_string());
        permissions.insert("players:write".to_string());
    }

    RbacPolicy {
        roles,
        permissions: permissions.into_iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// Refresh token
// ---------------------------------------------------------------------------

pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    to_hex(&bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    to_hex(&hasher.finalize())
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX_CHARS[(b >> 4) as usize] as char);
        out.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    out
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let value = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing Authorization header", "MISSING_AUTH"))?;

    let (scheme, token) = value
        .split_once(' ')
        .ok_or_else(|| ApiError::unauthorized("Invalid Authorization header", "INVALID_AUTH"))?;

    if scheme != "Bearer" || token.trim().is_empty() {
        return Err(ApiError::unauthorized(
            "Invalid bearer token",
            "INVALID_AUTH_TOKEN",
        ));
    }

    Ok(token.trim().to_string())
}

pub fn read_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "0.0.0.0".to_string())
}

pub fn read_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(255).collect())
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

pub fn normalize_username(username: &str) -> Result<String, ApiError> {
    let normalized = username.trim().to_uppercase();

    if normalized.len() < 3 || normalized.len() > 32 {
        return Err(ApiError::bad_request(
            "Username must be between 3 and 32 characters",
            "INVALID_USERNAME_LENGTH",
        ));
    }

    if !normalized
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err(ApiError::bad_request(
            "Username must contain only letters and numbers",
            "INVALID_USERNAME_CHARACTERS",
        ));
    }

    Ok(normalized)
}

pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 || password.len() > 64 {
        return Err(ApiError::bad_request(
            "Password must be between 8 and 64 characters",
            "INVALID_PASSWORD_LENGTH",
        ));
    }

    Ok(())
}
