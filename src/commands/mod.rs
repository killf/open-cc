//! Command system for slash commands
//! Re-exports CommandDefinition from library/config/project.rs

#![allow(dead_code)]

pub use crate::config::project::CommandDefinition;

use crate::error::CliError;
use std::collections::HashMap;
use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Built-in commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCommand {
    Help,
    Exit,
    Clear,
    History,
    Session,
    Model,
    TokenCount,
    Cost,
    Reset,
    Config,
}

impl BuiltinCommand {
    pub fn try_parse(s: &str) -> Option<Self> {
        match s {
            "/help" | "/h" => Some(Self::Help),
            "/exit" | "/quit" | "/q" => Some(Self::Exit),
            "/clear" | "/reset" => Some(Self::Clear),
            "/history" | "/hist" => Some(Self::History),
            "/session" | "/sess" => Some(Self::Session),
            "/model" | "/m" => Some(Self::Model),
            "/tokens" | "/tc" => Some(Self::TokenCount),
            "/cost" => Some(Self::Cost),
            "/config" | "/cfg" => Some(Self::Config),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Help => "/help",
            Self::Exit => "/exit",
            Self::Clear => "/clear",
            Self::History => "/history",
            Self::Session => "/session",
            Self::Model => "/model",
            Self::TokenCount => "/tokens",
            Self::Cost => "/cost",
            Self::Reset => "/reset",
            Self::Config => "/config",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Help => "Show this help message",
            Self::Exit => "Exit Claude Code",
            Self::Clear => "Clear the conversation",
            Self::History => "Show message history",
            Self::Session => "Manage sessions",
            Self::Model => "Show or change the model",
            Self::TokenCount => "Count tokens in text",
            Self::Cost => "Show estimated cost",
            Self::Reset => "Reset the conversation",
            Self::Config => "Open configuration",
        }
    }
}

/// Command registry
pub struct CommandRegistry {
    builtins: HashMap<String, BuiltinCommand>,
    custom: HashMap<String, CommandDefinition>,
    /// Optional session state for builtins that need live session data
    session_state: Arc<Mutex<Option<Box<dyn SessionState>>>>,
}

