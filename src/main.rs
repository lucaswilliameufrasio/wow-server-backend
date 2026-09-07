mod auth;
mod error;
mod handlers;
mod metrics;
mod middleware;
mod models;
mod observability;
mod repos;

use std::{env, fs, net::SocketAddr, sync::Arc};

use anyhow::Result;
use sqlx::{mysql::MySqlPoolOptions, postgres::PgPoolOptions};
use tokio::{net::TcpListener, signal};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::error::AppResult;
use crate::handlers::{build_metrics_router, build_router};
use error::AppError;
use models::{AppConfig, JwtConfig};
use repos::{
    AppState, LiveAccountRepo, LiveAuditRepo, LiveCharacterRepo, LiveItemRepo,
    LiveRefreshTokenRepo, LiveServiceTokenRepo,
};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let state = build_state().await?;
    let router = build_router(state);

    run(router).await?;
    Ok(())
}

async fn build_state() -> AppResult<AppState> {
    let acore_database_url = env::var("AZEROTH_CORE_MYSQL_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306".to_string());
    let app_database_url = env::var("APP_POSTGRES_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@127.0.0.1:5432/wow_app".to_string());

    let acore_pool = MySqlPoolOptions::new()
        .max_connections(20)
        .connect(&acore_database_url)
        .await?;
    let app_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&app_database_url)
        .await?;

    let config = AppConfig {
        auth_db: env::var("AZEROTH_CORE_AUTH_DB").unwrap_or_else(|_| "acore_auth".to_string()),
        characters_db: env::var("AZEROTH_CORE_CHARACTERS_DB")
            .unwrap_or_else(|_| "acore_characters".to_string()),
        world_db: env::var("AZEROTH_CORE_WORLD_DB").unwrap_or_else(|_| "acore_world".to_string()),
        srp6_core5_mode: parse_bool_env("SRP6_CORE5_MODE"),
        rate_limit: parse_bool_env_with_default("WOW_RATE_LIMIT", true),
        debug_enabled: parse_bool_env_with_default("WOW_DEBUG_ENABLED", false),
    };

    // Validate DB names to prevent SQL identifier injection via config.
    for name in [&config.auth_db, &config.characters_db, &config.world_db] {
        if !is_safe_identifier(name) {
            return Err(AppError::Config(format!(
                "unsafe database name configured: {name:?} (only [A-Za-z0-9_] allowed)"
            )));
        }
    }

    let jwt = build_jwt_config()?;
    run_app_migrations(&app_pool).await?;

    Ok(AppState {
        config: config.clone(),
        jwt: jwt.clone(),
        started_at: auth::unix_now(),
        accounts: Arc::new(LiveAccountRepo::new(
            acore_pool.clone(),
            config.auth_db.clone(),
            config.characters_db.clone(),
        )),
        characters: Arc::new(LiveCharacterRepo::new(
            acore_pool.clone(),
            config.characters_db.clone(),
        )),
        items: Arc::new(LiveItemRepo::new(acore_pool, config.world_db.clone())),
        refresh_tokens: Arc::new(LiveRefreshTokenRepo::new(
            app_pool.clone(),
            jwt.refresh_expires_days,
            jwt.expires_minutes,
        )),
        service_tokens: Arc::new(LiveServiceTokenRepo::new(app_pool.clone())),
        audit: Arc::new(LiveAuditRepo::new(app_pool)),
    })
}

fn build_jwt_config() -> AppResult<JwtConfig> {
    let private_key_pem =
        load_pem("JWT_PRIVATE_KEY_PEM", "JWT_PRIVATE_KEY_PATH").ok_or_else(|| {
            AppError::JwtConfig(
                "missing private key: set JWT_PRIVATE_KEY_PEM or JWT_PRIVATE_KEY_PATH".to_string(),
            )
        })?;
    let public_key_pem =
        load_pem("JWT_PUBLIC_KEY_PEM", "JWT_PUBLIC_KEY_PATH").ok_or_else(|| {
            AppError::JwtConfig(
                "missing public key: set JWT_PUBLIC_KEY_PEM or JWT_PUBLIC_KEY_PATH".to_string(),
            )
        })?;

    let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| AppError::JwtConfig(format!("invalid private key: {e}")))?;
    let decoding_key = jsonwebtoken::DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
        .map_err(|e| AppError::JwtConfig(format!("invalid public key: {e}")))?;

    let issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "wow-backend".to_string());
    let audience = env::var("JWT_AUDIENCE").unwrap_or_else(|_| "wow-web".to_string());
    let expires_minutes = env::var("JWT_EXPIRES_MINUTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(15);
    let refresh_expires_days = env::var("JWT_REFRESH_EXPIRES_DAYS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30);

    Ok(JwtConfig {
        encoding_key,
        decoding_key,
        issuer,
        audience,
        expires_minutes,
        refresh_expires_days,
    })
}

fn load_pem(content_var: &str, path_var: &str) -> Option<String> {
    if let Ok(v) = env::var(content_var) {
        let normalized = v.replace("\\n", "\n");
        if !normalized.trim().is_empty() {
            return Some(normalized);
        }
    }

    if let Ok(path) = env::var(path_var)
        && let Ok(content) = fs::read_to_string(path)
        && !content.trim().is_empty()
    {
        return Some(content);
    }

    None
}

pub(crate) async fn run_app_migrations(pool: &sqlx::PgPool) -> AppResult<()> {
    static APP_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
    APP_MIGRATOR.run(pool).await?;
    Ok(())
}

fn parse_bool_env(key: &str) -> bool {
    env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_bool_env_with_default(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => default,
    }
}

async fn run(app: axum::Router) -> AppResult<()> {
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let metrics_port: u16 = env::var("METRICS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9090);
    let metrics_bind = env::var("METRICS_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
    let metrics_addr: SocketAddr = format!("{metrics_bind}:{metrics_port}")
        .parse()
        .map_err(|_| AppError::Config(format!("invalid METRICS_BIND: {metrics_bind}")))?;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| AppError::Bind { source, addr })?;
    let metrics_listener =
        TcpListener::bind(metrics_addr)
            .await
            .map_err(|source| AppError::Bind {
                source,
                addr: metrics_addr,
            })?;

    info!(%addr, %port, "listening on {addr}");
    info!(addr = %metrics_addr, "metrics (Prometheus) listening");

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let metrics_app = build_metrics_router();
    let metrics_task = tokio::spawn(async move {
        axum::serve(metrics_listener, metrics_app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
            })
            .await
    });

    let shutdown = async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    };

    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await;

    let _ = metrics_task.await;
    result?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn init_tracing() -> AppResult<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .try_init()
        .map_err(|err| AppError::Tracing(err.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests;
