//! ReadMcpResource tool — read a specific resource from a connected MCP server

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input for ReadMcpResource
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadMcpResourceInput {
    /// The MCP server name that owns the resource.
    pub server: String,
    /// The URI of the resource to read.
    pub uri: String,
}

pub struct ReadMcpResourceTool;

impl ReadMcpResourceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadMcpResourceTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &str {
        "ReadMcpResource"
    }

    fn description(&self) -> String {
        "Read the content of a specific resource exposed by a connected MCP server. \
         Use ListMcpResources first to discover available resources and their URIs.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "The name of the MCP server that owns the resource."
                },
                "uri": {
                    "type": "string",
                    "description": "The URI of the resource to read (e.g. 'file:///path/to/resource')."
                }
            },
            "required": ["server", "uri"],
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
        let _input: ReadMcpResourceInput = match serde_json::from_value(args) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult::error(format!("invalid input: {e}")));
            }
        };

        // Stub: MCP resource reading requires a connected MCP server and
        // integration with the MCP client manager. This tool is a placeholder
        // until the MCP resource protocol is wired into the tool system.
        Ok(ToolResult::error(
            "MCP resources require a connected MCP server.",
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<ReadMcpResourceInput>(args.clone()) {
            format!("Reading MCP resource: {} from server: {}", input.uri, input.server)
        } else {
            "Reading MCP resource".to_string()
        }
    }
}
