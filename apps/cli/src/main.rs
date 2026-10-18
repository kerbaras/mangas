//! Standalone `mangas-cli` binary for development; releases ship the CLI inside the desktop
//! executable (`apps/desktop`), which runs the same code when given arguments.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match mangas_cli::run(std::env::args_os()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::ExitCode::from(mangas_cli::exit_code(&e))
        }
    }
}
