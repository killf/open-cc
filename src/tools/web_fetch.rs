//! WebFetch tool - fetch URL content

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WebFetchInput {
    url: String,
    #[serde(default)]
    prompt: Option<String>,
}

pub struct WebFetchTool {
    http_client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                },
                "prompt": {
                    "type": "string",
                    "description": "Optional instruction for what to extract"
                }
            },
            "required": ["url"]
        })
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["fetch".to_string(), "http".to_string(), "url".to_string()]
    }

    fn description(&self) -> String {
        "Fetch content from a URL (GET request). Returns the page content.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: WebFetchInput = serde_json::from_value(args)?;
        let start = Instant::now();

        let response = self
            .http_client
            .get(&input.url)
            .header("User-Agent", "Claude Code CLI/2.0")
            .send()
            .await
            .map_err(|e| CliError::ToolExecution(format!("fetch failed: {e}")))?;

        if !response.status().is_success() {
            return Ok(ToolResult::error(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| CliError::ToolExecution(format!("failed to read response: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let text = if content_type.contains("application/json") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                serde_json::to_string_pretty(&parsed).unwrap_or(body)
            } else {
                body
            }
        } else {
            body
        };

        let truncated = if text.len() > 50_000 {
            format!("{}...\n\n(Output truncated - {} chars total)", &text[..50_000], text.len())
        } else {
            text
        };

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text: truncated }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<WebFetchInput>(args.clone()) {
            format!("Fetching: {}", input.url)
        } else {
            "Fetching URL".to_string()
        }
    }
}
