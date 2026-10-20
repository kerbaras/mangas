//! `mangas ml`: detector support (the ONNX Runtime library plus the models).

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::Context;
use clap::{Args as ClapArgs, Subcommand};
use mangas_core::ml::{self, RuntimeStatus, Status};
use mangas_core::storage::human_size;

use crate::output::{outln, print_json};
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show whether the ONNX Runtime and the detector models are downloaded (the default)
    Status {
        /// Print the status as JSON (the shape the desktop app shows)
        #[arg(long)]
        json: bool,
        /// Check this models folder instead of the data folder's; the app's detector-status check
        #[arg(long, hide = true)]
        models_dir: Option<PathBuf>,
    },
    /// Download the ONNX Runtime library and the detector models
    Download,
    /// Delete them; the next `--detectors` export downloads them again
    Remove,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let command = args.command.unwrap_or(Command::Status {
        json: false,
        models_dir: None,
    });
    let dir = match &command {
        Command::Status {
            models_dir: Some(dir),
            ..
        } => dir.clone(),
        _ => paths::models()?,
    };
    match command {
        Command::Status { json: true, .. } => print_json(&ml::checked_status(&dir)),
        Command::Status { .. } => print_status(&ml::checked_status(&dir)),
        Command::Download => {
            let mut on_progress = |id: &str, done: u64, total: Option<u64>| {
                let total = total.map(|t| format!(" of {}", human_size(t)));
                eprint!(
                    "\r\x1b[2Kdownloading {id}: {}{}",
                    human_size(done),
                    total.unwrap_or_default()
                );
            };
            let never = AtomicBool::new(false);
            let network = mangas_core::library::get_settings(&mangas_core::library::open(
                &crate::paths::library_db()?,
            )?)?
            .network();
            let status = ml::download(&dir, &network, &mut on_progress, &never).await;
            eprintln!();
            print_status(&status.context("detector support")?)
        }
        Command::Remove => {
            let freed = ml::remove(&dir)?;
            match pending_line(&ml::status(&dir).runtime) {
                Some(line) => Ok(outln!("{line} ({} freed)", human_size(freed))?),
                None => Ok(outln!(
                    "Removed detector support: {} freed",
                    human_size(freed)
                )?),
            }
        }
    }
}

fn print_status(status: &Status) -> anyhow::Result<()> {
    if !status.enabled {
        return Ok(outln!(
            "This build has no detector support (build with --features ml)"
        )?);
    }
    outln!("{:<32} {}", "onnxruntime", runtime_line(&status.runtime))?;
    for model in &status.models {
        let state = if model.downloaded {
            "downloaded"
        } else {
            "not downloaded"
        };
        outln!("{:<32} {state} ({})", model.id, model.licence)?;
    }
    if let Some(line) = pending_line(&status.runtime) {
        outln!("{line}")?;
    }
    let provider = status.execution_provider.as_deref().unwrap_or("cpu");
    outln!("Execution provider: {provider}")?;
    outln!("On disk: {}", human_size(status.disk_bytes))?;
    if status.download_bytes > 0 {
        outln!(
            "Run `mangas ml download` to fetch ~{}",
            human_size(status.download_bytes)
        )?;
    }
    Ok(())
}

pub fn runtime_line(runtime: &RuntimeStatus) -> String {
    match runtime {
        RuntimeStatus::Unavailable => "not available on this platform".into(),
        RuntimeStatus::NotDownloaded { version } => format!("not downloaded (v{version})"),
        RuntimeStatus::Downloaded { version } => format!("downloaded v{version}"),
        RuntimeStatus::Failed { version, reason } => format!("failed (v{version}): {reason}"),
        RuntimeStatus::RemovePending { version, .. } => {
            format!("in use, removal pending (v{version})")
        }
    }
}

/// The partial-remove notice; the desktop shows the same words (`pendingRemoval`).
pub fn pending_line(runtime: &RuntimeStatus) -> Option<String> {
    match runtime {
        RuntimeStatus::RemovePending { bytes, .. } => Some(format!(
            "Detector models removed; the runtime ({}) is still in use — restart mangas and remove again",
            human_size(*bytes)
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_remove_says_the_runtime_is_still_in_use() {
        let pending = RuntimeStatus::RemovePending {
            version: "1.28.3".into(),
            bytes: 23_000_000,
        };
        let line = pending_line(&pending).unwrap_or_default();
        assert!(
            line.starts_with("Detector models removed; the runtime ("),
            "{line}"
        );
        assert!(line.ends_with("is still in use — restart mangas and remove again"));
        assert_eq!(runtime_line(&pending), "in use, removal pending (v1.28.3)");
        let downloaded = RuntimeStatus::Downloaded {
            version: "1.28.3".into(),
        };
        assert_eq!(pending_line(&downloaded), None);
    }
}
