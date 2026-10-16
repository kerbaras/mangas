//! Standalone `mangas-cli` binary for development; releases ship the CLI inside the desktop
//! executable (`apps/desktop`), which runs the same code when given arguments.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mangas_cli::run(std::env::args_os()).await
}
