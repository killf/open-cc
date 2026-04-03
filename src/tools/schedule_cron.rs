//! Schedule cron tool - create, delete, and list scheduled cron tasks
//!
//! Implements CronCreate, CronDelete, and CronList as a single tool with an `action` field.

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input for the ScheduleCron tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScheduleCronInput {
    /// Action to perform: "create", "delete", or "list"
    action: String,
    /// Cron expression (required for "create")
    #[serde(default)]
    cron: Option<String>,
    /// Task description (required for "create")
    #[serde(default)]
    description: Option<String>,
    /// Task ID (required for "delete")
    #[serde(default)]
    id: Option<String>,
}

pub struct ScheduleCronTool;

impl ScheduleCronTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "delete", "list"],
                    "description": "The action to perform: create, delete, or list"
                },
                "cron": {
                    "type": "string",
                    "description": "Cron expression (required for create)"
                },
                "description": {
                    "type": "string",
                    "description": "Task description (required for create)"
                },
                "id": {
                    "type": "string",
                    "description": "Task ID (required for delete)"
                }
            },
            "required": ["action"]
        })
    }
}

impl Default for ScheduleCronTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ScheduleCronTool {
    fn name(&self) -> &str {
        "CronCreate"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["CronDelete".to_string(), "CronList".to_string()]
    }

    fn description(&self) -> String {
        "Schedule, delete, or list cron tasks.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: ScheduleCronInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }
}
