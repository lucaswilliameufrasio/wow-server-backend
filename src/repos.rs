use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use sqlx::{MySql, MySqlPool, PgPool, QueryBuilder, Row, mysql::MySqlRow};
use tracing::error;
use uuid::Uuid;

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

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub jwt: JwtConfig,
    pub accounts: Arc<dyn AccountRepo>,
    pub characters: Arc<dyn CharacterRepo>,
    pub items: Arc<dyn ItemRepo>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepo>,
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
    async fn search_players(
        &self,
        search: Option<&str>,
        online: Option<bool>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AdminPlayerSummary>, ApiError>;
    async fn find_online_players(&self) -> Result<Vec<OnlinePlayerSummary>, ApiError>;
}

pub struct LiveAccountRepo {
    pool: MySqlPool,
    auth_db: String,
    characters_db: String,
}

impl LiveAccountRepo {
    pub fn new(pool: MySqlPool, auth_db: String, characters_db: String) -> Self {
        Self {
            pool,
            auth_db,
            characters_db,
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
            "UPDATE {}.account SET failed_logins = 0, last_login = NOW(), last_ip = ?, online = 1 WHERE id = ?",
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
            Some(row) => Ok(Some(AuthAccountRow {
                username: row_get(&row, "username")?,
                locked: row_get_bool(&row, "locked")?,
            })),
            None => Ok(None),
        }
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
        Ok(result.rows_affected() > 0)
    }

    async fn search_players(
        &self,
        search: Option<&str>,
        online: Option<bool>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AdminPlayerSummary>, ApiError> {
        let base_sql = format!(
            "SELECT \
                a.id, \
                a.username, \
                a.email, \
                UNIX_TIMESTAMP(a.joindate) AS joined_unix, \
                UNIX_TIMESTAMP(a.last_login) AS last_login_unix, \
                a.last_ip, \
                a.locked, \
                a.online AS account_online, \
                COALESCE(acc.gmlevel, 0) AS gm_level, \
                COALESCE(chars.character_count, 0) AS character_count \
             FROM {}.account a \
             LEFT JOIN (SELECT id, MAX(gmlevel) AS gmlevel FROM {}.account_access GROUP BY id) acc ON acc.id = a.id \
             LEFT JOIN (SELECT account, COUNT(*) AS character_count FROM {}.characters WHERE deleteDate IS NULL OR deleteDate = 0 GROUP BY account) chars ON chars.account = a.id \
             WHERE 1=1",
            self.auth_db, self.auth_db, self.characters_db
        );

        let mut qb = QueryBuilder::<MySql>::new(base_sql);

        if let Some(search) = search.filter(|s| !s.trim().is_empty()) {
            let like = format!("%{}%", search.trim());
            qb.push(" AND a.username LIKE ").push_bind(like);
        }

        if let Some(online) = online {
            qb.push(" AND a.online = ").push_bind(online);
        }

        qb.push(" ORDER BY a.id DESC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load admin players", err))?;

        let mut players = Vec::with_capacity(rows.len());
        for row in rows {
            players.push(AdminPlayerSummary {
                id: row_get::<u64>(&row, "id")?,
                username: row_get::<String>(&row, "username")?,
                email: row_get_opt::<String>(&row, "email")?,
                joined_unix: row_get_opt::<i64>(&row, "joined_unix")?.map(|v| v as u64),
                last_login_unix: row_get_opt::<i64>(&row, "last_login_unix")?.map(|v| v as u64),
                last_ip: row_get_opt::<String>(&row, "last_ip")?,
                locked: row_get_bool(&row, "locked")?,
                account_online: row_get_bool(&row, "account_online")?,
                gm_level: row_get::<i64>(&row, "gm_level")? as u8,
                character_count: row_get::<i64>(&row, "character_count")? as u32,
            });
        }

        Ok(players)
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
    async fn search(&self, query: &ItemQuery) -> Result<Vec<ItemSummary>, ApiError>;
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
    async fn search(&self, query: &ItemQuery) -> Result<Vec<ItemSummary>, ApiError> {
        let limit = query.limit.unwrap_or(50).min(200);
        let offset = query.offset.unwrap_or(0);

        let mut qb = QueryBuilder::<MySql>::new(format!(
            "SELECT entry, name, Quality, ItemLevel, class, subclass, displayid \
             FROM {}.item_template WHERE 1=1",
            self.world_db
        ));

        if let Some(class_id) = query.class {
            qb.push(" AND class = ").push_bind(class_id);
        }

        if let Some(ref search) = query.search
            && !search.trim().is_empty()
        {
            qb.push(" AND name LIKE ")
                .push_bind(format!("%{}%", search.trim()));
        }

        qb.push(" ORDER BY entry ASC LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|err| map_db_error("failed to load items", err))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(ItemSummary {
                entry: row_get::<u32>(&row, "entry")?,
                name: row_get::<String>(&row, "name")?,
                quality: row_get::<u8>(&row, "Quality")?,
                item_level: row_get::<u32>(&row, "ItemLevel")?,
                class: row_get::<u8>(&row, "class")?,
                subclass: row_get::<u8>(&row, "subclass")?,
                display_id: row_get::<u32>(&row, "displayid")?,
            });
        }

        Ok(items)
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

fn hash_refresh_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
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
        let generated_family = Uuid::new_v4().to_string();
        let family = family_id.unwrap_or(&generated_family);
        let account_id_i64 = i64::try_from(account_id)
            .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;

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
