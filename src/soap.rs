use std::time::Duration;

use async_trait::async_trait;
use axum::http::StatusCode;

use crate::error::ApiError;

#[async_trait]
pub trait SoapClient: Send + Sync {
    async fn execute(&self, command: &str) -> Result<String, ApiError>;
}

pub struct LiveAcoreSoapClient {
    http: reqwest::Client,
    base_url: String,
    user: String,
    password: String,
}

pub fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn unescape_xml(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub fn extract_soap_result(body: &str) -> Option<String> {
    for tag in ["ac:result", "result"] {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        if let Some(start) = body.find(&open)
            && let Some(rel_end) = body[start..].find(&close)
        {
            let inner = &body[start + open.len()..start + rel_end];
            return Some(unescape_xml(inner).trim().to_string());
        }
    }
    None
}

pub fn build_soap_envelope(command: &str) -> String {
    format!(
        "<SOAP-ENV:Envelope \
         xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" \
         xmlns:ac=\"urn:AC\">\
         <SOAP-ENV:Body><ac:executeCommand><ac:command>{}</ac:command>\
         </ac:executeCommand></SOAP-ENV:Body></SOAP-ENV:Envelope>",
        escape_xml(command)
    )
}

impl LiveAcoreSoapClient {
    pub fn new(
        base_url: impl Into<String>,
        user: String,
        password: String,
    ) -> Result<Self, ApiError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(ApiError::internal(
                "SOAP base URL is empty",
                "SOAP_BAD_CONFIG",
            ));
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| ApiError::internal("failed to build SOAP client", "SOAP_BAD_CONFIG"))?;

        Ok(Self {
            http,
            base_url,
            user,
            password,
        })
    }
}

#[async_trait]
impl SoapClient for LiveAcoreSoapClient {
    async fn execute(&self, command: &str) -> Result<String, ApiError> {
        let envelope = build_soap_envelope(command);

        let response = self
            .http
            .post(&self.base_url)
            .basic_auth(&self.user, Some(&self.password))
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(envelope)
            .send()
            .await
            .map_err(|err| {
                ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "WorldServer SOAP call failed",
                    "SOAP_CALL_FAILED",
                )
                .with_extra(serde_json::json!({ "error": err.to_string() }))
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "WorldServer SOAP returned an error",
                "SOAP_CALL_FAILED",
            )
            .with_extra(serde_json::json!({ "http_status": status.as_u16() })));
        }

        extract_soap_result(&body).ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                "WorldServer SOAP response has no result",
                "SOAP_BAD_RESPONSE",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_escapes_command() {
        let envelope = build_soap_envelope("kick Xe'rah & <boss>");
        assert!(envelope.contains("kick Xe&apos;rah &amp; &lt;boss&gt;"));
        assert!(envelope.starts_with("<SOAP-ENV:Envelope"));
    }

    #[test]
    fn parse_extracts_ac_result() {
        let body = "<env><body><ac:executeCommandResponse>\
                    <ac:result>OK &amp; done</ac:result></ac:executeCommandResponse></body></env>";
        let parsed = extract_soap_result(body).expect("result");
        assert_eq!(parsed, "OK & done");
    }

    #[test]
    fn parse_returns_none_without_result_tag() {
        assert!(extract_soap_result("<env></env>").is_none());
    }

    #[test]
    fn unescape_handles_entities() {
        assert_eq!(unescape_xml("a&amp;b&lt;c&gt;"), "a&b<c>");
    }

    #[test]
    fn reject_empty_base_url() {
        let result = LiveAcoreSoapClient::new("", "u".to_string(), "p".to_string());
        assert!(result.is_err());
    }
}
