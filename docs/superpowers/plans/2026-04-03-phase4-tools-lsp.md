# Phase 4: Tool Completeness, LSP Deep Integration, and Plugin System

> **For agentic workers:** Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining built-in tools (TaskTool, AgentTool), add LSP deep integration (hover, go-to-definition), and finalize the plugin system so plugins can actually execute tools end-to-end.

**Architecture:**
- `TaskTool` lets the agent create/manage sub-tasks (persisted to session directory, coordination via Coordinator).
- `AgentTool` lets the agent spawn a sub-agent that shares the session history (not a fresh session).
- `LspBackend` trait in the library allows the binary to inject LSP capabilities into `AgentContext`.
- `PluginTool::call()` already exists — the plugin system just needs tool schema discovery wired into `build_tools()`.

---

## Task 1: Wire Custom Commands Through CommandRegistry (Phase 3残余)

**Files:**
- Modify: `binary/src/tui/event_loop.rs`

### Step 1: Verify current state

Read `binary/src/tui/event_loop.rs` lines 235–260. The current `SlashCommand` branch for custom commands falls through to the agent (no custom command execution).

### Step 2: Implement custom command execution

In `run_repl`, replace the `SlashCommand::Custom` handling with:

```rust
// Route ALL commands (built-in + custom) through CommandRegistry
if let Some(cmd) = state.command_registry.resolve(&text) {
    let output = state.command_registry.execute(cmd).await;
    match output {
        Ok(out) => {
            // success=false signals /exit — exit the REPL
            if !out.success && out.stdout.is_empty() && out.stderr.is_empty() {
                return Ok(());
            }
            if !out.stdout.is_empty() { println!("{}", out.stdout); }
            if !out.stderr.is_empty() { eprintln!("{}", out.stderr); }
        }
        Err(e) => {
            eprintln!("[Command error: {e}]");
        }
    }
    continue;
}
```

Note: `state.command_registry` is already part of `ReplState` (added in Phase 3 Task 3a wiring). The `CommandRegistry` is already created in `cli/mod.rs` and registered with custom commands.

This means removing the separate `SlashCommand::Custom` branch if one exists, and instead checking `command_registry.resolve()` first for ALL input.

### Step 3: Verify compilation

```bash
cargo check -p claude-code-rust-cli
```
Expected: 0 errors

### Step 4: Commit

```bash
git add binary/src/tui/event_loop.rs
git commit -m "feat: wire custom commands through CommandRegistry in REPL"
```

---

## Task 2: Implement TaskTool — Create/List/Update Sub-tasks

**Files:**
- Create: `library/src/tools/task_tool.rs`
- Modify: `library/src/tools/mod.rs`
- Modify: `library/src/agent/engine.rs` (add tool to registry)

### Step 1: Read existing task types

Read `library/src/types/task.rs` to understand `TaskState`, `TaskType`, `TaskStatus`.

### Step 2: Implement TaskTool

