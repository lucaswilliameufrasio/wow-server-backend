use std::net::SocketAddr;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::migrate::MigrateError;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to initialize tracing: {0}")]
    Tracing(String),
    #[error("failed to bind to {addr}: {source}")]
    Bind {
        #[source]
        source: std::io::Error,
        addr: SocketAddr,
    },
    #[error("failed to create database pool: {0}")]
    DbConnect(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    Migrate(#[from] MigrateError),
    #[error("failed to create jwt config: {0}")]
    JwtConfig(String),
    #[error("server error: {0}")]
    Serve(#[from] std::io::Error),
}

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: &'static str,
    pub error_code: &'static str,
}

impl ApiError {
    pub fn new(status: StatusCode, message: &'static str, error_code: &'static str) -> Self {
        Self {
            status,
            message,
            error_code,
        }
    }

    pub fn bad_request(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message, error_code)
    }

    pub fn unauthorized(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message, error_code)
    }

    pub fn forbidden(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, message, error_code)
    }

    pub fn not_found(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message, error_code)
    }

    pub fn conflict(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, message, error_code)
    }

    pub fn internal(message: &'static str, error_code: &'static str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message, error_code)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                message: self.message,
                error_code: self.error_code,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: &'static str,
    pub error_code: &'static str,
}
