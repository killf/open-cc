//! TerminalCapture tool — documentation/prompt-only stub

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[allow(dead_code)]
pub struct TerminalCaptureTool;

impl TerminalCaptureTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalCaptureTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TerminalCaptureTool {
    fn name(&self) -> &str {
        "TerminalCapture"
    }

    fn description(&self) -> String {
        "Capture terminal output. This is a documentation/prompt-only tool.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn call(
        &self,
        _args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        Ok(ToolResult::text("This is a documentation/prompt-only tool."))
    }
}
