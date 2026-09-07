use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlinePlayerSummary {
    pub account_id: u64,
    pub username: String,
    pub guid: u64,
    pub name: String,
    pub level: u8,
    pub map: u16,
    pub zone: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPlayerSummary {
    pub id: u64,
    pub username: String,
    pub joined_unix: Option<u64>,
    pub last_login_unix: Option<u64>,
    pub locked: bool,
    pub online: bool,
    pub gm_level: u8,
    pub character_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPlayersResponse {
    pub limit: u32,
    pub cursor: Option<u64>,
    pub players: Vec<AdminPlayerSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAccountLocationsResponse {
    pub account_id: u64,
    pub username: String,
    pub locations: Vec<CharacterLocationResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSummary {
    pub entry: u32,
    pub name: String,
    pub quality: u8,
    pub item_level: u32,
    pub class: u8,
    pub subclass: u8,
    pub display_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemListResponse {
    pub limit: u32,
    pub cursor: Option<u32>,
    pub items: Vec<ItemSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountLockResponse {
    pub account_id: u64,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub actor_account_id: u64,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogListResponse {
    pub limit: u32,
    pub cursor: Option<i64>,
    pub entries: Vec<AuditLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoapCommandResponse {
    pub command: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTailResponse {
    pub file: String,
    pub lines: Vec<String>,
}
