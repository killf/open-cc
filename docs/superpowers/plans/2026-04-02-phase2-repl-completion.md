# Phase 2: Complete REPL, Session Persistence, Slash Commands, and Agent Quality

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the interactive REPL experience with session persistence, slash commands, context compaction, and a hook system.

**Architecture:**
- Sessions are saved after every REPL turn (both JSON `session.json` and NDJSON `transcript.ndjson`).
- Slash commands are processed before sending input to the agent; built-in commands (`/help`, `/exit`, `/clear`, etc.) are handled inline; custom commands from project config are executed via `CommandRegistry`.
- Context compaction summarises old messages when token budget is exceeded, keeping the most recent context within budget.
- Hooks are stored in `AgentContext` and called at pre/post tool-use points in the engine loop.

**Tech Stack:** tokio, tokio::fs, serde_json, uuid, chrono

---

## Task 1: Session Persistence — Save After Every REPL Turn

**Files:**
- Modify: `binary/src/cli/mod.rs:133-161`

The REPL currently saves sessions using `storage.save()` in `run_interactive`, but `run_resume()` never saves after editing. Also, `TranscriptManager::append()` is never called — transcript NDJSON is always empty.

**Files:**
- Modify: `binary/src/cli/mod.rs:85-163`, `binary/src/cli/mod.rs:226-281`

### Task 1a: Fix run_interactive() session persistence

- [ ] **Step 1: Examine current save logic**

Read `binary/src/cli/mod.rs` lines 133–163. The current code saves after each turn but uses `now_or_never()` which only works for immediately-ready futures. For truly async saves this is fine, but the pattern is fragile. Replace with a proper async block that logs warnings on failure.

```rust
// In run_interactive(), replace the now_or_never save with:
let engine_ref = Arc::new(tokio::sync::Mutex::new(engine));
let storage_ref = Arc::new(storage);
let transcript_ref = Arc::new(TranscriptManager::new(&session.id));

let engine_arc = engine_ref.clone();
let storage_arc = storage_ref.clone();
let transcript_arc = transcript_ref.clone();

run_repl(move |prompt: String| {
    let eng = engine_arc.clone();
    let stor = storage_arc.clone();
    let trans = transcript_arc.clone();
    async move {
        let mut eng = eng.lock().await;
        let outcome = eng.run(prompt).await;
        // Save session + append last user message to transcript
        let session = eng.session().clone();
        let _ = stor.save(&session).await;
        if let Some(msg) = session.messages.last() {
            let _ = trans.append(msg).await;
        }
        match outcome {
            Ok(AgentOutcome::Completed) => true,
            Ok(AgentOutcome::Error(msg)) => { eprintln!("[Error] {msg}"); true }
            Ok(AgentOutcome::Interrupted) => { println!("[Interrupted]"); true }
            Err(e) => { eprintln!("[Error] {e}"); true }
        }
    }
}).await
```

Note: `run_repl` currently takes `impl FnMut(String) -> bool`. This needs to change to `impl FnMut(String) -> BoxFuture<'static, bool>` to support async. **Also update `run_repl` signature in `event_loop.rs`.**

- [ ] **Step 2: Update event_loop.rs run_repl signature**

```rust
// binary/src/tui/event_loop.rs
use futures::FutureExt; // already imported

pub async fn run_repl(mut on_message: impl FnMut(String) -> BoxFuture<'static, bool>) -> Result<(), io::Error> {
    // ... rest unchanged
    let text = input.trim().to_string();
    if text.is_empty() { continue; }
    println!("\n[You] {text}");
    println!("[Claude]");

    if !on_message(text).await {
        break;
    }
}
```

- [ ] **Step 3: Add TranscriptManager to run_interactive imports**

