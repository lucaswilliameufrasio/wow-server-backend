use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone)]
pub struct AppConfig {
    pub auth_db: String,
    pub characters_db: String,
    pub world_db: String,
    pub srp6_core5_mode: bool,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub encoding_key: jsonwebtoken::EncodingKey,
    pub decoding_key: jsonwebtoken::DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub expires_minutes: u64,
    pub refresh_expires_days: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: u64,
    pub jti: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub gm_level: u8,
    pub iss: String,
    pub aud: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub account_id: u64,
    pub access_jti: String,
    pub username: String,
    pub gm_level: u8,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

pub struct RbacPolicy {
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct HealthCheckResponse {
    pub message: &'static str,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RegisterResponse {
    pub account_id: u64,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SignInRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignInResponse {
    pub account_id: u64,
    pub username: String,
    pub email: Option<String>,
    pub gm_level: u8,
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in_seconds: u64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub refresh_token: String,
    pub refresh_expires_in_seconds: u64,
}

#[derive(Serialize)]
pub struct AuthMeResponse {
    pub account_id: u64,
    pub username: String,
    pub gm_level: u8,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in_seconds: u64,
    pub refresh_token: String,
    pub refresh_expires_in_seconds: u64,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct CharacterSummary {
    pub guid: u64,
    pub name: String,
    pub race: u8,
    #[serde(rename = "class")]
    pub class_id: u8,
    pub gender: u8,
    pub level: u8,
    pub map: u16,
    pub zone: u32,
    pub online: bool,
    pub money: u64,
}

#[derive(Serialize)]
pub struct CharacterListResponse {
    pub account_id: u64,
    pub characters: Vec<CharacterSummary>,
}

#[derive(Clone, Serialize)]
pub struct CharacterLocationResponse {
    pub guid: u64,
    pub name: String,
    pub map: u16,
    pub zone: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub online: bool,
}

#[derive(Deserialize)]
pub struct AdminPlayersQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub search: Option<String>,
    pub online: Option<bool>,
}

#[derive(Serialize)]
pub struct AdminPlayerSummary {
    pub id: u64,
    pub username: String,
    pub email: Option<String>,
    pub joined_unix: Option<u64>,
    pub last_login_unix: Option<u64>,
    pub last_ip: Option<String>,
    pub locked: bool,
    pub account_online: bool,
    pub gm_level: u8,
    pub character_count: u32,
}

#[derive(Serialize)]
pub struct AdminPlayersResponse {
    pub limit: u32,
    pub offset: u32,
    pub players: Vec<AdminPlayerSummary>,
}

#[derive(Serialize)]
pub struct OnlinePlayerSummary {
    pub account_id: u64,
    pub username: String,
    pub guid: u64,
    pub name: String,
    pub level: u8,
    pub map: u16,
    pub zone: u32,
}

#[derive(Serialize)]
pub struct AdminAccountLocationsResponse {
    pub account_id: u64,
    pub username: String,
    pub locations: Vec<CharacterLocationResponse>,
}

#[derive(Deserialize)]
pub struct SetAccountLockRequest {
    pub locked: bool,
}

#[derive(Serialize)]
pub struct AccountLockResponse {
    pub account_id: u64,
    pub locked: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DiagnosticsResponse {
    pub app_version: &'static str,
    pub rust_version: &'static str,
    pub uptime_seconds: u64,
}

#[derive(Deserialize)]
pub struct ItemQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub search: Option<String>,
    pub class: Option<u8>,
}

#[derive(Clone, Serialize)]
pub struct ItemSummary {
    pub entry: u32,
    pub name: String,
    pub quality: u8,
    pub item_level: u32,
    pub class: u8,
    pub subclass: u8,
    pub display_id: u32,
}

#[derive(Serialize)]
pub struct ItemListResponse {
    pub limit: u32,
    pub offset: u32,
    pub items: Vec<ItemSummary>,
}

#[derive(Clone)]
pub struct SignInAccountRow {
    pub id: u64,
    pub username: String,
    pub email: Option<String>,
    pub salt: Vec<u8>,
    pub verifier: Vec<u8>,
    pub locked: bool,
}

#[derive(Clone)]
pub struct AuthAccountRow {
    pub username: String,
    pub locked: bool,
}

pub struct RefreshTokenRow {
    pub id: i64,
    pub account_id: u64,
    pub family_id: String,
    pub is_revoked: bool,
    pub is_expired: bool,
}
