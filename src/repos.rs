use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use sqlx::{MySql, MySqlPool, PgPool, QueryBuilder, Row, mysql::MySqlRow};
use tracing::error;
use uuid::Uuid;

use crate::auth::hash_refresh_token;
use crate::error::*;
use crate::models::*;

pub const APP_PG_SCHEMA: &str = "public";

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

pub fn map_db_error(context: &'static str, err: sqlx::Error) -> ApiError {
    error!(error = %err, "{context}");
    ApiError::internal("Database operation failed", "DB_OPERATION_FAILED").with_extra(json!({
        "context": context
    }))
}

pub fn row_get<T>(row: &MySqlRow, column: &str) -> Result<T, ApiError>
where
    T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
    row.try_get(column)
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))
}

pub fn row_get_opt<T>(row: &MySqlRow, column: &str) -> Result<Option<T>, ApiError>
where
    T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
    row.try_get(column)
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))
}

pub fn row_get_bool(row: &MySqlRow, column: &str) -> Result<bool, ApiError> {
    if let Ok(as_bool) = row.try_get::<bool, _>(column) {
        return Ok(as_bool);
    }
    if let Ok(as_u8) = row.try_get::<u8, _>(column) {
        return Ok(as_u8 != 0);
    }
    Err(ApiError::internal(
        "Invalid database row format",
        "ROW_DECODE_FAILED",
    ))
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// TTL cache for `find_auth` results (account_id -> username + locked flag).
/// Eliminates the per-request MySQL roundtrip in `load_auth_context`.
struct AuthCache {
    entries: Mutex<HashMap<u64, (Instant, AuthAccountRow)>>,
    ttl: Duration,
}

impl Default for AuthCache {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(30),
        }
    }
}

impl AuthCache {
    fn get(&self, account_id: u64) -> Option<AuthAccountRow> {
        let entries = self.entries.lock().unwrap();
        entries.get(&account_id).and_then(|(ts, row)| {
            if ts.elapsed() > self.ttl {
                None
            } else {
                Some(row.clone())
            }
        })
    }

    fn put(&self, account_id: u64, row: AuthAccountRow) {
        self.entries
            .lock()
            .unwrap()
            .insert(account_id, (Instant::now(), row));
    }

    fn invalidate(&self, account_id: u64) {
        self.entries.lock().unwrap().remove(&account_id);
    }
}

/// In-memory per-account login lockout (fixed window).
/// Tracks recent failed sign-in attempts to block brute force per account.
struct LoginLockout {
    attempts: Mutex<HashMap<u64, (Instant, u32)>>,
    window: Duration,
    max_attempts: u32,
}

impl Default for LoginLockout {
    fn default() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            window: Duration::from_secs(300),
            max_attempts: 5,
        }
    }
}

impl LoginLockout {
    fn is_locked(&self, account_id: u64) -> bool {
        let attempts = self.attempts.lock().unwrap();
        attempts
            .get(&account_id)
            .map(|(ts, n)| ts.elapsed() <= self.window && *n >= self.max_attempts)
            .unwrap_or(false)
    }

    fn record_failure(&self, account_id: u64) {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();
        let (ts, n) = attempts
            .get(&account_id)
            .map(|(ts, n)| (*ts, *n))
            .unwrap_or((now, 0));
        let n = if ts.elapsed() > self.window { 1 } else { n + 1 };
        attempts.insert(account_id, (now, n));
    }

