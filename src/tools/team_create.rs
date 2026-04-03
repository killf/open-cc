//! TeamCreate tool — create a new multi-agent swarm team

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input schema for TeamCreateTool
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum TeamCreateInput {
    /// Create a new team
    #[serde(rename = "create")]
    Create {
        /// Name for the new team to create.
        team_name: String,
        /// Team description/purpose.
        #[serde(default)]
        description: Option<String>,
        /// Type/role of the team lead (e.g., "researcher", "test-runner").
        #[serde(default)]
        agent_type: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// TeamCreateTool
// ---------------------------------------------------------------------------

pub struct TeamCreateTool;

impl TeamCreateTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team_name": {
                    "type": "string",
                    "description": "Name for the new team to create."
                },
                "description": {
                    "type": "string",
                    "description": "Team description/purpose."
                },
                "agent_type": {
                    "type": "string",
                    "description": "Type/role of the team lead (e.g., 'researcher', 'test-runner')."
                }
            },
            "required": ["team_name"]
        })
    }
}

impl Default for TeamCreateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &str {
        "TeamCreate"
    }

    fn description(&self) -> String {
        "Create a new team for coordinating multiple agents. A leader can only \
         manage one team at a time. Use TeamDelete to end the current team \
         before creating a new one."
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
        // Return an error stub until the coordinator system supports team creation.
        Ok(ToolResult::error(
            "Team creation requires the coordinator system.",
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Some(team_name) = args.get("team_name").and_then(|v| v.as_str()) {
            format!("Creating team: {}", team_name)
        } else {
            "Creating a new team".to_string()
        }
    }
}
