use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

/// Poll for a key event with timeout
/// Returns Some(KeyEvent) if a key was pressed, None on timeout or other events
pub fn poll_key(timeout: Duration) -> Option<KeyEvent> {
    if event::poll(timeout).ok()? {
        if let Event::Key(key) = event::read().ok()? {
            // Ignore key release events on some terminals
            if key.kind == crossterm::event::KeyEventKind::Press {
                return Some(key);
            }
        }
    }
    None
}

/// Key event helper methods
pub trait KeyEventExt {
    fn is_quit(&self) -> bool;
}

impl KeyEventExt for KeyEvent {
    fn is_quit(&self) -> bool {
        matches!(
            (self.code, self.modifiers),
            (KeyCode::Char('q'), KeyModifiers::NONE)
                | (KeyCode::Esc, _)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL)
        )
    }
}
