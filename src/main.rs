use std::{
    collections::HashSet,
    env, fs,
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind,
};
use num_bigint::{BigInt, Sign};
use once_cell::sync::Lazy;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use sqlx::{
    MySql, MySqlPool, PgPool, Postgres, QueryBuilder, Row, Transaction,
    migrate::MigrateError,
    mysql::{MySqlPoolOptions, MySqlRow},
    postgres::PgPoolOptions,
};
use thiserror::Error;
use tokio::{net::TcpListener, signal};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Debug, Error)]
enum AppError {
    #[error("failed to initialize tracing: {0}")]
    Tracing(String),
    #[error("failed to bind to {addr}: {source}")]
    Bind {
        #[source]
        source: std::io::Error,
        addr: SocketAddr,
    },
    #[error("failed to create database pool: {0}")]
    DbConnect(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    Migrate(#[from] MigrateError),
    #[error("failed to create jwt config: {0}")]
    JwtConfig(String),
    #[error("server error: {0}")]
    Serve(#[from] std::io::Error),
}

type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Clone)]
struct AppState {
    acore_pool: MySqlPool,
    app_pool: PgPool,
    config: AppConfig,
    jwt: JwtConfig,
}

#[derive(Clone)]
struct AppConfig {
    auth_db: String,
    characters_db: String,
    world_db: String,
    srp6_core5_mode: bool,
}

const APP_PG_SCHEMA: &str = "public";
static APP_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
struct JwtConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    expires_minutes: u64,
    refresh_expires_days: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JwtClaims {
    sub: u64,
    jti: String,
    username: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    gm_level: u8,
    iss: String,
    aud: String,
    iat: u64,
    nbf: u64,
    exp: u64,
}

#[derive(Debug, Clone)]
struct AuthContext {
    account_id: u64,
    access_jti: String,
    username: String,
    gm_level: u8,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: &'static str,
    error_code: &'static str,
}

impl ApiError {
    fn new(status: StatusCode, message: &'static str, error_code: &'static str) -> Self {
        Self {
            status,
            message,
            error_code,
        }
    }

    fn bad_request(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message, error_code)
    }

    fn unauthorized(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message, error_code)
    }

    fn forbidden(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, message, error_code)
    }

    fn not_found(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message, error_code)
    }

    fn conflict(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, message, error_code)
    }

    fn internal(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message, error_code)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                message: self.message,
                error_code: self.error_code,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    message: &'static str,
    error_code: &'static str,
}

#[derive(Serialize)]
struct HealthCheckResponse {
    message: &'static str,
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    email: Option<String>,
}

#[derive(Serialize)]
struct RegisterResponse {
    account_id: u64,
    username: String,
    email: Option<String>,
}

#[derive(Deserialize)]
struct SignInRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct SignInResponse {
    account_id: u64,
    username: String,
    email: Option<String>,
    gm_level: u8,
    access_token: String,
    token_type: &'static str,
    expires_in_seconds: u64,
    roles: Vec<String>,
    permissions: Vec<String>,
    refresh_token: String,
    refresh_expires_in_seconds: u64,
}

#[derive(Serialize)]
struct AuthMeResponse {
    account_id: u64,
    username: String,
    gm_level: u8,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[derive(Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct RefreshTokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in_seconds: u64,
    refresh_token: String,
    refresh_expires_in_seconds: u64,
}

#[derive(Deserialize)]
struct LogoutRequest {
    refresh_token: Option<String>,
}

#[derive(Serialize)]
struct CharacterSummary {
    guid: u64,
    name: String,
    race: u8,
    #[serde(rename = "class")]
    class_id: u8,
    gender: u8,
    level: u8,
    map: u16,
    zone: u32,
    online: bool,
    money: u64,
}

#[derive(Serialize)]
struct CharacterListResponse {
    account_id: u64,
    characters: Vec<CharacterSummary>,
}

#[derive(Serialize)]
struct CharacterLocationResponse {
    guid: u64,
    name: String,
    map: u16,
    zone: u32,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    orientation: f32,
    online: bool,
}

#[derive(Deserialize)]
struct AdminPlayersQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    online: Option<bool>,
}

#[derive(Serialize)]
struct AdminPlayerSummary {
    id: u64,
    username: String,
    email: Option<String>,
    joined_unix: Option<u64>,
    last_login_unix: Option<u64>,
    last_ip: Option<String>,
    locked: bool,
    account_online: bool,
    gm_level: u8,
    character_count: u32,
}

#[derive(Serialize)]
struct AdminPlayersResponse {
    limit: u32,
    offset: u32,
    players: Vec<AdminPlayerSummary>,
}

#[derive(Serialize)]
struct OnlinePlayerSummary {
    account_id: u64,
    username: String,
    guid: u64,
    name: String,
    level: u8,
    map: u16,
    zone: u32,
}

#[derive(Serialize)]
struct AdminAccountLocationsResponse {
    account_id: u64,
    username: String,
    locations: Vec<CharacterLocationResponse>,
}

#[derive(Deserialize)]
struct SetAccountLockRequest {
    locked: bool,
}

#[derive(Serialize)]
struct AccountLockResponse {
    account_id: u64,
    locked: bool,
}

#[derive(Deserialize)]
struct ItemQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    class: Option<u8>,
}

#[derive(Serialize)]
struct ItemSummary {
    entry: u32,
    name: String,
    quality: u8,
    item_level: u32,
    class: u8,
    subclass: u8,
    display_id: u32,
}

#[derive(Serialize)]
struct ItemListResponse {
    limit: u32,
    offset: u32,
    items: Vec<ItemSummary>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let state = build_state().await?;
    let router = build_router(state);

    run(router).await?;
    Ok(())
}

