use axum::{response::IntoResponse};
use once_cell::sync::Lazy;
use prometheus::{Encoder, TextEncoder, register_int_counter_vec, IntCounterVec};

pub static REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "wow_api_requests_total",
        "Total API requests by method and route",
        &["method", "route"]
    )
    .expect("metrics register")
});

pub static HEALTH_CHECKS: Lazy<prometheus::IntCounter> = Lazy::new(|| {
    prometheus::register_int_counter!(
        "wow_api_health_checks_total",
        "Total health check requests"
    )
    .expect("metrics register")
});

pub static ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "wow_api_errors_total",
        "Total API errors by route and status",
        &["route", "status"]
    )
    .expect("metrics register")
});

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("metrics encode");
    let body = String::from_utf8(buffer).unwrap_or_default();
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
}
