use axum::http::HeaderMap;
use jsonwebtoken::{Algorithm, Validation, decode};

use crate::auth::*;
use crate::error::*;
use crate::models::*;
use crate::repos::*;

pub async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<AuthContext, ApiError> {
    let token = extract_bearer_token(headers)?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[state.jwt.issuer.as_str()]);
    validation.set_audience(&[state.jwt.audience.as_str()]);
    validation.required_spec_claims = ["exp", "iat", "nbf", "iss", "aud"]
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

    if state
        .refresh_tokens
        .is_access_revoked(&decoded.claims.jti)
        .await?
    {
        return Err(ApiError::unauthorized("Token revoked", "TOKEN_REVOKED"));
    }

    load_auth_context(state, decoded.claims.sub, Some(decoded.claims.jti)).await
}

pub fn require_permission(auth: &AuthContext, permission: &str) -> Result<(), ApiError> {
    if has_permission(auth, permission) {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "You are not allowed to perform this action",
        "RBAC_FORBIDDEN",
    ))
}

pub fn has_permission(auth: &AuthContext, permission: &str) -> bool {
    auth.permissions.iter().any(|p| p == permission)
}

pub(crate) async fn load_auth_context(
    state: &AppState,
    account_id: u64,
    access_jti: Option<String>,
) -> Result<AuthContext, ApiError> {
    let auth_row = state
        .accounts
        .find_auth(account_id)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Account not found", "ACCOUNT_NOT_FOUND"))?;

    if auth_row.locked {
        return Err(ApiError::unauthorized(
            "Account is locked",
            "ACCOUNT_LOCKED",
        ));
    }

    let gm_level = state.accounts.get_gm_level(account_id).await;
    let policy = build_rbac_policy(gm_level);

    Ok(AuthContext {
        account_id,
        access_jti: access_jti.unwrap_or_default(),
        username: auth_row.username,
        gm_level,
        roles: policy.roles,
        permissions: policy.permissions,
    })
}
