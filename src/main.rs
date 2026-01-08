mod app;
mod config;
mod error;
mod event;
mod logging;
mod model;
mod service;
mod tui;
mod ui;
mod util;

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;

use app::App;
use error::AppResult;
use tui::Tui;

/// A TUI for managing Nix flake inputs
#[derive(Parser, Debug)]
#[command(name = "melt", version, about, long_about = None)]
struct Args {
    /// Path to flake directory or flake.nix file
    #[arg(default_value = ".")]
    flake: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    tui::install_panic_hook();

    // Initialize logging (ok to fail silently - app works without it)
    let _ = logging::init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> AppResult<()> {
    let args = Args::parse();
    let mut tui = Tui::new()?;
    let mut app = App::new(args.flake);
    app.run(&mut tui).await
}
