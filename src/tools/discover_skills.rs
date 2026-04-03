//! DiscoverSkills tool — documentation/prompt-only stub

use async_trait::async_trait;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

#[allow(dead_code)]
pub struct DiscoverSkillsTool;

impl DiscoverSkillsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiscoverSkillsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &str {
        "DiscoverSkills"
    }

    fn description(&self) -> String {
        "Discover available skills. This is a documentation/prompt-only tool.".to_string()
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