async fn build_state() -> AppResult<AppState> {
    let acore_database_url = env::var("AZEROTH_CORE_MYSQL_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306".to_string());
    let app_database_url = env::var("APP_POSTGRES_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@127.0.0.1:5432/wow_app".to_string());

    let acore_pool = MySqlPoolOptions::new()
        .max_connections(20)
        .connect(&acore_database_url)
        .await?;
    let app_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&app_database_url)
        .await?;

    let config = AppConfig {
        auth_db: env::var("AZEROTH_CORE_AUTH_DB").unwrap_or_else(|_| "acore_auth".to_string()),
        characters_db: env::var("AZEROTH_CORE_CHARACTERS_DB")
            .unwrap_or_else(|_| "acore_characters".to_string()),
        world_db: env::var("AZEROTH_CORE_WORLD_DB").unwrap_or_else(|_| "acore_world".to_string()),
        srp6_core5_mode: parse_bool_env("SRP6_CORE5_MODE"),
    };

    let jwt = build_jwt_config()?;
    run_app_migrations(&app_pool).await?;

    Ok(AppState {
        acore_pool,
        app_pool,
        config,
        jwt,
    })
}

fn build_jwt_config() -> AppResult<JwtConfig> {
    let private_key_pem =
        load_pem("JWT_PRIVATE_KEY_PEM", "JWT_PRIVATE_KEY_PATH").ok_or_else(|| {
            AppError::JwtConfig(
                "missing private key: set JWT_PRIVATE_KEY_PEM or JWT_PRIVATE_KEY_PATH".to_string(),
            )
        })?;
    let public_key_pem =
        load_pem("JWT_PUBLIC_KEY_PEM", "JWT_PUBLIC_KEY_PATH").ok_or_else(|| {
            AppError::JwtConfig(
                "missing public key: set JWT_PUBLIC_KEY_PEM or JWT_PUBLIC_KEY_PATH".to_string(),
            )
        })?;

    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| AppError::JwtConfig(format!("invalid private key: {e}")))?;
    let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
        .map_err(|e| AppError::JwtConfig(format!("invalid public key: {e}")))?;

    let issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "wow-backend".to_string());
    let audience = env::var("JWT_AUDIENCE").unwrap_or_else(|_| "wow-web".to_string());
    let expires_minutes = env::var("JWT_EXPIRES_MINUTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(15);
    let refresh_expires_days = env::var("JWT_REFRESH_EXPIRES_DAYS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30);

    Ok(JwtConfig {
        encoding_key,
        decoding_key,
        issuer,
        audience,
        expires_minutes,
        refresh_expires_days,
    })
}

fn load_pem(content_var: &str, path_var: &str) -> Option<String> {
    if let Ok(v) = env::var(content_var) {
        let normalized = v.replace("\\n", "\n");
        if !normalized.trim().is_empty() {
            return Some(normalized);
        }
    }

    if let Ok(path) = env::var(path_var)
        && let Ok(content) = fs::read_to_string(path)
        && !content.trim().is_empty()
    {
        return Some(content);
    }

    None
}

async fn run_app_migrations(pool: &PgPool) -> AppResult<()> {
    APP_MIGRATOR.run(pool).await?;
    Ok(())
}

fn parse_bool_env(key: &str) -> bool {
    env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health-check", get(health_check_handler))
        .route("/v1/auth/register", post(register_handler))
        .route("/v1/auth/sign-in", post(sign_in_handler))
        .route("/v1/auth/refresh", post(refresh_token_handler))
        .route("/v1/auth/logout", post(logout_handler))
        .route("/v1/auth/me", get(auth_me_handler))
        .route("/v1/characters", get(characters_handler))
        .route("/v1/player/characters", get(characters_handler))
        .route(
            "/v1/characters/{guid}/location",
            get(character_location_handler),
        )
        .route("/v1/items", get(items_handler))
        .route("/v1/items/{entry}", get(item_by_entry_handler))
        .route("/v1/admin/players", get(admin_players_handler))
        .route(
            "/v1/admin/players/{account_id}/locations",
            get(admin_player_locations_handler),
        )
        .route(
            "/v1/admin/players/{account_id}/lock",
            patch(admin_lock_player_handler),
        )
        .route(
            "/v1/admin/online-players",
            get(admin_online_players_handler),
        )
        .with_state(state)
        .fallback(not_found_handler)
}

async fn run(app: Router) -> AppResult<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| AppError::Bind { source, addr })?;

    info!(%addr, "listening on {addr}");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn init_tracing() -> AppResult<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .try_init()
        .map_err(|err| AppError::Tracing(err.to_string()))?;

    Ok(())
}

async fn health_check_handler() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse { message: "ok" })
}

async fn not_found_handler() -> ApiError {
    ApiError::not_found("Route not found", "ROUTE_NOT_FOUND")
}

async fn register_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let username = normalize_username(&body.username)?;
    validate_password(&body.password)?;
    let email = body
        .email
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let exists_sql = format!(
        "SELECT id FROM {}.account WHERE username = ? LIMIT 1",
        state.config.auth_db
    );
    let already_exists = sqlx::query_scalar::<_, u64>(&exists_sql)
        .bind(&username)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to check existing account", err))?;

    if already_exists.is_some() {
        return Err(ApiError::conflict(
            "Username already exists",
            "USERNAME_ALREADY_EXISTS",
        ));
    }

    let (salt, verifier) = srp6_register(&username, &body.password, state.config.srp6_core5_mode);
    let client_ip = read_client_ip(&headers);

    let insert_sql = format!(
        "INSERT INTO {}.account (username, salt, verifier, email, reg_mail, last_ip) VALUES (?, ?, ?, ?, ?, ?)",
        state.config.auth_db
    );

    let insert_result = sqlx::query(&insert_sql)
        .bind(&username)
        .bind(salt)
        .bind(verifier)
        .bind(email.as_deref())
        .bind(email.as_deref())
        .bind(client_ip)
        .execute(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to create account", err))?;

    Ok(Json(RegisterResponse {
        account_id: insert_result.last_insert_id(),
        username,
        email,
    }))
}