```rust
//! Task management tool — lets the agent create and manage sub-tasks

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::CliError;
use crate::types::{
    ResultContentBlock, Tool as ToolTrait, ToolContext, ToolResult,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskToolInput {
    pub action: String,  // "create" | "list" | "update" | "get"
    pub id: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,  // "pending" | "completed" | "failed"
}

pub struct TaskTool {
    tasks_dir: PathBuf,
}

impl TaskTool {
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self { tasks_dir }
    }

    fn tasks_file(&self) -> PathBuf {
        self.tasks_dir.join("tasks.json")
    }

    fn load_tasks(&self) -> Result<Vec<crate::types::TaskState>, CliError> {
        let path = self.tasks_file();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| CliError::Other(format!("failed to read tasks: {e}")))?;
            serde_json::from_str(&content)
                .map_err(|e| CliError::Other(format!("failed to parse tasks: {e}")))
        } else {
            Ok(Vec::new())
        }
    }

    fn save_tasks(&self, tasks: &[crate::types::TaskState]) -> Result<(), CliError> {
        let content = serde_json::to_string_pretty(tasks)
            .map_err(|e| CliError::Other(format!("failed to serialise tasks: {e}")))?;
        std::fs::create_dir_all(&self.tasks_dir)
            .map_err(|e| CliError::Other(format!("failed to create tasks dir: {e}")))?;
        std::fs::write(&self.tasks_file(), content)
            .map_err(|e| CliError::Other(format!("failed to write tasks: {e}")))
    }
}

#[async_trait]
impl ToolTrait for TaskTool {
    fn name(&self) -> &str { "task" }

    fn aliases(&self) -> Vec<String> {
        vec!["task_create".to_string(), "task_update".to_string(), "task_list".to_string()]
    }

    fn description(&self) -> String {
        "Create, list, update, or get the status of sub-tasks.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "update", "get"],
                    "description": "The action to perform"
                },
                "id": {
                    "type": "string",
                    "description": "Task ID (required for update/get)"
                },
                "description": {
                    "type": "string",
                    "description": "Task description (for create)"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "running", "completed", "failed"],
                    "description": "New status (for update)"
                }
            },
            "required": ["action"]
        })
    }

    async fn call(&self, args: serde_json::Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let input: TaskToolInput = serde_json::from_value(args)
            .map_err(|e| CliError::Other(format!("invalid task input: {e}")))?;

        match input.action.as_str() {
            "create" => {
                let description = input.description
                    .ok_or_else(|| CliError::Other("description required for create".to_string()))?;
                let id = format!("task-{}", uuid::Uuid::new_v4());
                let now = chrono::Utc::now().timestamp();

                let task = crate::types::TaskState {
                    id: id.clone(),
                    task_type: crate::types::TaskType::InProcessTeammate,
                    status: crate::types::TaskStatus::Pending,
                    description,
                    tool_use_id: None,
                    start_time: now,
                    end_time: None,
                    output_file: PathBuf::new(),
                    notified: false,
                };

                let mut tasks = self.load_tasks()?;
                tasks.push(task);
                self.save_tasks(&tasks)?;

                Ok(ToolResult {
                    content: vec![ResultContentBlock::Text {
                        text: format!("Created task: {}\n", id),
                    }],
                    is_error: false,
                    metrics: None,
                })
            }
            "list" => {
                let tasks = self.load_tasks()?;
                if tasks.is_empty() {
                    return Ok(ToolResult {
                        content: vec![ResultContentBlock::Text { text: "No tasks.\n".to_string() }],
                        is_error: false,
                        metrics: None,
                    });
                }
                let lines: Vec<String> = tasks.iter().map(|t| {
                    format!("  {}  [{:?}]  {}", t.id, t.status, t.description)
                }).collect();
                Ok(ToolResult {
                    content: vec![ResultContentBlock::Text {
                        text: format!("Tasks:\n{}\n", lines.join("\n")),
                    }],
                    is_error: false,
                    metrics: None,
                })
            }
            "update" => {
                let id = input.id
                    .ok_or_else(|| CliError::Other("id required for update".to_string()))?;
                let mut tasks = self.load_tasks()?;
                let task = tasks.iter_mut().find(|t| t.id == id)
                    .ok_or_else(|| CliError::Other(format!("task not found: {}", id)))?;

                if let Some(status) = input.status {
                    task.status = match status.as_str() {
                        "pending" => crate::types::TaskStatus::Pending,
                        "running" => crate::types::TaskStatus::Running,
                        "completed" => {
                            task.end_time = Some(chrono::Utc::now().timestamp());
                            crate::types::TaskStatus::Completed
                        }
                        "failed" => {
                            task.end_time = Some(chrono::Utc::now().timestamp());
                            crate::types::TaskStatus::Failed
                        }
                        _ => return Err(CliError::Other(format!("invalid status: {}", status))),
                    };
                }

                self.save_tasks(&tasks)?;
                Ok(ToolResult {
                    content: vec![ResultContentBlock::Text {
                        text: format!("Updated task: {}\n", id),
                    }],
                    is_error: false,
                    metrics: None,
                })
            }
            "get" => {
                let id = input.id
                    .ok_or_else(|| CliError::Other("id required for get".to_string()))?;
                let tasks = self.load_tasks()?;
                let task = tasks.iter().find(|t| t.id == id)
                    .ok_or_else(|| CliError::Other(format!("task not found: {}", id)))?;
                Ok(ToolResult {
                    content: vec![ResultContentBlock::Text {
                        text: format!("Task {}  [{:?}]  {}\n", task.id, task.status, task.description),
                    }],
                    is_error: false,
                    metrics: None,
                })
            }
            _ => Err(CliError::Other(format!("unknown action: {}", input.action))),
        }
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let (Some(action), Some(desc)) = (args.get("action"), args.get("description")) {
            format!("task action={} description={}", action, desc)
        } else if let Some(action) = args.get("action") {
            format!("task action={}", action)
        } else {
            "task".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        result.content.iter().filter_map(|b| {
            if let ResultContentBlock::Text { text } = b {
                Some(text.as_str())
            } else { None }
        }).collect::<Vec<_>>().join("")
    }
}
```

