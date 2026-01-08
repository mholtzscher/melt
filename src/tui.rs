use std::io::{self, Stdout};

use crossterm::{
    cursor, execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::warn;

use crate::error::AppResult;

/// Terminal wrapper that handles setup and teardown with RAII
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> AppResult<Self> {
        let terminal = Self::setup()?;
        Ok(Self { terminal })
    }

    fn setup() -> AppResult<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            Clear(ClearType::All),
            cursor::Hide
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(terminal)
    }

    fn restore() -> AppResult<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), cursor::Show, LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw<F>(&mut self, f: F) -> AppResult<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Err(e) = Self::restore() {
            eprintln!("Failed to restore terminal: {}", e);
            warn!(error = %e, "Failed to restore terminal");
        }
    }
}

pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
