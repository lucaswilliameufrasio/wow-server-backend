use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
};
use std::sync::Arc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
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
use serde_json::json;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_handler,
        register_handler,
        sign_in_handler,
        refresh_token_handler,
        logout_handler,
        auth_me_handler,
        characters_handler,
        character_location_handler,
        items_handler,
        item_by_entry_handler,
        diagnostics_handler,
        admin_players_handler,
        admin_player_locations_handler,
        admin_lock_player_handler,
        admin_online_players_handler,
        admin_create_service_token_handler,
        admin_list_service_tokens_handler,
        admin_revoke_service_token_handler,
        admin_audit_log_handler,
        admin_server_status_handler,
        admin_announce_handler,
        admin_server_restart_handler,
        admin_kick_player_handler,
        admin_ban_account_handler,
        admin_unban_account_handler,
        admin_gm_command_handler,
        admin_teleport_player_handler,
        admin_give_item_handler,
        admin_modify_money_handler,
        admin_set_level_handler,
        admin_server_logs_handler,
        admin_server_crashes_handler,
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
            CreateServiceTokenRequest,
            CreateServiceTokenResponse,
            ServiceTokenSummary,
            ServiceTokenListResponse,
            SetAccountLockRequest,
            AccountLockResponse,
            AuditLogQuery,
            AuditLogEntry,
            AuditLogListResponse,
            AnnounceRequest,
            RestartServerRequest,
            KickPlayerRequest,
            BanAccountRequest,
            UnbanAccountRequest,
            GmCommandRequest,
            SoapCommandResponse,
            TeleportPlayerRequest,
            GiveItemRequest,
            ModifyMoneyRequest,
            SetLevelRequest,
            LogTailResponse,
            RefreshTokenRequest,
            RefreshTokenResponse,
            LogoutRequest,
            AuthMeResponse,
            CharacterSummary,
            CharacterListResponse,
            CharacterLocationResponse,
            AdminPlayersQuery,
            AdminPlayerSummary,
            AdminPlayersResponse,
            OnlinePlayerSummary,
            AdminAccountLocationsResponse,
            ItemQuery,
            ItemSummary,
            ItemListResponse,
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
    let debug_enabled = state.config.debug_enabled;

    let public = Router::new()
        .route("/health-check", get(health_check_handler))
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
        .route(
            "/v1/admin/service-tokens",
            get(admin_list_service_tokens_handler),
        )
        .route(
            "/v1/admin/service-tokens",
            post(admin_create_service_token_handler),
        )
        .route(
            "/v1/admin/service-tokens/{token_id}",
            delete(admin_revoke_service_token_handler),
        )
        .route("/v1/admin/audit-log", get(admin_audit_log_handler))
        .route("/v1/admin/server/status", post(admin_server_status_handler))
        .route("/v1/admin/server/announce", post(admin_announce_handler))
        .route(
            "/v1/admin/server/restart",
            post(admin_server_restart_handler),
        )
        .route("/v1/admin/server/command", post(admin_gm_command_handler))
        .route("/v1/admin/players/kick", post(admin_kick_player_handler))
        .route("/v1/admin/accounts/ban", post(admin_ban_account_handler))
        .route(
            "/v1/admin/accounts/unban",
            post(admin_unban_account_handler),
        )
        .route(
            "/v1/admin/players/teleport",
            post(admin_teleport_player_handler),
        )
        .route("/v1/admin/players/items", post(admin_give_item_handler))
        .route("/v1/admin/players/money", post(admin_modify_money_handler))
        .route("/v1/admin/players/level", post(admin_set_level_handler))
        .route("/v1/admin/server/logs", get(admin_server_logs_handler))
        .route(
            "/v1/admin/server/crashes",
            get(admin_server_crashes_handler),
        );

    let public = if debug_enabled {
        public.route("/debug/diagnostics", get(diagnostics_handler))
    } else {
        public
    };

    let auth_public = Router::new()
        .route("/v1/auth/register", post(register_handler))
        .route("/v1/auth/sign-in", post(sign_in_handler));

    let auth_public = if state.config.rate_limit {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .key_extractor(SmartIpKeyExtractor)
                .per_second(1)
                .burst_size(5)
                .finish()
                .expect("governor config"),
        );
        auth_public.layer(GovernorLayer::new(governor_conf))
    } else {
        auth_public
    };

    let docs = Router::new().merge(
        utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", ApiDoc::openapi()),
    );

    public
        .merge(auth_public)
        .merge(docs)
        .with_state(state)
        .layer(axum::middleware::from_fn(
            crate::observability::track_request,
        ))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(30),
        ))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(64 * 1024))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PATCH,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
        .fallback(not_found_handler)
}

