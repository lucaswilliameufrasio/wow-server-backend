use axum::response::IntoResponse;
use once_cell::sync::Lazy;
use prometheus::{Encoder, Histogram, IntCounter, IntCounterVec, TextEncoder};

pub static HEALTH_CHECKS: Lazy<IntCounter> = Lazy::new(|| {
    prometheus::register_int_counter!("wow_api_health_checks_total", "Total health check requests")
        .expect("metrics register")
});

pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    prometheus::register_int_counter_vec!(
        "wow_api_http_requests_total",
        "Total HTTP requests by method, route and status class",
        &["method", "route", "status"]
    )
    .expect("metrics register")
});

pub static HTTP_REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    prometheus::register_histogram!(
        "wow_api_http_request_duration_seconds",
        "HTTP request latency in seconds",
        vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0
        ]
    )
    .expect("metrics register")
});

pub static LOGIN_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    prometheus::register_int_counter_vec!(
        "wow_api_login_total",
        "Sign-in attempts by result",
        &["result"]
    )
    .expect("metrics register")
});

pub static ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    prometheus::register_int_counter_vec!(
        "wow_api_errors_total",
        "API errors by error code",
        &["code"]
    )
    .expect("metrics register")
});

pub fn record_http(method: &str, route: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, route, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION.observe(duration_secs);
}

pub fn record_login(result: &str) {
    LOGIN_TOTAL.with_label_values(&[result]).inc();
}

pub fn record_error(code: &str) {
    ERRORS_TOTAL.with_label_values(&[code]).inc();
}

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("metrics encode");
    let body = String::from_utf8(buffer).unwrap_or_default();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        body,
    )
}
