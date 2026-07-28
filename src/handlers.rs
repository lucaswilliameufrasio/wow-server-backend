use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post},
};
use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::auth::*;
use crate::error::*;
use crate::metrics;
use crate::middleware::*;
use crate::models::*;
use crate::repos::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_handler,
        register_handler,
        sign_in_handler,
        refresh_token_handler,
        diagnostics_handler,
    ),
    components(
        schemas(
            HealthCheckResponse,
            ErrorResponse,
            RegisterRequest,
            RegisterResponse,
            SignInRequest,
            SignInResponse,
            DiagnosticsResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "admin", description = "Admin-only endpoints"),
    ),
    modifiers(&SecurityAddon),
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

pub fn build_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health-check", get(health_check_handler))
        .route("/v1/auth/register", post(register_handler))
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
        .route("/debug/diagnostics", get(diagnostics_handler))
        .route("/metrics", get(metrics::metrics_handler));

    let sign_in = Router::new().route("/v1/auth/sign-in", post(sign_in_handler));

    let docs = Router::new().merge(
        utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", ApiDoc::openapi()),
    );

    public
        .merge(sign_in)
        .merge(docs)
        .with_state(state)
        .fallback(not_found_handler)
}

#[utoipa::path(
    get,
    path = "/health-check",
    responses(
        (status = 200, description = "Service is healthy", body = HealthCheckResponse),
    ),
    tag = "health"
)]
async fn health_check_handler() -> Json<HealthCheckResponse> {
    metrics::HEALTH_CHECKS.inc();
    Json(HealthCheckResponse { message: "ok" })
}

async fn not_found_handler() -> ApiError {
    ApiError::not_found("Route not found", "ROUTE_NOT_FOUND")
}

