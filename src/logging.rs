//! Logging configuration for the application.
//!
//! Logs are written to a file since the TUI controls the terminal.
//! Log level can be controlled via the `RUST_LOG` environment variable.
//!
//! Log file locations (platform-dependent):
//! - macOS: `~/Library/Application Support/melt/melt.log`
//! - Linux: `~/.local/share/melt/melt.log`
//! - Windows: `C:\Users\<user>\AppData\Local\melt\melt.log`

use std::{fs, io, path::PathBuf};

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Returns the path to the log file.
///
/// Creates the parent directory if it doesn't exist.
pub fn log_file_path() -> Option<PathBuf> {
    let data_dir = dirs::data_local_dir()?.join("melt");
    fs::create_dir_all(&data_dir).ok()?;
    Some(data_dir.join("melt.log"))
}

/// Initialize the logging system.
///
/// Logs are written to `~/.local/share/melt/melt.log`.
/// The log level defaults to `info` but can be overridden with `RUST_LOG`.
///
/// Returns `Ok(())` if logging was initialized, `Err` if it failed
/// (e.g., couldn't create log file). The app can continue without logging.
pub fn init() -> io::Result<()> {
    let log_path = log_file_path().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not determine log directory")
    })?;

    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let file_layer = fmt::layer()
        .with_writer(log_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized, writing to {:?}", log_path);

    Ok(())
}
