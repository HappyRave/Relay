//! Diagnostics: a daily rolling log in `%APPDATA%\Relay\logs` (last 7 days)
//! and a panic hook that records crashes there. Logs say what happened
//! (sessions, triggers, errors), never what was typed.

use parking_lot::Mutex;
use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};

/// Keeps the log writer alive (dropping it flushes and stops logging).
pub struct LogGuard(#[allow(dead_code)] Mutex<Option<WorkerGuard>>);

pub fn init(data_dir: &Path) -> LogGuard {
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("relay")
        .filename_suffix("log")
        .max_log_files(7)
        .build(data_dir.join("logs"));
    let Ok(appender) = appender else { return LogGuard(Mutex::new(None)) };
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let _ = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        tracing::error!(%thread, "panic: {info}");
        default_hook(info);
    }));
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Relay started");
    LogGuard(Mutex::new(Some(guard)))
}