async fn sign_in_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApiError> {
    let username = normalize_username(&body.username)?;

    let query_sql = format!(
        "SELECT id, username, email, salt, verifier, locked FROM {}.account WHERE username = ? LIMIT 1",
        state.config.auth_db
    );

    let account_row = sqlx::query(&query_sql)
        .bind(&username)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to fetch account", err))?
        .ok_or_else(|| ApiError::unauthorized("Invalid credentials", "INVALID_CREDENTIALS"))?;

    let account_id: u64 = row_get(&account_row, "id")?;
    let account_username: String = row_get(&account_row, "username")?;
    let email: Option<String> = row_get_opt(&account_row, "email")?;
    let salt: Vec<u8> = row_get(&account_row, "salt")?;
    let verifier: Vec<u8> = row_get(&account_row, "verifier")?;
    let locked: bool = row_get_bool(&account_row, "locked")?;

    if locked {
        return Err(ApiError::unauthorized(
            "Account is locked",
            "ACCOUNT_LOCKED",
        ));
    }

    let password_ok = srp6_verify(
        &account_username,
        &body.password,
        &salt,
        &verifier,
        state.config.srp6_core5_mode,
    );

    if !password_ok {
        let fail_sql = format!(
            "UPDATE {}.account SET failed_logins = failed_logins + 1 WHERE id = ?",
            state.config.auth_db
        );
        let _ = sqlx::query(&fail_sql)
            .bind(account_id)
            .execute(&state.acore_pool)
            .await;

        return Err(ApiError::unauthorized(
            "Invalid credentials",
            "INVALID_CREDENTIALS",
        ));
    }

    let client_ip = read_client_ip(&headers);
    let update_sql = format!(
        "UPDATE {}.account SET failed_logins = 0, last_login = NOW(), last_ip = ?, online = 1 WHERE id = ?",
        state.config.auth_db
    );

    sqlx::query(&update_sql)
        .bind(client_ip)
        .bind(account_id)
        .execute(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to update account sign-in metadata", err))?;

    let gm_level = account_gm_level(&state, account_id).await;
    let policy = build_rbac_policy(gm_level);
    let token = issue_jwt(&state.jwt, account_id, &account_username, gm_level, &policy)?;
    let refresh_token = generate_refresh_token();
    let client_ip = read_client_ip(&headers);
    let user_agent = read_user_agent(&headers);
    store_new_refresh_token(
        &state,
        account_id,
        &refresh_token,
        None,
        None,
        client_ip.as_str(),
        user_agent.as_deref(),
    )
    .await?;

    Ok(Json(SignInResponse {
        account_id,
        username: account_username,
        email,
        gm_level,
        access_token: token,
        token_type: "Bearer",
        expires_in_seconds: state.jwt.expires_minutes * 60,
        roles: policy.roles,
        permissions: policy.permissions,
        refresh_token,
        refresh_expires_in_seconds: state.jwt.refresh_expires_days * 24 * 60 * 60,
    }))
}

async fn refresh_token_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    if body.refresh_token.trim().is_empty() {
        return Err(ApiError::bad_request(
            "Refresh token is required",
            "INVALID_REFRESH_TOKEN",
        ));
    }

    let client_ip = read_client_ip(&headers);
    let user_agent = read_user_agent(&headers);
    let refresh_hash = hash_refresh_token(&body.refresh_token);

    let mut tx = state
        .app_pool
        .begin()
        .await
        .map_err(|err| map_db_error("failed to start refresh transaction", err))?;

    let select_sql = format!(
        "SELECT id, account_id, family_id, revoked_at IS NOT NULL AS is_revoked, expires_at < NOW() AS is_expired \
         FROM {}.auth_refresh_tokens WHERE token_hash = $1 LIMIT 1 FOR UPDATE",
        APP_PG_SCHEMA
    );

    let row = sqlx::query(&select_sql)
        .bind(&refresh_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| map_db_error("failed to load refresh token", err))?
        .ok_or_else(|| ApiError::unauthorized("Invalid refresh token", "INVALID_REFRESH_TOKEN"))?;

    let token_id: i64 = row
        .try_get("id")
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;
    let account_id_raw: i64 = row
        .try_get("account_id")
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;
    let account_id = u64::try_from(account_id_raw)
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;
    let family_id: String = row
        .try_get("family_id")
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;
    let is_revoked: bool = row
        .try_get("is_revoked")
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;
    let is_expired: bool = row
        .try_get("is_expired")
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))?;

    if is_revoked || is_expired {
        revoke_refresh_family(
            &mut tx,
            &state,
            &family_id,
            if is_expired {
                "expired_refresh_reuse"
            } else {
                "revoked_refresh_reuse"
            },
        )
        .await?;
        tx.commit()
            .await
            .map_err(|err| map_db_error("failed to commit refresh revoke transaction", err))?;
        return Err(ApiError::unauthorized(
            "Refresh token is not valid",
            "INVALID_REFRESH_TOKEN",
        ));
    }

    let auth = load_auth_context(&state, account_id, None).await?;
    let refresh_token = generate_refresh_token();

    let new_refresh_id = insert_refresh_token_with_family(
        &mut tx,
        &state,
        account_id,
        &refresh_token,
        &family_id,
        Some(token_id),
        client_ip.as_str(),
        user_agent.as_deref(),
    )
    .await?;

    let revoke_sql = format!(
        "UPDATE {}.auth_refresh_tokens SET revoked_at = NOW(), replaced_by_token_id = $1, reason = 'rotated' WHERE id = $2",
        APP_PG_SCHEMA
    );
    sqlx::query(&revoke_sql)
        .bind(new_refresh_id)
        .bind(token_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| map_db_error("failed to rotate refresh token", err))?;

    tx.commit()
        .await
        .map_err(|err| map_db_error("failed to commit refresh transaction", err))?;

    let policy = build_rbac_policy(auth.gm_level);
    let access_token = issue_jwt(
        &state.jwt,
        auth.account_id,
        &auth.username,
        auth.gm_level,
        &policy,
    )?;

    Ok(Json(RefreshTokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in_seconds: state.jwt.expires_minutes * 60,
        refresh_token,
        refresh_expires_in_seconds: state.jwt.refresh_expires_days * 24 * 60 * 60,
    }))
}