```rust
// binary/src/cli/mod.rs - add to imports in run_interactive
use claude_code_rust::session::{SessionStorage, TranscriptManager};
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors (warnings OK)

- [ ] **Step 5: Commit**

```bash
git add binary/src/cli/mod.rs binary/src/tui/event_loop.rs
git commit -m "feat: make REPL callback async for proper session persistence"
```

---

### Task 1b: Fix run_resume() session persistence

**Files:**
- Modify: `binary/src/cli/mod.rs:226-281`

- [ ] **Step 1: Add session storage save after run**

Replace the `engine.run(String::new()).await` block to also save the session after completion:

```rust
// In run_resume(), after the engine.run() match block:
let session = engine.session().clone();
if let Err(e) = bootstrap.session_storage.save(&session).await {
    eprintln!("[Warning] Failed to save session: {e}");
}
// Also append all new messages to transcript
let transcript = TranscriptManager::new(session_id);
for msg in session.messages.iter().skip(/* messages that existed before this run */) {
    // Only append messages added during this run
    // This requires tracking message count before/after
}
```

**Simpler approach:** Just save the session after editing completes. The transcript will accumulate on next run_interactive. Don't overthink the per-message tracking for now.

```rust
match engine.run(String::new()).await {
    Ok(AgentOutcome::Completed) => {}
    Ok(AgentOutcome::Error(msg)) => {
        eprintln!("Agent error: {msg}");
        return Err(CliError::Other(msg));
    }
    Ok(AgentOutcome::Interrupted) => {}
    Err(e) => return Err(e),
}

