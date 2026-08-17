use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use tracing::{info, info_span};

use crate::metrics;

/// Middleware that records per-request metrics and emits a structured log span.
pub async fn track_request(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = Instant::now();

    let span = info_span!("http_request", method = %method, path = %path);
    let _enter = span.enter();

    let response = next.run(req).await;

    let status = response.status();
    let duration = started.elapsed().as_secs_f64();

    metrics::record_http(method.as_str(), &path, status.as_u16(), duration);

    if status.is_server_error() {
        tracing::error!(
            status = %status.as_u16(),
            duration_ms = started.elapsed().as_millis(),
            "request failed"
        );
    } else {
        info!(
            status = %status.as_u16(),
            duration_ms = started.elapsed().as_millis(),
            "request completed"
        );
    }

    drop(_enter);
    response
}
