//! Synthetic output tool - return structured output in a requested format
//!
//! Accepts any JSON input (flexible/passthrough schema) and validates it
//! against a provided JSON schema before returning structured output.

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Accepts any JSON input (passthrough schema).
/// Input for the SyntheticOutput tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SyntheticOutputInput {
    // Flexible: no required fields, accepts any JSON structure.
    // Deserialization will succeed for any valid JSON object.
    #[serde(default)]
    _extra: serde_json::Value,
}

pub struct SyntheticOutputTool;

impl SyntheticOutputTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        // Passthrough schema: accepts any JSON object.
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": true,
            "description": "Any JSON object matching the requested output schema"
        })
    }
}

impl Default for SyntheticOutputTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SyntheticOutputTool {
    fn name(&self) -> &str {
        "SyntheticOutput"
    }

    fn description(&self) -> String {
        "Return structured output in the requested format.".to_string()
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
        let _input: SyntheticOutputInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "This tool requires the TUI/coordinator system.",
        ))
    }
}
