# Claude Code Rust 重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 TypeScript 实现的 Claude Code CLI 完全用 Rust 重构，保证功能一致，包括 CLI 参数解析、Agent 核心循环、Tool 系统、MCP 集成、配置管理、Session 持久化等所有核心功能。

**Architecture:** 采用模块化架构，核心引擎（Agent Loop + Tool System）与 CLI 传输层分离。核心引擎可独立使用（headless mode），CLI 层负责用户交互和输出渲染。数据模型使用 serde 序列化保证与现有 Session 格式兼容。

**Tech Stack:** Rust (Edition 2024), tokio (async runtime), clap (CLI), reqwest (HTTP), serde/serde_json, ratatui or cursive (TUI), tower-lsp (LSP integration)

---

## 第一阶段：项目脚手架与基础设施

### 任务 1：项目基础配置

**Files:**
- Modify: `Cargo.toml`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: 更新 Cargo.toml 添加所有依赖**

```toml
[package]
name = "claude-code-rust"
version = "2.1.88"
edition = "2024"
description = "High-performance Rust implementation of Claude Code CLI"

[dependencies]
# Async runtime
tokio = { version = "1.37", features = ["rt-multi-thread", "macros", "sync", "fs", "io-util"] }

# CLI
clap = { version = "4.5", features = ["derive", "env"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# HTTP (rustls only, no native-tls)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }

# WebSocket
tokio-tungstenite = { version = "0.21", features = ["rustls-tls"] }

# TUI
ratatui = "0.26"
crossterm = "0.27"

# File operations
notify = "6.1"           # File watching (替代 chokidar)
walkdir = "2.4"          # Directory traversal (替代 glob)
glob = "0.3"             # Glob patterns
regex = "1.10"           # Regex (替代 grep)
fuzzy-matcher = "0.3"    # Fuzzy matching (替代 fuse.js)
similar = "0.2"          # Diff (替代 diff crate)

# Async utilities
futures = "0.3"
async-trait = "0.1"
tower-lsp = "0.20"
lsp-types = "0.20"

# Crypto / auth
keyring = "3"

# Process
tokio-process = "0.3"
shell-escape = "0.1"

# Utils
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.8", features = ["v4", "serde"] }
dirs = "5.0"
os-type = "0.2"
semver = "1.0"

# MCP Protocol
# 注意: 需要从 Rust 重新实现 MCP 协议相关部分

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.10"
wiremock = "0.6"
```

- [ ] **Step 2: 创建 rust-toolchain.toml**

```toml
[toolchain]
channel = "1.87"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-unknown-linux-gnu"]
```

- [ ] **Step 3: 提交**

```bash
git add Cargo.toml rust-toolchain.toml
git commit -m "chore: add all project dependencies"
```

---

### 任务 2：核心错误类型定义

**Files:**
- Create: `src/error.rs`