async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<LogoutRequest>>,
) -> Result<StatusCode, ApiError> {
    let auth = authenticate(&headers, &state).await?;

    revoke_access_jti(&state, auth.account_id, &auth.access_jti, "logout").await?;

    if let Some(refresh_token) = body
        .and_then(|v| v.0.refresh_token)
        .filter(|t| !t.trim().is_empty())
    {
        revoke_refresh_token_for_account(&state, auth.account_id, &refresh_token, "logout").await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn auth_me_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthMeResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;

    Ok(Json(AuthMeResponse {
        account_id: auth.account_id,
        username: auth.username,
        gm_level: auth.gm_level,
        roles: auth.roles,
        permissions: auth.permissions,
    }))
}

async fn characters_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CharacterListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let account_id = auth.account_id;

    let sql = format!(
        "SELECT guid, name, race, class, gender, level, map, zone, online, money FROM {}.characters WHERE account = ? AND (deleteDate IS NULL OR deleteDate = 0) ORDER BY `order` ASC, guid ASC",
        state.config.characters_db
    );

    let rows = sqlx::query(&sql)
        .bind(account_id)
        .fetch_all(&state.acore_pool)
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

    Ok(Json(CharacterListResponse {
        account_id,
        characters,
    }))
}

async fn character_location_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<u64>,
) -> Result<Json<CharacterLocationResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;

    let owner_sql = format!(
        "SELECT account FROM {}.characters WHERE guid = ? LIMIT 1",
        state.config.characters_db
    );
    let owner_account = sqlx::query_scalar::<_, u64>(&owner_sql)
        .bind(guid)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load character owner", err))?
        .ok_or_else(|| ApiError::not_found("Character not found", "CHARACTER_NOT_FOUND"))?;

    if owner_account != auth.account_id && !has_permission(&auth, "characters:location:any") {
        return Err(ApiError::forbidden(
            "You are not allowed to read this character location",
            "FORBIDDEN_CHARACTER_LOCATION",
        ));
    }

    let sql = format!(
        "SELECT guid, name, map, zone, position_x, position_y, position_z, orientation, online FROM {}.characters WHERE guid = ? LIMIT 1",
        state.config.characters_db
    );

    let row = sqlx::query(&sql)
        .bind(guid)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load character location", err))?
        .ok_or_else(|| ApiError::not_found("Character not found", "CHARACTER_NOT_FOUND"))?;

    Ok(Json(CharacterLocationResponse {
        guid: row_get::<u64>(&row, "guid")?,
        name: row_get::<String>(&row, "name")?,
        map: row_get::<u16>(&row, "map")?,
        zone: row_get::<u32>(&row, "zone")?,
        position_x: row_get::<f32>(&row, "position_x")?,
        position_y: row_get::<f32>(&row, "position_y")?,
        position_z: row_get::<f32>(&row, "position_z")?,
        orientation: row_get::<f32>(&row, "orientation")?,
        online: row_get_bool(&row, "online")?,
    }))
}

async fn admin_players_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminPlayersQuery>,
) -> Result<Json<AdminPlayersResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

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
        state.config.auth_db, state.config.auth_db, state.config.characters_db
    );

    let mut qb = QueryBuilder::<MySql>::new(base_sql);

    if let Some(search) = query.search.filter(|s| !s.trim().is_empty()) {
        let like = format!("%{}%", search.trim());
        qb.push(" AND a.username LIKE ").push_bind(like);
    }

    if let Some(online) = query.online {
        qb.push(" AND a.online = ").push_bind(online);
    }

    qb.push(" ORDER BY a.id DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load admin players", err))?;

    let mut players = Vec::with_capacity(rows.len());
    for row in rows {
        players.push(AdminPlayerSummary {
            id: row_get::<u64>(&row, "id")?,
            username: row_get::<String>(&row, "username")?,
            email: row_get_opt::<String>(&row, "email")?,
            joined_unix: row_get_opt::<u64>(&row, "joined_unix")?,
            last_login_unix: row_get_opt::<u64>(&row, "last_login_unix")?,
            last_ip: row_get_opt::<String>(&row, "last_ip")?,
            locked: row_get_bool(&row, "locked")?,
            account_online: row_get_bool(&row, "account_online")?,
            gm_level: row_get::<u8>(&row, "gm_level")?,
            character_count: row_get::<u32>(&row, "character_count")?,
        });
    }

    Ok(Json(AdminPlayersResponse {
        limit,
        offset,
        players,
    }))
}

async fn admin_player_locations_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(account_id): Path<u64>,
) -> Result<Json<AdminAccountLocationsResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let account_sql = format!(
        "SELECT username FROM {}.account WHERE id = ? LIMIT 1",
        state.config.auth_db
    );
    let username = sqlx::query_scalar::<_, String>(&account_sql)
        .bind(account_id)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load account", err))?
        .ok_or_else(|| ApiError::not_found("Account not found", "ACCOUNT_NOT_FOUND"))?;

    let locations_sql = format!(
        "SELECT guid, name, map, zone, position_x, position_y, position_z, orientation, online FROM {}.characters WHERE account = ? AND (deleteDate IS NULL OR deleteDate = 0) ORDER BY guid ASC",
        state.config.characters_db
    );

    let rows = sqlx::query(&locations_sql)
        .bind(account_id)
        .fetch_all(&state.acore_pool)
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

    Ok(Json(AdminAccountLocationsResponse {
        account_id,
        username,
        locations,
    }))
}

async fn admin_lock_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(account_id): Path<u64>,
    Json(body): Json<SetAccountLockRequest>,
) -> Result<Json<AccountLockResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:write")?;

    let sql = format!(
        "UPDATE {}.account SET locked = ? WHERE id = ?",
        state.config.auth_db
    );

    let result = sqlx::query(&sql)
        .bind(body.locked)
        .bind(account_id)
        .execute(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to update account lock", err))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "Account not found",
            "ACCOUNT_NOT_FOUND",
        ));
    }

    Ok(Json(AccountLockResponse {
        account_id,
        locked: body.locked,
    }))
}

