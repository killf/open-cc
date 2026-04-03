//! McpAuth tool — start OAuth flow for an MCP server that requires authentication

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

pub struct McpAuthTool;

impl McpAuthTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpAuthTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpAuthTool {
    fn name(&self) -> &str {
        "McpAuth"
    }

    fn description(&self) -> String {
        "Start the OAuth authentication flow for an MCP server that requires it. \
         This will return an authorization URL to share with the user; once they \
         complete the flow in their browser, the server's tools become available \
         automatically.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        _args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        // Stub: MCP OAuth flow requires the TUI to open the browser, handle the
        // callback, and manage the redirect URI. It cannot run in a headless
        // or non-interactive context.
        Ok(ToolResult::error(
            "MCP OAuth flow requires the TUI.".to_string(),
        ))
    }

    fn render_use_message(&self, _args: &serde_json::Value) -> String {
        "Starting MCP OAuth authentication".to_string()
    }
}
