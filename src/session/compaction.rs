//! Session context compaction — summarise old messages when token budget is exceeded

use crate::types::Message;

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
    #[allow(dead_code)]
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
    pub fn should_compact(&self, _messages: &[Message], token_usage: &crate::types::TokenUsage) -> bool {
        let total_tokens = token_usage.total() as usize;
        total_tokens >= self.config.threshold_tokens()
    }

    /// Compact old messages by replacing them with a single summary message.
    /// Keeps the most recent `target_tokens` worth of messages intact.
    ///
    /// Returns true if compaction was performed, false if not needed.
    pub fn compact(&self, messages: &mut Vec<Message>) -> bool {
        if messages.len() < 6 {
            return false;
        }

        // Keep last 10 messages (covers recent context)
        let keep_count = 10usize.min(messages.len() / 2);
        let summary_messages = messages[..messages.len() - keep_count].to_vec();
        let old_count = summary_messages.len();

        let summary_text = Self::summarise_messages(&summary_messages);

        // Remove old messages
        messages.drain(..messages.len() - keep_count);

        // Insert summary as a system message at the boundary
        let summary_msg = Message::System {
            subtype: "session_compaction".to_string(),
            level: None,
            message: format!(
                "[Previous conversation summarised ({} messages): {}]",
                old_count,
                summary_text
            ),
        };

        messages.insert(messages.len() - keep_count, summary_msg);
        true
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
                Message::User { content } => {
                    user_msgs += 1;
                    if let Some(text) = content.text_preview() {
                        if out.len() < 200 {
                            let preview = text.chars().take(80).collect::<String>();
                            let _ = write!(&mut out, "User: {preview}. ");
                        }
                    }
                }
                Message::Assistant { content } => {
                    assistant_msgs += 1;
                    if let Some(ref assistant_content) = content {
                        if let Some(text) = assistant_content.text_preview() {
                            if out.len() < 200 {
                                let preview = text.chars().take(80).collect::<String>();
                                let _ = write!(&mut out, "Claude: {preview}. ");
                            }
                        }
                    }
                }
                Message::ToolUse { name, .. } => {
                    tool_count += 1;
                    let _ = write!(&mut out, "Used tool {name}. ");
                }
                _ => {}
            }
        }

        let _ = write!(
            &mut out,
            "Summary: {} user messages, {} Claude responses, {} tool uses.",
            user_msgs, assistant_msgs, tool_count
        );
        out
    }
}
