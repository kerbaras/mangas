/// Errors cross IPC as their display string. Every command error goes through here exactly
/// once, logged at `error` inside the command's `#[tracing::instrument]` span, so the log line
/// names the command (`list_library: … no home directory`).
pub(super) fn to_ipc(e: impl std::fmt::Display) -> String {
    let message = e.to_string();
    tracing::error!("{message}");
    message
}

#[cfg(test)]
mod tests {
    use super::to_ipc;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

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

    #[tracing::instrument(skip_all)]
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
}
