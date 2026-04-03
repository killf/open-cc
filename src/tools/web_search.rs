//! WebSearch tool - search the web

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WebSearchInput {
    query: String,
    #[serde(default)]
    source: Option<String>,
}

pub struct WebSearchTool {
    http_client: reqwest::Client,
}

impl WebSearchTool {
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
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "source": {
                    "type": "string",
                    "description": "Search source (e.g., 'google', 'bing')"
                }
            },
            "required": ["query"]
        })
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["search".to_string(), "google".to_string(), "bing".to_string()]
    }

    fn description(&self) -> String {
        "Search the web for information.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: WebSearchInput = serde_json::from_value(args)?;
        let start = Instant::now();

        let query = urlencoding::encode(&input.query);
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}&kl=us-en",
            query
        );

        let response = self
            .http_client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Claude Code CLI/2.0)")
            .send()
            .await
            .map_err(|e| CliError::ToolExecution(format!("search failed: {e}")))?;

        if !response.status().is_success() {
            return Ok(ToolResult::error(format!(
                "Search failed: HTTP {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| CliError::ToolExecution(format!("failed to read response: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let title_re = regex::Regex::new(r#"<a class="result__a"[^>]*>([^<]+)</a>"#).ok();
        let link_re = regex::Regex::new(r#"href="([^"]+)""#).ok();

        let mut results = Vec::new();
        for line in body.lines() {
            if line.contains("result__title") || line.contains("web-result") {
                let title = title_re.as_ref().and_then(|r| r.captures(line))
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                    .unwrap_or_default();

                let link = link_re.as_ref().and_then(|r| r.captures(line))
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                    .unwrap_or_default();

                if !title.is_empty() && !link.is_empty() {
                    results.push(format!("- {} ({})", title, link));
                }
            }
        }

        let text = if results.is_empty() {
            format!(
                "Search results for: {}\n\nNo structured results found. Try using WebFetch directly.",
                input.query
            )
        } else {
            format!(
                "Search results for: {}\n\n{}\n\n(Results from DuckDuckGo)",
                input.query,
                results.join("\n")
            )
        };

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<WebSearchInput>(args.clone()) {
            format!("Searching web for: {}", input.query)
        } else {
            "Searching the web".to_string()
        }
    }
}
