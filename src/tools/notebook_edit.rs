//! NotebookEdit tool — edit Jupyter notebook (.ipynb) cells.
//
// Input: notebook_path (String), cell_id (Option<String>), new_source (String),
// cell_type (Option<String>), edit_mode (Option<String>)
// name: "NotebookEdit"

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input schema for the NotebookEdit tool.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NotebookEditInput {
    /// Absolute path to the .ipynb file to edit.
    notebook_path: String,
    /// ID of the cell to edit. When inserting, the new cell is inserted after
    /// this cell (or at the beginning if not specified).
    #[serde(default)]
    cell_id: Option<String>,
    /// The new source content for the cell.
    new_source: String,
    /// Cell type: "code" or "markdown". Required when edit_mode is "insert".
    #[serde(default)]
    cell_type: Option<String>,
    /// Edit mode: "replace" (default), "insert", or "delete".
    #[serde(default = "default_edit_mode")]
    edit_mode: String,
}

fn default_edit_mode() -> String {
    "replace".to_string()
}

pub struct NotebookEditTool;

impl NotebookEditTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "notebook_path": {
                    "type": "string",
                    "description": "The absolute path to the Jupyter notebook file to edit (must be absolute, not relative)"
                },
                "cell_id": {
                    "type": "string",
                    "description": "The ID of the cell to edit. When inserting a new cell, the new cell will be inserted after the cell with this ID, or at the beginning if not specified."
                },
                "new_source": {
                    "type": "string",
                    "description": "The new source for the cell"
                },
                "cell_type": {
                    "type": "string",
                    "enum": ["code", "markdown"],
                    "description": "The type of the cell (code or markdown). If not specified, it defaults to the current cell type. If using edit_mode=insert, this is required."
                },
                "edit_mode": {
                    "type": "string",
                    "enum": ["replace", "insert", "delete"],
                    "description": "The type of edit to make (replace, insert, delete). Defaults to replace."
                }
            },
            "required": ["notebook_path", "new_source"]
        })
    }
}

impl Default for NotebookEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str {
        "NotebookEdit"
    }

    fn description(&self) -> String {
        "Edits a Jupyter notebook (.ipynb) cell: replace source, insert a new cell, \
         or delete an existing cell. Validates that the notebook has been read before \
         editing to prevent silent data loss."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: NotebookEditInput = serde_json::from_value(args)
            .map_err(|e| CliError::ToolExecution(format!("Invalid input: {e}")))?;

        Err(CliError::ToolExecution(
            "This tool requires the TUI/system integration.".to_string(),
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<NotebookEditInput>(args.clone()) {
            let mode = input.edit_mode.as_str();
            format!(
                "Editing notebook {} (mode: {}): {}",
                input.notebook_path,
                mode,
                &input.new_source[..input.new_source.len().min(60)]
            )
        } else {
            "Editing notebook cell".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        if result.is_error {
            "NotebookEdit failed.".to_string()
        } else {
            "NotebookEdit completed.".to_string()
        }
    }
}
