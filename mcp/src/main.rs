mod api_client;
mod config;
mod dto;
#[cfg(test)]
mod mock;
mod wow_mcp;

use std::sync::Arc;

use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use crate::wow_mcp::WowMcp;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    let config = config::Config::from_env().map_err(anyhow::Error::msg)?;
    let api = Arc::new(api_client::ApiClient::new(
        &config.api_base_url,
        config.api_token.clone(),
        config.request_timeout_secs,
    )?);

    let service = WowMcp::new(api);
    let server = service.serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