    fn reset(&self, account_id: u64) {
        self.attempts.lock().unwrap().remove(&account_id);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub jwt: JwtConfig,
    pub accounts: Arc<dyn AccountRepo>,
    pub characters: Arc<dyn CharacterRepo>,
    pub items: Arc<dyn ItemRepo>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepo>,
    pub service_tokens: Arc<dyn ServiceTokenRepo>,
    pub audit: Arc<dyn AuditRepo>,
    pub soap: Option<Arc<dyn crate::soap::SoapClient>>,
    pub started_at: u64,
}

// ---------------------------------------------------------------------------
// AccountRepo
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AccountRepo: Send + Sync {
    async fn exists(&self, username: &str) -> Result<bool, ApiError>;
    async fn create(
        &self,
        username: &str,
        salt: &[u8],
        verifier: &[u8],
        email: Option<&str>,
        ip: &str,
    ) -> Result<u64, ApiError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<SignInAccountRow>, ApiError>;
    async fn increment_failed_logins(&self, account_id: u64) -> Result<(), ApiError>;
    async fn record_successful_login(&self, account_id: u64, ip: &str) -> Result<(), ApiError>;
    async fn get_gm_level(&self, account_id: u64) -> u8;
    async fn find_auth(&self, account_id: u64) -> Result<Option<AuthAccountRow>, ApiError>;
    async fn find_username(&self, account_id: u64) -> Result<Option<String>, ApiError>;
    async fn update_lock(&self, account_id: u64, locked: bool) -> Result<bool, ApiError>;
    fn login_is_locked(&self, account_id: u64) -> bool;
    fn record_login_failure(&self, account_id: u64);
    fn reset_login_lockout(&self, account_id: u64);
    async fn search_players(
        &self,
        search: Option<&str>,
        online: Option<bool>,
        limit: u32,
        cursor: Option<u64>,
    ) -> Result<(Vec<AdminPlayerSummary>, Option<u64>), ApiError>;
    async fn find_online_players(&self) -> Result<Vec<OnlinePlayerSummary>, ApiError>;
}

pub struct LiveAccountRepo {
    pool: MySqlPool,
    auth_db: String,
    characters_db: String,
    auth_cache: AuthCache,
    login_lockout: LoginLockout,
}

impl LiveAccountRepo {
    pub fn new(pool: MySqlPool, auth_db: String, characters_db: String) -> Self {
        Self {
            pool,
            auth_db,
            characters_db,
            auth_cache: AuthCache::default(),
            login_lockout: LoginLockout::default(),
        }
    }
}

#[async_trait]
impl AccountRepo for LiveAccountRepo {
    async fn exists(&self, username: &str) -> Result<bool, ApiError> {
        let sql = format!(
            "SELECT id FROM {}.account WHERE username = ? LIMIT 1",
            self.auth_db
        );
        let row = sqlx::query_scalar::<_, u64>(&sql)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to check existing account", err))?;
        Ok(row.is_some())
    }

