use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Clone)]
pub struct AppConfig {
    pub auth_db: String,
    pub characters_db: String,
    pub world_db: String,
    pub srp6_core5_mode: bool,
    pub rate_limit: bool,
    pub debug_enabled: bool,
    pub soap: Option<SoapConfig>,
    pub gm_command_enabled: bool,
    pub gm_command_allowlist: Option<Vec<String>>,
    pub acore_logs_dir: Option<String>,
}

#[derive(Clone)]
pub struct SoapConfig {
    pub base_url: String,
    pub user: String,
    pub password: String,
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

#[derive(Serialize, ToSchema)]
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

#[derive(Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

#[derive(Clone, Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct CharacterListResponse {
    pub account_id: u64,
    pub characters: Vec<CharacterSummary>,
}

#[derive(Clone, Serialize, ToSchema)]
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

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct AdminPlayersQuery {
    pub limit: Option<u32>,
    pub cursor: Option<u64>,
    pub search: Option<String>,
    pub online: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminPlayerSummary {
    pub id: u64,
    pub username: String,
    pub email: Option<String>,
    pub joined_unix: Option<u64>,
    pub last_login_unix: Option<u64>,
    pub last_ip: Option<String>,
    pub locked: bool,
    pub online: bool,
    pub gm_level: u8,
    pub character_count: u32,
}

#[derive(Serialize, ToSchema)]
pub struct AdminPlayersResponse {
    pub limit: u32,
    pub cursor: Option<u64>,
    pub players: Vec<AdminPlayerSummary>,
}

#[derive(Serialize, ToSchema)]
pub struct OnlinePlayerSummary {
    pub account_id: u64,
    pub username: String,
    pub guid: u64,
    pub name: String,
    pub level: u8,
    pub map: u16,
    pub zone: u32,
}

#[derive(Serialize, ToSchema)]
pub struct AdminAccountLocationsResponse {
    pub account_id: u64,
    pub username: String,
    pub locations: Vec<CharacterLocationResponse>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetAccountLockRequest {
    pub locked: bool,
    pub reason: Option<String>,
}

#[derive(Serialize, ToSchema)]
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

#[derive(Deserialize, ToSchema)]
pub struct CreateServiceTokenRequest {
    pub name: String,
    pub expires_in_days: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateServiceTokenResponse {
    pub id: i64,
    pub name: String,
    pub token: String,
    pub expires_at_unix: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceTokenSummary {
    pub id: i64,
    pub name: String,
    pub created_by: u64,
    pub created_at_unix: u64,
    pub expires_at_unix: Option<u64>,
    pub revoked_at_unix: Option<u64>,
    pub last_used_at_unix: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceTokenListResponse {
    pub tokens: Vec<ServiceTokenSummary>,
}

#[derive(Deserialize, ToSchema)]
pub struct AuditLogQuery {
    pub limit: Option<u32>,
    pub cursor: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLogEntry {
    pub id: i64,
    pub actor_account_id: u64,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at_unix: u64,
}

#[derive(Serialize, ToSchema)]
pub struct AuditLogListResponse {
    pub limit: u32,
    pub cursor: Option<i64>,
    pub entries: Vec<AuditLogEntry>,
}

#[derive(Deserialize, ToSchema)]
pub struct AnnounceRequest {
    pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RestartServerRequest {
    pub delay_seconds: Option<u32>,
}

#[derive(Deserialize, ToSchema)]
pub struct KickPlayerRequest {
    pub character_name: String,
    pub reason: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct BanAccountRequest {
    pub account: String,
    pub days: u32,
    pub reason: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UnbanAccountRequest {
    pub account: String,
}

#[derive(Deserialize, ToSchema)]
pub struct GmCommandRequest {
    pub command: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TeleportPlayerRequest {
    pub character_name: String,
    pub location: String,
}

#[derive(Deserialize, ToSchema)]
pub struct GiveItemRequest {
    pub character_name: String,
    pub item_entry: u32,
    pub count: Option<u32>,
}

#[derive(Deserialize, ToSchema)]
pub struct ModifyMoneyRequest {
    pub character_name: String,
    pub amount: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct SetLevelRequest {
    pub character_name: String,
    pub level: u8,
}

#[derive(Deserialize)]
pub struct LogTailQuery {
    pub lines: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct LogTailResponse {
    pub file: String,
    pub lines: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SoapCommandResponse {
    pub command: String,
    pub result: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ItemQuery {
    pub limit: Option<u32>,
    pub cursor: Option<u32>,
    pub search: Option<String>,
    pub class: Option<u8>,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct ItemSummary {
    pub entry: u32,
    pub name: String,
    pub quality: u8,
    pub item_level: u32,
    pub class: u8,
    pub subclass: u8,
    pub display_id: u32,
}

#[derive(Serialize, ToSchema)]
pub struct ItemListResponse {
    pub limit: u32,
    pub cursor: Option<u32>,
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
