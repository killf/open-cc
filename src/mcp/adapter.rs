//! MCP tool adapter - wraps an MCP tool as a `dyn Tool`

use std::sync::Arc;
use serde_json::Value as JsonValue;

use crate::error::CliError;
use crate::mcp::client::McpClient;
use crate::types::{
    ImageSource, ResultContentBlock, Tool as ToolTrait, ToolContext, ToolResult,
};

/// Adapter that wraps an MCP tool so it implements the `Tool` trait
pub struct McpToolAdapter {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: JsonValue,
    pub client: Arc<McpClient>,
}

impl McpToolAdapter {
    pub fn new(
        name: String,
        description: Option<String>,
        input_schema: JsonValue,
        client: Arc<McpClient>,
    ) -> Self {
        Self {
            name,
            description,
            input_schema,
            client,
        }
    }
}

#[async_trait::async_trait]
impl ToolTrait for McpToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> String {
        self.description.clone().unwrap_or_default()
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    fn is_read_only(&self) -> bool {
        // MCP tools are treated as potentially write operations
        false
    }

    async fn call(
        &self,
        args: JsonValue,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let result = self.client.call_tool(&self.name, Some(args)).await?;

        let is_error = result.is_error.unwrap_or(false);
        let content: Vec<ResultContentBlock> = result
            .content
            .into_iter()
            .map(|b| match b {
                crate::mcp::McpContentBlock::Text { text } => {
                    ResultContentBlock::Text { text }
                }
                crate::mcp::McpContentBlock::Image { data, mime_type } => {
                    ResultContentBlock::Image {
                        source: ImageSource {
                            source_type: "base64".to_string(),
                            media_type: mime_type.unwrap_or_else(|| "image/png".to_string()),
                            data,
                        },
                    }
                }
                crate::mcp::McpContentBlock::Resource { resource } => {
                    ResultContentBlock::Text {
                        text: format!("[resource: {}]", resource.uri)
                    }
                }
            })
            .collect();

        Ok(ToolResult {
            content,
            is_error,
            metrics: None,
        })
    }
}