### Step 3: Export from tools/mod.rs

Add to `library/src/tools/mod.rs`:
```rust
pub mod task_tool;
```

And add to `ToolRegistry::register_builtins()` in `library/src/tools/registry.rs`:
```rust
use crate::tools::task_tool::TaskTool;
registry.register(Arc::new(TaskTool::new(session_dir)));
```

Note: `TaskTool::new` needs a `tasks_dir`. Pass `working_directory` from the engine via context, or use a session-relative path. Simpler approach: use `std::env::temp_dir()` for now, or `context.session_id` from `ToolContext`.

Update `ToolRegistry::register_builtins` to accept an optional session ID for task tool initialization, or use a simpler path.

### Step 4: Verify compilation

```bash
cargo check -p claude-code-rust && cargo check -p claude-code-rust-cli
```
Expected: 0 errors

### Step 5: Add tests

Add tests to `library/src/tools/` for task create/list/update/get actions.

### Step 6: Commit

```bash
git add library/src/tools/task_tool.rs library/src/tools/mod.rs library/src/tools/registry.rs
git commit -m "feat: implement TaskTool for sub-task management"
```

---

## Task 3: Implement AgentTool — Spawn Sub-Agent

**Files:**
- Create: `library/src/tools/agent_tool.rs`
- Modify: `library/src/tools/mod.rs`
- Modify: `library/src/agent/engine.rs`

### Step 1: Design

`AgentTool` lets the main agent spawn a sub-agent to handle a specific subtask. Unlike `/multi` (which creates independent sessions), the sub-agent shares the parent's session context and can read/write to the same session.

Key difference from `Coordinator::spawn_agent`:
- `AgentTool` operates within the same session (adds messages to the parent session).
- It's a tool call — the main agent decides when to spawn it.
- The sub-agent's output is returned as a tool result to the parent agent.

```rust
pub struct AgentTool {
    coordinator: Arc<Coordinator>,  // already in ReplState
}
```

Wait — `Coordinator` is in the binary crate, not the library. `AgentTool` needs to be in the library.

**Revised design:** `AgentTool` accepts a callback/closure for spawning agents. The library defines an `AgentBackend` trait, and the binary implements it with `Coordinator`. This mirrors the `LspBackend` pattern.

### Step 2: Define AgentBackend trait in library

Create `library/src/tools/agent_tool.rs`:

```rust
//! AgentTool — spawn a sub-agent to handle a sub-task

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::error::CliError;
use crate::types::{
    ResultContentBlock, Tool as ToolTrait, ToolContext, ToolResult,
};

/// Backend for spawning sub-agents. Implemented by the binary.
#[async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run_agent(
        &self,
        prompt: String,
        system_prompt: Option<String>,
    ) -> Result<String, CliError>;
}

pub struct AgentTool {
    backend: Arc<dyn AgentBackend>,
}

impl AgentTool {
    pub fn new(backend: Arc<dyn AgentBackend>) -> Self {
        Self { backend }
    }
}

#[derive(serde::Deserialize)]
struct AgentToolInput {
    prompt: String,
    system_prompt: Option<String>,
}

#[async_trait]
impl ToolTrait for AgentTool {
    fn name(&self) -> &str { "agent" }

    fn aliases(&self) -> Vec<String> {
        vec!["subagent".to_string(), "spawn_agent".to_string()]
    }

    fn description(&self) -> String {
        "Spawn a sub-agent to handle a sub-task. The sub-agent runs \
         independently and returns its final output as the tool result.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The task description or question for the sub-agent"
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt override for this sub-agent"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn call(&self, args: Value, _context: ToolContext) -> Result<ToolResult, CliError> {
        let input: AgentToolInput = serde_json::from_value(args)
            .map_err(|e| CliError::Other(format!("invalid agent input: {e}")))?;

        let output = self.backend.run_agent(input.prompt, input.system_prompt).await?;

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text: output }],
            is_error: false,
            metrics: None,
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        args.get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| format!("agent: {}", &s[..s.len().min(80)]))
            .unwrap_or_else(|| "agent".to_string())
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        result.content.iter().filter_map(|b| {
            if let ResultContentBlock::Text { text } = b { Some(text.as_str()) } else { None }
        }).collect::<Vec<_>>().join("")
    }
}
```

