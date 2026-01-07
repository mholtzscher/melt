use std::io::{self, Stdout};

use crossterm::{
    cursor,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::error::AppResult;

/// Terminal wrapper that handles setup and teardown with RAII
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Create a new terminal instance and enter TUI mode
    pub fn new() -> AppResult<Self> {
        let terminal = Self::setup()?;
        Ok(Self { terminal })
    }

    /// Set up the terminal for TUI rendering
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

    /// Restore the terminal to its original state
    fn restore() -> AppResult<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), cursor::Show, LeaveAlternateScreen)?;
        Ok(())
    }

    /// Get mutable access to the underlying terminal
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Draw a frame using the provided closure
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
        }
    }
}

/// Install a panic hook that restores the terminal before printing the panic
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before printing panic
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