async fn admin_online_players_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<OnlinePlayerSummary>>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let sql = format!(
        "SELECT a.id AS account_id, a.username, c.guid, c.name, c.level, c.map, c.zone FROM {}.characters c INNER JOIN {}.account a ON a.id = c.account WHERE c.online = 1 ORDER BY c.name ASC",
        state.config.characters_db, state.config.auth_db
    );

    let rows = sqlx::query(&sql)
        .fetch_all(&state.acore_pool)
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

    Ok(Json(players))
}

async fn items_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ItemQuery>,
) -> Result<Json<ItemListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "items:read")?;

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    let mut qb = QueryBuilder::<MySql>::new(format!(
        "SELECT entry, name, Quality, ItemLevel, class, subclass, displayid FROM {}.item_template WHERE 1=1",
        state.config.world_db
    ));

    if let Some(class_id) = query.class {
        qb.push(" AND class = ").push_bind(class_id);
    }

    if let Some(search) = query.search.filter(|v| !v.trim().is_empty()) {
        qb.push(" AND name LIKE ")
            .push_bind(format!("%{}%", search.trim()));
    }

    qb.push(" ORDER BY entry ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.acore_pool)
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

    Ok(Json(ItemListResponse {
        limit,
        offset,
        items,
    }))
}

async fn item_by_entry_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entry): Path<u32>,
) -> Result<Json<ItemSummary>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "items:read")?;

    let sql = format!(
        "SELECT entry, name, Quality, ItemLevel, class, subclass, displayid FROM {}.item_template WHERE entry = ? LIMIT 1",
        state.config.world_db
    );

    let row = sqlx::query(&sql)
        .bind(entry)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load item", err))?
        .ok_or_else(|| ApiError::not_found("Item not found", "ITEM_NOT_FOUND"))?;

    Ok(Json(ItemSummary {
        entry: row_get::<u32>(&row, "entry")?,
        name: row_get::<String>(&row, "name")?,
        quality: row_get::<u8>(&row, "Quality")?,
        item_level: row_get::<u32>(&row, "ItemLevel")?,
        class: row_get::<u8>(&row, "class")?,
        subclass: row_get::<u8>(&row, "subclass")?,
        display_id: row_get::<u32>(&row, "displayid")?,
    }))
}

async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<AuthContext, ApiError> {
    let token = extract_bearer_token(headers)?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[state.jwt.issuer.as_str()]);
    validation.set_audience(&[state.jwt.audience.as_str()]);
    validation.required_spec_claims = ["exp", "iat", "nbf", "iss", "aud", "sub"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    validation.leeway = 0;
    validation.validate_nbf = true;

    let decoded = decode::<JwtClaims>(&token, &state.jwt.decoding_key, &validation)
        .map_err(map_jwt_error_to_api)?;
    if decoded.claims.sub == 0 {
        return Err(ApiError::unauthorized(
            "Invalid token subject",
            "INVALID_TOKEN",
        ));
    }

    if decoded.claims.jti.trim().is_empty() {
        return Err(ApiError::unauthorized("Invalid token", "INVALID_TOKEN"));
    }

    if is_access_token_revoked(state, &decoded.claims.jti).await? {
        return Err(ApiError::unauthorized("Token revoked", "TOKEN_REVOKED"));
    }

    load_auth_context(state, decoded.claims.sub, Some(decoded.claims.jti)).await
}

fn require_permission(auth: &AuthContext, permission: &str) -> Result<(), ApiError> {
    if has_permission(auth, permission) {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "You are not allowed to perform this action",
        "RBAC_FORBIDDEN",
    ))
}

