# Phase 3: Slash Command Execution, Session Fixes, CLI Args, and LSP Wiring

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix broken resume, wire slash command execution, wire CLI args, live token tracking in REPL, and start LSP servers.

**Architecture:**
- `CommandRegistry` is instantiated in `run_interactive()` and passed to the REPL. Builtin commands (`/exit`, `/clear`, `/session`, etc.) are handled inline. Custom commands from `project_config.custom_commands` are registered and executed via the registry.
- `run_resume()` reads the last user message from the session and uses it as the resume prompt.
- LSP servers are started from `global_config.lsp_servers` and the `LspClient` is stored in `AgentContext`.
- `--system-prompt` and `--mcp-config` override config values at startup.

**Tech Stack:** tokio, serde_json, crossterm

---

## Task 1: Fix run_resume() — Resume Should Use Last User Message

**Files:**
- Modify: `binary/src/cli/mod.rs`

The current `run_resume()` calls `engine.run(String::new())` — this sends an empty string as the resume prompt. Instead, it should resume the conversation by prompting for what the user wants to do with the existing session, or simply continue without an initial prompt (the session is already loaded with history).

The correct behavior: send an empty user message or a default "continue" prompt to resume the session's existing conversation.

- [ ] **Step 1: Fix run_resume in cli/mod.rs**

Find the `run_resume` function. Replace `engine.run(String::new())` with:

```rust
// Resume: send an empty user message to continue the conversation
match engine.run(String::new()).await {
```

Actually, the session is already loaded with all its messages. An empty `String::new()` creates a `Message::User { content: UserContent::text("") }` which is a blank message. Instead, skip adding a new user message for resume (the session already has messages):

But the current `engine.run()` always adds a user message at the start. For resume, we should NOT add a blank message. We need to add a method to `AgentEngine`:

Add to `library/src/agent/engine.rs`:

```rust
/// Run the agent with an existing session (no new user message added).
/// Used for resume — the session is already loaded with history.
pub async fn run_resume(&mut self) -> Result<AgentOutcome, CliError> {
    loop {
        let model = self.context.model().to_string();
        let max_tokens = self.context.config.max_tokens.unwrap_or(8192);

        let tools = if self.context.tools.is_empty() {
            None
        } else {
            Some(self.context.tools.as_slice())
        };

        let response = self.api_client.chat(&self.context.session, &model, max_tokens, tools).await?;

        // Track token usage and cost
        self.context.session.token_usage.add(&response.usage);
        let cost = self.api_client.estimate_cost(&response.usage, &model);
        self.context.session.cost += cost;

        // Check compaction
        if self.compactor.should_compact(&self.context.session.messages, &self.context.session.token_usage) {
            eprintln!("[Info] Compacting conversation to stay within token limit...");
            self.compactor.compact(&mut self.context.session.messages);
        }

        // Add assistant response
        self.context.session.add_message(Message::Assistant {
            content: Some(response.content.clone()),
        });

        // Process content blocks (same logic as run() — copy the tool execution loop)
        let mut tool_results = Vec::new();

        for block in &response.content.content {
            match block {
                ContentBlock::Text { text } => {
                    if !text.is_empty() {
                        println!("{}", text);
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    let tool = match self.context.find_tool(name) {
                        Some(t) => t,
                        None => {
                            let result = ToolResult::error(format!("Unknown tool: {}", name));
                            tool_results.push((id.clone(), result));
                            continue;
                        }
                    };

                    let decision = self.permission_checker
                        .check_tool(tool.as_ref(), input, "")
                        .await;

                    match decision {
                        PermissionDecision::Allow => {
                            let tool_ctx = ToolContext {
                                session_id: self.context.session.id.clone(),
                                agent_id: "main".to_string(),
                                working_directory: self.context.working_directory.clone(),
                                can_use_tool: true,
                                parent_message_id: None,
                                env: self.context.env.clone(),
                            };

                            // Run pre_tool_use hooks
                            for hook in self.context.hooks_of_type(HookType::PreToolUse) {
                                let payload = Hook::pre_tool_payload(tool.name(), input, &self.context.session.id);
                                if let Err(e) = hook.run(&payload).await {
                                    eprintln!("[Hook warning] {}: {e}", hook.name);
                                }
                            }

                            let result = tool.call(input.clone(), tool_ctx).await
                                .unwrap_or_else(|e| ToolResult::error(e.to_string()));

                            // Run post_tool_use hooks
                            for hook in self.context.hooks_of_type(HookType::PostToolUse) {
                                let payload = Hook::post_tool_payload(tool.name(), &result, &self.context.session.id);
                                if let Err(e) = hook.run(&payload).await {
                                    eprintln!("[Hook warning] {}: {e}", hook.name);
                                }
                            }

                            tool_results.push((id.clone(), result));
                        }
                        PermissionDecision::Ask { message, .. } => {
                            let result = ToolResult::error(format!(
                                "Permission required: {}\nPlease run with --permission-mode=acceptEdits to bypass.",
                                message
                            ));
                            tool_results.push((id.clone(), result));
                        }
                        PermissionDecision::Deny(msg) => {
                            let result = ToolResult::error(format!("Permission denied: {}", msg));
                            tool_results.push((id.clone(), result));
                        }
                        PermissionDecision::Passthrough(msg) => {
                            let result = ToolResult::text(msg);
                            tool_results.push((id.clone(), result));
                        }
                    }
                }
                ContentBlock::Image { .. } => {}
                ContentBlock::ToolResult { .. } => {}
            }
        }

        // Add tool results
        for (tool_use_id, result) in tool_results {
            self.context.session.add_message(Message::ToolResult {
                tool_use_id,
                content: result.content.iter().map(|b| b.preview()).collect::<Vec<_>>().join("\n"),
                is_error: result.is_error,
            });
        }

        // Check stop reason
        if response.stop_reason.as_deref() == Some("end_turn") {
            return Ok(AgentOutcome::Completed);
        }
        if response.stop_reason.as_deref() == Some("max_tokens") {
            continue;
        }
        if response.stop_reason.is_none() && response.content.content.is_empty() {
            return Ok(AgentOutcome::Completed);
        }
    }
}
```

