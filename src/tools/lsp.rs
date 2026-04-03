//! LSP tool — Language Server Protocol code intelligence (go-to-definition,
//! find-references, hover, document symbols, workspace symbols, etc.).
//
// Input: operation (String), file_path (String), line (u32), character (u32)
// name: "LSP"
// is_read_only: true
//
// NOTE: There is an existing LSP module at src/lsp/ (client, protocol, mod).
// This tool stub provides the CLI/tooling interface; the full implementation
// would integrate with that module.

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Valid LSP operations supported by the tool.
const LSP_OPERATIONS: &[&str] = &[
    "goToDefinition",
    "findReferences",
    "hover",
    "documentSymbol",
    "workspaceSymbol",
    "goToImplementation",
    "prepareCallHierarchy",
    "incomingCalls",
    "outgoingCalls",
];

/// Input for the LSP tool.
/// #[allow(dead_code)]: struct is used as a deserialization target; fields are validated
/// but the stub immediately discards the value.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LspInput {
    /// The LSP operation to perform.
    operation: String,
    /// The absolute or relative path to the file.
    file_path: String,
    /// Line number (1-based, as shown in editors).
    line: u32,
    /// Character offset (1-based, as shown in editors).
    character: u32,
}

pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": LSP_OPERATIONS,
                    "description": "The LSP operation to perform"
                },
                "file_path": {
                    "type": "string",
                    "description": "The absolute or relative path to the file"
                },
                "line": {
                    "type": "integer",
                    "description": "The line number (1-based, as shown in editors)"
                },
                "character": {
                    "type": "integer",
                    "description": "The character offset (1-based, as shown in editors)"
                }
            },
            "required": ["operation", "file_path", "line", "character"]
        })
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn description(&self) -> String {
        "Language Server Protocol code intelligence: go-to-definition, find-references, \
         hover, document symbols, workspace symbols, go-to-implementation, and call \
         hierarchy (incoming/outgoing calls). Requires an active LSP server for the \
         file type."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let _input: LspInput = serde_json::from_value(args)
            .map_err(|e| CliError::ToolExecution(format!("Invalid input: {e}")))?;

        Err(CliError::ToolExecution(
            "This tool requires the TUI/system integration.".to_string(),
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<LspInput>(args.clone()) {
            format!(
                "LSP {} on {}:{}:{}",
                input.operation, input.file_path, input.line, input.character
            )
        } else {
            "Running LSP operation".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        if result.is_error {
            "LSP operation failed.".to_string()
        } else {
            "LSP operation completed.".to_string()
        }
    }
}
