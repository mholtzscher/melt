mod app;
mod error;
mod event;
mod model;
mod service;
mod tui;
mod ui;
mod util;

use std::path::PathBuf;

use clap::Parser;

use crate::{app::App, error::AppResult, tui::Tui};

/// A TUI for managing Nix flake inputs
#[derive(Parser, Debug)]
#[command(name = "melt", version, about, long_about = None)]
struct Args {
    /// Path to flake directory or flake.nix file
    #[arg(default_value = ".")]
    flake: PathBuf,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Install panic hook to restore terminal on panic
    tui::install_panic_hook();

    // Initialize terminal
    let mut tui = match Tui::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialize terminal: {}", e);
            return Err(e);
        }
    };

    // Create and run app
    let mut app = App::new(args.flake);
    if let Err(e) = app.run(&mut tui).await {
        // Drop tui first to restore terminal
        drop(tui);
        eprintln!("Application error: {}", e);
        return Err(e);
    }

    Ok(())
}
