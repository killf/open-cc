//! Monitor tool (stub)

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[allow(dead_code)]
pub struct MonitorTool;

impl MonitorTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MonitorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MonitorTool {
    fn name(&self) -> &str {
        "Monitor"
    }

    fn description(&self) -> String {
        "Monitor system or process metrics.".to_string()
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
        "Running Monitor tool".to_string()
    }
}