fn has_permission(auth: &AuthContext, permission: &str) -> bool {
    auth.permissions.iter().any(|p| p == permission)
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
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

fn map_jwt_error_to_api(err: jsonwebtoken::errors::Error) -> ApiError {
    match err.kind() {
        ErrorKind::ExpiredSignature => ApiError::unauthorized("Token expired", "TOKEN_EXPIRED"),
        ErrorKind::InvalidToken
        | ErrorKind::InvalidIssuer
        | ErrorKind::InvalidAudience
        | ErrorKind::InvalidSignature
        | ErrorKind::ImmatureSignature => ApiError::unauthorized("Invalid token", "INVALID_TOKEN"),
        _ => ApiError::unauthorized("Authentication failed", "AUTH_FAILED"),
    }
}

fn issue_jwt(
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

struct RbacPolicy {
    roles: Vec<String>,
    permissions: Vec<String>,
}

fn build_rbac_policy(gm_level: u8) -> RbacPolicy {
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

fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

fn read_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(255).collect())
}

async fn store_new_refresh_token(
    state: &AppState,
    account_id: u64,
    refresh_token: &str,
    family_id: Option<&str>,
    parent_token_id: Option<i64>,
    client_ip: &str,
    user_agent: Option<&str>,
) -> Result<i64, ApiError> {
    let generated_family = Uuid::new_v4().to_string();
    let family = family_id.unwrap_or(generated_family.as_str());
    let mut tx = state
        .app_pool
        .begin()
        .await
        .map_err(|err| map_db_error("failed to start refresh insert transaction", err))?;
    let id = insert_refresh_token_with_family(
        &mut tx,
        state,
        account_id,
        refresh_token,
        family,
        parent_token_id,
        client_ip,
        user_agent,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|err| map_db_error("failed to commit refresh insert transaction", err))?;
    Ok(id)
}

async fn insert_refresh_token_with_family(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    account_id: u64,
    refresh_token: &str,
    family_id: &str,
    parent_token_id: Option<i64>,
    client_ip: &str,
    user_agent: Option<&str>,
) -> Result<i64, ApiError> {
    let token_hash = hash_refresh_token(refresh_token);
    let account_id_i64 = i64::try_from(account_id)
        .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;
    let refresh_days = i64::try_from(state.jwt.refresh_expires_days).unwrap_or(30);
    let sql = format!(
        "INSERT INTO {}.auth_refresh_tokens \
         (account_id, token_hash, family_id, parent_token_id, expires_at, created_ip, user_agent) \
         VALUES ($1, $2, $3, $4, NOW() + ($5::BIGINT * INTERVAL '1 day'), $6, $7) \
         RETURNING id",
        APP_PG_SCHEMA
    );
    let inserted_id = sqlx::query_scalar::<_, i64>(&sql)
        .bind(account_id_i64)
        .bind(token_hash)
        .bind(family_id)
        .bind(parent_token_id)
        .bind(refresh_days)
        .bind(client_ip)
        .bind(user_agent)
        .fetch_one(&mut **tx)
        .await
        .map_err(|err| map_db_error("failed to insert refresh token", err))?;

    Ok(inserted_id)
}

async fn revoke_refresh_family(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    family_id: &str,
    reason: &str,
) -> Result<(), ApiError> {
    let sql = format!(
        "UPDATE {}.auth_refresh_tokens SET revoked_at = NOW(), reason = $1 \
         WHERE family_id = $2 AND revoked_at IS NULL",
        APP_PG_SCHEMA
    );
    sqlx::query(&sql)
        .bind(reason)
        .bind(family_id)
        .execute(&mut **tx)
        .await
        .map_err(|err| map_db_error("failed to revoke refresh family", err))?;
    Ok(())
}

async fn revoke_refresh_token_for_account(
    state: &AppState,
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
        .execute(&state.app_pool)
        .await
        .map_err(|err| map_db_error("failed to revoke refresh token", err))?;
    Ok(())
}

async fn is_access_token_revoked(state: &AppState, jti: &str) -> Result<bool, ApiError> {
    let sql = format!(
        "SELECT jti FROM {}.auth_revoked_access_tokens WHERE jti = $1 AND expires_at > NOW() LIMIT 1",
        APP_PG_SCHEMA
    );

    let row = sqlx::query_scalar::<_, String>(&sql)
        .bind(jti)
        .fetch_optional(&state.app_pool)
        .await
        .map_err(|err| map_db_error("failed to check access token revocation", err))?;

    Ok(row.is_some())
}

async fn revoke_access_jti(
    state: &AppState,
    account_id: u64,
    jti: &str,
    reason: &str,
) -> Result<(), ApiError> {
    let account_id_i64 = i64::try_from(account_id)
        .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;
    let expiry_minutes = i64::try_from(state.jwt.expires_minutes).unwrap_or(15);
    let sql = format!(
        "INSERT INTO {}.auth_revoked_access_tokens (jti, account_id, expires_at, reason) \
         VALUES ($1, $2, NOW() + ($3::BIGINT * INTERVAL '1 minute'), $4) \
         ON CONFLICT (jti) DO UPDATE SET revoked_at = CURRENT_TIMESTAMP, reason = EXCLUDED.reason",
        APP_PG_SCHEMA
    );

    sqlx::query(&sql)
        .bind(jti)
        .bind(account_id_i64)
        .bind(expiry_minutes)
        .bind(reason)
        .execute(&state.app_pool)
        .await
        .map_err(|err| map_db_error("failed to revoke access token", err))?;
    Ok(())
}

async fn load_auth_context(
    state: &AppState,
    account_id: u64,
    access_jti: Option<String>,
) -> Result<AuthContext, ApiError> {
    let sql = format!(
        "SELECT username, locked FROM {}.account WHERE id = ? LIMIT 1",
        state.config.auth_db
    );

    let row = sqlx::query(&sql)
        .bind(account_id)
        .fetch_optional(&state.acore_pool)
        .await
        .map_err(|err| map_db_error("failed to load auth account", err))?
        .ok_or_else(|| ApiError::unauthorized("Account not found", "ACCOUNT_NOT_FOUND"))?;

    let locked = row_get_bool(&row, "locked")?;
    if locked {
        return Err(ApiError::unauthorized(
            "Account is locked",
            "ACCOUNT_LOCKED",
        ));
    }

    let username: String = row_get(&row, "username")?;
    let gm_level = account_gm_level(state, account_id).await;
    let policy = build_rbac_policy(gm_level);

    Ok(AuthContext {
        account_id,
        access_jti: access_jti.unwrap_or_default(),
        username,
        gm_level,
        roles: policy.roles,
        permissions: policy.permissions,
    })
}

async fn account_gm_level(state: &AppState, account_id: u64) -> u8 {
    let gm_level_sql = format!(
        "SELECT COALESCE(MAX(gmlevel), 0) AS gmlevel FROM {}.account_access WHERE id = ?",
        state.config.auth_db
    );

    sqlx::query_scalar::<_, u8>(&gm_level_sql)
        .bind(account_id)
        .fetch_one(&state.acore_pool)
        .await
        .unwrap_or(0)
}

fn read_client_ip(headers: &HeaderMap) -> String {
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

fn normalize_username(username: &str) -> Result<String, ApiError> {
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

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 || password.len() > 64 {
        return Err(ApiError::bad_request(
            "Password must be between 8 and 64 characters",
            "INVALID_PASSWORD_LENGTH",
        ));
    }

    Ok(())
}

fn map_db_error(context: &'static str, err: sqlx::Error) -> ApiError {
    error!(error = %err, "{context}");
    ApiError::internal("Database operation failed", "DB_OPERATION_FAILED")
}

fn row_get<T>(row: &MySqlRow, column: &str) -> Result<T, ApiError>
where
    T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
    row.try_get(column)
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))
}

fn row_get_opt<T>(row: &MySqlRow, column: &str) -> Result<Option<T>, ApiError>
where
    T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
    row.try_get(column)
        .map_err(|_| ApiError::internal("Invalid database row format", "ROW_DECODE_FAILED"))
}

fn row_get_bool(row: &MySqlRow, column: &str) -> Result<bool, ApiError> {
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

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix time should be valid")
        .as_secs()
}

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
    let up = format!("{}:{}", username.to_uppercase(), password);
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