// Save session after resume edit
let session = engine.session().clone();
if let Err(e) = bootstrap.session_storage.save(&session).await {
    eprintln!("[Warning] Failed to save session: {e}");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors

- [ ] **Step 3: Commit**

```bash
git add binary/src/cli/mod.rs
git commit -m "feat: save session after run_resume edits"
```

---

## Task 2: Slash Commands — Process /help, /exit, /clear, /session, /model

**Files:**
- Modify: `binary/src/tui/event_loop.rs`
- Modify: `binary/src/commands/mod.rs`

Slash commands are prefixed with `/`. When the user types `/help`, the REPL should print the help text and NOT send anything to the agent. `/exit` should exit. `/clear` should clear the printed conversation history. `/session` and `/model` are informational.

### Task 2a: Add slash command processor to event_loop

- [ ] **Step 1: Add a SlashCommand enum to binary/src/tui/event_loop.rs**

```rust
/// Built-in slash commands handled by the REPL (not sent to the agent)
#[derive(Debug)]
pub enum SlashCommand {
    Help,
    Exit,
    Clear,
    Session,
    Model,
    Tokens,
    Cost,
    Config,
    /// Not a command — pass to agent
    None,
    /// Custom command from project config
    Custom(String),
}

impl SlashCommand {
    /// Parse a raw input string into a slash command
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return SlashCommand::None;
        }
        let rest = trimmed.strip_prefix('/').unwrap_or("");
        match rest.split_whitespace().next().unwrap_or("").to_lowercase().as_str() {
            "help" | "h" => SlashCommand::Help,
            "exit" | "quit" | "q" => SlashCommand::Exit,
            "clear" | "reset" => SlashCommand::Clear,
            "session" | "sess" => SlashCommand::Session,
            "model" | "m" => SlashCommand::Model,
            "tokens" | "tc" => SlashCommand::Tokens,
            "cost" => SlashCommand::Cost,
            "config" | "cfg" => SlashCommand::Config,
            "" => SlashCommand::None,
            cmd => SlashCommand::Custom(cmd.to_string()),
        }
    }

    pub fn is_agent_input(&self) -> bool {
        matches!(self, SlashCommand::None | SlashCommand::Custom(_))
    }

    pub fn description(&self) -> &'static str {
        match self {
            SlashCommand::Help => "Show available slash commands",
            SlashCommand::Exit => "Exit Claude Code",
            SlashCommand::Clear => "Clear conversation history",
            SlashCommand::Session => "Show current session info",
            SlashCommand::Model => "Show current model",
            SlashCommand::Tokens => "Show token usage",
            SlashCommand::Cost => "Show estimated cost",
            SlashCommand::Config => "Show configuration",
            SlashCommand::None => "",
            SlashCommand::Custom(_) => "Custom command",
        }
    }
}
```

- [ ] **Step 2: Add a REPL state struct to track conversation history for /clear**

```rust
/// REPL conversation history (lines printed to stdout)
pub struct ReplHistory {
    lines: Vec<String>,
}

impl ReplHistory {
    pub fn new() -> Self { Self { lines: Vec::new() } }
    pub fn add_user(&mut self, text: &str) { self.lines.push(format!("[You] {text}")); }
    pub fn add_response(&mut self, text: &str) { self.lines.push(format!("[Claude] {text}")); }
    pub fn add_system(&mut self, text: &str) { self.lines.push(format!("[System] {text}")); }
    pub fn add_tool(&mut self, name: &str) { self.lines.push(format!("[Tool: {name}]")); }
    pub fn clear(&mut self) {
        // Print ANSI clear screen
        println!("\x1b[2J\x1b[H");
        self.lines.clear();
    }
    pub fn print_help(&self) {
        println!("\nClaude Code — Available commands:");
        let cmds = [
            ("/help, /h", "Show this help"),
            ("/exit, /q", "Exit Claude Code"),
            ("/clear", "Clear conversation"),
            ("/session", "Show session info"),
            ("/model, /m", "Show current model"),
            ("/tokens, /tc", "Show token usage"),
            ("/cost", "Show estimated cost"),
            ("/config, /cfg", "Show configuration"),
        ];
        for (cmd, desc) in &cmds {
            println!("  {:<18} {}", cmd, desc);
        }
        println!();
    }
}

impl Default for ReplHistory {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 3: Rewrite run_repl to use slash commands and ReplHistory**

```rust
pub async fn run_repl(
    mut on_message: impl FnMut(String) -> BoxFuture<'static, bool>,
    session_info: ReplSessionInfo,
) -> Result<(), io::Error> {
    let mut history = ReplHistory::new();
    let mut reader = EventStream::new();

    println!("Claude Code Interactive Mode");
    println!("Type your message and press Enter. Ctrl+C or Esc to exit.");
    println!("Type /help for commands.\n---\n");

    loop {
        print!("\n> ");
        io::stdout().flush()?;

        let input = read_line(&mut reader).await?;
        let text = input.trim().to_string();
        if text.is_empty() { continue; }

        // Parse slash command
        match SlashCommand::parse(&text) {
            SlashCommand::Exit => {
                println!("Goodbye!");
                return Ok(());
            }
            SlashCommand::Help => {
                history.print_help();
                continue;
            }
            SlashCommand::Clear => {
                history.clear();
                continue;
            }
            SlashCommand::Session => {
                println!("\nSession: {}", session_info.id);
                println!("Model: {}", session_info.model);
                println!();
                continue;
            }
            SlashCommand::Model => {
                println!("\nModel: {}\n", session_info.model);
                continue;
            }
            SlashCommand::Tokens => {
                println!("\nTokens: (tracked in session)\n");
                continue;
            }
            SlashCommand::Cost => {
                println!("\nCost: ${:.4f} (tracked in session)\n", session_info.cost);
                continue;
            }
            SlashCommand::Config => {
                println!("\nConfig: provider={}, model={}\n", session_info.provider, session_info.model);
                continue;
            }
            SlashCommand::Custom(_) => {
                // For now, treat custom commands as regular agent input
                // (CommandRegistry execution will be added in Task 3)
            }
            SlashCommand::None => {}
        }

        history.add_user(&text);
        print!("\n[Claude]\n");

        if !on_message(text).await {
            break;
        }
    }

    println!("\nGoodbye!");
    Ok(())
}
```

- [ ] **Step 4: Add ReplSessionInfo struct**

```rust
/// Session info passed to the REPL for /session, /model, /cost commands
#[derive(Clone)]
pub struct ReplSessionInfo {
    pub id: String,
    pub model: String,
    pub provider: String,
    pub cost: f64,
}
```

- [ ] **Step 5: Update the call site in cli/mod.rs**

The `run_repl` call in `run_interactive()` needs to pass `ReplSessionInfo`:

```rust
use crate::tui::event_loop::{run_repl, ReplSessionInfo};

let session_info = ReplSessionInfo {
    id: engine.session().id.clone(),
    model: session_config.model.clone(),
    provider: bootstrap.provider.to_string(),
    cost: 0.0,
};

run_repl(on_message, session_info).await
```

The `cost` field is initially 0.0 in `run_interactive` since the session is new. For `run_resume`, pass the loaded session's cost.

- [ ] **Step 6: Add the async read_line helper**

```rust
async fn read_line(reader: &mut EventStream) -> Result<String, io::Error> {
    let mut input = String::new();
    loop {
        let timeout = tokio::time::timeout(Duration::from_millis(100), reader.next()).await;
        match timeout {
            Ok(Some(Ok(crossterm::event::Event::Key(key)))) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok("".to_string());
                }
                match key.code {
                    KeyCode::Enter => { break; }
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{c}");
                        io::stdout().flush()?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            print!("\x08 \x08");
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Esc => { return Ok("".to_string()); }
                    _ => {}
                }
            }
            Ok(None) | Err(_) => { return Ok(input); }
            _ => {}
        }
    }
    Ok(input)
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors

