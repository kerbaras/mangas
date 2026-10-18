//! Log setup shared by the desktop app and the CLI. The desktop writes a daily-rolling file
//! under the data dir; the CLI writes to stderr. `log` records from dependencies (and from our
//! own `log::` calls) are bridged into `tracing`, so both end up in the same sink.

use std::ffi::OsStr;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::{Error, Result};

/// Overrides the level, in `EnvFilter` syntax (`debug`, `mangas_providers=trace,warn`).
pub const ENV_VAR: &str = "MANGAS_LOG";
/// Folder under the data dir that holds the log files.
pub const LOG_DIR: &str = "logs";
/// Files are named `mangas.YYYY-MM-DD.log`, one per day.
pub const FILE_PREFIX: &str = "mangas";
pub const FILE_SUFFIX: &str = "log";
/// Daily files kept; older ones are deleted when a new day starts.
pub const KEEP_DAYS: usize = 5;

/// Our crates; everything else (reqwest, tauri, …) only logs warnings unless `MANGAS_LOG` says so.
const OWN_CRATES: [&str; 5] = [
    "mangas",
    "mangas_core",
    "mangas_providers",
    "mangas_cli",
    "mangas_desktop_lib",
];

pub fn log_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(LOG_DIR)
}

/// The log folder under `data_dir`, created if logging never managed to.
pub fn ensure_log_dir(data_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = log_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Default directives: our crates at `level`, dependencies at `warn`.
pub fn default_directives(level: &str) -> String {
    std::iter::once("warn".to_string())
        .chain(OWN_CRATES.iter().map(|c| format!("{c}={level}")))
        .collect::<Vec<_>>()
        .join(",")
}

/// The filter from `env` (the `MANGAS_LOG` value) when it is set and valid, else the defaults.
pub fn filter(env: Option<&str>, level: &str) -> EnvFilter {
    env.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|d| EnvFilter::try_new(d).ok())
        .unwrap_or_else(|| EnvFilter::new(default_directives(level)))
}

/// `-v` count to level: warnings by default, `-v` info, `-vv` debug, more is trace.
pub fn cli_level(verbosity: u8) -> &'static str {
    match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    }
}

/// Colour codes only for a terminal: redirected stderr (`2> log.txt`) gets plain text, and a
/// non-empty `NO_COLOR` (https://no-color.org) turns them off everywhere.
pub fn use_ansi(is_terminal: bool, no_color: Option<&OsStr>) -> bool {
    is_terminal && no_color.is_none_or(OsStr::is_empty)
}

fn stderr_ansi() -> bool {
    use_ansi(
        std::io::stderr().is_terminal(),
        std::env::var_os("NO_COLOR").as_deref(),
    )
}

/// Daily-rolling appender in `dir` keeping [`KEEP_DAYS`] files; creates `dir` if needed.
pub fn file_appender(dir: &Path) -> Result<RollingFileAppender> {
    std::fs::create_dir_all(dir)?;
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(FILE_PREFIX)
        .filename_suffix(FILE_SUFFIX)
        .max_log_files(KEEP_DAYS)
        .build(dir)
        .map_err(|e| Error::Other(format!("log file in {}: {e}", dir.display())))
}

/// Desktop logging: the rolling file in `dir` plus stderr (visible under `tauri dev`). Writes
/// are synchronous so nothing is lost when the app aborts on a panic, which is logged too.
pub fn init_file(dir: &Path) -> Result<()> {
    let file = file_appender(dir)?;
    let env = std::env::var(ENV_VAR).ok();
    tracing_subscriber::registry()
        .with(filter(env.as_deref(), "info"))
        .with(fmt::layer().with_writer(file).with_ansi(false))
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(stderr_ansi()),
        )
        .try_init()
        .map_err(|e| Error::Other(format!("logging: {e}")))?;
    log_panics();
    Ok(())
}

/// CLI logging to stderr; `verbosity` is the number of `-v` flags.
pub fn init_stderr(verbosity: u8) -> Result<()> {
    let env = std::env::var(ENV_VAR).ok();
    tracing_subscriber::registry()
        .with(filter(env.as_deref(), cli_level(verbosity)))
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(stderr_ansi())
                .without_time()
                .with_target(verbosity > 1),
        )
        .try_init()
        .map_err(|e| Error::Other(format!("logging: {e}")))
}

fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "mangas", "panic: {info}");
        previous(info);
    }));
}