fn srp6_register(username: &str, password: &str, core5: bool) -> (Vec<u8>, Vec<u8>) {
    let mut salt = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);

    let verifier = srp6_calculate_verifier(username, password, &salt, core5);
    (salt, verifier)
}

fn srp6_verify(username: &str, password: &str, salt: &[u8], verifier: &[u8], core5: bool) -> bool {
    let expected = srp6_calculate_verifier(username, password, salt, core5);
    expected == verifier
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, header::AUTHORIZATION},
    };
    use http_body_util::BodyExt;
    use serde_json::json;
    use std::env;
    use tower::ServiceExt;

    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAy4YTCsiShT5nN0rrfStqzQ6M3xqf+WjKs64+h0cYf9fbQ3VI
Oc2YDxY8N7fMu6D7fQ6x7Qj9Rj6kD8VcC9jvvl/mvUgb6q+o2f7k8uio1jZjWzbl
xO5X4iYh6IfgqQ6/4wMIEtA8Tqh1sY3K+2p9W0nVAEeC4QkK6WfZmgmY9RE1aT9Y
DN8ixfB7uZ9r3JYVQ4hw3m3XLx9eQ7aP9SGlhYif9L1D6n2dZq3+mE8xFMeQG2cL
4Q1g1jFQ71BmyWQ2bS8m2DB9lWlQz5ycS5aZz5QLCz8xC7r8V1vJ+z9+G8+4S/pv
xQ7fVnFz7zS86qH4sI8n7+eF9K0yZ54Gmx6U3QIDAQABAoIBABG7vO7T4+OXhW1R
MxN+63vD/1TS8V4Q0Z0gbiDg9s7EJ6iS3FzTWnJw0CGYdV7VX9f8WfA2n8E2ym4a
7q6x9+W2g1jA2HkRPKf6fWjE1D/V6Y2pV2J2gpxnA3S2m8w/1V0vQ0h3WQx3XivW
mV4y4rIzqv6Y6P0C0l3l5WlI6m9QfQ2Y6+4o9f+W4Q7u0vV2hG3Q/2LzS8Y7vZ3K
M8d3JQf6s7K7byuI3kT3b9E6v2qQ7P7vXw5v9xQx6x0lP1zYB2eJ9u9c6T4J3r9A
4FQv7s8W9XWQw5D1Yf4V5m0Q9g0VfYH3oY2q5XQ5LxW5gY8v0S2Q8D8lJwV6Qf3T
O8s0g2ECgYEA8fVwM7tL4L4D3qM4YQjB5i1mLQwKp9QmB4jvQh2sWJq4Jm8J5j2g
m3rP2kKpQbA4mM6g0s8VY5mNQ0Q7W9g5aKx6QzXJQ6Y4s6M6Q2K1P4rLx0Y0wM3Q
A0zJ8yTq2C8dWm1sK9n+gL2QvV5J8A1K4n7D3QyQK2n6fQm1R5T8g2sCgYEA1y3S
b4H0s9Yx6f2uR0Qx0w1f1a3t6v9L7fQx1qS8Y2V6y4r5u8b3p9f6t2v4d8w1Q4b2
6g3m9k2z8q5v7t1w4y3m2r9s8u1v4x6z9b2m5n8q1r4t7y0u3i6o9p2a5s8d1f4h
7j0k3l6m9n2p5q8r1s4t7u0v3w6x9y2z5a8b1c4d7e0f3g6h9i2j5k8l1m4n7o0p
Q1R2S3T4U5V6W7X8Y9Z0CgYEAjE2x0iXWfM8s+qM6f6mX2a2J6pM8qK9pV0qU2aV
7wK8M6mQ2oX0K4cP9N2vH6fX5vM9yJ3uQ8kP4mN2gV7bS9mQ3cV4pM8qJ2lN6uR0
aP3mX7sQ2vK9mN4gV1bS6mQ5cV8pM2qJ7lN3uR1aP4mX8sQ3vK0mN5gV2bS7mQ6c
V9pM3qJ8lN4uR2aP5mX9sQ4vK1mN6gV3bS8mQ7cV0pM4qJ9lN5uR3aP6mX0sQ5vK
2mN7gV4bS9mQ8cV1pM5qJ0lN6uR4aP7mX1sQ6vK3mN8gV5bS0mQ9cV2pM6qJ1lN7
uR5aP8mX2sQ7vK4CgYEAq1y3z5x7c9v1b3n5m7k9j1h3g5f7d9s1a3q5w7e9r1t3
y5u7i9o1p3a5s7d9f1g3h5j7k9l1z3x5c7v9b1n3m5k7j9h1g3f5d7s9a1q3w5e7
r9t1y3u5i7o9p1a3s5d7f9g1h3j5k7l9z1x3c5v7b9n1m3k5j7h9g1f3d5s7a9q1
w3e5r7t9y1u3i5o7p9a1s3d5f7g9h1j3k5l7z9x1c3v5b7n9m1k3j5h7g9f1d3s5
A1Q2W3E4R5T6Y7U8I9O0CgYEAo0P9l8K7j6H5g4F3d2S1a0Q9w8E7r6T5y4U3i2O
p1A0s9D8f7G6h5J4k3L2z1X0c9V8b7N6m5K4j3H2g1F0d9S8a7Q6w5E4r3T2y1U0
i9O8p7A6s5D4f3G2h1J0k9L8z7X6c5V4b3N2m1K0j9H8g7F6d5S4a3Q2w1E0r9T8
y7U6i5O4p3A2s1D0f9G8h7J6k5L4z3X2c1V0b9N8m7K6j5H4g3F2d1S0a9Q8w7E6
r5T4y3U2i1O0p9A8s7D6f5G4h3J2k1L0z9X8c7V6b5N4m3K2j1H0g9F8d7S6a5Q4
-----END RSA PRIVATE KEY-----"#;
    const TEST_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAy4YTCsiShT5nN0rrfStq
