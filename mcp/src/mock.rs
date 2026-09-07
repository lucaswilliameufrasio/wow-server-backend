use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, patch, post},
};
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
        )
        .route(
            "/v1/admin/players/5/lock",
            patch(|Json(body): Json<serde_json::Value>| async move {
                Json(json!({
                    "account_id": 5,
                    "locked": body.get("locked").and_then(|v| v.as_bool()).unwrap_or(false)
                }))
            }),
        )
        .route(
            "/v1/admin/audit-log",
            get(|| async {
                Json(json!({
                    "limit": 50,
                    "cursor": null,
                    "entries": [{
                        "id": 2,
                        "actor_account_id": 1,
                        "action": "account.lock",
                        "target_type": "account",
                        "target_id": "5",
                        "details": {"locked": true, "actor_username": "ADMIN"},
                        "created_at_unix": 1700001000
                    }]
                }))
            }),
        )
        .route(
            "/v1/admin/server/status",
            post(|| async {
                Json(json!({
                    "command": "server info",
                    "result": "Online connections: 3. Uptime: 2h."
                }))
            }),
        )
        .route(
            "/v1/admin/server/announce",
            post(|| async {
                Json(json!({
                    "command": "announce test",
                    "result": "Announcement sent by console."
                }))
            }),
        )
        .route(
            "/v1/admin/server/restart",
            post(|| async {
                Json(json!({
                    "command": "server restart 60",
                    "result": "Server restart scheduled in 60 seconds."
                }))
            }),
        )
        .route(
            "/v1/admin/server/command",
            post(|| async {
                Json(json!({
                    "command": "server info",
                    "result": "Online connections: 3. Uptime: 2h."
                }))
            }),
        )
        .route(
            "/v1/admin/players/kick",
            post(|| async {
                Json(json!({
                    "command": "kick Xerath",
                    "result": "Player Xerath kicked by console."
                }))
            }),
        )
        .route(
            "/v1/admin/accounts/ban",
            post(|| async {
                Json(json!({
                    "command": "ban account Cheater 7d exploits",
                    "result": "Account Cheater banned for 7 days."
                }))
            }),
        )
        .route(
            "/v1/admin/accounts/unban",
            post(|| async {
                Json(json!({
                    "command": "unban account Cheater",
                    "result": "Account Cheater unbanned."
                }))
            }),
        )
        .route(
            "/v1/admin/server/logs",
            get(|| async {
                Json(json!({
                    "file": "Server.log",
                    "lines": ["2026-09-07 03:32:00 Worldserver: restart scheduled", "2026-09-07 03:47:00 Worldserver: shutting down"]
                }))
            }),
        )
        .route(
            "/v1/admin/server/crashes",
            get(|| async {
                Json(json!({
                    "file": "Crash.log",
                    "lines": ["2026-09-07 03:31:58 Crash: sigsegv in Map::Update"]
                }))
            }),
        )
        .route(
            "/v1/admin/players/teleport",
            post(|| async {
                Json(json!({
                    "command": "tele name Xerath Stormwind City",
                    "result": "You teleport Xerath to Stormwind City."
                }))
            }),
        )
        .route(
            "/v1/admin/players/items",
            post(|| async {
                Json(json!({
                    "command": "additem name Xerath 19019 2",
                    "result": "Added item 19019 to Xerath."
                }))
            }),
        )
        .route(
            "/v1/admin/players/money",
            post(|| async {
                Json(json!({
                    "command": "modify money name Xerath 12345g67s89c",
                    "result": "Money modified."
                }))
            }),
        )
        .route(
            "/v1/admin/players/level",
            post(|| async {
                Json(json!({
                    "command": "setlevel name Xerath 80",
                    "result": "Level set to 80."
                }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}
