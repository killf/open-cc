//! Tool search tool - search for deferred tools by keyword or direct selection

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input for the ToolSearch tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ToolSearchInput {
    query: String,
    #[serde(default)]
    max_results: Option<u32>,
}

pub struct ToolSearchTool;

impl ToolSearchTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Query to find deferred tools. Use \"select:<tool_name>\" for direct selection, or keywords to search."
                },
                "max_results": {
                    "type": "number",
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }
}

impl Default for ToolSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }

    fn description(&self) -> String {
        "Search for deferred tools by keyword or direct tool selection.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: ToolSearchInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }
}