    async fn create(
        &self,
        username: &str,
        salt: &[u8],
        verifier: &[u8],
        email: Option<&str>,
        ip: &str,
    ) -> Result<u64, ApiError> {
        let sql = format!(
            "INSERT INTO {}.account (username, salt, verifier, email, reg_mail, last_ip) VALUES (?, ?, ?, ?, ?, ?)",
            self.auth_db
        );
        let result = sqlx::query(&sql)
            .bind(username)
            .bind(salt)
            .bind(verifier)
            .bind(email.unwrap_or(""))
            .bind(email.unwrap_or(""))
            .bind(ip)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to create account", err))?;
        Ok(result.last_insert_id())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<SignInAccountRow>, ApiError> {
        let sql = format!(
            "SELECT id, username, email, salt, verifier, locked FROM {}.account WHERE username = ? LIMIT 1",
            self.auth_db
        );
        let row = sqlx::query(&sql)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to fetch account", err))?;
        match row {
            Some(row) => Ok(Some(SignInAccountRow {
                id: row_get(&row, "id")?,
                username: row_get(&row, "username")?,
                email: row_get_opt(&row, "email")?,
                salt: row_get(&row, "salt")?,
                verifier: row_get(&row, "verifier")?,
                locked: row_get_bool(&row, "locked")?,
            })),
            None => Ok(None),
        }
    }

    async fn increment_failed_logins(&self, account_id: u64) -> Result<(), ApiError> {
        let sql = format!(
            "UPDATE {}.account SET failed_logins = failed_logins + 1 WHERE id = ?",
            self.auth_db
        );
        sqlx::query(&sql)
            .bind(account_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to increment failed logins", err))?;
        Ok(())
    }

    async fn record_successful_login(&self, account_id: u64, ip: &str) -> Result<(), ApiError> {
        let sql = format!(
            "UPDATE {}.account SET failed_logins = 0, last_login = NOW(), last_ip = ? WHERE id = ?",
            self.auth_db
        );
        sqlx::query(&sql)
            .bind(ip)
            .bind(account_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to update account sign-in metadata", err))?;
        Ok(())
    }

    async fn get_gm_level(&self, account_id: u64) -> u8 {
        let sql = format!(
            "SELECT COALESCE(MAX(gmlevel), 0) AS gmlevel FROM {}.account_access WHERE id = ?",
            self.auth_db
        );
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(account_id as i32)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as u8
    }

    async fn find_auth(&self, account_id: u64) -> Result<Option<AuthAccountRow>, ApiError> {
        if let Some(row) = self.auth_cache.get(account_id) {
            return Ok(Some(row));
        }
        let sql = format!(
            "SELECT username, locked FROM {}.account WHERE id = ? LIMIT 1",
            self.auth_db
        );
        let row = sqlx::query(&sql)
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load auth account", err))?;
        match row {
            Some(row) => {
                let auth_row = AuthAccountRow {
                    username: row_get(&row, "username")?,
                    locked: row_get_bool(&row, "locked")?,
                };
                self.auth_cache.put(account_id, auth_row.clone());
                Ok(Some(auth_row))
            }
            None => Ok(None),
        }
    }

    fn login_is_locked(&self, account_id: u64) -> bool {
        self.login_lockout.is_locked(account_id)
    }

    fn record_login_failure(&self, account_id: u64) {
        self.login_lockout.record_failure(account_id);
    }

    fn reset_login_lockout(&self, account_id: u64) {
        self.login_lockout.reset(account_id);
        self.auth_cache.invalidate(account_id);
    }

    async fn find_username(&self, account_id: u64) -> Result<Option<String>, ApiError> {
        let sql = format!(
            "SELECT username FROM {}.account WHERE id = ? LIMIT 1",
            self.auth_db
        );
        sqlx::query_scalar::<_, String>(&sql)
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load account", err))
    }

    async fn update_lock(&self, account_id: u64, locked: bool) -> Result<bool, ApiError> {
        let sql = format!(
            "UPDATE {}.account SET locked = ? WHERE id = ?",
            self.auth_db
        );
        let result = sqlx::query(&sql)
            .bind(locked)
            .bind(account_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to update account lock", err))?;
        self.auth_cache.invalidate(account_id);
        Ok(result.rows_affected() > 0)
    }

    async fn search_players(
        &self,
        search: Option<&str>,
        online: Option<bool>,
        limit: u32,
        cursor: Option<u64>,
    ) -> Result<(Vec<AdminPlayerSummary>, Option<u64>), ApiError> {
        let base_sql = format!(
            "SELECT \
                a.id, \
                a.username, \
                a.email, \
                UNIX_TIMESTAMP(a.joindate) AS joined_unix, \
                UNIX_TIMESTAMP(a.last_login) AS last_login_unix, \
                a.last_ip, \
                a.locked, \
                COALESCE(acc.gmlevel, 0) AS gm_level, \
                COALESCE(chars.character_count, 0) AS character_count, \
                COALESCE(online_chars.online_count, 0) AS online_count \
             FROM {}.account a \
             LEFT JOIN (SELECT id, MAX(gmlevel) AS gmlevel FROM {}.account_access GROUP BY id) acc ON acc.id = a.id \
             LEFT JOIN (SELECT account, COUNT(*) AS character_count FROM {}.characters WHERE deleteDate IS NULL OR deleteDate = 0 GROUP BY account) chars ON chars.account = a.id \
             LEFT JOIN (SELECT account, COUNT(*) AS online_count FROM {}.characters WHERE online = 1 GROUP BY account) online_chars ON online_chars.account = a.id \
             WHERE 1=1",
            self.auth_db, self.auth_db, self.characters_db, self.characters_db
        );

        let mut qb = QueryBuilder::<MySql>::new(base_sql);

        if let Some(search) = search.filter(|s| !s.trim().is_empty()) {
            let like = format!("%{}%", search.trim());
            qb.push(" AND a.username LIKE ").push_bind(like);
        }

        if let Some(c) = cursor {
            qb.push(" AND a.id > ").push_bind(c);
        }

        if let Some(online) = online {
            if online {
                qb.push(" AND online_chars.online_count > 0");
            } else {
                qb.push(
                    " AND (online_chars.online_count IS NULL OR online_chars.online_count = 0)",
                );
            }
        }

        qb.push(" ORDER BY a.id ASC LIMIT ").push_bind(limit + 1);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load admin players", err))?;

        let mut players = Vec::with_capacity(rows.len());
        let mut next_cursor: Option<u64> = None;
        if rows.len() > limit as usize {
            next_cursor = Some(row_get::<u64>(&rows[limit as usize - 1], "id")?);
        }
        for row in rows.iter().take(limit as usize) {
            players.push(AdminPlayerSummary {
                id: row_get::<u64>(row, "id")?,
                username: row_get::<String>(row, "username")?,
                email: row_get_opt::<String>(row, "email")?,
                joined_unix: row_get_opt::<i64>(row, "joined_unix")?.map(|v| v as u64),
                last_login_unix: row_get_opt::<i64>(row, "last_login_unix")?.map(|v| v as u64),
                last_ip: row_get_opt::<String>(row, "last_ip")?,
                locked: row_get_bool(row, "locked")?,
                online: row_get::<i64>(row, "online_count")? > 0,
                gm_level: row_get::<i64>(row, "gm_level")? as u8,
                character_count: row_get::<i64>(row, "character_count")? as u32,
            });
        }

        Ok((players, next_cursor))
    }

    async fn find_online_players(&self) -> Result<Vec<OnlinePlayerSummary>, ApiError> {
        let sql = format!(
            "SELECT a.id AS account_id, a.username, c.guid, c.name, c.level, c.map, c.zone \
             FROM {}.characters c \
             INNER JOIN {}.account a ON a.id = c.account \
             WHERE c.online = 1 ORDER BY c.name ASC",
            self.characters_db, self.auth_db
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load online players", err))?;

        let mut players = Vec::with_capacity(rows.len());
        for row in rows {
            players.push(OnlinePlayerSummary {
                account_id: row_get::<u64>(&row, "account_id")?,
                username: row_get::<String>(&row, "username")?,
                guid: row_get::<u64>(&row, "guid")?,
                name: row_get::<String>(&row, "name")?,
                level: row_get::<u8>(&row, "level")?,
                map: row_get::<u16>(&row, "map")?,
                zone: row_get::<u32>(&row, "zone")?,
            });
        }

        Ok(players)
    }
}

// ---------------------------------------------------------------------------
// CharacterRepo
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CharacterRepo: Send + Sync {
    async fn list_for_account(&self, account_id: u64) -> Result<Vec<CharacterSummary>, ApiError>;
    async fn find_owner(&self, guid: u64) -> Result<Option<u64>, ApiError>;
    async fn find_location(&self, guid: u64)
    -> Result<Option<CharacterLocationResponse>, ApiError>;
    async fn find_locations_by_account(
        &self,
        account_id: u64,
    ) -> Result<Vec<CharacterLocationResponse>, ApiError>;
}

pub struct LiveCharacterRepo {
    pool: MySqlPool,
    characters_db: String,
}

impl LiveCharacterRepo {
    pub fn new(pool: MySqlPool, characters_db: String) -> Self {
        Self {
            pool,
            characters_db,
        }
    }
}

#[async_trait]
impl CharacterRepo for LiveCharacterRepo {
    async fn list_for_account(&self, account_id: u64) -> Result<Vec<CharacterSummary>, ApiError> {
        let sql = format!(
            "SELECT guid, name, race, class, gender, level, map, zone, online, money \
             FROM {}.characters \
             WHERE account = ? AND (deleteDate IS NULL OR deleteDate = 0) \
             ORDER BY `order` ASC, guid ASC",
            self.characters_db
        );
        let rows = sqlx::query(&sql)
            .bind(account_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load characters", err))?;

        let mut characters = Vec::with_capacity(rows.len());
        for row in rows {
            characters.push(CharacterSummary {
                guid: row_get::<u64>(&row, "guid")?,
                name: row_get::<String>(&row, "name")?,
                race: row_get::<u8>(&row, "race")?,
                class_id: row_get::<u8>(&row, "class")?,
                gender: row_get::<u8>(&row, "gender")?,
                level: row_get::<u8>(&row, "level")?,
                map: row_get::<u16>(&row, "map")?,
                zone: row_get::<u32>(&row, "zone")?,
                online: row_get_bool(&row, "online")?,
                money: row_get::<u64>(&row, "money")?,
            });
        }
        Ok(characters)
    }

    async fn find_owner(&self, guid: u64) -> Result<Option<u64>, ApiError> {
        let sql = format!(
            "SELECT account FROM {}.characters WHERE guid = ? LIMIT 1",
            self.characters_db
        );
        sqlx::query_scalar::<_, u64>(&sql)
            .bind(guid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load character owner", err))
    }

    async fn find_location(
        &self,
        guid: u64,
    ) -> Result<Option<CharacterLocationResponse>, ApiError> {
        let sql = format!(
            "SELECT guid, name, map, zone, position_x, position_y, position_z, orientation, online \
             FROM {}.characters WHERE guid = ? LIMIT 1",
            self.characters_db
        );
        let row = sqlx::query(&sql)
            .bind(guid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load character location", err))?;
        match row {
            Some(row) => Ok(Some(CharacterLocationResponse {
                guid: row_get::<u64>(&row, "guid")?,
                name: row_get::<String>(&row, "name")?,
                map: row_get::<u16>(&row, "map")?,
                zone: row_get::<u32>(&row, "zone")?,
                position_x: row_get::<f32>(&row, "position_x")?,
                position_y: row_get::<f32>(&row, "position_y")?,
                position_z: row_get::<f32>(&row, "position_z")?,
                orientation: row_get::<f32>(&row, "orientation")?,
                online: row_get_bool(&row, "online")?,
            })),
            None => Ok(None),
        }
    }

    async fn find_locations_by_account(
        &self,
        account_id: u64,
    ) -> Result<Vec<CharacterLocationResponse>, ApiError> {
        let sql = format!(
            "SELECT guid, name, map, zone, position_x, position_y, position_z, orientation, online \
             FROM {}.characters WHERE account = ? AND (deleteDate IS NULL OR deleteDate = 0) \
             ORDER BY guid ASC",
            self.characters_db
        );
        let rows = sqlx::query(&sql)
            .bind(account_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load player locations", err))?;

        let mut locations = Vec::with_capacity(rows.len());
        for row in rows {
            locations.push(CharacterLocationResponse {
                guid: row_get::<u64>(&row, "guid")?,
                name: row_get::<String>(&row, "name")?,
                map: row_get::<u16>(&row, "map")?,
                zone: row_get::<u32>(&row, "zone")?,
                position_x: row_get::<f32>(&row, "position_x")?,
                position_y: row_get::<f32>(&row, "position_y")?,
                position_z: row_get::<f32>(&row, "position_z")?,
                orientation: row_get::<f32>(&row, "orientation")?,
                online: row_get_bool(&row, "online")?,
            });
        }
        Ok(locations)
    }
}

// ---------------------------------------------------------------------------
// ItemRepo
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ItemRepo: Send + Sync {
    async fn search(&self, query: &ItemQuery) -> Result<(Vec<ItemSummary>, Option<u32>), ApiError>;
    async fn find_by_entry(&self, entry: u32) -> Result<Option<ItemSummary>, ApiError>;
}

pub struct LiveItemRepo {
    pool: MySqlPool,
    world_db: String,
}

impl LiveItemRepo {
    pub fn new(pool: MySqlPool, world_db: String) -> Self {
        Self { pool, world_db }
    }
}

#[async_trait]
impl ItemRepo for LiveItemRepo {
    async fn search(&self, query: &ItemQuery) -> Result<(Vec<ItemSummary>, Option<u32>), ApiError> {
        let limit = query.limit.unwrap_or(50).min(200);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let base = format!(
            "SELECT entry, name, Quality, ItemLevel, class, subclass, displayid \
             FROM {}.item_template WHERE 1=1",
            self.world_db
        );

        let build_query = |use_fulltext: bool| {
            let mut qb = QueryBuilder::<MySql>::new(base.clone());
            if let Some(class_id) = query.class {
                qb.push(" AND class = ").push_bind(class_id);
            }
            if let Some(c) = query.cursor {
                qb.push(" AND entry > ").push_bind(c);
            }
            if let Some(s) = search {
                if use_fulltext {
                    qb.push(" AND MATCH (name) AGAINST (")
                        .push_bind(s)
                        .push(" IN BOOLEAN MODE)");
                } else {
                    qb.push(" AND name LIKE ").push_bind(format!("%{s}%"));
                }
            }
            qb.push(" ORDER BY entry ASC LIMIT ").push_bind(limit + 1);
            qb
        };

        let rows = match build_query(true).build().fetch_all(&self.pool).await {
            Ok(rows) => rows,
            Err(_) if search.is_some() => build_query(false)
                .build()
                .fetch_all(&self.pool)
                .await
                .map_err(|err| map_db_error("failed to load items", err))?,
            Err(err) => return Err(map_db_error("failed to load items", err)),
        };

        let mut items = Vec::with_capacity(rows.len());
        let mut next_cursor: Option<u32> = None;
        if rows.len() > limit as usize {
            next_cursor = Some(row_get::<u32>(&rows[limit as usize - 1], "entry")?);
        }
        for row in rows.iter().take(limit as usize) {
            items.push(ItemSummary {
                entry: row_get::<u32>(row, "entry")?,
                name: row_get::<String>(row, "name")?,
                quality: row_get::<u8>(row, "Quality")?,
                item_level: row_get::<u32>(row, "ItemLevel")?,
                class: row_get::<u8>(row, "class")?,
                subclass: row_get::<u8>(row, "subclass")?,
                display_id: row_get::<u32>(row, "displayid")?,
            });
        }

        Ok((items, next_cursor))
    }

    async fn find_by_entry(&self, entry: u32) -> Result<Option<ItemSummary>, ApiError> {
        let sql = format!(
            "SELECT entry, name, Quality, ItemLevel, class, subclass, displayid \
             FROM {}.item_template WHERE entry = ? LIMIT 1",
            self.world_db
        );
        let row = sqlx::query(&sql)
            .bind(entry)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load item", err))?;
        match row {
            Some(row) => Ok(Some(ItemSummary {
                entry: row_get::<u32>(&row, "entry")?,
                name: row_get::<String>(&row, "name")?,
                quality: row_get::<u8>(&row, "Quality")?,
                item_level: row_get::<u32>(&row, "ItemLevel")?,
                class: row_get::<u8>(&row, "class")?,
                subclass: row_get::<u8>(&row, "subclass")?,
                display_id: row_get::<u32>(&row, "displayid")?,
            })),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// RefreshTokenRepo
// ---------------------------------------------------------------------------

#[async_trait]
pub trait RefreshTokenRepo: Send + Sync {
    async fn store(
        &self,
        account_id: u64,
        refresh_token: &str,
        family_id: Option<&str>,
        parent_id: Option<i64>,
        ip: &str,
        ua: Option<&str>,
    ) -> Result<i64, ApiError>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshTokenRow>, ApiError>;
    async fn revoke_family(&self, family_id: &str, reason: &str) -> Result<(), ApiError>;
    async fn revoke_for_account(
        &self,
        account_id: u64,
        refresh_token: &str,
        reason: &str,
    ) -> Result<(), ApiError>;
    async fn mark_rotated(&self, old_id: i64, new_id: i64) -> Result<(), ApiError>;
    async fn is_access_revoked(&self, jti: &str) -> Result<bool, ApiError>;
    async fn revoke_access(
        &self,
        account_id: u64,
        jti: &str,
        reason: &str,
        expires_minutes: i64,
    ) -> Result<(), ApiError>;
}

pub struct LiveRefreshTokenRepo {
    pool: PgPool,
    refresh_expires_days: i64,
    _access_expires_minutes: i64,
}

impl LiveRefreshTokenRepo {
    pub fn new(pool: PgPool, refresh_expires_days: u64, access_expires_minutes: u64) -> Self {
        Self {
            pool,
            refresh_expires_days: refresh_expires_days as i64,
            _access_expires_minutes: access_expires_minutes as i64,
        }
    }
}

#[async_trait]
impl RefreshTokenRepo for LiveRefreshTokenRepo {
    async fn store(
        &self,
        account_id: u64,
        refresh_token: &str,
        family_id: Option<&str>,
        parent_id: Option<i64>,
        ip: &str,
        ua: Option<&str>,
    ) -> Result<i64, ApiError> {
        let token_hash = hash_refresh_token(refresh_token);
        let account_id_i64 = i64::try_from(account_id)
            .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;

        let owned_family;
        let family: &str = match family_id {
            Some(existing) => existing,
            None => {
                owned_family = Uuid::new_v4().to_string();
                &owned_family
            }
        };

        let sql = format!(
            "INSERT INTO {}.auth_refresh_tokens \
             (account_id, token_hash, family_id, parent_token_id, expires_at, created_ip, user_agent) \
             VALUES ($1, $2, $3, $4, NOW() + ($5::BIGINT * INTERVAL '1 day'), $6, $7) \
             RETURNING id",
            APP_PG_SCHEMA
        );

        let id = sqlx::query_scalar::<_, i64>(&sql)
            .bind(account_id_i64)
            .bind(token_hash)
            .bind(family)
            .bind(parent_id)
            .bind(self.refresh_expires_days)
            .bind(ip)
            .bind(ua)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to store refresh token", err))?;

        Ok(id)
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshTokenRow>, ApiError> {
        let sql = format!(
            "SELECT id, account_id, family_id, \
             revoked_at IS NOT NULL AS is_revoked, \
             expires_at < NOW() AS is_expired \
             FROM {}.auth_refresh_tokens WHERE token_hash = $1 LIMIT 1",
            APP_PG_SCHEMA
        );
        let row = sqlx::query(&sql)
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load refresh token", err))?;

        match row {
            Some(row) => {
                let id: i64 = row.try_get("id").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let account_id_raw: i64 = row.try_get("account_id").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let account_id = u64::try_from(account_id_raw).map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let family_id: String = row.try_get("family_id").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let is_revoked: bool = row.try_get("is_revoked").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let is_expired: bool = row.try_get("is_expired").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                Ok(Some(RefreshTokenRow {
                    id,
                    account_id,
                    family_id,
                    is_revoked,
                    is_expired,
                }))
            }
            None => Ok(None),
        }
    }

    async fn revoke_family(&self, family_id: &str, reason: &str) -> Result<(), ApiError> {
        let sql = format!(
            "UPDATE {}.auth_refresh_tokens SET revoked_at = NOW(), reason = $1 \
             WHERE family_id = $2 AND revoked_at IS NULL",
            APP_PG_SCHEMA
        );
        sqlx::query(&sql)
            .bind(reason)
            .bind(family_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to revoke refresh family", err))?;
        Ok(())
    }

    async fn revoke_for_account(
        &self,
        account_id: u64,
        refresh_token: &str,
        reason: &str,
    ) -> Result<(), ApiError> {
        let token_hash = hash_refresh_token(refresh_token);
        let account_id_i64 = i64::try_from(account_id)
            .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;
        let sql = format!(
            "UPDATE {}.auth_refresh_tokens SET revoked_at = NOW(), reason = $1 \
             WHERE account_id = $2 AND token_hash = $3 AND revoked_at IS NULL",
            APP_PG_SCHEMA
        );
        sqlx::query(&sql)
            .bind(reason)
            .bind(account_id_i64)
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to revoke refresh token", err))?;
        Ok(())
    }

    async fn mark_rotated(&self, old_id: i64, new_id: i64) -> Result<(), ApiError> {
        let sql = format!(
            "UPDATE {}.auth_refresh_tokens SET revoked_at = NOW(), replaced_by_token_id = $1, reason = 'rotated' WHERE id = $2",
            APP_PG_SCHEMA
        );
        sqlx::query(&sql)
            .bind(new_id)
            .bind(old_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to rotate refresh token", err))?;
        Ok(())
    }

    async fn is_access_revoked(&self, jti: &str) -> Result<bool, ApiError> {
        let sql = format!(
            "SELECT jti FROM {}.auth_revoked_access_tokens WHERE jti = $1 AND expires_at > NOW() LIMIT 1",
            APP_PG_SCHEMA
        );
        let row = sqlx::query_scalar::<_, String>(&sql)
            .bind(jti)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to check access token revocation", err))?;
        Ok(row.is_some())
    }

    async fn revoke_access(
        &self,
        account_id: u64,
        jti: &str,
        reason: &str,
        expires_minutes: i64,
    ) -> Result<(), ApiError> {
        let account_id_i64 = i64::try_from(account_id)
            .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;
        let sql = format!(
            "INSERT INTO {}.auth_revoked_access_tokens (jti, account_id, expires_at, reason) \
             VALUES ($1, $2, NOW() + ($3::BIGINT * INTERVAL '1 minute'), $4) \
             ON CONFLICT (jti) DO UPDATE SET revoked_at = CURRENT_TIMESTAMP, reason = EXCLUDED.reason",
            APP_PG_SCHEMA
        );
        sqlx::query(&sql)
            .bind(jti)
            .bind(account_id_i64)
            .bind(expires_minutes)
            .bind(reason)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to revoke access token", err))?;
        Ok(())
    }
}

#[cfg(test)]
mod login_lockout_tests {
    use super::LoginLockout;

    #[test]
    fn locks_after_max_attempts() {
        let lockout = LoginLockout::default();
        assert!(!lockout.is_locked(1));
        for _ in 0..5 {
            lockout.record_failure(1);
        }
        assert!(lockout.is_locked(1));
    }

    #[test]
    fn resets_after_success() {
        let lockout = LoginLockout::default();
        for _ in 0..6 {
            lockout.record_failure(1);
        }
        assert!(lockout.is_locked(1));
        lockout.reset(1);
        assert!(!lockout.is_locked(1));
    }
}

// ---------------------------------------------------------------------------
// ServiceTokenRepo
// ---------------------------------------------------------------------------

pub const SERVICE_TOKEN_PREFIX: &str = "wowst_";

#[derive(Debug, Clone)]
pub struct ServiceTokenRow {
    pub id: i64,
    pub name: String,
    pub created_by: i64,
    pub created_at_unix: i64,
    pub expires_at_unix: Option<i64>,
    pub revoked_at_unix: Option<i64>,
    pub last_used_at_unix: Option<i64>,
}

#[async_trait]
pub trait ServiceTokenRepo: Send + Sync {
    async fn create(
        &self,
        token_hash: &str,
        name: &str,
        created_by: i64,
        expires_at_unix: Option<i64>,
    ) -> Result<i64, ApiError>;
    async fn list(&self) -> Result<Vec<ServiceTokenRow>, ApiError>;
    async fn find_active_by_hash(&self, token_hash: &str) -> Result<Option<(i64, i64)>, ApiError>;
    async fn revoke(&self, token_id: i64) -> Result<bool, ApiError>;
    async fn touch_last_used(&self, token_id: i64) -> Result<(), ApiError>;
}

pub struct LiveServiceTokenRepo {
    pool: PgPool,
}

impl LiveServiceTokenRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceTokenRepo for LiveServiceTokenRepo {
    async fn create(
        &self,
        token_hash: &str,
        name: &str,
        created_by: i64,
        expires_at_unix: Option<i64>,
    ) -> Result<i64, ApiError> {
        let sql = format!(
            "INSERT INTO {}.auth_service_tokens \
             (token_hash, name, created_by, expires_at) \
             VALUES ($1, $2, $3, CASE WHEN $4::BIGINT IS NULL THEN NULL \
               ELSE to_timestamp($4::BIGINT) END) \
             RETURNING id",
            APP_PG_SCHEMA
        );

        let id = sqlx::query_scalar::<_, i64>(&sql)
            .bind(token_hash)
            .bind(name)
            .bind(created_by)
            .bind(expires_at_unix)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to create service token", err))?;

        Ok(id)
    }

    async fn list(&self) -> Result<Vec<ServiceTokenRow>, ApiError> {
        let sql = format!(
            "SELECT id, name, created_by, \
             EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at_unix, \
             EXTRACT(EPOCH FROM expires_at)::BIGINT AS expires_at_unix, \
             EXTRACT(EPOCH FROM revoked_at)::BIGINT AS revoked_at_unix, \
             EXTRACT(EPOCH FROM last_used_at)::BIGINT AS last_used_at_unix \
             FROM {}.auth_service_tokens ORDER BY created_at DESC",
            APP_PG_SCHEMA
        );

        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to list service tokens", err))?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            tokens.push(ServiceTokenRow {
                id: row.try_get("id").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?,
                name: row.try_get("name").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?,
                created_by: row.try_get("created_by").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?,
                created_at_unix: row.try_get::<i64, _>("created_at_unix").unwrap_or(0),
                expires_at_unix: row.try_get("expires_at_unix").unwrap_or(None),
                revoked_at_unix: row.try_get("revoked_at_unix").unwrap_or(None),
                last_used_at_unix: row.try_get("last_used_at_unix").unwrap_or(None),
            });
        }

        Ok(tokens)
    }

    async fn find_active_by_hash(&self, token_hash: &str) -> Result<Option<(i64, i64)>, ApiError> {
        let sql = format!(
            "SELECT id, created_by FROM {}.auth_service_tokens \
             WHERE token_hash = $1 AND revoked_at IS NULL \
               AND (expires_at IS NULL OR expires_at > NOW()) \
             LIMIT 1",
            APP_PG_SCHEMA
        );

        let row = sqlx::query(&sql)
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load service token", err))?;

        match row {
            Some(row) => {
                let id = row.try_get("id").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                let created_by = row.try_get("created_by").map_err(|_| {
                    ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                })?;
                Ok(Some((id, created_by)))
            }
            None => Ok(None),
        }
    }

    async fn revoke(&self, token_id: i64) -> Result<bool, ApiError> {
        let sql = format!(
            "UPDATE {}.auth_service_tokens SET revoked_at = NOW() \
             WHERE id = $1 AND revoked_at IS NULL",
            APP_PG_SCHEMA
        );

        let result = sqlx::query(&sql)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to revoke service token", err))?;

        Ok(result.rows_affected() > 0)
    }

    async fn touch_last_used(&self, token_id: i64) -> Result<(), ApiError> {
        let sql = format!(
            "UPDATE {}.auth_service_tokens SET last_used_at = NOW() WHERE id = $1",
            APP_PG_SCHEMA
        );

        sqlx::query(&sql)
            .bind(token_id)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to update service token last used", err))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AuditRepo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuditEntryNew {
    pub actor_account_id: i64,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AuditLogRow {
    pub id: i64,
    pub actor_account_id: i64,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at_unix: i64,
}

#[async_trait]
pub trait AuditRepo: Send + Sync {
    async fn insert(&self, entry: AuditEntryNew) -> Result<(), ApiError>;
    async fn list(&self, limit: u32, before_id: Option<i64>) -> Result<Vec<AuditLogRow>, ApiError>;
}

pub struct LiveAuditRepo {
    pool: PgPool,
}

impl LiveAuditRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepo for LiveAuditRepo {
    async fn insert(&self, entry: AuditEntryNew) -> Result<(), ApiError> {
        let sql = format!(
            "INSERT INTO {}.admin_audit_log \
             (actor_account_id, action, target_type, target_id, details) \
             VALUES ($1, $2, $3, $4, $5)",
            APP_PG_SCHEMA
        );

        sqlx::query(&sql)
            .bind(entry.actor_account_id)
            .bind(entry.action)
            .bind(entry.target_type)
            .bind(entry.target_id)
            .bind(entry.details)
            .execute(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to write audit log", err))?;

        Ok(())
    }

    async fn list(&self, limit: u32, before_id: Option<i64>) -> Result<Vec<AuditLogRow>, ApiError> {
        let sql = format!(
            "SELECT id, actor_account_id, action, target_type, target_id, details, \
             EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at_unix \
             FROM {}.admin_audit_log \
             WHERE ($1::BIGINT IS NULL OR id < $1) \
             ORDER BY id DESC LIMIT $2",
            APP_PG_SCHEMA
        );

        let rows = sqlx::query(&sql)
            .bind(before_id)
            .bind(i32::try_from(limit).unwrap_or(200))
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to list audit log", err))?;

        rows.into_iter()
            .map(|row| {
                Ok(AuditLogRow {
                    id: row.try_get("id").map_err(|_| {
                        ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                    })?,
                    actor_account_id: row.try_get("actor_account_id").map_err(|_| {
                        ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                    })?,
                    action: row.try_get("action").map_err(|_| {
                        ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                    })?,
                    target_type: row.try_get("target_type").map_err(|_| {
                        ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED")
                    })?,
                    target_id: row.try_get("target_id").unwrap_or(None),
                    details: row.try_get("details").unwrap_or(None),
                    created_at_unix: row.try_get::<i64, _>("created_at_unix").unwrap_or(0),
                })
            })
            .collect()
    }
}