Add the necessary imports at the top of `engine.rs`:
```rust
use crate::agent::hooks::{Hook, HookType};
use crate::types::{ContentBlock, ToolContext, ToolResult};
```

Then update `cli/mod.rs` to call `engine.run_resume()` instead of `engine.run(String::new())`.

- [ ] **Step 2: Update run_resume in cli/mod.rs**

Replace the engine.run call in `run_resume`:
```rust
match engine.run_resume().await {
    Ok(AgentOutcome::Completed) => {}
    Ok(AgentOutcome::Error(msg)) => {
        eprintln!("Agent error: {msg}");
        return Err(CliError::Other(msg));
    }
    Ok(AgentOutcome::Interrupted) => {}
    Err(e) => return Err(e),
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p claude-code-rust && cargo check -p claude-code-rust-cli
```
Expected: 0 errors

- [ ] **Step 4: Commit**

```bash
git add library/src/agent/engine.rs binary/src/cli/mod.rs
git commit -m "feat: add run_resume() to AgentEngine and wire from CLI"
```

---

## Task 2: Wire CLI Args — --system-prompt and --mcp-config

**Files:**
- Modify: `binary/src/cli/bootstrap.rs`
- Modify: `binary/src/cli/mod.rs`

### Task 2a: Wire --system-prompt

- [ ] **Step 1: Update Bootstrap::new to accept system_prompt_arg**

In `binary/src/cli/bootstrap.rs`, add `system_prompt_arg: Option<String>` to `Bootstrap::new()`:

```rust
pub async fn new(
    _model: Option<String>,
    permission_mode_arg: Option<String>,
    dangerously_skip_permission: Option<String>,
    add_env_arg: Vec<String>,
    system_prompt_arg: Option<String>,
    verbose: bool,
) -> Result<Self, CliError> {
```

After `extra_env` parsing, add:
```rust
// System prompt: CLI arg overrides config
let system_prompt = system_prompt_arg.or_else(|| {
    global_config.model_preferences.system_prompt.clone()
});
```

Add to the returned struct:
```rust
pub struct Bootstrap {
    // ... existing fields ...
    pub system_prompt: Option<String>,
}
```

And in the return:
```rust
Ok(Self {
    // ... existing fields ...
    system_prompt,
})
```

- [ ] **Step 2: Update all call sites of Bootstrap::new in cli/mod.rs**

In `run_interactive`, `run_non_interactive`, and `run_resume`, add `args.system_prompt.clone()` as the 5th argument:

```rust
let bootstrap = bootstrap::Bootstrap::new(
    args.model.clone(),
    args.permission_mode.clone(),
    args.dangerously_skip_permission.clone(),
    args.add_env.clone(),
    args.system_prompt.clone(),  // NEW
    args.verbose,
)
.await?;
```

- [ ] **Step 3: Use system_prompt when creating session**

