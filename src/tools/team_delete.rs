//! TeamDelete tool — disband a swarm team and clean up

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

// ---------------------------------------------------------------------------
// TeamDeleteTool
// ---------------------------------------------------------------------------

pub struct TeamDeleteTool;

impl TeamDeleteTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }
}

impl Default for TeamDeleteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TeamDeleteTool {
    fn name(&self) -> &str {
        "TeamDelete"
    }

    fn description(&self) -> String {
        "Clean up team and task directories when the swarm is complete. \
         Fails if any teammates are still active — use requestShutdown to \
         terminate them first."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        _args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        // Coordinator team management is not yet wired up in the Rust codebase.
        // Return an error stub until the coordinator system supports team deletion.
        Ok(ToolResult::error(
            "Team deletion requires the coordinator system.",
        ))
    }

    fn render_use_message(&self, _args: &serde_json::Value) -> String {
        "Cleaning up team".to_string()
    }
}