#[utoipa::path(
    post,
    path = "/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Account created", body = RegisterResponse),
        (status = 400, description = "Invalid username or password", body = ErrorResponse),
        (status = 409, description = "Username already exists", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

    if state.accounts.exists(&username).await? {
        return Err(ApiError::conflict(
            "Username already exists",
            "USERNAME_ALREADY_EXISTS",
        ));
    }

    let (salt, verifier) = srp6_register(&username, &body.password, state.config.srp6_core5_mode);
    let client_ip = read_client_ip(&headers);

    let account_id = state
        .accounts
        .create(&username, &salt, &verifier, email.as_deref(), &client_ip)
        .await?;

    Ok(Json(RegisterResponse {
        account_id,
        username,
        email,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/auth/sign-in",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "Sign in successful", body = SignInResponse),
        (status = 401, description = "Invalid credentials or account locked", body = ErrorResponse),
    ),
    tag = "auth"
)]
async fn sign_in_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApiError> {
    let username = normalize_username(&body.username)?;

    let account = state
        .accounts
        .find_by_username(&username)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Invalid credentials", "INVALID_CREDENTIALS"))?;

    if account.locked {
        return Err(ApiError::unauthorized(
            "Account is locked",
            "ACCOUNT_LOCKED",
        ));
    }

    let password_ok = srp6_verify(
        &account.username,
        &body.password,
        &account.salt,
        &account.verifier,
        state.config.srp6_core5_mode,
    );

    if !password_ok {
        state.accounts.increment_failed_logins(account.id).await?;
        return Err(ApiError::unauthorized(
            "Invalid credentials",
            "INVALID_CREDENTIALS",
        ));
    }

    let client_ip = read_client_ip(&headers);
    state
        .accounts
        .record_successful_login(account.id, &client_ip)
        .await?;

    let gm_level = state.accounts.get_gm_level(account.id).await;
    let policy = build_rbac_policy(gm_level);
    let token = issue_jwt(&state.jwt, account.id, &account.username, gm_level, &policy)?;
    let refresh_token = generate_refresh_token();
    let user_agent = read_user_agent(&headers);

    state
        .refresh_tokens
        .store(
            account.id,
            &refresh_token,
            None,
            None,
            &client_ip,
            user_agent.as_deref(),
        )
        .await?;

    Ok(Json(SignInResponse {
        account_id: account.id,
        username: account.username,
        email: account.email,
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

#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Tokens refreshed", body = RefreshTokenResponse),
        (status = 400, description = "Missing refresh token", body = ErrorResponse),
        (status = 401, description = "Invalid or revoked refresh token", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

    let token_row = state
        .refresh_tokens
        .find_by_hash(&refresh_hash)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Invalid refresh token", "INVALID_REFRESH_TOKEN"))?;

    if token_row.is_revoked || token_row.is_expired {
        state
            .refresh_tokens
            .revoke_family(
                &token_row.family_id,
                if token_row.is_expired {
                    "expired_refresh_reuse"
                } else {
                    "revoked_refresh_reuse"
                },
            )
            .await?;
        return Err(ApiError::unauthorized(
            "Refresh token is not valid",
            "INVALID_REFRESH_TOKEN",
        ));
    }

    let auth = crate::middleware::load_auth_context(&state, token_row.account_id, None).await?;
    let new_refresh_token = generate_refresh_token();

    let new_token_id = state
        .refresh_tokens
        .store(
            token_row.account_id,
            &new_refresh_token,
            Some(&token_row.family_id),
            Some(token_row.id),
            &client_ip,
            user_agent.as_deref(),
        )
        .await?;

    state
        .refresh_tokens
        .mark_rotated(token_row.id, new_token_id)
        .await?;

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
        refresh_token: new_refresh_token,
        refresh_expires_in_seconds: state.jwt.refresh_expires_days * 24 * 60 * 60,
    }))
}

async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<LogoutRequest>>,
) -> Result<StatusCode, ApiError> {
    let auth = authenticate(&headers, &state).await?;

    state
        .refresh_tokens
        .revoke_access(
            auth.account_id,
            &auth.access_jti,
            "logout",
            state.jwt.expires_minutes as i64,
        )
        .await?;

    if let Some(refresh_token) = body
        .and_then(|v| v.0.refresh_token)
        .filter(|t| !t.trim().is_empty())
    {
        state
            .refresh_tokens
            .revoke_for_account(auth.account_id, &refresh_token, "logout")
            .await?;
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
    let characters = state.characters.list_for_account(auth.account_id).await?;

    Ok(Json(CharacterListResponse {
        account_id: auth.account_id,
        characters,
    }))
}

async fn character_location_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<u64>,
) -> Result<Json<CharacterLocationResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;

    let owner_account = state
        .characters
        .find_owner(guid)
        .await?
        .ok_or_else(|| ApiError::not_found("Character not found", "CHARACTER_NOT_FOUND"))?;

    if owner_account != auth.account_id && !has_permission(&auth, "characters:location:any") {
        return Err(ApiError::forbidden(
            "You are not allowed to read this character location",
            "FORBIDDEN_CHARACTER_LOCATION",
        ));
    }

    let location = state
        .characters
        .find_location(guid)
        .await?
        .ok_or_else(|| ApiError::not_found("Character not found", "CHARACTER_NOT_FOUND"))?;

    Ok(Json(location))
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

    let players = state
        .accounts
        .search_players(query.search.as_deref(), query.online, limit, offset)
        .await?;

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

    let username = state
        .accounts
        .find_username(account_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Account not found", "ACCOUNT_NOT_FOUND"))?;

    let locations = state
        .characters
        .find_locations_by_account(account_id)
        .await?;

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

    let updated = state.accounts.update_lock(account_id, body.locked).await?;

    if !updated {
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

    let players = state.accounts.find_online_players().await?;
    Ok(Json(players))
}

async fn items_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ItemQuery>,
) -> Result<Json<ItemListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "items:read")?;

    let items = state.items.search(&query).await?;

    Ok(Json(ItemListResponse {
        limit: query.limit.unwrap_or(50).min(200),
        offset: query.offset.unwrap_or(0),
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

    let item = state
        .items
        .find_by_entry(entry)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found", "ITEM_NOT_FOUND"))?;

    Ok(Json(item))
}

#[utoipa::path(
    get,
    path = "/debug/diagnostics",
    responses(
        (status = 200, description = "Diagnostics info", body = DiagnosticsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    tag = "admin"
)]
async fn diagnostics_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DiagnosticsResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let uptime = crate::auth::unix_now().saturating_sub(state.started_at);

    Ok(Json(DiagnosticsResponse {
        app_version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
        uptime_seconds: uptime,
    }))
}