- [ ] **Step 8: Commit**

```bash
git add binary/src/tui/event_loop.rs binary/src/cli/mod.rs
git commit -m "feat: add slash command processor and REPL history to event loop"
```

---

## Task 3: Context Compaction — Prevent Token Overflow

**Files:**
- Create: `library/src/session/compaction.rs`
- Modify: `library/src/session/mod.rs`
- Modify: `library/src/agent/engine.rs:1-50`

When the session accumulates many messages, the token count grows unbounded. We need to detect when we're approaching the model's context window limit and compress old messages into a summary.

### Task 3a: Session compaction module

- [ ] **Step 1: Write the compaction module**

Create `library/src/session/compaction.rs`:

```rust
//! Session context compaction — summarise old messages when token budget is exceeded

use crate::types::{Message, AssistantContent, UserContent};

/// Configuration for compaction behaviour
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Start compacting when this fraction of max tokens is reached (0.0–1.0)
    pub threshold_ratio: f64,
    /// Target token count after compaction (leave recent context intact)
    pub target_tokens: usize,
    /// Max model context window (used if threshold_tokens is None)
    pub max_context_tokens: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_ratio: 0.80,
            target_tokens: 30_000,
            max_context_tokens: 200_000,
        }
    }
}

impl CompactionConfig {
    /// Returns the absolute token threshold at which compaction should trigger
    pub fn threshold_tokens(&self) -> usize {
        (self.max_context_tokens as f64 * self.threshold_ratio) as usize
    }
}

/// SessionCompactor handles detecting when to compact and generating summary messages
pub struct SessionCompactor {
    config: CompactionConfig,
}

impl SessionCompactor {
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Estimate whether the current session should be compacted
    pub fn should_compact(&self, messages: &[Message], token_usage: &crate::types::TokenUsage) -> bool {
        let total_tokens = token_usage.total();
        total_tokens >= self.config.threshold_tokens()
    }

    /// Compact old messages by replacing them with a single summary message.
    /// Keeps the most recent `target_tokens` worth of messages intact.
    ///
    /// Returns the compacted message list, or None if no compaction needed.
    pub fn compact(&self, messages: &mut Vec<Message>) -> Option<()> {
        if messages.len() < 6 {
            return None; // Need at least a few turns to make summarisation worthwhile
        }

        // Keep last 10 messages (covers recent context)
        let keep_count = 10usize.min(messages.len() / 2);
        let summary_messages = messages[..messages.len() - keep_count].to_vec();

        let summary_text = Self::summarise_messages(&summary_messages);
        let old_count = summary_messages.len();

        // Remove old messages
        messages.drain(..messages.len() - keep_count);

        // Insert summary as a system message at the start of the kept section
        let summary_msg = Message::System {
            message: format!(
                "[Previous conversation summarised ({} messages): {}]",
                old_count,
                summary_text
            ),
        };

        messages.insert(messages.len() - keep_count, summary_msg);
        Some(())
    }

    /// Generate a brief text summary of a list of messages
    fn summarise_messages(messages: &[Message]) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let mut tool_count = 0usize;
        let mut user_msgs = 0usize;
        let mut assistant_msgs = 0usize;

        for msg in messages {
            match msg {
                Message::User { .. } => user_msgs += 1,
                Message::Assistant { content } => {
                    assistant_msgs += 1;
                    if let Some(c) = content {
                        if let Some(text) = c.text_preview() {
                            if out.len() < 200 {
                                let preview = text.chars().take(80).collect::<String>();
                                let _ = write!(&mut out, "Response: {preview}. ");
                            }
                        }
                    }
                }
                Message::ToolUse { name, .. } => tool_count += 1,
                _ => {}
            }
        }

        let _ = write!(
            &mut out,
            "{} user messages, {} Claude responses, {} tool uses. ",
            user_msgs, assistant_msgs, tool_count
        );
        out
    }
}
```

