//! Sleep tool - pause execution for a specified duration

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input for the Sleep tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SleepInput {
    #[serde(default)]
    duration_secs: Option<u64>,
}

pub struct SleepTool;

impl SleepTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "duration_secs": {
                    "type": "number",
                    "description": "Duration to sleep in seconds"
                }
            }
        })
    }
}

impl Default for SleepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }

    fn description(&self) -> String {
        "Pause execution for a specified duration.".to_string()
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
        let _input: SleepInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }
}
