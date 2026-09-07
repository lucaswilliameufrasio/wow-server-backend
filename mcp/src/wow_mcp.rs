use std::sync::Arc;

use reqwest::StatusCode;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::wrapper::Parameters,
    model::*, schemars, service::RequestContext, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::api_client::{ApiClient, ApiError};
use crate::dto::{
    AccountLockResponse, AdminAccountLocationsResponse, AdminPlayersResponse, AuditLogListResponse,
    HealthCheckResponse, ItemListResponse, ItemSummary, LogTailResponse, OnlinePlayerSummary,
    SoapCommandResponse,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchPlayersArgs {
    pub search: Option<String>,
    pub online: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PlayerLocationsArgs {
    pub account_id: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchItemsArgs {
    pub search: Option<String>,
    pub class: Option<u8>,
    pub limit: Option<u32>,
    pub cursor: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetItemArgs {
    pub entry: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LockAccountArgs {
    #[schemars(description = "Account id to lock or unlock")]
    pub account_id: u64,
    #[schemars(description = "true to lock the account, false to unlock it")]
    pub locked: bool,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually apply the change after reviewing the preview."
    )]
    pub dry_run: Option<bool>,
    #[schemars(description = "Optional reason recorded in the server audit log")]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAuditLogArgs {
    #[schemars(description = "Max entries to return (1-200, default 50)")]
    pub limit: Option<u32>,
    #[schemars(description = "Pagination cursor: last entry id from the previous page")]
    pub cursor: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnnounceArgs {
    #[schemars(description = "Message broadcast to all players (max 256 chars, single line)")]
    pub message: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RestartServerArgs {
    #[schemars(
        description = "Delay in seconds before the worldserver restarts (1-86400, default 60)"
    )]
    pub delay_seconds: Option<u32>,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually schedule the restart."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct KickPlayerArgs {
    #[schemars(description = "In-game character name (2-12 letters)")]
    pub character_name: String,
    #[schemars(description = "Optional reason recorded in the audit log")]
    pub reason: Option<String>,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually kick the player."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BanAccountArgs {
    #[schemars(description = "Account name to ban")]
    pub account: String,
    #[schemars(description = "Ban duration in days (1-3650)")]
    pub days: u32,
    #[schemars(description = "Optional reason recorded in the audit log")]
    pub reason: Option<String>,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually ban the account."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UnbanAccountArgs {
    #[schemars(description = "Account name to unban")]
    pub account: String,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually unban the account."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GmCommandArgs {
    #[schemars(description = "Raw GM command to send to the worldserver (e.g. 'server info')")]
    pub command: String,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually execute the command."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ServerStatusArgs {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LogTailArgs {
    #[schemars(description = "Number of trailing lines to return (1-1000, default 200)")]
    pub lines: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeleportPlayerArgs {
    #[schemars(description = "In-game character name (2-12 letters)")]
    pub character_name: String,
    #[schemars(
        description = "Teleport location name as shown in .tele list (e.g. 'Stormwind City')"
    )]
    pub location: String,
    #[schemars(description = "Defaults to true. Run with dry_run=false to actually teleport.")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GiveItemArgs {
    #[schemars(description = "In-game character name (2-12 letters)")]
    pub character_name: String,
    #[schemars(description = "Item template entry id (e.g. 19019 for Thunderfury)")]
    pub item_entry: u32,
    #[schemars(description = "Stack count (1-1000, default 1)")]
    pub count: Option<u32>,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually give the item."
    )]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ModifyMoneyArgs {
    #[schemars(description = "In-game character name (2-12 letters)")]
    pub character_name: String,
    #[schemars(description = "Amount in copper; negative removes money. |amount| <= 2147483647")]
    pub amount: i64,
    #[schemars(description = "Defaults to true. Run with dry_run=false to actually modify money.")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetLevelArgs {
    #[schemars(description = "In-game character name (2-12 letters)")]
    pub character_name: String,
    #[schemars(description = "Absolute level to set (1-80)")]
    pub level: u8,
    #[schemars(
        description = "Defaults to true. Run with dry_run=false to actually set the level."
    )]
    pub dry_run: Option<bool>,
}

fn pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

fn json_result<T: Serialize>(value: &T) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(pretty(value))])
}

fn api_tool_error(err: ApiError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("API error: {err}"))])
}