- [ ] **Step 2: Add text_preview() helper to AssistantContent**

Modify `library/src/types/message.rs` — find the `AssistantContent` struct and add:

```rust
impl AssistantContent {
    /// Get a short text preview of the first text block, if any
    pub fn text_preview(&self) -> Option<String> {
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                return Some(text.clone());
            }
        }
        None
    }
}
```

- [ ] **Step 3: Export from session/mod.rs**

Add to `library/src/session/mod.rs`:

```rust
pub mod storage;
pub mod transcript;
pub mod compaction;

pub use storage::*;
pub use transcript::*;
pub use compaction::*;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p claude-code-rust`
Expected: Compiles with 0 errors

- [ ] **Step 5: Commit**

```bash
git add library/src/session/compaction.rs library/src/session/mod.rs library/src/types/message.rs
git commit -m "feat: add session compaction to prevent token overflow"
```

---

### Task 3b: Wire compaction into AgentEngine

**Files:**
- Modify: `library/src/agent/engine.rs`

- [ ] **Step 1: Add compaction fields to AgentEngine**

```rust
use crate::session::SessionCompactor;

pub struct AgentEngine {
    api_client: ApiClient,
    context: AgentContext,
    permission_checker: PermissionChecker,
    compactor: SessionCompactor,
}
```

- [ ] **Step 2: Update AgentEngine::new to create the compactor**

```rust
impl AgentEngine {
    pub fn new(api_client: ApiClient, context: AgentContext) -> Self {
        let permission_checker = PermissionChecker::new(context.permission_mode());
        let compactor = SessionCompactor::new(crate::session::CompactionConfig::default());
        Self { api_client, context, permission_checker, compactor }
    }
}
```

- [ ] **Step 3: Add compaction check after token tracking**

In the `run()` loop, after `self.context.session.token_usage.add(&response.usage)`:

```rust
// Check if we need to compact
if self.compactor.should_compact(&self.context.session.messages, &self.context.session.token_usage) {
    eprintln!("[Info] Compacting conversation context to stay within token limit...");
    self.compactor.compact(&mut self.context.session.messages);
}
```

Add after the session save in `session_mut`:

```rust
/// Get session for persistence
pub fn session(&self) -> &crate::types::Session {
    &self.context.session
}

/// Get mutable session for compaction and persistence
pub fn session_mut(&mut self) -> &mut crate::types::Session {
    &mut self.context.session
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p claude-code-rust`
Expected: Compiles with 0 errors

- [ ] **Step 5: Commit**

```bash
git add library/src/agent/engine.rs
git commit -m "feat: wire session compaction into agent engine loop"
```

---

## Task 4: Hook System — Pre/Post Tool-Use Hooks

**Files:**
- Create: `library/src/agent/hooks.rs`
- Modify: `library/src/agent/mod.rs`
- Modify: `library/src/agent/context.rs`
- Modify: `library/src/agent/engine.rs`

The hook system lets the config define `pre_tool_use`, `post_tool_use`, `pre_query`, and `post_query` hooks that run before/after tool execution and agent turns.

### Task 4a: Hook types and runner

- [ ] **Step 1: Create library/src/agent/hooks.rs**

