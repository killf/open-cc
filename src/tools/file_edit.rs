//! FileEdit tool - edit file contents using diff

use async_trait::async_trait;
use similar::text::{ChangeTag, TextDiff};
use std::path::PathBuf;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to replace (must match exactly)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Deserialize)]
struct EditInput {
    path: String,
    old_string: String,
    new_string: String,
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["edit".to_string(), "patch".to_string(), "replace".to_string()]
    }

    fn description(&self) -> String {
        "Edit a file by replacing a specific string with new content. \
         The old_string must match exactly. Use for small, targeted changes.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: EditInput = serde_json::from_value(args)?;
        let path = PathBuf::from(&input.path);

        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            context.working_directory.join(&path)
        };

        let start = Instant::now();

        let original = tokio::fs::read_to_string(&abs_path).await
            .map_err(|e| CliError::ToolExecution(format!("cannot read file: {e}")))?;

        if !original.contains(&input.old_string) {
            return Ok(ToolResult::error(
                "old_string not found in file. Make sure to match the exact text including whitespace.".to_string(),
            ));
        }

        let new_content = original.replace(&input.old_string, &input.new_string);

        tokio::fs::write(&abs_path, &new_content).await
            .map_err(|e| CliError::ToolExecution(format!("cannot write file: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let diff = TextDiff::from_lines(&original, &new_content);
        let mut summary = String::new();
        for op in diff.ops() {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                summary.push_str(&format!("{}{:?}", sign, change));
            }
        }

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text: summary }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<EditInput>(args.clone()) {
            format!("Editing: {}", input.path)
        } else {
            "Editing file".to_string()
        }
    }
}
