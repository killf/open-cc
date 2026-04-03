//! Agent engine - core loop

use crate::api::ApiClient;
use crate::error::CliError;
use crate::session::{CompactionConfig, SessionCompactor};
use crate::types::{
    ContentBlock, Message, PermissionDecision, ToolContext, ToolResult,
    UserContent,
};

use super::context::AgentContext;
use super::hooks::{Hook, HookType};
use super::permission::PermissionChecker;

/// Result of agent execution
#[derive(Debug)]
pub enum AgentOutcome {
    Completed,
    Error(String),
    Interrupted,
}

/// Core agent engine
pub struct AgentEngine {
    api_client: ApiClient,
    context: AgentContext,
    permission_checker: PermissionChecker,
    compactor: SessionCompactor,
}

impl AgentEngine {
    pub fn new(
        api_client: ApiClient,
        context: AgentContext,
    ) -> Self {
        let permission_checker = PermissionChecker::new(context.permission_mode());
        let compactor = SessionCompactor::new(CompactionConfig::default());
        Self {
            api_client,
            context,
            permission_checker,
            compactor,
        }
    }

    /// Run the agent with an initial prompt
    pub async fn run(&mut self, initial_prompt: String) -> Result<AgentOutcome, CliError> {
        // Add user message
        self.context.session.add_message(Message::User {
            content: UserContent::text(initial_prompt),
        });

        loop {
            // Build messages for API
            let model = self.context.model().to_string();
            let max_tokens = self.context.config.max_tokens.unwrap_or(8192);

            // Send to API
            let tools = if self.context.tools.is_empty() {
                None
            } else {
                Some(self.context.tools.as_slice())
            };
            let response = self.api_client.chat(&self.context.session, &model, max_tokens, tools).await?;

            // Track token usage
            self.context.session.token_usage.add(&response.usage);

            // Check if we need to compact context
            if self.compactor.should_compact(&self.context.session.messages, &self.context.session.token_usage) {
                eprintln!("[Info] Compacting conversation to stay within token limit...");
                self.compactor.compact(&mut self.context.session.messages);
            }

            // Track cost
            let cost = self.api_client.estimate_cost(&response.usage, &model);
            self.context.session.cost += cost;

            // Add assistant response
            self.context.session.add_message(Message::Assistant {
                content: Some(response.content.clone()),
            });

            // Process content blocks
            let mut tool_results = Vec::new();

            for block in &response.content.content {
                match block {
                    ContentBlock::Text { text } => {
                        if !text.is_empty() {
                            println!("{}", text);
                        }
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        // Find the tool
                        let tool = match self.context.find_tool(name) {
                            Some(t) => t,
                            None => {
                                let result = ToolResult::error(format!("Unknown tool: {}", name));
                                tool_results.push((id.clone(), result));
                                continue;
                            }
                        };

                        // Check permissions
                        let decision = self.permission_checker
                            .check_tool(tool.as_ref(), input, "")
                            .await;

                        match decision {
                            PermissionDecision::Allow => {
                                // Execute tool
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
                                // In interactive mode, would ask user
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

            // Add tool results to session
            for (tool_use_id, result) in tool_results {
                self.context.session.add_message(Message::ToolResult {
                    tool_use_id,
                    content: result.content.iter()
                        .map(|b| b.preview())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    is_error: result.is_error,
                });
            }

            // Check stop reason
            if response.stop_reason.as_deref() == Some("end_turn") {
                return Ok(AgentOutcome::Completed);
            }

            if response.stop_reason.as_deref() == Some("max_tokens") {
                // Continue with another turn
                continue;
            }

            if response.stop_reason.is_none() && response.content.content.is_empty() {
                return Ok(AgentOutcome::Completed);
            }
        }
    }

    /// Run in streaming mode
    pub async fn run_streaming(
        &mut self,
        initial_prompt: String,
    ) -> Result<AgentOutcome, CliError> {
        self.context.session.add_message(Message::User {
            content: UserContent::text(initial_prompt),
        });

        let model = self.context.model().to_string();
        let max_tokens = self.context.config.max_tokens.unwrap_or(8192);

        let tools = if self.context.tools.is_empty() {
            None
        } else {
            Some(self.context.tools.as_slice())
        };

        let mut collected_text = String::new();

        self.api_client
            .chat_streaming(&self.context.session, &model, max_tokens, tools, |text| {
                print!("{}", text);
                collected_text.push_str(&text);
            })
            .await?;

        println!();

        // Parse collected text into content blocks
        let content = ContentBlock::Text { text: collected_text };

        self.context.session.add_message(Message::Assistant {
            content: Some(crate::types::AssistantContent {
                content: vec![content],
                model: model.clone(),
                stop_reason: None,
            }),
        });

        Ok(AgentOutcome::Completed)
    }

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

            self.context.session.token_usage.add(&response.usage);
            let cost = self.api_client.estimate_cost(&response.usage, &model);
            self.context.session.cost += cost;

            if self.compactor.should_compact(&self.context.session.messages, &self.context.session.token_usage) {
                eprintln!("[Info] Compacting conversation to stay within token limit...");
                self.compactor.compact(&mut self.context.session.messages);
            }

            self.context.session.add_message(Message::Assistant {
                content: Some(response.content.clone()),
            });

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

                                for hook in self.context.hooks_of_type(HookType::PreToolUse) {
                                    let payload = Hook::pre_tool_payload(tool.name(), input, &self.context.session.id);
                                    if let Err(e) = hook.run(&payload).await {
                                        eprintln!("[Hook warning] {}: {e}", hook.name);
                                    }
                                }

                                let result = tool.call(input.clone(), tool_ctx).await
                                    .unwrap_or_else(|e| ToolResult::error(e.to_string()));

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

            for (tool_use_id, result) in tool_results {
                self.context.session.add_message(Message::ToolResult {
                    tool_use_id,
                    content: result.content.iter().map(|b| b.preview()).collect::<Vec<_>>().join("\n"),
                    is_error: result.is_error,
                });
            }

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

    /// Get session reference
    pub fn session(&self) -> &crate::types::Session {
        &self.context.session
    }

    /// Get session for persistence
    pub fn session_mut(&mut self) -> &mut crate::types::Session {
        &mut self.context.session
    }
}