```rust
//! Hook system for pre/post tool and query callbacks

use crate::error::CliError;
use crate::types::ToolResult;
use std::collections::HashMap;

/// A hook definition loaded from config
#[derive(Debug, Clone)]
pub struct Hook {
    pub name: String,
    pub hook_type: HookType,
    /// Shell command to run. The command receives data via stdin as JSON.
    pub command: String,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookType {
    PreToolUse,
    PostToolUse,
    PreQuery,
    PostQuery,
}

impl Hook {
    /// Build the JSON payload sent to a pre_tool_use hook
    pub fn pre_tool_payload(tool_name: &str, tool_input: &serde_json::Value, session_id: &str) -> serde_json::Value {
        serde_json::json!({
            "hook": "pre_tool_use",
            "tool_name": tool_name,
            "tool_input": tool_input,
            "session_id": session_id,
        })
    }

    /// Build the JSON payload sent to a post_tool_use hook
    pub fn post_tool_payload(tool_name: &str, result: &ToolResult, session_id: &str) -> serde_json::Value {
        serde_json::json!({
            "hook": "post_tool_use",
            "tool_name": tool_name,
            "result": {
                "is_error": result.is_error,
                "content": result.content.iter().map(|b| b.preview()).collect::<Vec<_>>(),
            },
            "session_id": session_id,
        })
    }

    /// Run the hook command with the given JSON payload on stdin
    pub async fn run(&self, payload: &serde_json::Value) -> Result<String, CliError> {
        let json = serde_json::to_string(payload)
            .map_err(|e| CliError::Other(format!("hook payload error: {e}")))?;

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
           .arg(&self.command)
           .kill_on_drop(true);

        let mut child = cmd
            .stdin(tokio::process::Stdio::piped())
            .stdout(tokio::process::Stdio::piped())
            .stderr(tokio::process::Stdio::piped())
            .spawn()
            .map_err(|e| CliError::Other(format!("hook spawn failed: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            tokio::io::AsyncWriteExt::write_all(&mut stdin, json.as_bytes()).await
                .map_err(|e| CliError::Other(format!("hook stdin error: {e}")))?;
            drop(stdin);
        }

        let timeout = std::time::Duration::from_secs(self.timeout_secs);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| CliError::Other(format!("hook '{}' timed out", self.name)))?
            .map_err(|e| CliError::Other(format!("hook '{}' failed: {e}", self.name)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::Other(format!(
                "hook '{}' returned non-zero: {}",
                self.name, stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Loads hooks from a config hooks map
pub fn load_hooks_from_config(hooks_config: &HashMap<String, serde_json::Value>) -> Vec<Hook> {
    let mut hooks = Vec::new();

    for (name, cfg) in hooks_config {
        let hook_type = match name.as_str() {
            "pre_tool_use" => HookType::PreToolUse,
            "post_tool_use" => HookType::PostToolUse,
            "pre_query" => HookType::PreQuery,
            "post_query" => HookType::PostQuery,
            _ => continue,
        };

        let command = cfg.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if command.is_empty() {
            continue;
        }

        let timeout_secs = cfg
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        hooks.push(Hook {
            name: name.clone(),
            hook_type,
            command,
            timeout_secs,
        });
    }

    hooks
}
```

- [ ] **Step 2: Create library/src/agent/mod.rs with hook exports**

Replace `library/src/agent/mod.rs` content:

```rust
//! Agent core modules

pub mod engine;
pub mod context;
pub mod hooks;
pub mod permission;

pub use engine::*;
pub use context::*;
pub use hooks::*;
pub use permission::*;
```

- [ ] **Step 3: Add HookRunner to AgentContext**

Modify `library/src/agent/context.rs`:

```rust
use super::hooks::{Hook, HookType};

pub struct AgentContext {
    pub session: Session,
    pub config: SessionConfig,
    pub tools: Vec<Arc<dyn Tool>>,
    pub working_directory: PathBuf,
    pub global_config: crate::config::GlobalConfig,
    pub project_config: crate::config::ProjectConfig,
    pub env: std::collections::HashMap<String, String>,
    pub hooks: Vec<Hook>,
}

impl AgentContext {
    pub fn new(
        session: Session,
        config: SessionConfig,
        tools: Vec<Arc<dyn Tool>>,
        working_directory: PathBuf,
        global_config: crate::config::GlobalConfig,
        project_config: crate::config::ProjectConfig,
        env: std::collections::HashMap<String, String>,
    ) -> Self {
        let hooks = super::hooks::load_hooks_from_config(&global_config.hooks);
        Self { session, config, tools, working_directory, global_config, project_config, env, hooks }
    }

    /// Find hooks of a specific type
    pub fn hooks_of_type(&self, hook_type: HookType) -> Vec<&Hook> {
        self.hooks.iter().filter(|h| h.hook_type == hook_type).collect()
    }
}
```

- [ ] **Step 4: Wire hooks into engine tool execution**

