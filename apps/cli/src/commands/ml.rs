//! `mangas ml`: detector support (the ONNX Runtime library plus the models).

use std::sync::atomic::AtomicBool;

use anyhow::Context;
use clap::{Args as ClapArgs, Subcommand};
use mangas_core::ml::{self, RuntimeStatus, Status};
use mangas_core::storage::human_size;

use crate::output::outln;
use crate::paths;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show whether the ONNX Runtime and the detector models are downloaded (the default)
    Status,
    /// Download the ONNX Runtime library and the detector models
    Download,
    /// Delete them; the next `--detectors` export downloads them again
    Remove,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let dir = paths::models()?;
    match args.command.unwrap_or(Command::Status) {
        Command::Status => print_status(&ml::status(&dir)),
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
            let status = ml::download(&dir, &mut on_progress, &never).await;
            eprintln!();
            print_status(&status.context("detector support")?)
        }
        Command::Remove => {
            let freed = ml::remove(&dir)?;
            Ok(outln!(
                "Removed detector support: {} freed",
                human_size(freed)
            )?)
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
    }
}
