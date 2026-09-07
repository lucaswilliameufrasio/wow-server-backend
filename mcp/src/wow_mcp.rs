use std::sync::Arc;

use reqwest::StatusCode;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::wrapper::Parameters,
    model::*, schemars, service::RequestContext, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::api_client::{ApiClient, ApiError};
use crate::dto::{
    AdminAccountLocationsResponse, AdminPlayersResponse, HealthCheckResponse, ItemListResponse,
    ItemSummary, OnlinePlayerSummary,
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
}

#[tool_router]
impl WowMcp {
    pub fn new(api: Arc<ApiClient>, metrics_api: Arc<ApiClient>) -> Self {
        Self { api, metrics_api }
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
        let api = Arc::new(ApiClient::new(base_url.to_string(), None, 5).unwrap());
        let metrics_api = Arc::new(ApiClient::new(base_url.to_string(), None, 5).unwrap());
        WowMcp::new(api, metrics_api)
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
        let mcp = WowMcp::new(api, metrics_api);
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
    async fn read_uri_unknown_resource() {
        let base = mock::spawn().await;
        let err = client(&base).read_uri("guild://1").await.unwrap_err();
        assert_eq!(err.status_code(), Some(StatusCode::NOT_FOUND));
    }
}