### Step 3: Implement AgentBackend in binary

In `binary/src/plugins/tool.rs` (or create new `binary/src/tools/agent.rs`):

```rust
use claude_code_rust::tools::agent_tool::AgentBackend;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CoordinatorAgentBackend {
    coordinator: Arc<crate::coordinator::Coordinator>,
    api_client: Arc<claude_code_rust::api::client::ApiClient>,
    model: String,
    tools: Arc<Vec<Arc<dyn claude_code_rust::types::Tool>>>,
    working_directory: std::path::PathBuf,
    env: std::collections::HashMap<String, String>,
}

#[async_trait]
impl AgentBackend for CoordinatorAgentBackend {
    async fn run_agent(
        &self,
        prompt: String,
        system_prompt: Option<String>,
    ) -> Result<String, claude_code_rust::error::CliError> {
        self.coordinator.spawn_agent(
            (*self.api_client).clone(),
            self.model.clone(),
            (*self.tools).clone(),
            self.working_directory.clone(),
            self.env.clone(),
            prompt,
        ).await
    }
}
```

### Step 4: Wire AgentTool into build_tools

In `binary/src/cli/mod.rs`, in `build_tools()`, after loading built-in tools:

```rust
// AgentTool needs a backend — create it from coordinator
use crate::tools::agent::CoordinatorAgentBackend;
let agent_backend = Arc::new(CoordinatorAgentBackend {
    coordinator: coordinator.clone(),
    api_client: Arc::new(api_client.clone()),
    model: session_model.clone(),
    tools: Arc::new(tools.clone()),
    working_directory: working_dir.clone(),
    env: env.clone(),
});
let agent_tool = Arc::new(claude_code_rust::tools::agent_tool::AgentTool::new(agent_backend));
all_tools.push(agent_tool);
```

Wait — this creates a chicken-and-egg problem: `build_tools` is called before `coordinator` is created. Move `build_tools` call to after coordinator creation, or pass coordinator into `build_tools`.

**Simpler approach:** Pass `coordinator` and `api_client` into `build_tools`:

```rust
async fn build_tools(
    global_config: &claude_code_rust::config::GlobalConfig,
    project_config: &claude_code_rust::config::ProjectConfig,
    extra_mcp_servers: Option<&std::collections::HashMap<String, claude_code_rust::config::McpServerConfig>>,
    coordinator: Option<&Arc<Coordinator>>,
    api_client: Option<&Arc<ApiClient>>,
    model: Option<&str>,
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<Vec<Arc<dyn Tool>>, CliError>
```

At the call site in `run_interactive`, pass all the needed values. This is a moderate refactor — do it carefully.

### Step 5: Verify compilation

```bash
cargo check -p claude-code-rust && cargo check -p claude-code-rust-cli
```
Expected: 0 errors

### Step 6: Commit

```bash
git add library/src/tools/agent_tool.rs binary/src/plugins/tool.rs binary/src/cli/mod.rs library/src/tools/mod.rs
git commit -m "feat: implement AgentTool for sub-agent spawning"
```

---

## Task 4: LSP Deep Integration — Hover and Go-to-Definition

**Files:**
- Create: `library/src/agent/lsp.rs` (trait definition)
- Modify: `library/src/agent/context.rs`
- Modify: `library/src/agent/engine.rs`
- Modify: `binary/src/lsp/client.rs`