/// State needed by builtin commands
pub trait SessionState: Send + Sync {
    fn session_id(&self) -> String;
    fn model(&self) -> String;
    fn cost(&self) -> f64;
    fn input_tokens(&self) -> u64;
    fn output_tokens(&self) -> u64;
    fn total_tokens(&self) -> u64;
    fn message_count(&self) -> usize;
    fn provider(&self) -> String;
    fn print_help(&self);
    fn print_exit(&self);
    fn print_clear(&self);
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut builtins = HashMap::new();
        for cmd in [
            BuiltinCommand::Help,
            BuiltinCommand::Exit,
            BuiltinCommand::Clear,
            BuiltinCommand::History,
            BuiltinCommand::Session,
            BuiltinCommand::Model,
            BuiltinCommand::TokenCount,
            BuiltinCommand::Cost,
            BuiltinCommand::Reset,
            BuiltinCommand::Config,
        ] {
            builtins.insert(cmd.name().to_string(), cmd);
        }
        Self {
            builtins,
            custom: HashMap::new(),
            session_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Register a custom command
    pub fn register(&mut self, cmd: CommandDefinition) {
        self.custom.insert(format!("/{}", cmd.name), cmd);
    }

    /// Register custom commands from a config
    pub fn register_all(&mut self, commands: Vec<CommandDefinition>) {
        for cmd in commands {
            self.register(cmd);
        }
    }

    /// Set session state for builtin commands that need live data
    pub async fn set_session_state(&self, state: Box<dyn SessionState>) {
        let mut guard = self.session_state.lock().await;
        *guard = Some(state);
    }

    /// Resolve a command string to a definition
    pub fn resolve(&self, input: &str) -> Option<Command> {
        let trimmed = input.trim();

        // Check builtin commands
        for (name, builtin) in &self.builtins {
            if trimmed == *name || trimmed.starts_with(&format!("{name} ")) {
                let args = trimmed.strip_prefix(&format!("{name} "))
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                return Some(Command::Builtin(*builtin, args));
            }
        }

        // Check custom commands
        for (name, def) in &self.custom {
            if trimmed == *name || trimmed.starts_with(&format!("{name} ")) {
                let args = trimmed.strip_prefix(&format!("{name} "))
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                return Some(Command::Custom(def.clone(), args));
            }
        }

        None
    }

    /// Execute a command
    pub async fn execute(&self, cmd: Command) -> Result<CommandOutput, CliError> {
        match cmd {
            Command::Builtin(builtin, args) => {
                self.execute_builtin(builtin, args).await
            }
            Command::Custom(def, args) => {
                self.execute_custom(&def, args).await
            }
        }
    }

    async fn execute_builtin(
        &self,
        builtin: BuiltinCommand,
        _args: Option<String>,
    ) -> Result<CommandOutput, CliError> {
        let state_guard = self.session_state.lock().await;

        match builtin {
            BuiltinCommand::Help => {
                let mut lines = Vec::new();
                lines.push("Available commands:".to_string());
                for (name, cmd) in &self.builtins {
                    lines.push(format!("  {name:<12} {cmd}", name = name, cmd = cmd.description()));
                }
                for (name, def) in &self.custom {
                    lines.push(format!("  /{name:<11} {desc}", name = name, desc = def.description));
                }
                Ok(CommandOutput::new(true, lines.join("\n"), String::new()))
            }
            BuiltinCommand::Session => {
                if let Some(ref state) = *state_guard {
                    let mut out = String::new();
                    out.push_str(&format!("Session: {}\n", state.session_id()));
                    out.push_str(&format!("Model:   {}\n", state.model()));
                    out.push_str(&format!("Messages: {}\n", state.message_count()));
                    out.push_str(&format!("Cost:     ${:.4}\n", state.cost()));
                    out.push_str(&format!("Tokens:   {} input / {} output\n",
                        state.input_tokens(), state.output_tokens()));
                    Ok(CommandOutput::new(true, out, String::new()))
                } else {
                    Ok(CommandOutput::new(true, "Session: <no active session>".to_string(), String::new()))
                }
            }
            BuiltinCommand::Model => {
                if let Some(ref state) = *state_guard {
                    Ok(CommandOutput::new(true, format!("Model: {}\n", state.model()), String::new()))
                } else {
                    Ok(CommandOutput::new(true, "Model: <unknown>\n".to_string(), String::new()))
                }
            }
            BuiltinCommand::Cost => {
                if let Some(ref state) = *state_guard {
                    Ok(CommandOutput::new(true, format!("Cost: ${:.4}\n", state.cost()), String::new()))
                } else {
                    Ok(CommandOutput::new(true, "Cost: $0.0000\n".to_string(), String::new()))
                }
            }
            BuiltinCommand::TokenCount => {
                if let Some(ref state) = *state_guard {
                    Ok(CommandOutput::new(true,
                        format!("Tokens: {} input / {} output / {} total\n",
                            state.input_tokens(), state.output_tokens(), state.total_tokens()),
                        String::new()))
                } else {
                    Ok(CommandOutput::new(true, "Tokens: 0 input / 0 output / 0 total\n".to_string(), String::new()))
                }
            }
            BuiltinCommand::Config => {
                if let Some(ref state) = *state_guard {
                    Ok(CommandOutput::new(true, format!("Provider: {}\n", state.provider()), String::new()))
                } else {
                    Ok(CommandOutput::new(true, "Provider: <unknown>\n".to_string(), String::new()))
                }
            }
            BuiltinCommand::Exit => {
                if let Some(ref state) = *state_guard {
                    state.print_exit();
                } else {
                    println!("Goodbye!");
                }
                Ok(CommandOutput::exit())
            }
            BuiltinCommand::Clear | BuiltinCommand::Reset => {
                if let Some(ref state) = *state_guard {
                    state.print_clear();
                } else {
                    print!("\x1b[2J\x1b[H");
                    let _ = std::io::stdout().flush();
                }
                Ok(CommandOutput::new(true, String::new(), String::new()))
            }
            BuiltinCommand::History => {
                // For now, just show message count
                if let Some(ref state) = *state_guard {
                    Ok(CommandOutput::new(true,
                        format!("{} messages in session\n", state.message_count()),
                        String::new()))
                } else {
                    Ok(CommandOutput::new(true, "0 messages in session\n".to_string(), String::new()))
                }
            }
        }
    }

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

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref wd) = def.working_directory {
            cmd.current_dir(wd);
        }
        if let Some(ref env_map) = def.env {
            for (k, v) in env_map {
                cmd.env(k, v);
            }
        }

        let timeout = def.timeout_secs.unwrap_or(30);
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            cmd.output(),
        )
        .await
        .map_err(|_| CliError::Other("command timed out".to_string()))?
        .map_err(|e| CliError::Other(format!("command failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput::new(output.status.success(), stdout, stderr))
    }

    /// Get all builtin commands
    pub fn builtins(&self) -> impl Iterator<Item = (&str, &str)> {
        self.builtins.iter().map(|(name, cmd)| (name.as_str(), cmd.description()))
    }

    /// Get all custom commands
    pub fn custom_commands(&self) -> impl Iterator<Item = (&str, &CommandDefinition)> {
        self.custom.iter().map(|(name, def)| (name.as_str(), def))
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Command types
pub enum Command {
    Builtin(BuiltinCommand, Option<String>),
    Custom(CommandDefinition, Option<String>),
}

/// Command execution output
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn new(success: bool, stdout: String, stderr: String) -> Self {
        Self { success, stdout, stderr }
    }

    /// Signal that the REPL should exit
    pub fn exit() -> Self {
        Self { success: false, stdout: String::new(), stderr: String::new() }
    }
}
