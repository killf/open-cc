//! Tungsten tool (stub)

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[allow(dead_code)]
pub struct TungstenTool;

impl TungstenTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TungstenTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TungstenTool {
    fn name(&self) -> &str {
        "Tungsten"
    }

    fn description(&self) -> String {
        "Tungsten heavy-duty task execution engine.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(
        &self,
        _args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }

    fn render_use_message(&self, _args: &serde_json::Value) -> String {
        "Running Tungsten tool".to_string()
    }
}
