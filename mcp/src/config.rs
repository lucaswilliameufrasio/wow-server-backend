pub struct Config {
    pub api_base_url: String,
    pub metrics_base_url: String,
    pub api_token: Option<String>,
    pub request_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let api_base_url = std::env::var("MCP_API_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        if api_base_url.trim().is_empty() {
            return Err("MCP_API_BASE_URL is empty".to_string());
        }

        let api_token = std::env::var("MCP_API_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let request_timeout_secs = std::env::var("MCP_REQUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(30);

        let metrics_base_url = std::env::var("MCP_METRICS_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:9090".to_string());
        if metrics_base_url.trim().is_empty() {
            return Err("MCP_METRICS_BASE_URL is empty".to_string());
        }

        Ok(Self {
            api_base_url,
            metrics_base_url,
            api_token,
            request_timeout_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn defaults_when_env_missing() {
        let _guard = env_lock().lock().unwrap();
        unsafe { std::env::remove_var("MCP_API_BASE_URL") };
        unsafe { std::env::remove_var("MCP_API_TOKEN") };
        unsafe { std::env::remove_var("MCP_REQUEST_TIMEOUT_SECS") };

        let config = Config::from_env().expect("config");

        assert_eq!(config.api_base_url, "http://127.0.0.1:3000");
        assert_eq!(config.metrics_base_url, "http://127.0.0.1:9090");
        assert!(config.api_token.is_none());
        assert_eq!(config.request_timeout_secs, 30);
    }

    #[test]
    fn rejects_empty_base_url() {
        let _guard = env_lock().lock().unwrap();

        unsafe { std::env::set_var("MCP_API_BASE_URL", " ") };

        let config = Config::from_env();

        assert!(config.is_err());
        unsafe { std::env::remove_var("MCP_API_BASE_URL") };
    }
}
