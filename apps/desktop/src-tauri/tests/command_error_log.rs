//! `to_ipc` logs a command error once, under the command's span. This test has a binary of its
//! own: tracing caches each callsite's interest process-wide, and a callsite first hit on another
//! test thread while this test installs its subscriber can cache "never" over it (#317). Keep it
//! the only test here.

#[path = "../src/commands/errors.rs"]
mod errors;

use std::io::Write;
use std::sync::{Arc, Mutex};

use errors::to_ipc;

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn data_dir() -> Result<(), String> {
    Err(to_ipc("data dir is read-only"))
}

#[tracing::instrument(level = "error", skip_all)]
fn some_command() -> Result<(), String> {
    data_dir()?;
    Ok(())
}

#[test]
fn logs_the_error_once_under_the_command_name() {
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::ERROR)
        .finish();
    let result = tracing::subscriber::with_default(subscriber, some_command);
    assert_eq!(result, Err("data dir is read-only".to_string()));
    let log = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert_eq!(log.matches("data dir is read-only").count(), 1, "{log}");
    assert!(
        log.contains("ERROR") && log.contains("some_command"),
        "{log}"
    );
}
