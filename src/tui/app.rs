//! TUI application state

#![allow(dead_code)]

use crate::types::Message;
use std::collections::VecDeque;

/// Maximum messages to keep in history
const MAX_HISTORY: usize = 1000;

/// State for the interactive TUI
pub struct TuiApp {
    /// Chat history
    pub messages: VecDeque<Message>,
    /// Current input buffer
    pub input: String,
    /// Whether agent is currently running
    pub is_running: bool,
    /// Token usage for current session
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Current status message
    pub status: String,
    /// Error message (if any)
    pub error: Option<String>,
    /// Show permission prompt
    pub permission_prompt: Option<PermissionPrompt>,
    /// Scroll position in history
    pub scroll_offset: usize,
}

/// Permission prompt state
pub struct PermissionPrompt {
    pub tool_name: String,
    pub command: String,
    pub details: String,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            input: String::new(),
            is_running: false,
            input_tokens: 0,
            output_tokens: 0,
            status: "Ready".to_string(),
            error: None,
            permission_prompt: None,
            scroll_offset: 0,
        }
    }

    pub fn add_message(&mut self, msg: Message) {
        if self.messages.len() >= MAX_HISTORY {
            self.messages.pop_front();
        }
        self.messages.push_back(msg);
    }

    pub fn set_running(&mut self, running: bool) {
        self.is_running = running;
        if running {
            self.status = "Thinking...".to_string();
        } else {
            self.status = "Ready".to_string();
        }
    }

    pub fn set_error(&mut self, msg: Option<String>) {
        self.error = msg;
        if self.error.is_some() {
            self.status = "Error".to_string();
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