### Step 1: Define LspBackend trait in library

Create `library/src/agent/lsp.rs`:

```rust
//! LSP backend trait — implemented by the binary to provide code intelligence

use crate::error::CliError;

#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// LSP-backed code intelligence for the agent
#[async_trait]
pub trait LspBackend: Send + Sync {
    /// Get hover information for a position in a file
    async fn hover(&self, file: &str, line: u32, col: u32) -> Result<Option<String>, CliError>;

    /// Go to definition of a symbol
    async fn goto_definition(&self, file: &str, line: u32, col: u32) -> Result<Option<Location>, CliError>;

    /// Find all references to a symbol
    async fn find_references(&self, file: &str, line: u32, col: u32) -> Result<Vec<Location>, CliError>;
}
```

Add `pub mod lsp;` to `library/src/agent/mod.rs`.

### Step 2: Add lsp_backend to AgentContext

In `library/src/agent/context.rs`:

```rust
use super::lsp::LspBackend;

pub struct AgentContext {
    // ... existing fields ...
    pub lsp_backend: Option<Arc<dyn LspBackend>>,
}
```

Update `AgentContext::new()` to accept `lsp_backend`.

### Step 3: Implement LspBackend in binary

In `binary/src/lsp/client.rs` (read it first to understand the `LspClient` interface):

Add a method to `LspClient` that wraps the tower-lsp `Client` calls:

```rust
use tower_lsp::lsp_types::*;

impl LspClient {
    pub async fn hover(&self, uri: &str, line: u32, col: u32) -> Result<Option<String>, CliError> {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: url.clone() },
                position: Position { line, character: col },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = self.client.hover(params).await
            .map_err(|e| CliError::Other(format!("hover failed: {e}")))?;

        Ok(result.map(|hover| {
            hover.contents.to_string()
        }))
    }

    pub async fn goto_definition(&self, uri: &str, line: u32, col: u32) -> Result<Option<crate::agent::lsp::Location>, CliError> {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: url.clone() },
                position: Position { line, character: col },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = self.client.goto_definition(params).await
            .map_err(|e| CliError::Other(format!("goto_definition failed: {e}")))?;

        Ok(result.map(|locations| {
            locations.into_iter().next().map(|loc| {
                crate::agent::lsp::Location {
                    file: loc.uri.to_file_path().unwrap_or_default().to_string_lossy().to_string(),
                    line: loc.range.start.line,
                    column: loc.range.start.character,
                }
            })
        }))
    }
}
```

Note: If `LspClient` doesn't expose the tower-lsp `Client`, you may need to add an inner `Arc<Mutex<Client>>` field. Read `binary/src/lsp/client.rs` first.

### Step 4: Wire LspClient into AgentContext

In `binary/src/cli/mod.rs`, in `run_interactive()`:

```rust
use crate::lsp::client::LspClient;
use claude_code_rust::agent::lsp::LspBackend;

let lsp_client = Arc::new(lsp_client); // the lsp_client already created for LSP startup

let lsp_backend: Arc<dyn LspBackend> = lsp_client; // LspClient implements LspBackend

// Update AgentContext::new call to pass lsp_backend
let context = AgentContext::new(
    session,
    session_config,
    tools,
    working_dir.clone(),
    bootstrap.global_config,
    bootstrap.project_config,
    env,
    Some(lsp_backend),
);
```

### Step 5: Verify compilation

```bash
cargo check -p claude-code-rust && cargo check -p claude-code-rust-cli
```

### Step 6: Commit

```bash
git add library/src/agent/lsp.rs library/src/agent/context.rs library/src/agent/mod.rs binary/src/lsp/client.rs binary/src/cli/mod.rs
git commit -m "feat: add LSP deep integration (hover, goto-definition)"
```

---

## Task 5: Final Build and Test

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p claude-code-rust
```

Expected: All tests passing

- [ ] **Step 2: Build release binary**

```bash
cargo build -p claude-code-rust-cli --release
```

- [ ] **Step 3: Check for warnings**

```bash
cargo check 2>&1 | grep -E "^error"
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: phase 4 complete — TaskTool, AgentTool, LSP deep integration"
```
