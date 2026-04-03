//! Input handling for TUI

#![allow(dead_code)]

use crate::tui::app::TuiApp;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event, returns true if the event was consumed
pub fn handle_key_event(event: Event, app: &mut TuiApp) -> bool {
    match event {
        Event::Key(KeyEvent { code, modifiers, .. }) => {
            handle_key(code, modifiers, app)
        }
        Event::Paste(text) => {
            app.input.push_str(&text);
            true
        }
        _ => false,
    }
}

fn handle_key(code: KeyCode, modifiers: KeyModifiers, app: &mut TuiApp) -> bool {
    if app.is_running {
        return false;
    }

    // Handle permission prompt
    if app.permission_prompt.is_some() {
        return handle_permission_key(code, app);
    }

    match code {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'c' => {
                        app.input.clear();
                        true
                    }
                    'u' => {
                        app.input.clear();
                        true
                    }
                    'w' => {
                        if let Some(pos) = app.input.rfind(' ') {
                            app.input.truncate(pos);
                        } else {
                            app.input.clear();
                        }
                        true
                    }
                    _ => false,
                }
            } else {
                app.input.push(c);
                true
            }
        }
        KeyCode::Backspace => {
            app.input.pop();
            true
        }
        KeyCode::Delete => false,
        KeyCode::Enter => !app.input.trim().is_empty(),
        KeyCode::Esc => {
            app.input.clear();
            true
        }
        _ => false,
    }
}

fn handle_permission_key(code: KeyCode, app: &mut TuiApp) -> bool {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.permission_prompt = None;
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.permission_prompt = None;
            true
        }
        _ => true,
    }
}

/// Extract and clear the current input, returning the submitted text
pub fn consume_input(app: &mut TuiApp) -> Option<String> {
    let input = app.input.trim().to_string();
    if input.is_empty() {
        return None;
    }
    app.input.clear();
    Some(input)
}