#[derive(Clone)]
pub struct WowMcp {
    api: Arc<ApiClient>,
    metrics_api: Arc<ApiClient>,
    gm_commands_enabled: bool,
}

#[tool_router]
impl WowMcp {
    pub fn new(
        api: Arc<ApiClient>,
        metrics_api: Arc<ApiClient>,
        gm_commands_enabled: bool,
    ) -> Self {
        Self {
            api,
            metrics_api,
            gm_commands_enabled,
        }
    }

    #[tool(
        description = "List all players currently online in the realm",
        annotations(read_only_hint = true)
    )]
    async fn list_online_players(&self) -> Result<CallToolResult, McpError> {
        match self.fetch_online_players().await {
            Ok(players) => Ok(json_result(&players)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Search player accounts by username and online status",
        annotations(read_only_hint = true)
    )]
    async fn search_players(
        &self,
        Parameters(SearchPlayersArgs {
            search,
            online,
            limit,
            cursor,
        }): Parameters<SearchPlayersArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_players(search, online, limit, cursor).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get the current location of every character owned by an account",
        annotations(read_only_hint = true)
    )]
    async fn get_player_locations(
        &self,
        Parameters(PlayerLocationsArgs { account_id }): Parameters<PlayerLocationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_player_locations(account_id).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Search game items by name and item class",
        annotations(read_only_hint = true)
    )]
    async fn search_items(
        &self,
        Parameters(SearchItemsArgs {
            search,
            class,
            limit,
            cursor,
        }): Parameters<SearchItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_items(search, class, limit, cursor).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get a single item by its entry id",
        annotations(read_only_hint = true)
    )]
    async fn get_item(
        &self,
        Parameters(GetItemArgs { entry }): Parameters<GetItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_item(entry).await {
            Ok(item) => Ok(json_result(&item)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get the backend API health check status",
        annotations(read_only_hint = true)
    )]
    async fn get_health(&self) -> Result<CallToolResult, McpError> {
        match self.fetch_health().await {
            Ok(health) => Ok(json_result(&health)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get the last lines of the worldserver Server.log. Requires the backend to have AZEROTH_CORE_LOGS_DIR configured.",
        annotations(read_only_hint = true)
    )]
    async fn get_server_logs(
        &self,
        Parameters(LogTailArgs { lines }): Parameters<LogTailArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_server_logs(lines).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get the last lines of the worldserver Crash.log. Requires the backend to have AZEROTH_CORE_LOGS_DIR configured.",
        annotations(read_only_hint = true)
    )]
    async fn get_crashes(
        &self,
        Parameters(LogTailArgs { lines }): Parameters<LogTailArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_crashes(lines).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get worldserver status via the backend SOAP bridge (server info)",
        annotations(read_only_hint = true)
    )]
    async fn get_server_status(
        &self,
        Parameters(ServerStatusArgs {}): Parameters<ServerStatusArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_server_status().await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Broadcast an announcement to all online players",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    async fn send_announcement(
        &self,
        Parameters(AnnounceArgs { message }): Parameters<AnnounceArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_announce(&message).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Disconnect a player from the worldserver. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn kick_player(
        &self,
        Parameters(KickPlayerArgs {
            character_name,
            reason,
            dry_run,
        }): Parameters<KickPlayerArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "character_name": character_name,
                "action": "kick",
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_kick(&character_name, reason.as_deref()).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Ban an account for a number of days. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn ban_account(
        &self,
        Parameters(BanAccountArgs {
            account,
            days,
            reason,
            dry_run,
        }): Parameters<BanAccountArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "account": account,
                "days": days,
                "action": "ban",
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_ban(&account, days, reason.as_deref()).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Remove the ban from an account",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true
        )
    )]
    async fn unban_account(
        &self,
        Parameters(UnbanAccountArgs { account, dry_run }): Parameters<UnbanAccountArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "account": account,
                "action": "unban",
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_unban(&account).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Schedule a worldserver restart after a delay in seconds. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn schedule_restart(
        &self,
        Parameters(RestartServerArgs {
            delay_seconds,
            dry_run,
        }): Parameters<RestartServerArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "delay_seconds": delay_seconds.unwrap_or(60),
                "action": "server restart",
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_restart(delay_seconds).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Execute a raw GM command on the worldserver. DISABLED unless the MCP was started with MCP_ENABLE_GM_COMMANDS=true AND the backend enables it. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn run_gm_command(
        &self,
        Parameters(GmCommandArgs { command, dry_run }): Parameters<GmCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        if !self.gm_commands_enabled {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "GM command execution is disabled in the MCP (set MCP_ENABLE_GM_COMMANDS=true to enable). The backend also enforces its own flag and allowlist.",
            )]));
        }

        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "command": command,
                "next_step": "Call again with dry_run=false to execute this command."
            })));
        }

        match self.fetch_gm_command(&command).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Teleport a character to a known .tele location. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn teleport_player(
        &self,
        Parameters(TeleportPlayerArgs {
            character_name,
            location,
            dry_run,
        }): Parameters<TeleportPlayerArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "character_name": character_name,
                "location": location,
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_teleport(&character_name, &location).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Give an item to a character's inventory. DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn give_item(
        &self,
        Parameters(GiveItemArgs {
            character_name,
            item_entry,
            count,
            dry_run,
        }): Parameters<GiveItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "character_name": character_name,
                "item_entry": item_entry,
                "count": count.unwrap_or(1),
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self
            .fetch_give_item(&character_name, item_entry, count)
            .await
        {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Add or remove money from a character (copper; negative amount removes). DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn modify_money(
        &self,
        Parameters(ModifyMoneyArgs {
            character_name,
            amount,
            dry_run,
        }): Parameters<ModifyMoneyArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "character_name": character_name,
                "amount_copper": amount,
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_modify_money(&character_name, amount).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Set a character's absolute level (1-80). DESTRUCTIVE: requires explicit confirmation via dry_run=false.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_level(
        &self,
        Parameters(SetLevelArgs {
            character_name,
            level,
            dry_run,
        }): Parameters<SetLevelArgs>,
    ) -> Result<CallToolResult, McpError> {
        if dry_run.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "dry_run": true,
                "character_name": character_name,
                "level": level,
                "next_step": "Call again with dry_run=false to apply this change."
            })));
        }

        match self.fetch_set_level(&character_name, level).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Lock or unlock a player account. DESTRUCTIVE: requires explicit operator confirmation. First call with dry_run=true (default) to preview, then call again with dry_run=false to apply.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn lock_account(
        &self,
        Parameters(LockAccountArgs {
            account_id,
            locked,
            dry_run,
            reason,
        }): Parameters<LockAccountArgs>,
    ) -> Result<CallToolResult, McpError> {
        let dry_run = dry_run.unwrap_or(true);

        if dry_run {
            let mut preview = serde_json::json!({
                "dry_run": true,
                "account_id": account_id,
                "would_set_locked": locked,
                "next_step": "Call again with dry_run=false to apply this change."
            });
            if let Some(r) = reason.as_deref().filter(|r| !r.trim().is_empty()) {
                preview["reason"] = serde_json::json!(r);
            }
            return Ok(json_result(&preview));
        }

        let mut body = serde_json::json!({ "locked": locked });
        if let Some(r) = reason.as_deref().filter(|r| !r.trim().is_empty()) {
            body["reason"] = serde_json::json!(r);
        }

        match self.fetch_lock_account(account_id, &body).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get recent admin audit log entries (mutations and denied attempts), newest first",
        annotations(read_only_hint = true)
    )]
    async fn get_audit_log(
        &self,
        Parameters(GetAuditLogArgs { limit, cursor }): Parameters<GetAuditLogArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetch_audit_log(limit, cursor).await {
            Ok(response) => Ok(json_result(&response)),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    #[tool(
        description = "Get Prometheus metrics from the backend API",
        annotations(read_only_hint = true)
    )]
    async fn get_metrics(&self) -> Result<CallToolResult, McpError> {
        match self.fetch_metrics().await {
            Ok(text) => Ok(CallToolResult::success(vec![ContentBlock::text(text)])),
            Err(err) => Ok(api_tool_error(err)),
        }
    }

    pub async fn fetch_online_players(&self) -> Result<Vec<OnlinePlayerSummary>, ApiError> {
        self.api.get_json("/v1/admin/online-players").await
    }

    pub async fn fetch_players(
        &self,
        search: Option<String>,
        online: Option<bool>,
        limit: Option<u32>,
        cursor: Option<u64>,
    ) -> Result<AdminPlayersResponse, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(v) = search {
            query.push(("search", v));
        }
        if let Some(v) = online {
            query.push(("online", v.to_string()));
        }
        if let Some(v) = limit {
            query.push(("limit", v.to_string()));
        }
        if let Some(v) = cursor {
            query.push(("cursor", v.to_string()));
        }
        self.api
            .get_json_with_query("/v1/admin/players", &query)
            .await
    }

    pub async fn fetch_player_locations(
        &self,
        account_id: u64,
    ) -> Result<AdminAccountLocationsResponse, ApiError> {
        self.api
            .get_json(&format!("/v1/admin/players/{account_id}/locations"))
            .await
    }

    pub async fn fetch_items(
        &self,
        search: Option<String>,
        class: Option<u8>,
        limit: Option<u32>,
        cursor: Option<u32>,
    ) -> Result<ItemListResponse, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(v) = search {
            query.push(("search", v));
        }
        if let Some(v) = class {
            query.push(("class", v.to_string()));
        }
        if let Some(v) = limit {
            query.push(("limit", v.to_string()));
        }
        if let Some(v) = cursor {
            query.push(("cursor", v.to_string()));
        }
        self.api.get_json_with_query("/v1/items", &query).await
    }

    pub async fn fetch_item(&self, entry: u32) -> Result<ItemSummary, ApiError> {
        self.api.get_json(&format!("/v1/items/{entry}")).await
    }

    pub async fn fetch_health(&self) -> Result<HealthCheckResponse, ApiError> {
        self.api.get_json("/health-check").await
    }

    pub async fn fetch_server_status(&self) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json("/v1/admin/server/status", &serde_json::json!({}))
            .await
    }

    pub async fn fetch_announce(&self, message: &str) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/server/announce",
                &serde_json::json!({ "message": message }),
            )
            .await
    }

    pub async fn fetch_restart(
        &self,
        delay_seconds: Option<u32>,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/server/restart",
                &serde_json::json!({ "delay_seconds": delay_seconds }),
            )
            .await
    }

    pub async fn fetch_kick(
        &self,
        character_name: &str,
        reason: Option<&str>,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/players/kick",
                &serde_json::json!({ "character_name": character_name, "reason": reason }),
            )
            .await
    }

    pub async fn fetch_ban(
        &self,
        account: &str,
        days: u32,
        reason: Option<&str>,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/accounts/ban",
                &serde_json::json!({ "account": account, "days": days, "reason": reason }),
            )
            .await
    }

    pub async fn fetch_unban(&self, account: &str) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/accounts/unban",
                &serde_json::json!({ "account": account }),
            )
            .await
    }

    pub async fn fetch_gm_command(&self, command: &str) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/server/command",
                &serde_json::json!({ "command": command }),
            )
            .await
    }

    pub async fn fetch_server_logs(&self, lines: Option<u32>) -> Result<LogTailResponse, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(v) = lines {
            query.push(("lines", v.to_string()));
        }
        self.api
            .get_json_with_query("/v1/admin/server/logs", &query)
            .await
    }

    pub async fn fetch_crashes(&self, lines: Option<u32>) -> Result<LogTailResponse, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(v) = lines {
            query.push(("lines", v.to_string()));
        }
        self.api
            .get_json_with_query("/v1/admin/server/crashes", &query)
            .await
    }

    pub async fn fetch_teleport(
        &self,
        character_name: &str,
        location: &str,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/players/teleport",
                &serde_json::json!({ "character_name": character_name, "location": location }),
            )
            .await
    }

    pub async fn fetch_give_item(
        &self,
        character_name: &str,
        item_entry: u32,
        count: Option<u32>,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/players/items",
                &serde_json::json!({
                    "character_name": character_name,
                    "item_entry": item_entry,
                    "count": count
                }),
            )
            .await
    }

    pub async fn fetch_modify_money(
        &self,
        character_name: &str,
        amount: i64,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/players/money",
                &serde_json::json!({ "character_name": character_name, "amount": amount }),
            )
            .await
    }

    pub async fn fetch_set_level(
        &self,
        character_name: &str,
        level: u8,
    ) -> Result<SoapCommandResponse, ApiError> {
        self.api
            .post_json(
                "/v1/admin/players/level",
                &serde_json::json!({ "character_name": character_name, "level": level }),
            )
            .await
    }

    pub async fn fetch_lock_account(
        &self,
        account_id: u64,
        body: &serde_json::Value,
    ) -> Result<AccountLockResponse, ApiError> {
        self.api
            .patch_json(&format!("/v1/admin/players/{account_id}/lock"), body)
            .await
    }

    pub async fn fetch_audit_log(
        &self,
        limit: Option<u32>,
        cursor: Option<i64>,
    ) -> Result<AuditLogListResponse, ApiError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(v) = limit {
            query.push(("limit", v.to_string()));
        }
        if let Some(v) = cursor {
            query.push(("cursor", v.to_string()));
        }
        self.api
            .get_json_with_query("/v1/admin/audit-log", &query)
            .await
    }

    pub async fn fetch_metrics(&self) -> Result<String, ApiError> {
        self.metrics_api.get_text("/metrics").await
    }

    pub async fn read_uri(&self, uri: &str) -> Result<String, ApiError> {
        match uri {
            "server://health" => {
                let health: HealthCheckResponse = self.api.get_json("/health-check").await?;
                Ok(pretty(&health))
            }
            "server://metrics" => self.metrics_api.get_text("/metrics").await,
            "players://online" => {
                let players: Vec<OnlinePlayerSummary> =
                    self.api.get_json("/v1/admin/online-players").await?;
                Ok(pretty(&players))
            }
            _ if uri.starts_with("item://") => {
                let entry = uri.trim_start_matches("item://");
                if entry.is_empty() || !entry.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ApiError::Status {
                        status: StatusCode::BAD_REQUEST,
                        code: "INVALID_RESOURCE_URI".to_string(),
                        message: format!("item resource URI must be item://<numeric entry>: {uri}"),
                    });
                }
                let item: ItemSummary = self.api.get_json(&format!("/v1/items/{entry}")).await?;
                Ok(pretty(&item))
            }
            _ => Err(ApiError::Status {
                status: StatusCode::NOT_FOUND,
                code: "UNKNOWN_RESOURCE".to_string(),
                message: format!("unknown resource: {uri}"),
            }),
        }
    }
}