In `run_interactive()`, after creating the session:
```rust
session.system_prompt = bootstrap.system_prompt.clone()
    .or(Some(claude_code_rust::types::DEFAULT_SYSTEM_PROMPT.to_string()));
```

Replace the existing `session.system_prompt = Some(claude_code_rust::types::DEFAULT_SYSTEM_PROMPT.to_string())` line.

### Task 2b: Wire --mcp-config

- [ ] **Step 4: Update Bootstrap::new to parse mcp_config_arg**

Add `mcp_config_arg: Option<String>` to `Bootstrap::new()` signature.

```rust
pub async fn new(
    _model: Option<String>,
    permission_mode_arg: Option<String>,
    dangerously_skip_permission: Option<String>,
    add_env_arg: Vec<String>,
    system_prompt_arg: Option<String>,
    mcp_config_arg: Option<String>,  // NEW
    verbose: bool,
) -> Result<Self, CliError> {
```

After system_prompt parsing, add:
```rust
// MCP config: CLI arg overrides config
let extra_mcp_servers: Option<HashMap<String, McpServerConfig>> = if let Some(json_str) = mcp_config_arg {
    match serde_json::from_str(&json_str) {
        Ok(servers) => Some(servers),
        Err(e) => {
            eprintln!("[Warning] Failed to parse --mcp-config: {e}");
            None
        }
    }
} else {
    None
};
```

Add to Bootstrap struct:
```rust
pub struct Bootstrap {
    // ...
    pub extra_mcp_servers: Option<HashMap<String, McpServerConfig>>,
}
```

Return it in `Ok(Self { ..., extra_mcp_servers })`.

Also add the import: `use std::collections::HashMap;` at the top of bootstrap.rs.

- [ ] **Step 5: Wire extra_mcp_servers into build_tools()**

In `binary/src/cli/mod.rs`, update `build_tools()` to accept and merge extra MCP servers:

```rust
async fn build_tools(
    global_config: &claude_code_rust::config::GlobalConfig,
    project_config: &claude_code_rust::config::ProjectConfig,
    extra_mcp_servers: Option<&std::collections::HashMap<String, claude_code_rust::config::McpServerConfig>>,
) -> Result<Vec<Arc<dyn claude_code_rust::types::Tool>>, CliError> {
```

In the MCP servers merging block:
```rust
// Collect MCP servers (project overrides global for same-named servers)
let mut servers: HashMap<String, McpServerConfig> = global_config.mcp_servers.clone();
if let Some(ref project_servers) = project_config.mcp_servers {
    for (name, config) in project_servers {
        servers.insert(name.clone(), config.clone());
    }
}
// CLI --mcp-config overrides all
if let Some(ref extra) = extra_mcp_servers {
    for (name, config) in extra {
        servers.insert(name.clone(), config.clone());
    }
}
```

Update all call sites in `run_interactive`, `run_non_interactive`, `run_resume`:
```rust
let tools = build_tools(&bootstrap.global_config, &bootstrap.project_config, bootstrap.extra_mcp_servers.as_ref()).await?;
```

Also add `mcp_config_arg` to all Bootstrap::new() call sites.

- [ ] **Step 6: Verify compilation**

```bash
cargo check -p claude-code-rust-cli
```
Expected: 0 errors

- [ ] **Step 7: Commit**

```bash
git add binary/src/cli/bootstrap.rs binary/src/cli/mod.rs
git commit -m "feat: wire --system-prompt and --mcp-config CLI arguments"
```

---

## Task 3: Slash Command Execution — Builtins + Custom Commands

**Files:**
- Modify: `binary/src/tui/event_loop.rs`
- Modify: `binary/src/cli/mod.rs`

### Task 3a: Make builtin commands actually work

Currently `/session`, `/model`, `/cost`, `/tokens` print stub values. They need live data from the engine. However, the REPL doesn't have engine access. The solution: `ReplSessionInfo` should contain callbacks or an `Arc<Mutex<Session>>` reference.

**Approach:** Replace `ReplSessionInfo` with an `Arc<tokio::sync::Mutex<Session>>` so the REPL can read live session data.

In `binary/src/cli/mod.rs`, update `ReplSessionInfo` usage:

```rust
// Instead of ReplSessionInfo, use:
use crate::tui::event_loop::ReplState;
use std::sync::Arc;
use tokio::sync::Mutex;

let session_arc = Arc::new(Mutex::new(session));
let session_for_repl = session_arc.clone();

let on_message = move |prompt: String| {
    // ... existing closure logic ...
    // After engine.run(), update the session arc
};

// Create ReplState from session_arc and engine data
let repl_state = ReplState {
    session: session_arc,
    provider: bootstrap.provider.to_string(),
    tools: Arc::new(tools),  // for /help builtins
};
```