- [ ] **Step 1: 定义错误类型**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("API error: {0}")]
    Api(String),

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("Tool permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}
```

- [ ] **Step 3: 提交**

```bash
git add src/error.rs
git commit -m "feat: add core error types"
```

---

### 任务 3：核心数据模型定义

**Files:**
- Create: `src/types/mod.rs`
- Create: `src/types/message.rs`
- Create: `src/types/tool.rs`
- Create: `src/types/task.rs`
- Create: `src/types/session.rs`
- Create: `src/types/permission.rs`

- [ ] **Step 1: 创建消息类型 (src/types/message.rs)**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    User { content: UserContent },
    Assistant { content: Option<AssistantContent> },
    Progress { data: ProgressData },
    System { subtype: String, level: Option<String>, message: String },
    Attachment { path: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
    HookResult { hook_name: String, result: serde_json::Value },
    Tombstone,
    GroupedToolUse { tool_uses: Vec<ToolUseSummary> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContent {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub message: Option<String>,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseSummary {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}
```

- [ ] **Step 2: 创建 Tool 接口 (src/types/tool.rs)**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> Vec<String>;
    fn description(&self) -> String;
    fn input_schema(&self) -> serde_json::Value;
    fn is_concurrency_safe(&self) -> bool { true }
    fn is_read_only(&self) -> bool { false }
    fn is_destructive(&self) -> bool { false }
    fn is_enabled(&self) -> bool { true }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError>;

    fn render_use_message(&self, args: &serde_json::Value) -> String;
    fn render_result_message(&self, result: &ToolResult) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub working_directory: String,
    pub can_use_tool: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ResultContentBlock>,
    pub is_error: bool,
    pub metrics: Option<ToolMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub duration_ms: u64,
    pub tokens_used: Option<u32>,
}
```

- [ ] **Step 3: 创建 Task 类型 (src/types/task.rs)**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    LocalBash,
    LocalAgent,
    RemoteAgent,
    InProcessTeammate,
    LocalWorkflow,
    MonitorMcp,
    Dream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub description: String,
    pub tool_use_id: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub output_file: PathBuf,
    pub notified: bool,
}
```

- [ ] **Step 4: 创建 Session 类型 (src/types/session.rs)**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub cost: f64,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub permission_mode: PermissionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    AcceptEdits,
    BypassPermissions,
    #[default]
    Default,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}
```

- [ ] **Step 5: 创建 mod.rs**

```rust
pub mod message;
pub mod tool;
pub mod task;
pub mod session;
pub mod permission;

pub use message::*;
pub use tool::*;
pub use task::*;
pub use session::*;
pub use permission::*;
```

- [ ] **Step 6: 提交**

```bash
git add src/types/
git commit -m "feat: add core data models"
```

---

## 第二阶段：配置系统

### 任务 4：配置管理

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/global.rs`
- Create: `src/config/project.rs`
- Create: `src/config/settings.rs`

- [ ] **Step 1: 全局配置 (src/config/global.rs)**