#[tool_handler(
    name = "wow-mcp",
    version = "0.1.0",
    instructions = "Read-only administration of the WoW server backend. Use tools to query players and items; use resources for server health, metrics, and online players."
)]
impl ServerHandler for WowMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            result_type: None,
            meta: None,
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            resources: vec![
                Resource::new("server://health", "server-health"),
                Resource::new("server://metrics", "server-metrics"),
                Resource::new("players://online", "online-players"),
            ],
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            result_type: None,
            meta: None,
            next_cursor: None,
            ttl_ms: None,
            cache_scope: None,
            resource_templates: vec![ResourceTemplate::new("item://{entry}", "item-by-entry")],
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        let uri = request.uri.to_string();
        match self.read_uri(&uri).await {
            Ok(text) => {
                Ok(ReadResourceResult::new(vec![ResourceContents::text(text, &uri)]).into())
            }
            Err(err) => Err(McpError::resource_not_found(
                format!("resource read failed: {err}"),
                Some(serde_json::json!({ "uri": uri })),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    fn client(base_url: &str) -> WowMcp {
        client_with_gm(base_url, false)
    }

    fn client_with_gm(base_url: &str, gm_commands_enabled: bool) -> WowMcp {
        let api = Arc::new(ApiClient::new(base_url.to_string(), None, 5).unwrap());
        let metrics_api = Arc::new(ApiClient::new(base_url.to_string(), None, 5).unwrap());
        WowMcp::new(api, metrics_api, gm_commands_enabled)
    }

    #[tokio::test]
    async fn fetch_online_players_returns_players() {
        let base = mock::spawn().await;
        let players = client(&base).fetch_online_players().await.unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Uther");
    }

    #[tokio::test]
    async fn fetch_players_omits_pii_fields() {
        let base = mock::spawn().await;
        let response = client(&base)
            .fetch_players(Some("AD".to_string()), Some(true), Some(10), None)
            .await
            .unwrap();

        let player = &response.players[0];
        assert_eq!(player.username, "ADMIN");
        let serialized = pretty(player);
        assert!(!serialized.contains("secret@example.com"));
        assert!(!serialized.contains("203.0.113.9"));
    }

    #[tokio::test]
    async fn fetch_player_locations_returns_locations() {
        let base = mock::spawn().await;
        let response = client(&base).fetch_player_locations(1).await.unwrap();
        assert_eq!(response.username, "ADMIN");
        assert_eq!(response.locations[0].name, "Uther");
    }

    #[tokio::test]
    async fn fetch_items_and_item() {
        let base = mock::spawn().await;
        let list = client(&base)
            .fetch_items(Some("fury".to_string()), None, None, None)
            .await
            .unwrap();
        assert_eq!(list.items[0].entry, 19019);

        let item = client(&base).fetch_item(19019).await.unwrap();
        assert!(item.name.contains("Thunderfury"));
    }

    #[tokio::test]
    async fn get_health_tool_returns_success() {
        let base = mock::spawn().await;
        let result = client(&base).get_health().await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn tool_error_when_api_unreachable() {
        let api = Arc::new(ApiClient::new("http://127.0.0.1:1", None, 1).unwrap());
        let metrics_api = Arc::new(ApiClient::new("http://127.0.0.1:1", None, 1).unwrap());
        let mcp = WowMcp::new(api, metrics_api, false);
        let result = mcp.get_health().await.unwrap();
        assert!(result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn read_uri_health_and_metrics() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        let health = mcp.read_uri("server://health").await.unwrap();
        assert!(health.contains("\"message\": \"ok\""));

        let metrics = mcp.read_uri("server://metrics").await.unwrap();
        assert!(metrics.contains("wow_api_health_checks_total 7"));
    }

    #[tokio::test]
    async fn read_uri_players_and_item_template() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        let players = mcp.read_uri("players://online").await.unwrap();
        assert!(players.contains("Uther"));

        let item = mcp.read_uri("item://19019").await.unwrap();
        assert!(item.contains("Thunderfury"));
    }

    #[tokio::test]
    async fn read_uri_rejects_invalid_item_entry() {
        let base = mock::spawn().await;
        let err = client(&base)
            .read_uri("item://not-a-number")
            .await
            .unwrap_err();
        assert_eq!(err.status_code(), Some(StatusCode::BAD_REQUEST));
    }

    #[tokio::test]
    async fn lock_account_dry_run_by_default_never_calls_patch() {
        let base = mock::spawn().await;
        let mcp = client(&base);
        let result = mcp
            .lock_account(Parameters(LockAccountArgs {
                account_id: 5,
                locked: true,
                dry_run: None,
                reason: Some("cheating".to_string()),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let text = format!("{result:?}");
        assert!(text.contains("dry_run"));
        assert!(text.contains("would_set_locked"));
    }

    #[tokio::test]
    async fn lock_account_applies_on_confirmed_call() {
        let base = mock::spawn().await;
        let body = serde_json::json!({ "locked": true, "reason": "cheating" });
        let response = client(&base).fetch_lock_account(5, &body).await.unwrap();
        assert_eq!(response.account_id, 5);
        assert!(response.locked);
    }

    #[tokio::test]
    async fn fetch_audit_log_returns_entries() {
        let base = mock::spawn().await;
        let response = client(&base).fetch_audit_log(Some(10), None).await.unwrap();
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].action, "account.lock");
    }

    #[tokio::test]
    async fn server_status_and_announce_call_backend() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        let status = mcp.fetch_server_status().await.unwrap();
        assert!(status.command.contains("server info"));

        let announce = mcp.fetch_announce("reinício em 15 minutos").await.unwrap();
        assert!(announce.result.contains("Announcement sent"));
    }

    #[tokio::test]
    async fn kick_dry_run_never_calls_backend() {
        let base = mock::spawn().await;
        let mcp = client(&base);
        let result = mcp
            .kick_player(Parameters(KickPlayerArgs {
                character_name: "Xerath".to_string(),
                reason: Some("afk arena".to_string()),
                dry_run: None,
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert!(format!("{result:?}").contains("dry_run"));
    }

    #[tokio::test]
    async fn kick_confirmed_calls_backend() {
        let base = mock::spawn().await;
        let mcp = client(&base);
        let response = mcp.fetch_kick("Xerath", Some("afk arena")).await.unwrap();
        assert!(response.command.contains("kick Xerath"));
    }

    #[tokio::test]
    async fn ban_confirmed_sends_command() {
        let base = mock::spawn().await;
        let response = client(&base)
            .fetch_ban("Cheater", 7, Some("exploits"))
            .await
            .unwrap();
        assert!(response.command.contains("ban account Cheater 7d"));
    }

    #[tokio::test]
    async fn gm_command_disabled_in_mcp() {
        let base = mock::spawn().await;
        let mcp = client_with_gm(&base, false);
        let result = mcp
            .run_gm_command(Parameters(GmCommandArgs {
                command: "server info".to_string(),
                dry_run: Some(false),
            }))
            .await
            .unwrap();
        assert!(result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn gm_command_dry_run_and_enabled_path() {
        let base = mock::spawn().await;
        let mcp = client_with_gm(&base, true);

        let preview = mcp
            .run_gm_command(Parameters(GmCommandArgs {
                command: "server info".to_string(),
                dry_run: None,
            }))
            .await
            .unwrap();
        assert!(!preview.is_error.unwrap_or(false));
        assert!(format!("{preview:?}").contains("dry_run"));

        let executed = mcp
            .run_gm_command(Parameters(GmCommandArgs {
                command: "server info".to_string(),
                dry_run: Some(false),
            }))
            .await
            .unwrap();
        assert!(!executed.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn restart_dry_run_previews_delay() {
        let base = mock::spawn().await;
        let result = client(&base)
            .schedule_restart(Parameters(RestartServerArgs {
                delay_seconds: Some(900),
                dry_run: None,
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert!(format!("{result:?}").contains("900"));
    }

    #[tokio::test]
    async fn player_modify_dry_run_previews() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        for (result, needle) in [
            (
                mcp.teleport_player(Parameters(TeleportPlayerArgs {
                    character_name: "Xerath".to_string(),
                    location: "Stormwind City".to_string(),
                    dry_run: None,
                }))
                .await
                .unwrap(),
                "Stormwind City",
            ),
            (
                mcp.give_item(Parameters(GiveItemArgs {
                    character_name: "Xerath".to_string(),
                    item_entry: 19019,
                    count: None,
                    dry_run: None,
                }))
                .await
                .unwrap(),
                "19019",
            ),
            (
                mcp.modify_money(Parameters(ModifyMoneyArgs {
                    character_name: "Xerath".to_string(),
                    amount: -5000,
                    dry_run: None,
                }))
                .await
                .unwrap(),
                "-5000",
            ),
            (
                mcp.set_level(Parameters(SetLevelArgs {
                    character_name: "Xerath".to_string(),
                    level: 80,
                    dry_run: None,
                }))
                .await
                .unwrap(),
                "80",
            ),
        ] {
            assert!(!result.is_error.unwrap_or(false));
            assert!(format!("{result:?}").contains(needle));
        }
    }

    #[tokio::test]
    async fn player_modify_confirmed_calls_backend() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        let teleport = mcp
            .fetch_teleport("Xerath", "Stormwind City")
            .await
            .unwrap();
        assert!(teleport.command.contains("tele name Xerath"));

        let item = mcp.fetch_give_item("Xerath", 19019, Some(2)).await.unwrap();
        assert!(item.command.contains("additem name Xerath 19019 2"));

        let money = mcp.fetch_modify_money("Xerath", 123_456_789).await.unwrap();
        assert!(money.command.contains("12345g67s89c"));

        let level = mcp.fetch_set_level("Xerath", 80).await.unwrap();
        assert!(level.command.contains("setlevel name Xerath 80"));
    }

    #[tokio::test]
    async fn server_logs_and_crashes_fetch() {
        let base = mock::spawn().await;
        let mcp = client(&base);

        let logs = mcp.fetch_server_logs(Some(50)).await.unwrap();
        assert_eq!(logs.file, "Server.log");
        assert!(logs.lines[0].contains("restart"));

        let crashes = mcp.fetch_crashes(None).await.unwrap();
        assert_eq!(crashes.file, "Crash.log");
    }

    #[tokio::test]
    async fn read_uri_unknown_resource() {
        let base = mock::spawn().await;
        let err = client(&base).read_uri("guild://1").await.unwrap_err();
        assert_eq!(err.status_code(), Some(StatusCode::NOT_FOUND));
    }
}