In `engine.rs`, before `tool.call(...)` (inside `PermissionDecision::Allow`):

```rust
// Run pre_tool_use hooks
for hook in self.context.hooks_of_type(HookType::PreToolUse) {
    let payload = Hook::pre_tool_payload(tool.name(), input, &self.context.session.id);
    if let Err(e) = hook.run(&payload).await {
        eprintln!("[Hook warning] {}: {e}", hook.name);
    }
}
```

After `tool.call(...)` result:

```rust
// Run post_tool_use hooks
for hook in self.context.hooks_of_type(HookType::PostToolUse) {
    let payload = Hook::post_tool_payload(tool.name(), &result, &self.context.session.id);
    if let Err(e) = hook.run(&payload).await {
        eprintln!("[Hook warning] {}: {e}", hook.name);
    }
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p claude-code-rust`
Expected: Compiles with 0 errors

- [ ] **Step 6: Commit**

```bash
git add library/src/agent/hooks.rs library/src/agent/mod.rs library/src/agent/context.rs library/src/agent/engine.rs
git commit -m "feat: add hook system for pre/post tool and query callbacks"
```

---

## Task 5: Plugin Loading — Wire load_all() into build_tools()

**Files:**
- Modify: `binary/src/cli/mod.rs:17-61`

The plugin registry's `load_all()` is defined but never called. Plugins can provide tools, so they should be loaded before building the tool list.

- [ ] **Step 1: Load plugins in build_tools()**

In `build_tools()`, before registering built-in tools:

```rust
use crate::plugins::PluginRegistry;

async fn build_tools(...) -> Result<Vec<Arc<dyn Tool>>, CliError> {
    // Load plugins
    let mut plugin_registry = PluginRegistry::new();
    if let Err(e) = plugin_registry.load_all().await {
        eprintln!("[Warning] Plugin loading failed: {e}");
    }

    let plugin_tools: Vec<Arc<dyn Tool>> = plugin_registry.list()
        .iter()
        .flat_map(|p| p.tools.iter())
        .filter_map(|tool_name| {
            // For now, plugin tools would need to be loaded as MCP or external processes
            // Just log what plugins are available
            eprintln!("[Plugin] {} provides tool: {}", p.name, tool_name);
            None
        })
        .collect();

    // ... rest unchanged
}
```

Note: Full plugin tool loading (spawning plugin processes and talking MCP to them) is a larger task. For now, just call `load_all()` to discover available plugins and log them. This makes the scaffold functional.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors

- [ ] **Step 3: Commit**

```bash
git add binary/src/cli/mod.rs
git commit -m "feat: wire plugin discovery into build_tools"
```

---

## Task 6: Consolidate CommandDefinition — Remove Duplicate Types

**Files:**
- Modify: `binary/src/commands/mod.rs`

There are two `CommandDefinition` types: one in `library/src/config/project.rs` (richer, with `timeout_secs`, `env`, etc.) and one in `binary/src/commands/mod.rs` (simpler). The config version should be canonical; the binary's `CommandRegistry` should use it.

- [ ] **Step 1: Replace binary CommandDefinition with library re-export**

Replace the top of `binary/src/commands/mod.rs`:

```rust
//! Command system for slash commands
//! Uses CommandDefinition from library/config/project.rs

use claude_code_rust::config::project::CommandDefinition;
use claude_code_rust::error::CliError;
use std::collections::HashMap;
use std::process::Stdio;
```

Remove the local `CommandDefinition` struct and `CommandType` enum. Update `CommandRegistry.custom` to `HashMap<String, CommandDefinition>`.

Update the `register` method:

```rust
pub fn register(&mut self, cmd: CommandDefinition) {
    self.custom.insert(format!("/{}", cmd.name), cmd);
}
```

The `execute_custom` method uses `def.command` — but the config `CommandDefinition` uses `script: Option<String>` instead. Update `execute_custom` to use `script`:

```rust
async fn execute_custom(
    &self,
    def: &CommandDefinition,
    args: Option<String>,
) -> Result<CommandOutput, CliError> {
    let script = def.script.as_ref()
        .ok_or_else(|| CliError::Other("custom command has no script".to_string()))?;

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(script);
    if let Some(ref a) = args {
        cmd.arg(a);
    }
    // ... rest of the method unchanged (timeout, env, working_dir from config)
}
```

