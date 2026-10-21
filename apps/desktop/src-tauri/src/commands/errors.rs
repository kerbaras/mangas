/// Errors cross IPC as their display string. Every command error goes through here exactly
/// once, logged at `error` inside the command's `#[tracing::instrument]` span, so the log line
/// names the command (`list_library: … no home directory`). The spans are `error` level too, so
/// they stay enabled under the strictest log filter. Tested in `tests/command_error_log.rs`.
pub(super) fn to_ipc(e: impl std::fmt::Display) -> String {
    let message = e.to_string();
    tracing::error!("{message}");
    message
}