zQ6M3xqf+WjKs64+h0cYf9fbQ3VIOc2YDxY8N7fMu6D7fQ6x7Qj9Rj6kD8VcC9jv
vl/mvUgb6q+o2f7k8uio1jZjWzblxO5X4iYh6IfgqQ6/4wMIEtA8Tqh1sY3K+2p9
W0nVAEeC4QkK6WfZmgmY9RE1aT9YDN8ixfB7uZ9r3JYVQ4hw3m3XLx9eQ7aP9SGl
hYif9L1D6n2dZq3+mE8xFMeQG2cL4Q1g1jFQ71BmyWQ2bS8m2DB9lWlQz5ycS5aZ
z5QLCz8xC7r8V1vJ+z9+G8+4S/pvxQ7fVnFz7zS86qH4sI8n7+eF9K0yZ54Gmx6U
3QIDAQAB
-----END PUBLIC KEY-----"#;

    fn test_jwt_config() -> JwtConfig {
        JwtConfig {
            encoding_key: EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY.as_bytes()).expect("private"),
            decoding_key: DecodingKey::from_rsa_pem(TEST_PUBLIC_KEY.as_bytes()).expect("public"),
            issuer: "wow-backend-test".to_string(),
            audience: "wow-web-test".to_string(),
            expires_minutes: 15,
            refresh_expires_days: 30,
        }
    }

    fn test_state() -> AppState {
        let acore_pool = MySqlPoolOptions::new()
            .connect_lazy("mysql://root:password@127.0.0.1:3306")
            .expect("lazy pool to build");
        let app_pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:password@127.0.0.1:5432/wow_app")
            .expect("lazy pg pool to build");

        AppState {
            acore_pool,
            app_pool,
            config: AppConfig {
                auth_db: "acore_auth".to_string(),
                characters_db: "acore_characters".to_string(),
                world_db: "acore_world".to_string(),
                srp6_core5_mode: false,
            },
            jwt: test_jwt_config(),
        }
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let app = build_router(test_state());

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
        let app = build_router(test_state());

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
        let mut headers = HeaderMap::new();
        assert!(extract_bearer_token(&headers).is_err());

        headers.insert(AUTHORIZATION, "Basic abc".parse().expect("header"));
        assert!(extract_bearer_token(&headers).is_err());

        headers.insert(AUTHORIZATION, "Bearer".parse().expect("header"));
        assert!(extract_bearer_token(&headers).is_err());
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_token() {
        let app = build_router(test_state());

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
        let app = build_router(test_state());

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
        assert_eq!(body["error_code"], "INVALID_TOKEN");
    }

    #[tokio::test]
    async fn auth_refresh_requires_payload_token() {
        let app = build_router(test_state());
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
        let app = build_router(test_state());
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

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body to collect")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("valid json")
    }

    async fn integration_state_from_env() -> Option<AppState> {
        let acore_database_url = env::var("TEST_AZEROTH_CORE_MYSQL_DATABASE_URL").ok()?;
        let app_database_url = env::var("TEST_APP_POSTGRES_DATABASE_URL").ok()?;
        let auth_db =
            env::var("TEST_AZEROTH_CORE_AUTH_DB").unwrap_or_else(|_| "acore_auth_test".to_string());
        let acore_pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&acore_database_url)
            .await
            .ok()?;
        let app_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&app_database_url)
            .await
            .ok()?;

        let create_db_sql = format!("CREATE DATABASE IF NOT EXISTS {auth_db}");
        sqlx::query(&create_db_sql)
            .execute(&acore_pool)
            .await
            .ok()?;

        let state = AppState {
            acore_pool,
            app_pool,
            config: AppConfig {
                auth_db,
                characters_db: "acore_characters".to_string(),
                world_db: "acore_world".to_string(),
                srp6_core5_mode: false,
            },
            jwt: test_jwt_config(),
        };

        run_app_migrations(&state.app_pool).await.ok()?;
        Some(state)
    }

    #[tokio::test]
    #[ignore = "requires TEST_AZEROTH_CORE_MYSQL_DATABASE_URL and TEST_APP_POSTGRES_DATABASE_URL"]
    async fn integration_refresh_token_store_and_revoke() {
        let state = integration_state_from_env()
            .await
            .expect("integration db state");

        let refresh_token = generate_refresh_token();
        let token_id = store_new_refresh_token(
            &state,
            999001,
            &refresh_token,
            None,
            None,
            "127.0.0.1",
            Some("integration-test"),
        )
        .await
        .expect("store refresh");

        assert!(token_id > 0);

        let count_sql = format!(
            "SELECT COUNT(*) AS total FROM {}.auth_refresh_tokens WHERE id = $1 AND revoked_at IS NULL",
            APP_PG_SCHEMA
        );
        let count = sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(token_id)
            .fetch_one(&state.app_pool)
            .await
            .expect("count query");
        assert_eq!(count, 1);

        revoke_refresh_token_for_account(&state, 999001, &refresh_token, "integration")
            .await
            .expect("revoke refresh");

        let revoked_count_sql = format!(
            "SELECT COUNT(*) AS total FROM {}.auth_refresh_tokens WHERE id = $1 AND revoked_at IS NOT NULL",
            APP_PG_SCHEMA
        );
        let revoked = sqlx::query_scalar::<_, i64>(&revoked_count_sql)
            .bind(token_id)
            .fetch_one(&state.app_pool)
            .await
            .expect("revoked count query");
        assert_eq!(revoked, 1);
    }

    #[tokio::test]
    #[ignore = "requires TEST_AZEROTH_CORE_MYSQL_DATABASE_URL and TEST_APP_POSTGRES_DATABASE_URL"]
    async fn integration_access_token_revocation_roundtrip() {
        let state = integration_state_from_env()
            .await
            .expect("integration db state");

        let jti = Uuid::new_v4().to_string();
        assert!(
            !is_access_token_revoked(&state, &jti)
                .await
                .expect("check before")
        );

        revoke_access_jti(&state, 999002, &jti, "integration")
            .await
            .expect("revoke access jti");

        assert!(
            is_access_token_revoked(&state, &jti)
                .await
                .expect("check after")
        );
    }
}
