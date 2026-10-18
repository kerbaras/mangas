#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! One executable for both faces of Mangas: `mangas <command>` runs the CLI, plain `mangas`
//! opens the desktop app.

fn main() {
    if std::env::args_os().len() > 1 {
        run_cli();
    } else {
        mangas_desktop_lib::run();
    }
}

fn run_cli() {
    attach_parent_console();
    let outcome = tokio::runtime::Runtime::new()
        .map_err(anyhow::Error::from)
        .and_then(|rt| rt.block_on(mangas_cli::run(std::env::args_os())));
    if let Err(e) = outcome {
        eprintln!("error: {e:#}");
        std::process::exit(mangas_cli::exit_code(&e).into());
    }
}

/// Release builds on Windows hide the console; reattach to the parent's so CLI output shows.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    // SAFETY: plain Win32 call with a constant argument; failure just means no console.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}