Add the missing timeout/env fields to the config's `CommandDefinition` if they're not there already. Check `library/src/config/project.rs` — if `timeout_secs` and `env` are missing, add them:

```rust
// In library/src/config/project.rs CommandDefinition:
pub timeout_secs: Option<u64>,
pub env: Option<std::collections::HashMap<String, String>>,
pub working_directory: Option<String>,
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors

- [ ] **Step 3: Commit**

```bash
git add binary/src/commands/mod.rs library/src/config/project.rs
git commit -m "refactor: consolidate CommandDefinition into library/config"
```

---

## Task 7: System Prompt — Populate Session.system_prompt

**Files:**
- Modify: `library/src/agent/context.rs`
- Modify: `binary/src/cli/mod.rs`

`Session.system_prompt` is always `None`. The agent needs a system prompt to know it's Claude Code.

- [ ] **Step 1: Add a default system prompt constant**

Add to `library/src/types/session.rs`:

```rust
/// Default system prompt for Claude Code sessions
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are Claude Code, an AI assistant built by Anthropic.
Your knowledge cutoff is 2024-04. You are helpful, creative, and care about writing good software.

Available tools: bash, read, write, edit, grep, glob, web_fetch, web_search.
- Use bash to run commands.
- Use read to view files.
- Use write to create or overwrite files.
- Use edit to make targeted changes.
- Use grep to search file contents.
- Use glob to find files by pattern.
- Use web_fetch to get content from a URL.
- Use web_search to search the web.

When using tools:
- Be precise about file paths.
- Prefer the most targeted tool for the job.
- Always prefer existing files over creating new ones.
- Check your work by reading files back.

For code tasks:
- Write clean, readable code.
- Follow the project's existing conventions.
- Write tests when adding significant logic.

If you don't know something, say so rather than guessing.
"#;
```

- [ ] **Step 2: Use it when creating sessions**

In `binary/src/cli/mod.rs`, when creating `Session::new()`:

```rust
let mut session = Session::new(
    uuid::Uuid::new_v4().to_string(),
    session_config.model.clone(),
);
session.system_prompt = Some(claude_code_rust::types::DEFAULT_SYSTEM_PROMPT.to_string());
```

Also update `run_resume()` where the session is loaded from storage — the loaded session already has whatever system_prompt was saved (possibly None for old sessions).

- [ ] **Step 3: Add system prompt to API messages**

In `library/src/api/client.rs`, in `messages_into_api()`, add a system message before user messages if `session.system_prompt` is Some:

```rust
// At the start of messages_into_api, check if we should add a system prompt
if let Some(ref system_prompt) = session.system_prompt {
    api_messages.insert(0, ApiMessage {
        role: "system".to_string(),
        content: vec![ApiContent::Text { text: system_prompt.clone() }],
    });
}
```

Pass `session` (or just `system_prompt: Option<String>`) into the function. Add `session: &Session` parameter:

```rust
pub fn messages_into_api(messages: &[Message], session: &Session) -> Vec<ApiMessage>
```

Update the single call site in `engine.rs`:

```rust
let messages: Vec<Message> = self.context.session.messages.clone();
let api_messages = self.api_client.messages_into_api(&messages, &self.context.session);
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p claude-code-rust && cargo check -p claude-code-rust-cli`
Expected: Compiles with 0 errors

- [ ] **Step 5: Commit**

```bash
git add library/src/types/session.rs library/src/api/client.rs library/src/agent/engine.rs binary/src/cli/mod.rs
git commit -m "feat: populate session system prompt and send to API"
```

---

## Task 8: Final Build and Test

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p claude-code-rust
```

Expected: 20+ tests passing

- [ ] **Step 2: Build release binary**

```bash
cargo build -p claude-code-rust-cli --release
```

Expected: Builds successfully, binary at `target/release/claude-code-rust-cli`

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p claude-code-rust --test integration
```

Expected: All tests pass

- [ ] **Step 4: Check for warnings**

```bash
cargo check 2>&1 | grep -E "^error|^warning"
```

Address any new warnings introduced by this phase.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: phase 2 complete — persistence, slash commands, compaction, hooks"
```
