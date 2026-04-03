//! ListMcpResources tool — list available resources from a connected MCP server

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input for ListMcpResources
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMcpResourcesInput {
    /// Optional MCP server name to list resources from.
    /// If omitted, lists resources from all connected servers.
    #[serde(default)]
    pub server: Option<String>,
}

pub struct ListMcpResourcesTool;

impl ListMcpResourcesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListMcpResourcesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &str {
        "ListMcpResources"
    }

    fn description(&self) -> String {
        "List available resources exposed by a connected MCP server. \
         MCP resources are typed content (files, configs, etc.) that servers \
         make available for tools to read. Pass a server name to scope results, \
         or omit it to list resources from all connected servers.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "Optional MCP server name to list resources from. If omitted, lists resources from all connected servers."
                }
            },
            "additionalProperties": false
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: ListMcpResourcesInput = match serde_json::from_value(args) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult::error(format!("invalid input: {e}")));
            }
        };

        // Stub: MCP resource listing requires a connected MCP server and
        // integration with the MCP client manager. This tool is a placeholder
        // until the MCP resource protocol is wired into the tool system.
        Ok(ToolResult::error(
            "MCP resources require a connected MCP server.",
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<ListMcpResourcesInput>(args.clone()) {
            if let Some(ref server) = input.server {
                format!("Listing MCP resources from server: {server}")
            } else {
                "Listing MCP resources from all servers".to_string()
            }
        } else {
            "Listing MCP resources".to_string()
        }
    }
}
