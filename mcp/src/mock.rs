use axum::{Json, Router, http::StatusCode, routing::get};
use serde_json::json;

pub async fn spawn() -> String {
    let app = Router::new()
        .route(
            "/health-check",
            get(|| async { Json(json!({"message": "ok"})) }),
        )
        .route(
            "/metrics",
            get(|| async { "wow_api_health_checks_total 7" }),
        )
        .route(
            "/v1/items/19019",
            get(|| async {
                Json(json!({
                    "entry": 19019,
                    "name": "Thunderfury, Blessed Blade of the Windseeker",
                    "quality": 5,
                    "item_level": 80,
                    "class": 2,
                    "subclass": 6,
                    "display_id": 30606
                }))
            }),
        )
        .route(
            "/v1/items",
            get(|| async {
                Json(json!({
                    "limit": 50,
                    "cursor": null,
                    "items": [{
                        "entry": 19019,
                        "name": "Thunderfury, Blessed Blade of the Windseeker",
                        "quality": 5,
                        "item_level": 80,
                        "class": 2,
                        "subclass": 6,
                        "display_id": 30606
                    }]
                }))
            }),
        )
        .route(
            "/v1/admin/online-players",
            get(|| async {
                Json(json!([{
                    "account_id": 1,
                    "username": "ADMIN",
                    "guid": 10,
                    "name": "Uther",
                    "level": 80,
                    "map": 0,
                    "zone": 12
                }]))
            }),
        )
        .route(
            "/v1/admin/players",
            get(|| async {
                Json(json!({
                    "limit": 50,
                    "cursor": null,
                    "players": [{
                        "id": 1,
                        "username": "ADMIN",
                        "email": "secret@example.com",
                        "joined_unix": 1700000000,
                        "last_login_unix": 1700001000,
                        "last_ip": "203.0.113.9",
                        "locked": false,
                        "online": true,
                        "gm_level": 3,
                        "character_count": 2
                    }]
                }))
            }),
        )
        .route(
            "/v1/admin/players/1/locations",
            get(|| async {
                Json(json!({
                    "account_id": 1,
                    "username": "ADMIN",
                    "locations": [{
                        "guid": 10,
                        "name": "Uther",
                        "map": 0,
                        "zone": 12,
                        "position_x": 1.0,
                        "position_y": 2.0,
                        "position_z": 3.0,
                        "orientation": 0.0,
                        "online": true
                    }]
                }))
            }),
        )
        .route(
            "/forbidden",
            get(|| async {
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "message": "You are not allowed to perform this action",
                        "error_code": "RBAC_FORBIDDEN"
                    })),
                )
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}