Update `ReplSessionInfo` to `ReplState` in `event_loop.rs`:
- Replace `ReplSessionInfo` with `ReplState` (a better name)
- Replace fields with `session: Arc<tokio::sync::Mutex<crate::types::Session>>` and `provider: String`
- Update `/session` handler to read from locked session
- Update `/model` handler to read from locked session
- Update `/cost` handler to read from locked session
- Update `/tokens` handler to read token_usage
- `/clear` stays the same
- `/exit` stays the same
- `/config` stays the same

**New ReplState in event_loop.rs:**
```rust
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared state for REPL commands
#[derive(Clone)]
pub struct ReplState {
    pub session: Arc<Mutex<crate::types::Session>>,
    pub provider: String,
}

impl ReplState {
    pub async fn print_session(&self) {
        let s = self.session.lock().await;
        println!("\nSession: {}", s.id);
        println!("Model: {}", s.model);
        println!("Messages: {}", s.messages.len());
        println!("Cost: ${:.4}", s.cost);
        println!("Tokens: {} input / {} output",
            s.token_usage.input_tokens, s.token_usage.output_tokens);
        println!();
    }

    pub async fn print_model(&self) {
        let s = self.session.lock().await;
        println!("\nModel: {}\n", s.model);
    }

    pub async fn print_cost(&self) {
        let s = self.session.lock().await;
        println!("\nCost: ${:.4}\n", s.cost);
    }

    pub async fn print_tokens(&self) {
        let s = self.session.lock().await;
        let total = s.token_usage.total();
        println!(
            "\nTokens: {} input / {} output / {} total\n",
            s.token_usage.input_tokens,
            s.token_usage.output_tokens,
            total
        );
    }

    pub fn print_config(&self) {
        println!("\nProvider: {}\n", self.provider);
    }
}
```

**Update run_repl signature:**
```rust
pub async fn run_repl(
    on_message: impl FnMut(String) -> BoxFuture<'static, bool>,
    state: ReplState,
) -> Result<(), io::Error> {
```

**Update slash command handlers:**
```rust
SlashCommand::Session => {
    state.print_session().await;
    continue;
}
SlashCommand::Model => {
    state.print_model().await;
    continue;
}
SlashCommand::Cost => {
    state.print_cost().await;
    continue;
}
SlashCommand::Tokens => {
    state.print_tokens().await;
    continue;
}
```

Note: `ReplSessionInfo` is the old type name. Replace all occurrences.

### Task 3b: Wire CommandRegistry for custom commands

In `binary/src/cli/mod.rs`, in `run_interactive()`:

```rust
use crate::commands::{CommandRegistry, Command};

// After building the engine:
let mut cmd_registry = CommandRegistry::new();
cmd_registry.register_all(bootstrap.project_config.custom_commands.clone());
let cmd_registry = Arc::new(cmd_registry);
```

Pass the registry to the REPL via a new `ReplState` field or separate argument:

```rust
pub struct ReplState {
    pub session: Arc<Mutex<Session>>,
    pub provider: String,
    pub command_registry: Arc<CommandRegistry>,
}
```

In `event_loop.rs`, add custom command handling in `run_repl`:

```rust
SlashCommand::None => {
    // Check for custom command
    if let Some(cmd) = state.command_registry.resolve(&text) {
        let output = state.command_registry.execute(cmd).await;
        match output {
            Ok(out) => {
                if !out.stdout.is_empty() {
                    println!("\n{}", out.stdout);
                }
                if !out.stderr.is_empty() {
                    eprintln!("\n{}", out.stderr);
                }
                if !out.success {
                    eprintln!("[Command exited with status: non-zero]");
                }
            }
            Err(e) => {
                eprintln!("[Command error: {e}]");
            }
        }
        continue;
    }
}
```

Note: `state.command_registry` needs to be part of `ReplState` for this to compile. Add it.

### Task 3c: Update run_repl call site

In `cli/mod.rs`, update the `run_repl` call:
```rust
use crate::tui::event_loop::{run_repl, ReplState};

let repl_state = ReplState {
    session: session_arc,
    provider: bootstrap.provider.to_string(),
    command_registry: cmd_registry,
};

run_repl(on_message, repl_state).await
```

