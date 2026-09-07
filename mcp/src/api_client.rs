use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("API request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("API returned {status}: {code} ({message})")]
    Status {
        status: StatusCode,
        code: String,
        message: String,
    },
}

impl ApiError {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn status_code(&self) -> Option<StatusCode> {
        match self {
            ApiError::Status { status, .. } => Some(*status),
            _ => None,
        }
    }
}

pub struct ApiClient {
    http: Client,
    base_url: String,
    token: Option<String>,
}

#[derive(serde::Deserialize)]
struct ErrorBody {
    message: Option<String>,
    error_code: Option<String>,
}

impl ApiClient {
    pub fn new(
        base_url: impl Into<String>,
        token: Option<String>,
        request_timeout_secs: u64,
    ) -> Result<Self, ApiError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty()
            || !(base_url.starts_with("http://") || base_url.starts_with("https://"))
        {
            return Err(ApiError::InvalidBaseUrl(base_url));
        }

        let http = Client::builder()
            .timeout(Duration::from_secs(request_timeout_secs))
            .build()
            .map_err(ApiError::Transport)?;

        Ok(Self {
            http,
            base_url,
            token,
        })
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.send(path, None)
            .await?
            .json()
            .await
            .map_err(ApiError::Transport)
    }

    pub async fn get_json_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, ApiError> {
        self.send(path, Some(query))
            .await?
            .json()
            .await
            .map_err(ApiError::Transport)
    }

    pub async fn get_text(&self, path: &str) -> Result<String, ApiError> {
        self.send(path, None)
            .await?
            .text()
            .await
            .map_err(ApiError::Transport)
    }

    async fn send(
        &self,
        path: &str,
        query: Option<&[(&str, String)]>,
    ) -> Result<reqwest::Response, ApiError> {
        let url = format!("{}{}", self.base_url, path);

        let mut request = self.http.get(&url);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(query) = query {
            request = request.query(query);
        }

        let response = request.send().await.map_err(ApiError::Transport)?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let (code, message) = parse_error_body(&body);
            return Err(ApiError::Status {
                status,
                code,
                message,
            });
        }

        Ok(response)
    }
}

fn parse_error_body(body: &str) -> (String, String) {
    match serde_json::from_str::<ErrorBody>(body) {
        Ok(parsed) => (
            parsed.error_code.unwrap_or_else(|| "UNKNOWN".to_string()),
            parsed.message.unwrap_or_else(|| truncate(body, 300)),
        ),
        Err(_) => ("UNKNOWN".to_string(), truncate(body, 300)),
    }
}

fn truncate(body: &str, max: usize) -> String {
    if body.len() <= max {
        body.to_string()
    } else {
        format!("{}...", &body[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AdminPlayersResponse, HealthCheckResponse, ItemSummary};
    use crate::mock;

    fn client(base_url: &str) -> ApiClient {
        ApiClient::new(base_url.to_string(), None, 5).expect("client")
    }

    #[tokio::test]
    async fn get_json_parses_health() {
        let base = mock::spawn().await;
        let health: HealthCheckResponse = client(&base).get_json("/health-check").await.unwrap();
        assert_eq!(health.message, "ok");
    }

    #[tokio::test]
    async fn get_json_parses_item() {
        let base = mock::spawn().await;
        let item: ItemSummary = client(&base).get_json("/v1/items/19019").await.unwrap();
        assert_eq!(item.entry, 19019);
        assert!(item.name.contains("Thunderfury"));
    }

    #[tokio::test]
    async fn admin_players_redact_pii_at_boundary() {
        let base = mock::spawn().await;
        let response: AdminPlayersResponse =
            client(&base).get_json("/v1/admin/players").await.unwrap();

        let player = response.players.first().unwrap();
        assert_eq!(player.username, "ADMIN");
        assert_eq!(player.gm_level, 3);
    }

    #[tokio::test]
    async fn status_error_carries_api_error_code() {
        let base = mock::spawn().await;
        let result: Result<serde_json::Value, ApiError> =
            client(&base).get_json("/forbidden").await;

        let err = result.unwrap_err();
        assert_eq!(err.status_code(), Some(StatusCode::FORBIDDEN));
        assert!(err.to_string().contains("RBAC_FORBIDDEN"));
    }

    #[tokio::test]
    async fn get_text_reads_metrics() {
        let base = mock::spawn().await;
        let text = client(&base).get_text("/metrics").await.unwrap();
        assert!(text.contains("wow_api_health_checks_total 7"));
    }

    #[tokio::test]
    async fn rejects_invalid_base_url() {
        let result = ApiClient::new("ftp://nope", None, 5);
        assert!(matches!(result, Err(ApiError::InvalidBaseUrl(_))));
    }

    #[test]
    fn parse_error_body_falls_back_to_raw() {
        let (code, message) = parse_error_body("<html>oops</html>");
        assert_eq!(code, "UNKNOWN");
        assert_eq!(message, "<html>oops</html>");
    }
}
