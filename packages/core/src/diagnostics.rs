//! The "Copy diagnostics" report: what a bug report needs to start from, as plain text.

use serde::Serialize;

use crate::library::ProviderSession;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub log_dir: String,
    pub providers: Vec<ProviderStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub status: String,
}

/// One-line state of a provider: ready, connected (with the session date) or not connected.
pub fn session_status(needs_session: bool, session: Option<&ProviderSession>) -> String {
    match (needs_session, session) {
        (false, _) => "ready".to_string(),
        (true, None) => "needs a browser session".to_string(),
        (true, Some(s)) => match s.saved_at.as_deref() {
            Some(at) => format!("connected (session saved {at})"),
            None => "connected".to_string(),
        },
    }
}

/// The report as text, ready to paste into an issue.
pub fn render(report: &Report) -> String {
    let providers = report
        .providers
        .iter()
        .map(|p| format!("  {}: {}", p.id, p.status))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Mangas {}\nOS: {} ({})\nData folder: {}\nLogs: {}\nProviders:\n{}\n",
        report.version, report.os, report.arch, report.data_dir, report.log_dir, providers
    )
}