Also update the on_message closure to lock the session and store results:
```rust
let session_ref = session_arc.clone();
async move {
    // ... existing engine.run() call ...
    // After run, update the session arc
    if let Ok(AgentOutcome::Completed) = outcome {
        // Update session
    }
    // ... rest unchanged ...
}
```

Note: The current on_message closure creates `eng_clone`, `stor_clone`, `trans_clone` but doesn't update `session_arc`. Add session arc update:
```rust
let session_ref = session_arc.clone();
// In the async block after engine.run():
{
    let session = eng.session().clone();
    *session_ref.lock().await = session;
}
```

### Task 3d: Verify compilation

```bash
cargo check -p claude-code-rust-cli
```

Expected: 0 errors

### Task 3e: Commit

```bash
git add binary/src/tui/event_loop.rs binary/src/cli/mod.rs
git commit -m "feat: live session data in REPL and slash command execution via CommandRegistry"
```

---

## Task 4: Wire LSP Servers — Start from Config and Pass to Engine

**Files:**
- Modify: `binary/src/cli/mod.rs`
- Modify: `library/src/agent/context.rs`
- Modify: `library/src/agent/engine.rs`

The LSP client should start language servers from `global_config.lsp_servers` at startup and be accessible to the agent engine.

### Task 4a: Add LspClient to AgentContext

- [ ] **Step 1: Read the LSP module**

Read `binary/src/lsp/mod.rs` to understand `LspServerConfig`.

- [ ] **Step 2: Add lsp_client field to AgentContext**

In `library/src/agent/context.rs`:

```rust
use std::sync::Arc;

// Add a new field (Option so it's optional when not initialized)
pub lsp_client: Option<Arc<LspClient>>,
```

Add to `AgentContext::new()`:
```rust
pub fn new(
    // ... existing params ...
    lsp_client: Option<Arc<LspClient>>,
) -> Self {
    // ...
    Self {
        // ... existing fields ...
        lsp_client,
    }
}
```

Note: `LspClient` is in the binary crate, not the library. This creates a dependency problem. Alternative: define a trait `LspBackend` in the library and have the binary provide the implementation. For now, use a simpler approach: store LSP state as `serde_json::Value` or a type-erased trait.

**Simpler approach — use a trait in the library:**

Define in `library/src/agent/mod.rs`:
```rust
/// Trait for LSP-backed code intelligence. Implemented by the binary.
#[async_trait]
pub trait LspBackend: Send + Sync {
    async fn hover(&self, file: &str, line: u32, col: u32) -> Result<Option<String>, CliError>;
    async fn goto_definition(&self, file: &str, line: u32, col: u32) -> Result<Option<String>, CliError>;
}
```

Add to `AgentContext`:
```rust
pub lsp_backend: Option<Arc<dyn LspBackend>>,
```

Update `AgentContext::new()` accordingly. Implement `LspBackend` in the binary by wrapping `LspClient`.

This is complex — for Phase 3, do the simpler version: just start the LSP servers from the binary, log them, but don't pass to engine yet. Mark LSP integration as Phase 4.

**Revised scope for Task 4 (Phase 3):** Start LSP servers in `run_interactive()` and log them. The engine integration can come later.

```rust
// In run_interactive():
use crate::lsp::client::LspClient;
use std::path::PathBuf;

// Start LSP servers
let mut lsp_client = LspClient::new(std::env::current_dir().unwrap_or_default());
for (lang, config) in &bootstrap.global_config.lsp_servers {
    if let Err(e) = lsp_client.start_server(lang, config).await {
        eprintln!("[Warning] Failed to start LSP server for {lang}: {e}");
    } else {
        eprintln!("[LSP] Started server for {lang}");
    }
}
```

Update `cli/mod.rs` to add the import `crate::lsp::client::LspClient`.

Note: This creates a `lsp_client` that goes out of scope at the end of `run_interactive`. That's OK for Phase 3 — servers will start and stop cleanly.

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p claude-code-rust-cli
```

Expected: 0 errors

- [ ] **Step 3: Commit**

```bash
git add binary/src/cli/mod.rs
git commit -m "feat: start LSP servers from global config on startup"
```

---

## Task 5: Final Build and Test

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p claude-code-rust
```

Expected: 20+ tests passing

- [ ] **Step 2: Build release binary**

```bash
cargo build -p claude-code-rust-cli --release
```

Expected: Builds successfully

- [ ] **Step 3: Check for new warnings**

```bash
cargo check 2>&1 | grep -E "^error"
```

Address any new errors.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: phase 3 complete — slash commands, CLI args, LSP startup, resume fix"
```
