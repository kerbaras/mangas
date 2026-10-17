use mangas_core::diagnostics::{render, session_status, ProviderStatus, Report};
use mangas_core::library::ProviderSession;
use mangas_core::logging::{cli_level, default_directives, file_appender, filter, log_dir};
use std::io::Write;

#[test]
fn session_status_covers_every_state() {
    assert_eq!(session_status(false, None), "ready");
    assert_eq!(session_status(true, None), "needs a browser session");
    let session = ProviderSession {
        saved_at: Some("2026-10-17T10:00:00Z".into()),
        ..ProviderSession::default()
    };
    assert_eq!(
        session_status(true, Some(&session)),
        "connected (session saved 2026-10-17T10:00:00Z)"
    );
    assert_eq!(
        session_status(true, Some(&ProviderSession::default())),
        "connected"
    );
}

#[test]
fn report_renders_every_field() {
    let report = Report {
        version: "0.1.0".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        data_dir: "/data".into(),
        log_dir: "/data/logs".into(),
        providers: vec![ProviderStatus {
            id: "mangadex".into(),
            status: "ready".into(),
        }],
    };
    assert_eq!(
        render(&report),
        "Mangas 0.1.0\nOS: linux (x86_64)\nData folder: /data\nLogs: /data/logs\nProviders:\n  mangadex: ready\n"
    );
}

#[test]
fn verbosity_maps_to_levels() {
    assert_eq!(cli_level(0), "warn");
    assert_eq!(cli_level(1), "info");
    assert_eq!(cli_level(2), "debug");
    assert_eq!(cli_level(9), "trace");
}

#[test]
fn defaults_keep_dependencies_quiet() {
    let d = default_directives("debug");
    assert!(d.starts_with("warn,"));
    assert!(d.contains("mangas_core=debug"));
    assert!(d.contains("mangas_providers=debug"));
}

#[test]
fn env_filter_wins_unless_blank_or_invalid() {
    assert_eq!(filter(Some("trace"), "info").to_string(), "trace");
    let defaults = filter(None, "info").to_string();
    assert!(defaults.contains("mangas_core=info"));
    assert_eq!(filter(Some("  "), "info").to_string(), defaults);
    assert_eq!(filter(Some("=[bad"), "info").to_string(), defaults);
}

#[test]
fn appender_creates_a_dated_log_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = log_dir(tmp.path());
    let mut appender = file_appender(&dir).unwrap();
    writeln!(appender, "hello").unwrap();
    appender.flush().unwrap();
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with("mangas.") && names[0].ends_with(".log"));
}
