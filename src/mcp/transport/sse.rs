//! SSE transport for remote MCP servers

use futures::StreamExt;
use std::collections::HashMap;

use crate::error::CliError;
use crate::mcp::protocol::McpMessage;
use crate::mcp::client::McpTransport;

use async_trait::async_trait;

pub struct SseTransport {
    #[allow(dead_code)]
    event_source_url: String,
    #[allow(dead_code)]
    post_url: String,
    http_client: reqwest::Client,
}

impl SseTransport {
    pub fn new(url: &str, headers: &HashMap<String, String>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .default_headers(
                headers.iter()
                    .map(|(k, v)| (k.parse().unwrap_or_else(|_| reqwest::header::HeaderName::from_static("x-custom")), v.parse().unwrap_or_else(|_| reqwest::header::HeaderValue::from_static(""))))
                    .collect()
            )
            .build()
            .unwrap_or_default();

        Self {
            event_source_url: url.to_string(),
            post_url: url.to_string(),
            http_client,
        }
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn send(&self, _msg: McpMessage) -> Result<(), CliError> {
        // SSE is primarily a receive channel
        Err(CliError::Mcp("SSE send not implemented".into()))
    }

    async fn recv(&self) -> Result<McpMessage, CliError> {
        let response = self.http_client
            .get(&self.event_source_url)
            .send()
            .await
            .map_err(|e| CliError::Mcp(format!("SSE request failed: {e}")))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| CliError::Mcp(format!("SSE stream error: {e}")))?;
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(msg) = serde_json::from_str::<McpMessage>(data) {
                        return Ok(msg);
                    }
                }
            }
        }
        Err(CliError::Mcp("SSE stream ended".into()))
    }
}
