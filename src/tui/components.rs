//! TUI rendering components

#![allow(dead_code)]

use crate::tui::app::TuiApp;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the main TUI layout
pub fn render(frame: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),      // Messages
            Constraint::Length(1),  // Status bar
            Constraint::Length(6),   // Input area
        ])
        .split(frame.size());

    render_header(frame, app, chunks[0]);
    render_messages(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
    render_input(frame, app, chunks[3]);

    if let Some(ref prompt) = app.permission_prompt {
        render_permission_prompt(frame, prompt, chunks[1]);
    }
}

/// Render the header bar
fn render_header(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let title = if app.is_running {
        "Claude Code (running...)"
    } else {
        "Claude Code"
    };

    let widget = Paragraph::new(Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  |  tokens: "),
        Span::raw(format!("{}/{}", app.input_tokens, app.output_tokens)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(widget, area);
}

/// Render the message history
fn render_messages(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let items: Vec<ListItem> = app
        .messages
        .iter()
        .map(|msg| {
            let content = format_message(msg);
            ListItem::new(Line::from(content))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Messages")
                .style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

fn format_message(msg: &crate::types::Message) -> Vec<Span<'static>> {
    use crate::types::Message;
    match msg {
        Message::User { content } => {
            let text = format!("{:?}", content);
            vec![Span::styled(
                format!("[You] {text}"),
                Style::default().fg(Color::Green),
            )]
        }
        Message::Assistant { content } => {
            let text = content.as_ref().map(|c| format!("{:?}", c)).unwrap_or_default();
            vec![Span::styled(
                format!("[Claude] {text}"),
                Style::default().fg(Color::Blue),
            )]
        }
        Message::ToolUse { name, .. } => {
            vec![Span::styled(
                format!("[Tool: {name}]"),
                Style::default().fg(Color::Yellow),
            )]
        }
        Message::ToolResult { content, is_error, .. } => {
            let color = if *is_error { Color::Red } else { Color::DarkGray };
            let preview = if content.len() > 100 {
                format!("{}...", &content[..100])
            } else {
                content.clone()
            };
            vec![Span::styled(
                format!("[Result] {preview}"),
                Style::default().fg(color),
            )]
        }
        Message::System { message, .. } => {
            vec![Span::styled(
                format!("[System] {message}"),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )]
        }
        _ => vec![Span::raw(format!("{:?}", msg))],
    }
}

/// Render the status bar
fn render_status(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let color = if app.error.is_some() {
        Color::Red
    } else if app.is_running {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let status_text = if let Some(ref err) = app.error {
        format!("Error: {err}")
    } else {
        app.status.clone()
    };

    let widget = Paragraph::new(Line::from(vec![Span::styled(
        status_text,
        Style::default().fg(color),
    )]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(widget, area);
}

/// Render the input area
fn render_input(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let border_color = if app.is_running {
        Color::DarkGray
    } else {
        Color::Cyan
    };

    let input_display = if app.input.is_empty() {
        "Type your message..."
    } else {
        &app.input
    };

    let input_style = if app.input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let widget = Paragraph::new(Line::from(vec![Span::styled(input_display, input_style)]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(if app.is_running {
                    "Input (disabled)"
                } else {
                    "Input"
                })
                .style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(widget, area);
}

/// Render a permission prompt overlay
fn render_permission_prompt(frame: &mut Frame, prompt: &crate::tui::app::PermissionPrompt, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Permission: {}", prompt.tool_name))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = Paragraph::new(vec![
        Line::from(vec![Span::raw("Command: "), Span::raw(&prompt.command)]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw(&prompt.details)]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw("Allow? [y/n/always/never]")]),
    ])
    .style(Style::default().fg(Color::White));

    frame.render_widget(text, inner);
}