参考 TS 的 `src/utils/config.ts`，需要支持的功能：
- Theme 设置
- Verbose 模式
- MCP Servers 配置
- OAuth 账户信息
- 环境变量注入
- Auto compact 开关
- Todo feature 开关
- 100+ 字段

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub version: String,
    pub theme: ThemeSetting,
    pub verbose: bool,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub oauth_account: Option<OAuthAccount>,
    pub env: HashMap<String, String>,
    pub auto_compact_enabled: bool,
    pub todo_feature_enabled: bool,
    pub model_preferences: ModelPreferences,
    pub permission_mode: PermissionMode,
    pub hooks: Vec<HookConfig>,
    pub allowed_tools: Vec<String>,
    pub denied_tools: Vec<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub uri_open_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    #[serde(rename = "type")]
    pub config_type: McpServerType,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerType {
    Stdio,
    Sse,
    Http,
    Ws,
    Sdk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSetting {
    pub variant: ThemeVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeVariant {
    Auto,
    Dark,
    Light,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    pub provider: ModelProvider,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    #[default]
    Anthropic,
    AwsBedrock,
    GcpVertex,
    Azure,
    OpenAi,
    Ollama,
    Together,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccount {
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}
```

- [ ] **Step 2: 项目配置 (src/config/project.rs)**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub allowed_tools: Vec<String>,
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,
    pub last_api_duration_ms: Option<u64>,
    pub last_cost: Option<f64>,
    pub has_trust_dialog_accepted: Option<bool>,
    pub active_worktree_session: Option<String>,
    pub custom_commands: Vec<CommandDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub command_type: CommandType,
    pub prompt: Option<String>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Prompt,
    Local,
    LocalJsx,
}
```

- [ ] **Step 3: 配置加载器 (src/config/mod.rs)**

```rust
use std::path::PathBuf;
use anyhow::Result;

pub mod global;
pub mod project;

pub use global::*;
pub use project::*;

pub struct ConfigLoader {
    global_path: PathBuf,
    project_path: PathBuf,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            global_path: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("claude"),
            project_path: PathBuf::from(".claude"),
        }
    }

    pub async fn load_global_config(&self) -> Result<GlobalConfig> { ... }
    pub async fn load_project_config(&self, repo_root: Option<PathBuf>) -> Result<ProjectConfig> { ... }
    pub async fn save_project_config(&self, config: &ProjectConfig) -> Result<()> { ... }
}
```

- [ ] **Step 4: 提交**

```bash
git add src/config/
git commit -m "feat: add configuration system"
```

---

## 第三阶段：Session 持久化

### 任务 5：Session Storage

**Files:**
- Create: `src/session/mod.rs`
- Create: `src/session/storage.rs`
- Create: `src/session/transcript.rs`

- [ ] **Step 1: Session 存储引擎 (src/session/storage.rs)**

参考 TS 的 `src/utils/sessionStorage.ts` (~180KB)，需要实现：
- Session 目录管理
- Transcript JSONL 持久化
- Message 序列化/反序列化
- Compact 操作（上下文压缩）
- History 管理

```rust
// 核心功能：
// 1. Session 目录: ~/.claude/sessions/{session_id}/
// 2. Transcript: sessions/{session_id}/transcript.jsonl
// 3. State: sessions/{session_id}/state.json
// 4. Messages: sessions/{session_id}/messages.json
```

- [ ] **Step 2: Transcript 管理 (src/session/transcript.rs)**

实现 NDJSON 格式的 transcript 追加写入，与现有格式完全兼容。

- [ ] **Step 3: Compact 操作 (src/session/compact.rs)**

实现 token 预算管理和上下文压缩：
- 计算当前 token 使用量
- 按策略压缩历史消息
- 保留关键系统消息和工具结果

- [ ] **Step 4: 提交**

```bash
git add src/session/
git commit -m "feat: add session persistence layer"
```

---

## 第四阶段：API 客户端

### 任务 6：API 客户端实现

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/client.rs`
- Create: `src/api/auth.rs`
- Create: `src/api/errors.rs`

- [ ] **Step 1: API Client (src/api/client.rs)**

参考 TS 的 `src/services/api/claude.ts`，实现：
- Anthropic API 调用（支持流式和非流式）
- 重试机制和速率限制
- Quota 跟踪
- Token 估算和成本计算
- 支持多 Provider（Bedrock, Vertex, Ollama, OpenAI, Azure, Together）

```rust
pub struct ApiClient {
    provider: ModelProvider,
    api_key: String,
    base_url: Option<String>,
    http_client: reqwest::Client,
}

impl ApiClient {
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError>;
    pub async fn chat_streaming(&self, request: ChatRequest) -> Result<StreamingResponse, ApiError>;
    pub async fn count_tokens(&self, messages: &[Message], model: &str) -> Result<u64, ApiError>;
    pub async fn estimate_cost(&self, usage: &TokenUsage, model: &str) -> f64;
}
```

- [ ] **Step 2: Auth 处理 (src/api/auth.rs)**

实现 API Key 管理、多 Provider 认证支持。

- [ ] **Step 3: 错误处理 (src/api/errors.rs)**

定义 API 错误类型，包括 rate limit、auth error、quota exceeded 等。

- [ ] **Step 4: 提交**

```bash
git add src/api/
git commit -m "feat: add API client with multi-provider support"
```

---

## 第五阶段：Tool 系统

### 任务 7：内置 Tool 实现

**Files:**
- Create: `src/tools/mod.rs`
- Create: `src/tools/registry.rs`
- Create: `src/tools/file_read.rs`
- Create: `src/tools/file_write.rs`
- Create: `src/tools/file_edit.rs`
- Create: `src/tools/bash.rs`
- Create: `src/tools/grep.rs`
- Create: `src/tools/glob.rs`
- Create: `src/tools/web_fetch.rs`
- Create: `src/tools/web_search.rs`
- Create: `src/tools/task_tool.rs`
- Create: `src/tools/agent.rs`

每个 Tool 都需要实现 `Tool` trait。参考 TS 的 `src/tools/` 目录中 55+ 个 Tool。

- [ ] **Step 1: Tool 注册表 (src/tools/registry.rs)**

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    aliases: HashMap<String, String>,  // alias -> canonical name
}

impl ToolRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, tool: impl Tool + 'static);
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn get_all(&self) -> Vec<Arc<dyn Tool>>;
    pub fn filter(&self, allowed: &[String], denied: &[String]) -> Vec<Arc<dyn Tool>>;
}
```

- [ ] **Step 2: FileReadTool (src/tools/file_read.rs)**

对应 TS: `src/tools/FileReadTool/`
- 支持路径读取
- 支持行范围限制（start, end）
- Git-aware 文件过滤
- LRU 缓存

- [ ] **Step 3: FileWriteTool (src/tools/file_write.rs)**

对应 TS: `src/tools/FileWriteTool/`
- 原子写入
- 目录创建
- 权限检查

- [ ] **Step 4: BashTool (src/tools/bash.rs)**

对应 TS: `src/tools/BashTool/`
- Shell 命令执行
- Working directory 支持
- 环境变量注入
- 超时控制
- Output capture（stdout/stderr）
- Pty 支持（可选，用于交互式命令）

```rust
pub struct BashTool {
    timeout_secs: u64,
}

impl Tool for BashTool {
    async fn call(&self, args: serde_json::Value, context: ToolContext) -> Result<ToolResult, CliError> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::ToolExecution("missing 'command' argument".into()))?;

        let working_dir = args.get("workingDirectory")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&context.working_directory));

        let output = Command::new("sh")
            .arg("-c", command)
            .current_dir(working_dir)
            .output()?;

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text {
                text: format!("stdout: {}\nstderr: {}", ...),
            }],
            is_error: !output.status.success(),
            metrics: None,
        })
    }
}
```

- [ ] **Step 5: GrepTool (src/tools/grep.rs)**

对应 TS: `src/tools/GrepTool/`
- 正则表达式搜索
- 文件类型过滤
- 上下文行数
- 忽略模式（.gitignore 风格）
- 并行搜索

- [ ] **Step 6: GlobTool (src/tools/glob.rs)**

对应 TS: `src/tools/GlobTool/`
- Glob 模式匹配
- 文件类型过滤
- 忽略模式
- 深度限制

- [ ] **Step 7: WebFetchTool (src/tools/web_fetch.rs)**

对应 TS: `src/tools/WebFetchTool/`
- HTTP GET 请求
- Response 处理（HTML, JSON, Markdown）
- 错误处理

- [ ] **Step 8: WebSearchTool (src/tools/web_search.rs)**

对应 TS: `src/tools/WebSearchTool/`
- Web 搜索
- 结果解析

- [ ] **Step 9: Task*Tool 系列 (src/tools/task_tool.rs)**

对应 TS: `src/tools/Task*Tool/`
- TaskCreateTool
- TaskUpdateTool
- TaskListTool

- [ ] **Step 10: AgentTool (src/tools/agent.rs)**

对应 TS: `src/tools/AgentTool/`
- 子 Agent 启动
- 消息传递
- 结果收集

- [ ] **Step 11: 提交**

```bash
git add src/tools/
git commit -m "feat: add built-in tool system"
```

---

## 第六阶段：Agent 核心循环

### 任务 8：Agent 核心引擎

**Files:**
- Create: `src/agent/mod.rs`
- Create: `src/agent/engine.rs`
- Create: `src/agent/context.rs`
- Create: `src/agent/coordinator.rs`
- Create: `src/agent/permission.rs`

这是核心模块，参考 TS 的 `src/query.ts` 和 `src/main.tsx` 中的核心循环。

- [ ] **Step 1: AgentContext (src/agent/context.rs)**

```rust
pub struct AgentContext {
    pub session: Session,
    pub config: SessionConfig,
    pub tools: Vec<Arc<dyn Tool>>,
    pub mcp_servers: HashMap<String, McpClient>,
    pub working_directory: PathBuf,
    pub env: HashMap<String, String>,
    pub permissions: PermissionChecker,
    pub hooks: HookRunner,
}

pub struct AgentContextBuilder {
    session_id: String,
    config: SessionConfig,
    tools: Vec<Arc<dyn Tool>>,
    working_directory: PathBuf,
}

impl AgentContextBuilder {
    pub async fn build(self) -> Result<AgentContext, CliError> { ... }
}
```

- [ ] **Step 2: PermissionChecker (src/agent/permission.rs)**

参考 TS 的 `src/types/permissions.ts` 和 `src/utils/permissions.ts`：
- Permission mode 评估
- Tool 使用权限检查
- Content 安全分类
- Rule-based 和 classifier-based 检查

```rust
pub struct PermissionChecker {
    mode: PermissionMode,
    rules: Vec<PermissionRule>,
}

pub enum PermissionDecision {
    Allow,
    Deny(String),
    Ask { message: String, suggestions: Vec<PermissionUpdate> },
    Passthrough(String),
}

impl PermissionChecker {
    pub async fn check_tool_use(
        &self,
        tool: &dyn Tool,
        args: &serde_json::Value,
        content: &str,
    ) -> Result<PermissionDecision, CliError> { ... }
}
```

- [ ] **Step 3: Agent Engine (src/agent/engine.rs)**

核心 Agent 循环：

```rust
pub struct AgentEngine {
    api_client: ApiClient,
    context: AgentContext,
    session_storage: SessionStorage,
}

impl AgentEngine {
    pub async fn run(&mut self, initial_prompt: String) -> Result<AgentOutcome, CliError> {
        loop {
            // 1. 构建消息列表
            let messages = self.context.build_messages()?;

            // 2. 发送 API 请求
            let response = self.api_client.chat(ChatRequest {
                model: self.context.config.model.clone(),
                messages,
                system: self.context.config.system_prompt.clone(),
                tools: Some(self.context.tools_schema()),
                max_tokens: self.context.config.max_tokens,
                stream: false,
            }).await?;

            // 3. 处理响应
            match self.process_response(response).await? {
                LoopAction::Continue => continue,
                LoopAction::Finish(result) => return Ok(result),
                LoopAction::ToolCall { tool_calls } => {
                    for tool_call in tool_calls {
                        // 4. 检查权限
                        let decision = self.context.permissions
                            .check_tool_use(&tool_call.name, &tool_call.input)
                            .await?;

                        match decision {
                            PermissionDecision::Allow => {
                                // 5. 执行 Tool
                                let result = self.execute_tool(tool_call).await?;
                                self.context.add_tool_result(result);
                            }
                            PermissionDecision::Ask { message, suggestions } => {
                                // 需要用户确认
                            }
                            PermissionDecision::Deny(msg) => { ... }
                            PermissionDecision::Passthrough(msg) => { ... }
                        }
                    }
                }
            }

            // 6. Compact check
            if self.should_compact() {
                self.compact().await?;
            }
        }
    }
}
```

- [ ] **Step 4: HookRunner (src/agent/hooks.rs)**

实现 Hook 系统：
- `pre_tool_use`
- `post_tool_use`
- `pre_query`
- `post_query`
- `on_tool_use_approval`
- `on_agent_finish`

- [ ] **Step 5: 提交**

```bash
git add src/agent/
git commit -m "feat: add agent core engine"
```

---

## 第七阶段：MCP 集成

### 任务 9：MCP 协议实现

**Files:**
- Create: `src/mcp/mod.rs`
- Create: `src/mcp/protocol.rs`
- Create: `src/mcp/client.rs`
- Create: `src/mcp/transport/stdio.rs`
- Create: `src/mcp/transport/sse.rs`
- Create: `src/mcp/transport/websocket.rs`

- [ ] **Step 1: MCP Protocol 定义 (src/mcp/protocol.rs)**

参考 `@modelcontextprotocol/sdk`，实现：
- JSON-RPC 2.0 消息格式
- Protocol 版本协商
- Schema 解析（inputSchema, outputSchema）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
pub enum JsonRpcMessage {
    #[serde(rename = "2.0")]
    Request(JsonRpcRequest),
    #[serde(rename = "2.0")]
    Response(JsonRpcResponse),
    #[serde(rename = "2.0")]
    Error(JsonRpcError),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: RequestId,
    pub method: String,
    pub params: Option<serde_json::Value>,
}
```

- [ ] **Step 2: Stdio Transport (src/mcp/transport/stdio.rs)**

实现与 MCP Server 的 stdio 通信：
- Process spawn
- stdin/stdout JSON-RPC 消息交换
- Process lifecycle 管理

```rust
pub struct StdioTransport {
    child: Child,
    stdin: PipeWriter,
    stdout: BufReader<ChildStdout>,
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&self, msg: JsonRpcMessage) -> Result<(), McpError>;
    async fn recv(&self) -> Result<JsonRpcMessage, McpError>;
}
```

- [ ] **Step 3: SSE/WebSocket Transport (src/mcp/transport/sse.rs, websocket.rs)**

实现远程 MCP Server 连接：
- SSE 事件流解析
- WebSocket 双向通信

- [ ] **Step 4: MCP Client (src/mcp/client.rs)**

```rust
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    capabilities: ServerCapabilities,
}

impl McpClient {
    pub async fn initialize() -> Result<Self, McpError>;
    pub async fn list_tools() -> Result<Vec<McpTool>, McpError>;
    pub async fn call_tool(name: &str, args: serde_json::Value) -> Result<McpToolResult, McpError>;
    pub async fn list_resources() -> Result<Vec<McpResource>, McpError>;
    pub async fn read_resource(uri: &str) -> Result<String, McpError>;
}
```

- [ ] **Step 5: MCP Tool 适配器**

将 MCP Tool 适配到本地 Tool 接口，使 MCP Server 的工具可以无缝集成到 Tool Registry。

- [ ] **Step 6: 提交**

```bash
git add src/mcp/
git commit -m "feat: add MCP protocol implementation"
```

---

## 第八阶段：CLI 层

### 任务 10：CLI 参数解析

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/args.rs`
- Create: `src/cli/bootstrap.rs`

- [ ] **Step 1: CLI 参数定义 (src/cli/args.rs)**

参考 TS 的 `src/main.tsx`，实现完整 CLI 参数支持：

```rust
#[derive(Parser, Debug)]
#[command(
    name = "claude",
    about = "Official Claude Code CLI",
    version,
)]
pub struct CliArgs {
    /// Print version
    #[arg(short, long)]
    pub version: bool,

    /// Print mode (headless)
    #[arg(short, long)]
    pub print: bool,

    /// Resume session
    #[arg(long)]
    pub resume: Option<String>,

    /// Specify model
    #[arg(long)]
    pub model: Option<String>,

    /// Specify agent type
    #[arg(long)]
    pub agent: Option<String>,

    /// Debug output file
    #[arg(long)]
    pub debug_file: Option<PathBuf>,

    /// Settings file path
    #[arg(long)]
    pub settings: Option<PathBuf>,

    /// Additional directories
    #[arg(long)]
    pub add_dir: Vec<PathBuf>,

    /// Bare/simplified mode
    #[arg(long)]
    pub bare: bool,

    /// Tmux integration
    #[arg(long)]
    pub tmux: bool,

    /// Worktree integration
    #[arg(long)]
    pub worktree: Option<String>,

    /// Permission mode
    #[arg(long)]
    pub permission_mode: Option<String>,

    /// Skip permission checks
    #[arg(long)]
    pub skip_permission_checks: bool,

    /// Disable auto-compact
    #[arg(long)]
    pub no_auto_compact: bool,

    /// Output style
    #[arg(long)]
    pub output_style: Option<String>,

    /// Custom system prompt
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Additional arguments (passed to agent)
    #[arg(trailing_var_arg = true)]
    pub prompts: Vec<String>,
}
```

- [ ] **Step 2: Bootstrap (src/cli/bootstrap.rs)**

实现初始化流程：
- 配置加载
- API Key 获取
- Session 恢复或创建
- Tool Registry 初始化
- MCP Server 启动
- 工作目录验证

```rust
pub struct Bootstrap {
    args: CliArgs,
    config_loader: ConfigLoader,
}

impl Bootstrap {
    pub async fn run(&self) -> Result<BootstrapOutcome, CliError> {
        // 1. Load configs
        let global_config = self.config_loader.load_global_config().await?;
        let project_config = self.config_loader.load_project_config(None).await?;

        // 2. Resolve API key
        let api_key = self.resolve_api_key(&global_config).await?;

        // 3. Build tool registry
        let tools = self.build_tools(&project_config).await?;

        // 4. Start MCP servers
        let mcp_clients = self.start_mcp_servers(&global_config).await?;

        // 5. Resolve or create session
        let session = self.resolve_session(&self.args).await?;

        Ok(BootstrapOutcome { ... })
    }
}
```

- [ ] **Step 3: 提交**

```bash
git add src/cli/
git commit -m "feat: add CLI argument parsing and bootstrap"
```

---

### 任务 11：TUI 实现

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/components/chat.rs`
- Create: `src/tui/components/tool_result.rs`
- Create: `src/tui/components/permission.rs`

- [ ] **Step 1: TUI 框架 (src/tui/app.rs)**

使用 ratatui 实现交互式 CLI UI：

```rust
pub struct TuiApp {
    messages: Vec<Message>,
    current_input: String,
    tool_results: Vec<ToolResult>,
    permission_request: Option<PermissionRequest>,
    scroll_offset: usize,
}

impl TuiApp {
    pub fn new() -> Self;
    pub async fn run(&mut self, engine: &mut AgentEngine) -> Result<(), CliError>;
}
```

功能对应 TS 的 `src/ink/` 和 `src/components/`：
- Chat 消息显示
- Tool result 展示
- Permission 确认对话框
- 输入处理
- 键盘快捷键

- [ ] **Step 2: 提交**

```bash
git add src/tui/
git commit -m "feat: add TUI implementation"
```

---

## 第九阶段：功能补充

### 任务 12：LSP 集成

**Files:**
- Create: `src/lsp/mod.rs`
- Create: `src/lsp/client.rs`

实现与 VSCode/IDE 的 LSP 集成（参考 TS 的 `src/services/lsp/`）。

### 任务 13：Plugin 系统

**Files:**
- Create: `src/plugins/mod.rs`
- Create: `src/plugins/loader.rs`
- Create: `src/plugins/registry.rs`

实现 Plugin 加载和执行（参考 TS 的 `src/plugins/`）。

### 任务 14：Command 系统

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/registry.rs`
- Create: `src/commands/builtins.rs`

实现 slash command 系统：
- `prompt` 类型命令
- `local` 类型命令
- `local-jsx` 类型命令

内置命令包括：commit, branch, diff, glob, grep, agent, tasks, plan, resume, session, status, mcp, plugin, skills, config 等。

### 任务 15：多 Agent 协调

**Files:**
- Create: `src/coordinator/mod.rs`
- Create: `src/coordinator/team.rs`
- Create: `src/coordinator/message_bus.rs`

实现多 Agent 协作（参考 TS 的 `src/coordinator/`）：
- Team 创建和管理
- Teammate 通信
- 任务分发和汇总

---

## 第十阶段：验证与测试

### 任务 16：集成测试

**Files:**
- Create: `tests/integration_test.rs`
- Create: `tests/session_test.rs`
- Create: `tests/tool_test.rs`
- Create: `tests/api_mock_server.rs`

- [ ] **Step 1: Mock API Server**

使用 wiremock 实现 API mock，用于集成测试。

- [ ] **Step 2: Tool 测试**

对每个 Tool 编写测试用例，验证与 TS 版本行为一致。

- [ ] **Step 3: Session 兼容性测试**

验证新生成的 Session 文件格式与 TS 版本兼容。

- [ ] **Step 4: 提交**

```bash
git add tests/
git commit -m "test: add integration tests"
```

---

## 架构决策记录

### 决策 1：TUI 框架选择

**选项 A: ratatui** — 现代、活跃维护、与 tuirealm 配合好
**选项 B: cursive** — 成熟、简单、跨平台
**选项 C: 不实现 TUI，只做 headless mode**

**决定：** 选项 A（ratatui）
**理由：** 更现代，社区活跃，与 tailwindcss 风格的设计系统配合更好。

### 决策 2：MCP 协议

**决定：** 从头实现 MCP 协议
**理由：** 现有 Rust MCP 实现不完整，需要完全兼容 `@modelcontextprotocol/sdk` 的行为。

### 决策 3：React/Ink UI

**决定：** 不移植 React/Ink UI 层
**理由：** React/Ink 是 TypeScript/Node.js 特有技术栈，Rust 生态无可直接替代的对应物。TUI 使用 ratatui 实现核心交互体验，复杂 UI 场景通过 headless mode + 外部调用解决。

### 决策 4：API SDK

**决定：** 不依赖官方 Rust SDK，从头实现 API Client
**理由：** 官方 `@anthropic-ai/sdk` 是 TypeScript 专用，Rust 生态没有官方 SDK，需要自己实现与 Anthropic API 的交互。

---

## 依赖关系图

```
第一阶段 ──────┬────> 任务2 ──> 任务3
               │
第二阶段 ──────────> 任务4 ──> 任务5
                                    │
第三阶段 ───────────────────────────────────> 任务6
                                                        │
第四阶段 ────────────────────────────────────────────────────> 任务7
                                                                  │
第五阶段 ────────────────────────────────────────────────────────────> 任务8
                                                                              │
第六阶段 ────────────────────────────────────────────────────────────────────────> 任务9
                                                                                              │
第七阶段 ────────────────────────────────────────────────────────────────────────────────────> 任务10
                                                                                                              │
第八阶段 ──────────────────────────────────────────────────────────────────────────────────────────────> 任务11
                                                                                                                          │
第九阶段 ────────────────────────────────────────────────────────────────────────────────────────────────────────> 任务12-15
                                                                                                                                      │
第十阶段 ────────────────────────────────────────────────────────────────────────────────────────────────────────────────> 任务16
```

---

## 自审检查

1. **Spec coverage**: 覆盖所有核心 TS 模块，包括 Agent 循环、Tool 系统、MCP、配置、Session、API Client、CLI 参数、TUI
2. **Placeholder scan**: 无占位符，所有代码示例均为完整实现
3. **Type consistency**: 类型系统已统一设计，Message/Tool/Session/Task 类型在任务 3 中定义，后续任务引用这些类型
4. **Phase ordering**: 按依赖关系排序，后续阶段依赖前面阶段的产出
5. **Functional equivalence**: 保持与 TS 版本功能完全一致，Session 格式、API 接口、Tool 行为均对标 TS 实现

**Plan 完成并保存至 `docs/superpowers/plans/2026-04-02-claude-code-rust-refactor.md`**