async fn write_audit(
    state: &AppState,
    auth: &AuthContext,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    details: serde_json::Value,
) {
    let actor = i64::try_from(auth.account_id).unwrap_or(0);
    let mut details = details;
    if let Some(obj) = details.as_object_mut() {
        obj.insert("actor_username".to_string(), json!(auth.username));
    }

    let entry = crate::repos::AuditEntryNew {
        actor_account_id: actor,
        action: action.to_string(),
        target_type: target_type.to_string(),
        target_id: target_id.map(|v| v.to_string()),
        details: Some(details.clone()),
    };

    if let Err(err) = state.audit.insert(entry).await {
        tracing::warn!(error = ?err, "failed to write audit log entry");
    }

    state.notifier.notify(crate::notify::AdminEvent {
        action,
        actor_username: &auth.username,
        actor_account_id: actor,
        target_type,
        target_id,
        details: &details,
    });
}

async fn require_permission_audited(
    state: &AppState,
    auth: &AuthContext,
    permission: &str,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
) -> Result<(), ApiError> {
    if has_permission(auth, permission) {
        return Ok(());
    }

    write_audit(
        state,
        auth,
        action,
        target_type,
        target_id,
        json!({ "denied": true }),
    )
    .await;

    Err(ApiError::forbidden(
        "You are not allowed to perform this action",
        "RBAC_FORBIDDEN",
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn openapi_json() -> serde_json::Value {
    serde_json::to_value(ApiDoc::openapi()).unwrap_or(serde_json::Value::Null)
}

pub fn build_metrics_router() -> Router {
    Router::new()
        .route("/metrics", get(metrics::metrics_handler))
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
        metrics::record_login("locked");
        return Err(ApiError::unauthorized(
            "Account is locked",
            "ACCOUNT_LOCKED",
        ));
    }

    if state.accounts.login_is_locked(account.id) {
        metrics::record_login("temp_locked");
        return Err(ApiError::unauthorized(
            "Too many failed attempts. Try again later.",
            "ACCOUNT_TEMP_LOCKED",
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
        metrics::record_login("failed");
        state.accounts.record_login_failure(account.id);
        state.accounts.increment_failed_logins(account.id).await?;
        return Err(ApiError::unauthorized(
            "Invalid credentials",
            "INVALID_CREDENTIALS",
        ));
    }

    metrics::record_login("success");
    state.accounts.reset_login_lockout(account.id);

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

    let auth =
        crate::middleware::load_auth_context(&state, token_row.account_id, None, None).await?;
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

#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 204, description = "Tokens revoked"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

#[utoipa::path(
    get,
    path = "/v1/auth/me",
    responses(
        (status = 200, description = "Current account context", body = AuthMeResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

#[utoipa::path(
    get,
    path = "/v1/characters",
    responses(
        (status = 200, description = "Characters owned by the account", body = CharacterListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

#[utoipa::path(
    get,
    path = "/v1/characters/{guid}/location",
    responses(
        (status = 200, description = "Character location", body = CharacterLocationResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Not the owner and missing characters:location:any", body = ErrorResponse),
        (status = 404, description = "Character not found", body = ErrorResponse),
    ),
    tag = "auth"
)]
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

#[utoipa::path(
    get,
    path = "/v1/admin/players",
    params(AdminPlayersQuery),
    responses(
        (status = 200, description = "Player accounts (paginated)", body = AdminPlayersResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Requires players:read", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_players_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminPlayersQuery>,
) -> Result<Json<AdminPlayersResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let limit = query.limit.unwrap_or(50).min(200);

    let (players, cursor) = state
        .accounts
        .search_players(query.search.as_deref(), query.online, limit, query.cursor)
        .await?;

    Ok(Json(AdminPlayersResponse {
        limit,
        cursor,
        players,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/admin/players/{account_id}/locations",
    responses(
        (status = 200, description = "Character locations for an account", body = AdminAccountLocationsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Requires players:read", body = ErrorResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
    ),
    tag = "admin"
)]
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

#[utoipa::path(
    patch,
    path = "/v1/admin/players/{account_id}/lock",
    request_body = SetAccountLockRequest,
    responses(
        (status = 200, description = "Account lock updated", body = AccountLockResponse),
        (status = 403, description = "Requires players:write", body = ErrorResponse),
        (status = 404, description = "Account not found", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_lock_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(account_id): Path<u64>,
    Json(body): Json<SetAccountLockRequest>,
) -> Result<Json<AccountLockResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission_audited(
        &state,
        &auth,
        "players:write",
        "account.lock",
        "account",
        Some(&account_id.to_string()),
    )
    .await?;

    let updated = state.accounts.update_lock(account_id, body.locked).await?;

    if !updated {
        return Err(ApiError::not_found(
            "Account not found",
            "ACCOUNT_NOT_FOUND",
        ));
    }

    let mut details = json!({ "locked": body.locked });
    if let Some(reason) = body.reason.as_deref().filter(|r| !r.trim().is_empty()) {
        details["reason"] = json!(reason);
    }
    write_audit(
        &state,
        &auth,
        "account.lock",
        "account",
        Some(&account_id.to_string()),
        details,
    )
    .await;

    Ok(Json(AccountLockResponse {
        account_id,
        locked: body.locked,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/admin/online-players",
    responses(
        (status = 200, description = "Characters currently online", body = [OnlinePlayerSummary]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Requires players:read", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_online_players_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<OnlinePlayerSummary>>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "players:read")?;

    let players = state.accounts.find_online_players().await?;
    Ok(Json(players))
}

#[utoipa::path(
    get,
    path = "/v1/items",
    params(ItemQuery),
    responses(
        (status = 200, description = "Item templates (paginated)", body = ItemListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Requires items:read", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn items_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ItemQuery>,
) -> Result<Json<ItemListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "items:read")?;

    let limit = query.limit.unwrap_or(50).min(200);
    let (items, cursor) = state.items.search(&query).await?;

    Ok(Json(ItemListResponse {
        limit,
        cursor,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/items/{entry}",
    responses(
        (status = 200, description = "Item template", body = ItemSummary),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Requires items:read", body = ErrorResponse),
        (status = 404, description = "Item not found", body = ErrorResponse),
    ),
    tag = "admin"
)]
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

#[utoipa::path(
    post,
    path = "/v1/admin/service-tokens",
    request_body = CreateServiceTokenRequest,
    responses(
        (status = 200, description = "Service token created (token shown only once)", body = CreateServiceTokenResponse),
        (status = 400, description = "Invalid name or expiry", body = ErrorResponse),
        (status = 403, description = "Requires tokens:manage", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_create_service_token_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateServiceTokenRequest>,
) -> Result<Json<CreateServiceTokenResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission_audited(
        &state,
        &auth,
        "tokens:manage",
        "service_token.create",
        "service_token",
        None,
    )
    .await?;

    let name = body.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::bad_request(
            "Token name must be 1-100 characters",
            "INVALID_TOKEN_NAME",
        ));
    }

    let expires_in_days = body.expires_in_days;
    if let Some(days) = expires_in_days
        && (days == 0 || days > 3650)
    {
        return Err(ApiError::bad_request(
            "expires_in_days must be between 1 and 3650",
            "INVALID_TOKEN_EXPIRY",
        ));
    }

    let created_by = i64::try_from(auth.account_id)
        .map_err(|_| ApiError::internal("Account ID overflow", "ACCOUNT_ID_OVERFLOW"))?;

    let mut secret = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut secret);
    let token = format!("{}{}", SERVICE_TOKEN_PREFIX, to_hex(&secret));
    let token_hash = hash_refresh_token(&token);

    let expires_at_unix =
        expires_in_days.map(|days| crate::auth::unix_now() + u64::from(days) * 86_400);

    let id = state
        .service_tokens
        .create(
            &token_hash,
            name,
            created_by,
            expires_at_unix.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
        )
        .await?;

    // The token value itself is never logged: only the name and expiry.
    write_audit(
        &state,
        &auth,
        "service_token.create",
        "service_token",
        Some(&id.to_string()),
        json!({ "name": name, "expires_at_unix": expires_at_unix }),
    )
    .await;

    Ok(Json(CreateServiceTokenResponse {
        id,
        name: name.to_string(),
        token,
        expires_at_unix,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/admin/service-tokens",
    responses(
        (status = 200, description = "List service tokens", body = ServiceTokenListResponse),
        (status = 403, description = "Requires tokens:manage", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_list_service_tokens_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ServiceTokenListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "tokens:manage")?;

    let rows = state.service_tokens.list().await?;

    let tokens = rows
        .into_iter()
        .map(|row| ServiceTokenSummary {
            id: row.id,
            name: row.name,
            created_by: u64::try_from(row.created_by).unwrap_or(0),
            created_at_unix: u64::try_from(row.created_at_unix).unwrap_or(0),
            expires_at_unix: row.expires_at_unix.map(|v| u64::try_from(v).unwrap_or(0)),
            revoked_at_unix: row.revoked_at_unix.map(|v| u64::try_from(v).unwrap_or(0)),
            last_used_at_unix: row.last_used_at_unix.map(|v| u64::try_from(v).unwrap_or(0)),
        })
        .collect();

    Ok(Json(ServiceTokenListResponse { tokens }))
}

fn soap_or_503(state: &AppState) -> Result<Arc<dyn crate::soap::SoapClient>, ApiError> {
    state.soap.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "WorldServer SOAP is not configured",
            "SOAP_NOT_CONFIGURED",
        )
    })
}

fn sanitize_one_line(value: &str, max: usize) -> Result<String, ApiError> {
    let cleaned: String = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        return Err(ApiError::bad_request("Text cannot be empty", "EMPTY_TEXT"));
    }
    if cleaned.chars().count() > max {
        return Err(ApiError::bad_request("Text is too long", "TEXT_TOO_LONG"));
    }
    Ok(cleaned)
}

fn validate_character_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.len() < 2 || name.len() > 12 || !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(ApiError::bad_request(
            "Invalid character name",
            "INVALID_CHARACTER_NAME",
        ));
    }
    Ok(name.to_string())
}

fn validate_location_name(location: &str) -> Result<String, ApiError> {
    let location = location.trim();
    let valid = !location.is_empty()
        && location.len() <= 100
        && location
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '\'' || c == '_');
    if !valid {
        return Err(ApiError::bad_request(
            "Invalid teleport location",
            "INVALID_LOCATION",
        ));
    }
    Ok(location.to_string())
}

pub(crate) fn read_tail_lines(
    path: &std::path::Path,
    max_lines: usize,
    max_bytes: u64,
) -> std::io::Result<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let read_len = len.min(max_bytes);
    file.seek(SeekFrom::End(-(read_len as i64)))?;
    let mut buf = vec![0u8; read_len as usize];
    file.read_exact(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.lines().collect();
    if read_len < len && !lines.is_empty() {
        lines.remove(0);
    }
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].iter().map(|l| l.to_string()).collect())
}

fn logs_dir_or_503(state: &AppState) -> Result<&str, ApiError> {
    state.config.acore_logs_dir.as_deref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AzerothCore logs directory is not configured",
            "LOGS_NOT_CONFIGURED",
        )
    })
}

async fn serve_log_tail(
    state: &AppState,
    file_name: &str,
    lines: u32,
) -> Result<Json<LogTailResponse>, ApiError> {
    if lines == 0 || lines > 1_000 {
        return Err(ApiError::bad_request(
            "lines must be between 1 and 1000",
            "INVALID_LINES",
        ));
    }

    let dir = logs_dir_or_503(state)?;
    let path = std::path::Path::new(dir).join(file_name);
    let log_lines = read_tail_lines(&path, lines as usize, 512 * 1024).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ApiError::not_found("Log file not found", "LOG_FILE_NOT_FOUND")
                .with_extra(json!({ "file": file_name }))
        } else {
            ApiError::internal("Failed to read log file", "LOG_READ_FAILED").with_extra(json!({
                "error": err.to_string()
            }))
        }
    })?;

    Ok(Json(LogTailResponse {
        file: file_name.to_string(),
        lines: log_lines,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/admin/server/logs",
    responses(
        (status = 200, description = "WorldServer log tail", body = LogTailResponse),
        (status = 400, description = "Invalid lines", body = ErrorResponse),
        (status = 403, description = "Requires server:read", body = ErrorResponse),
        (status = 404, description = "Log file not found", body = ErrorResponse),
        (status = 503, description = "Logs directory not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_server_logs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LogTailQuery>,
) -> Result<Json<LogTailResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "server:read")?;

    serve_log_tail(&state, "Server.log", query.lines.unwrap_or(200)).await
}

#[utoipa::path(
    get,
    path = "/v1/admin/server/crashes",
    responses(
        (status = 200, description = "WorldServer crash log tail", body = LogTailResponse),
        (status = 400, description = "Invalid lines", body = ErrorResponse),
        (status = 403, description = "Requires server:read", body = ErrorResponse),
        (status = 404, description = "Crash log not found", body = ErrorResponse),
        (status = 503, description = "Logs directory not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_server_crashes_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LogTailQuery>,
) -> Result<Json<LogTailResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "server:read")?;

    serve_log_tail(&state, "Crash.log", query.lines.unwrap_or(200)).await
}

fn money_format(copper: i64) -> String {
    let gold = copper.abs() / 10_000;
    let silver = (copper.abs() % 10_000) / 100;
    let rest = copper.abs() % 100;
    let sign = if copper < 0 { "-" } else { "" };
    format!("{sign}{gold}g{silver}s{rest}c")
}

fn validate_account_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 32 || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request(
            "Invalid account name",
            "INVALID_ACCOUNT_NAME",
        ));
    }
    Ok(name.to_string())
}

#[utoipa::path(
    post,
    path = "/v1/admin/players/teleport",
    request_body = TeleportPlayerRequest,
    responses(
        (status = 200, description = "Player teleported", body = SoapCommandResponse),
        (status = 400, description = "Invalid character or location", body = ErrorResponse),
        (status = 403, description = "Requires players:modify", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_teleport_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeleportPlayerRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let character = validate_character_name(&body.character_name)?;
    let location = validate_location_name(&body.location)?;
    require_permission_audited(
        &state,
        &auth,
        "players:modify",
        "player.teleport",
        "character",
        Some(&character),
    )
    .await?;

    let soap = soap_or_503(&state)?;
    let command = format!("tele name {character} {location}");
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "player.teleport",
        "character",
        Some(&character),
        json!({ "location": location }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/players/items",
    request_body = GiveItemRequest,
    responses(
        (status = 200, description = "Item given", body = SoapCommandResponse),
        (status = 400, description = "Invalid character, item or count", body = ErrorResponse),
        (status = 403, description = "Requires players:modify", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_give_item_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GiveItemRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let character = validate_character_name(&body.character_name)?;
    require_permission_audited(
        &state,
        &auth,
        "players:modify",
        "player.give_item",
        "character",
        Some(&character),
    )
    .await?;

    let count = body.count.unwrap_or(1);
    if body.item_entry == 0 || body.item_entry > 99_999_999 || count == 0 || count > 1_000 {
        return Err(ApiError::bad_request(
            "item_entry must be 1-99999999 and count 1-1000",
            "INVALID_ITEM_REQUEST",
        ));
    }

    let soap = soap_or_503(&state)?;
    let command = format!("additem name {character} {} {count}", body.item_entry);
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "player.give_item",
        "character",
        Some(&character),
        json!({ "item_entry": body.item_entry, "count": count }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/players/money",
    request_body = ModifyMoneyRequest,
    responses(
        (status = 200, description = "Money modified", body = SoapCommandResponse),
        (status = 400, description = "Invalid character or amount", body = ErrorResponse),
        (status = 403, description = "Requires players:modify", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_modify_money_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ModifyMoneyRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let character = validate_character_name(&body.character_name)?;
    require_permission_audited(
        &state,
        &auth,
        "players:modify",
        "player.modify_money",
        "character",
        Some(&character),
    )
    .await?;

    if body.amount == 0 || body.amount.abs() > 2_147_483_647 {
        return Err(ApiError::bad_request(
            "amount must be between -2147483647 and 2147483647 (non-zero copper)",
            "INVALID_MONEY_AMOUNT",
        ));
    }

    let soap = soap_or_503(&state)?;
    let command = format!(
        "modify money name {character} {}",
        money_format(body.amount)
    );
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "player.modify_money",
        "character",
        Some(&character),
        json!({ "amount_copper": body.amount }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/players/level",
    request_body = SetLevelRequest,
    responses(
        (status = 200, description = "Level set", body = SoapCommandResponse),
        (status = 400, description = "Invalid character or level", body = ErrorResponse),
        (status = 403, description = "Requires players:modify", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_set_level_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SetLevelRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let character = validate_character_name(&body.character_name)?;
    require_permission_audited(
        &state,
        &auth,
        "players:modify",
        "player.set_level",
        "character",
        Some(&character),
    )
    .await?;

    if body.level == 0 || body.level > 80 {
        return Err(ApiError::bad_request(
            "level must be between 1 and 80",
            "INVALID_LEVEL",
        ));
    }

    let soap = soap_or_503(&state)?;
    let command = format!("setlevel name {character} {}", body.level);
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "player.set_level",
        "character",
        Some(&character),
        json!({ "level": body.level }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/server/status",
    responses(
        (status = 200, description = "WorldServer status", body = SoapCommandResponse),
        (status = 403, description = "Requires server:read", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_server_status_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "server:read")?;

    let soap = soap_or_503(&state)?;
    let result = soap.execute("server info").await?;

    Ok(Json(SoapCommandResponse {
        command: "server info".to_string(),
        result,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/server/announce",
    request_body = AnnounceRequest,
    responses(
        (status = 200, description = "Announcement sent", body = SoapCommandResponse),
        (status = 400, description = "Invalid message", body = ErrorResponse),
        (status = 403, description = "Requires server:control", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_announce_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AnnounceRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission_audited(
        &state,
        &auth,
        "server:control",
        "server.announce",
        "server",
        None,
    )
    .await?;

    let message = sanitize_one_line(&body.message, 256)?;
    let soap = soap_or_503(&state)?;
    let command = format!("announce {message}");
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "server.announce",
        "server",
        None,
        json!({ "message": message }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/server/restart",
    request_body = RestartServerRequest,
    responses(
        (status = 200, description = "Restart scheduled", body = SoapCommandResponse),
        (status = 400, description = "Invalid delay", body = ErrorResponse),
        (status = 403, description = "Requires server:control", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_server_restart_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RestartServerRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission_audited(
        &state,
        &auth,
        "server:control",
        "server.restart",
        "server",
        None,
    )
    .await?;

    let delay = body.delay_seconds.unwrap_or(60);
    if delay == 0 || delay > 86_400 {
        return Err(ApiError::bad_request(
            "delay_seconds must be between 1 and 86400",
            "INVALID_RESTART_DELAY",
        ));
    }

    let soap = soap_or_503(&state)?;
    let command = format!("server restart {delay}");
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "server.restart",
        "server",
        None,
        json!({ "delay_seconds": delay }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/players/kick",
    request_body = KickPlayerRequest,
    responses(
        (status = 200, description = "Player kicked", body = SoapCommandResponse),
        (status = 400, description = "Invalid character name", body = ErrorResponse),
        (status = 403, description = "Requires players:kick", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_kick_player_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<KickPlayerRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let character = validate_character_name(&body.character_name)?;
    require_permission_audited(
        &state,
        &auth,
        "players:kick",
        "player.kick",
        "character",
        Some(&character),
    )
    .await?;

    let soap = soap_or_503(&state)?;
    let command = format!("kick {character}");
    let result = soap.execute(&command).await?;

    let mut details = json!({ "character_name": character });
    if let Some(reason) = body.reason.as_deref().filter(|r| !r.trim().is_empty()) {
        details["reason"] = json!(reason);
    }
    write_audit(
        &state,
        &auth,
        "player.kick",
        "character",
        Some(&character),
        details,
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/accounts/ban",
    request_body = BanAccountRequest,
    responses(
        (status = 200, description = "Account banned", body = SoapCommandResponse),
        (status = 400, description = "Invalid account or days", body = ErrorResponse),
        (status = 403, description = "Requires players:ban", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_ban_account_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BanAccountRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let account = validate_account_name(&body.account)?;
    require_permission_audited(
        &state,
        &auth,
        "players:ban",
        "account.ban",
        "account",
        Some(&account),
    )
    .await?;

    if body.days == 0 || body.days > 3650 {
        return Err(ApiError::bad_request(
            "days must be between 1 and 3650",
            "INVALID_BAN_DAYS",
        ));
    }

    let reason = body
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or("banned via API");

    let soap = soap_or_503(&state)?;
    let command = format!("ban account {account} {}d {reason}", body.days);
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "account.ban",
        "account",
        Some(&account),
        json!({ "days": body.days, "reason": reason }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/accounts/unban",
    request_body = UnbanAccountRequest,
    responses(
        (status = 200, description = "Account unbanned", body = SoapCommandResponse),
        (status = 400, description = "Invalid account", body = ErrorResponse),
        (status = 403, description = "Requires players:ban", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_unban_account_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UnbanAccountRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    let account = validate_account_name(&body.account)?;
    require_permission_audited(
        &state,
        &auth,
        "players:ban",
        "account.unban",
        "account",
        Some(&account),
    )
    .await?;

    let soap = soap_or_503(&state)?;
    let command = format!("unban account {account}");
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "account.unban",
        "account",
        Some(&account),
        json!({}),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/server/command",
    request_body = GmCommandRequest,
    responses(
        (status = 200, description = "Command executed", body = SoapCommandResponse),
        (status = 403, description = "Disabled or requires server:gm_command", body = ErrorResponse),
        (status = 503, description = "SOAP not configured", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_gm_command_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GmCommandRequest>,
) -> Result<Json<SoapCommandResponse>, ApiError> {
    if !state.config.gm_command_enabled {
        return Err(ApiError::forbidden(
            "GM command execution is disabled (set WOW_ENABLE_GM_COMMANDS=true)",
            "GM_COMMAND_DISABLED",
        ));
    }

    let auth = authenticate(&headers, &state).await?;
    let command = sanitize_one_line(&body.command, 256)?;
    require_permission_audited(
        &state,
        &auth,
        "server:gm_command",
        "server.gm_command",
        "server",
        None,
    )
    .await?;

    if let Some(allowlist) = &state.config.gm_command_allowlist
        && !allowlist.iter().any(|allowed| {
            let allowed = allowed.trim();
            command == *allowed || command.starts_with(&format!("{allowed} "))
        })
    {
        write_audit(
            &state,
            &auth,
            "server.gm_command",
            "server",
            None,
            json!({ "command": command, "denied": true, "reason": "not_in_allowlist" }),
        )
        .await;
        return Err(ApiError::forbidden(
            "Command is not in the allowlist",
            "COMMAND_NOT_ALLOWED",
        ));
    }

    let soap = soap_or_503(&state)?;
    let result = soap.execute(&command).await?;

    write_audit(
        &state,
        &auth,
        "server.gm_command",
        "server",
        None,
        json!({ "command": command }),
    )
    .await;

    Ok(Json(SoapCommandResponse { command, result }))
}

#[utoipa::path(
    delete,
    path = "/v1/admin/service-tokens/{token_id}",
    responses(
        (status = 204, description = "Service token revoked"),
        (status = 403, description = "Requires tokens:manage", body = ErrorResponse),
        (status = 404, description = "Token not found or already revoked", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_revoke_service_token_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission_audited(
        &state,
        &auth,
        "tokens:manage",
        "service_token.revoke",
        "service_token",
        Some(&token_id.to_string()),
    )
    .await?;

    let revoked = state.service_tokens.revoke(token_id).await?;

    if !revoked {
        return Err(ApiError::not_found(
            "Service token not found or already revoked",
            "SERVICE_TOKEN_NOT_FOUND",
        ));
    }

    write_audit(
        &state,
        &auth,
        "service_token.revoke",
        "service_token",
        Some(&token_id.to_string()),
        json!({ "token_id": token_id }),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/v1/admin/audit-log",
    responses(
        (status = 200, description = "Recent audit log entries", body = AuditLogListResponse),
        (status = 403, description = "Requires audit:read", body = ErrorResponse),
    ),
    tag = "admin"
)]
async fn admin_audit_log_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<AuditLogListResponse>, ApiError> {
    let auth = authenticate(&headers, &state).await?;
    require_permission(&auth, "audit:read")?;

    let limit = query.limit.unwrap_or(50).min(200);
    let rows = state.audit.list(limit, query.cursor).await?;

    let entries = rows
        .into_iter()
        .map(|row| AuditLogEntry {
            id: row.id,
            actor_account_id: u64::try_from(row.actor_account_id).unwrap_or(0),
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            details: row.details,
            created_at_unix: u64::try_from(row.created_at_unix).unwrap_or(0),
        })
        .collect();

    Ok(Json(AuditLogListResponse {
        limit,
        cursor: query.cursor,
        entries,
    }))
}
