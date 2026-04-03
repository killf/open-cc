//! ExitPlanMode tool - present the plan for approval and start coding

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Prompt-based permission requested by the plan
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllowedPrompt {
    /// The tool this prompt applies to (e.g. "Bash")
    pub tool: String,
    /// Semantic description of the action (e.g. "run tests", "install dependencies")
    pub prompt: String,
}

/// Input for the ExitPlanMode tool
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitPlanModeInput {
    /// Prompt-based permissions needed to implement the plan.
    /// These describe categories of actions rather than specific commands.
    #[serde(default)]
    allowed_prompts: Option<Vec<AllowedPrompt>>,
}

pub struct ExitPlanModeTool;

impl ExitPlanModeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExitPlanModeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }

    fn description(&self) -> String {
        "Prompts the user to exit plan mode and start coding.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "allowedPrompts": {
                    "type": "array",
                    "description": "Prompt-based permissions needed to implement the plan. These describe categories of actions rather than specific commands.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": {
                                "type": "string",
                                "description": "The tool this prompt applies to (e.g. \"Bash\")"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Semantic description of the action, e.g. \"run tests\", \"install dependencies\""
                            }
                        },
                        "required": ["tool", "prompt"],
                        "additionalProperties": false
                    }
                }
            },
            "additionalProperties": false
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: ExitPlanModeInput = serde_json::from_value(args)?;
        Ok(ToolResult::error(
            "Plan mode exit is not available in this context.",
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<ExitPlanModeInput>(args.clone()) {
            if let Some(prompts) = &input.allowed_prompts {
                let n = prompts.len();
                format!("Exiting plan mode ({} allowed prompt{})", n, if n == 1 { "" } else { "s" })
            } else {
                "Exiting plan mode".to_string()
            }
        } else {
            "Exiting plan mode".to_string()
        }
    }
}
