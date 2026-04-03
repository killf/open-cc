//! Output rendering for TUI

#![allow(dead_code)]

use crate::types::Message;

/// Print a message to the terminal (non-interactive)
pub fn print_message(msg: &Message) {
    match msg {
        Message::User { content } => {
            println!("\n[You]");
            println!("{:?}", content);
        }
        Message::Assistant { content } => {
            println!("\n[Claude]");
            if let Some(c) = content {
                for block in &c.content {
                    if let crate::types::ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
        }
        Message::ToolUse { name, .. } => {
            println!("\n[Using tool: {name}]");
        }
        Message::ToolResult { content, is_error, .. } => {
            if *is_error {
                eprintln!("\n[Tool Error]");
            } else {
                println!("\n[Tool Result]");
            }
            let preview = if content.len() > 500 {
                format!("{}...\n[truncated]", &content[..500])
            } else {
                content.clone()
            };
            println!("{preview}");
        }
        Message::System { message, .. } => {
            println!("\n[System] {message}");
        }
        _ => {}
    }
}

/// Clear the terminal
pub fn clear_screen() {
    print!("\x1B[2J\x1B[H");
}

/// Print a spinner/animation frame
pub fn print_spinner(frame: usize) {
    let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    print!("\x1B[2K"); // Clear line
    print!("\r{} ", spinners[frame % spinners.len()]);
}

/// Print a progress update
pub fn print_progress(current: usize, total: usize, label: &str) {
    let pct = if total > 0 {
        (current as f64 / total as f64 * 100.0) as usize
    } else {
        0
    };
    print!("\x1B[2K"); // Clear line
    println!("\r{label}: {current}/{total} ({pct}%)");
}
