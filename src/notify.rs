//! Best-effort async notifications for admin events (Discord + Telegram).
//!
//! The API never blocks or fails a request because of a notification:
//! every send runs in a spawned task with a timeout and a single retry,
//! and failures are only logged.

use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);
const NOTIFY_RETRY_DELAY: Duration = Duration::from_secs(1);
const DISCORD_MAX_CHARS: usize = 1900;
const TELEGRAM_MAX_CHARS: usize = 4000;

/// Keys that must never leave the process through notifications.
const REDACTED_KEYS: [&str; 9] = [
    "password",
    "token",
    "access_token",
    "refresh_token",
    "verifier",
    "secret",
    "authorization",
    "private_key",
    "api_key",
];

#[derive(Clone, Default)]
pub struct NotifyConfig {
    pub enabled: bool,
    pub discord_webhook_url: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl NotifyConfig {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn disabled() -> Self {
        Self::default()
    }

    fn has_channels(&self) -> bool {
        self.discord_webhook_url.is_some()
            || (self.telegram_bot_token.is_some() && self.telegram_chat_id.is_some())
    }

    fn active(&self) -> bool {
        self.enabled && self.has_channels()
    }
}

#[derive(Clone)]
pub struct Notifier {
    inner: Arc<Option<NotifierInner>>,
}

struct NotifierInner {
    config: NotifyConfig,
    client: reqwest::Client,
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new(NotifyConfig::disabled())
    }
}

impl Notifier {
    pub fn new(config: NotifyConfig) -> Self {
        if !config.active() {
            return Self {
                inner: Arc::new(None),
            };
        }
        let client = reqwest::Client::builder()
            .timeout(NOTIFY_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            inner: Arc::new(Some(NotifierInner { config, client })),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn disabled() -> Self {
        Self::default()
    }

    fn active(&self) -> bool {
        self.inner.is_some()
    }

    /// Queue an admin event notification. Never blocks the caller with
    /// network I/O and never fails: delivery happens on a spawned task.
    pub fn notify(&self, event: AdminEvent<'_>) {
        if !self.active() {
            return;
        }

        let text = format_message(&event);
        let Some(inner) = self.inner.as_ref() else {
            return;
        };
        let client = inner.client.clone();
        let discord_webhook_url = inner.config.discord_webhook_url.clone();
        let telegram_bot_token = inner.config.telegram_bot_token.clone();
        let telegram_chat_id = inner.config.telegram_chat_id.clone();

        tokio::spawn(async move {
            if let Some(url) = discord_webhook_url {
                send_with_retry(|| {
                    client
                        .post(&url)
                        .json(&json!({ "content": truncate(&text, DISCORD_MAX_CHARS) }))
                        .timeout(NOTIFY_TIMEOUT)
                })
                .await;
            }
            if let (Some(bot_token), Some(chat_id)) = (telegram_bot_token, telegram_chat_id) {
                let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
                send_with_retry(|| {
                    client
                        .post(&url)
                        .form(&[
                            ("chat_id", chat_id.as_str()),
                            ("text", truncate(&text, TELEGRAM_MAX_CHARS)),
                        ])
                        .timeout(NOTIFY_TIMEOUT)
                })
                .await;
            }
        });
    }
}

pub struct AdminEvent<'a> {
    pub action: &'a str,
    pub actor_username: &'a str,
    pub actor_account_id: i64,
    pub target_type: &'a str,
    pub target_id: Option<&'a str>,
    pub details: &'a Value,
}

impl AdminEvent<'_> {
    fn target_label(&self) -> String {
        match self.target_id {
            Some(id) => format!("{}:{id}", self.target_type),
            None => self.target_type.to_string(),
        }
    }
}

fn format_message(event: &AdminEvent<'_>) -> String {
    let details = sanitize(event.details);
    let details = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string());
    format!(
        "[wow-backend] {}\nactor: {} ({})\ntarget: {}\ndetails: {}",
        event.action,
        event.actor_username,
        event.actor_account_id,
        event.target_label(),
        details
    )
}

/// Recursively removes sensitive keys from a JSON value.
pub fn sanitize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                if REDACTED_KEYS
                    .iter()
                    .any(|sensitive| key.eq_ignore_ascii_case(sensitive))
                {
                    out.insert(key.clone(), json!("[redacted]"));
                } else {
                    out.insert(key.clone(), sanitize(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize).collect()),
        other => other.clone(),
    }
}

fn truncate(text: &str, max_chars: usize) -> &str {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}

async fn send_with_retry<F>(build_request: F)
where
    F: Fn() -> reqwest::RequestBuilder,
{
    for attempt in 1..=2 {
        let result = build_request().send().await;
        match result {
            Ok(response) if response.status().is_success() => return,
            Ok(response) => {
                let status = response.status();
                if attempt == 2 {
                    tracing::warn!(status = %status, "webhook notification rejected");
                    return;
                }
            }
            Err(err) => {
                if attempt == 2 {
                    tracing::warn!(error = ?err, "webhook notification failed");
                    return;
                }
            }
        }
        tokio::time::sleep(NOTIFY_RETRY_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_redacts_sensitive_keys_recursively() {
        let input = json!({
            "character_name": "Xerath",
            "command": "kick Xerath",
            "Password": "hunter2",
            "nested": {
                "access_token": "wowst_secret",
                "safe": 1,
                "list": [{ "secret": "x", "ok": true }]
            }
        });

        let out = sanitize(&input);

        assert_eq!(out["character_name"], json!("Xerath"));
        assert_eq!(out["command"], json!("kick Xerath"));
        assert_eq!(out["Password"], json!("[redacted]"));
        assert_eq!(out["nested"]["access_token"], json!("[redacted]"));
        assert_eq!(out["nested"]["safe"], json!(1));
        assert_eq!(out["nested"]["list"][0]["secret"], json!("[redacted]"));
        assert_eq!(out["nested"]["list"][0]["ok"], json!(true));
    }

    #[test]
    fn format_message_includes_action_actor_target_details() {
        let event = AdminEvent {
            action: "player.kick",
            actor_username: "Admin",
            actor_account_id: 42,
            target_type: "character",
            target_id: Some("Xerath"),
            details: &json!({ "reason": "afk arena", "password": "leak" }),
        };

        let text = format_message(&event);

        assert!(text.contains("[wow-backend] player.kick"));
        assert!(text.contains("actor: Admin (42)"));
        assert!(text.contains("target: character:Xerath"));
        assert!(text.contains("\"reason\":\"afk arena\""));
        assert!(text.contains("[redacted]"));
        assert!(!text.contains("leak"));
    }

    #[test]
    fn disabled_notifier_is_inert() {
        let notifier = Notifier::disabled();
        assert!(!notifier.active());

        let event = AdminEvent {
            action: "player.ban",
            actor_username: "Admin",
            actor_account_id: 1,
            target_type: "account",
            target_id: Some("Cheater"),
            details: &json!({}),
        };
        notifier.notify(event); // must not panic without a runtime
    }
}
