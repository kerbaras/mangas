//! #219: `mangas providers` exits 1 only when every provider is unreachable; a partial outage
//! exits 0. `--json` keeps one row per provider and adds `all_unreachable`. Providers are probed
//! at mock bases (`--base`), never the real sites.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};

use mangas_core::library::{self, ProviderSession};
use serde_json::Value;

const IDS: [&str; 6] = [
    "mangadex",
    "mangadot",
    "weebcentral",
    "tapas",
    "comick",
    "webtoons",
];

/// A server answering every request with `200 {}`.
fn up() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for mut sock in listener.incoming().flatten() {
            let _ = sock.read(&mut [0; 4096]);
            let reply = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}";
            let _ = sock.write_all(reply.as_bytes());
        }
    });
    base
}

/// A base nothing listens on.
fn down() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

/// A home whose settings hold a Comick session, so Comick is probed too.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let mut settings = library::get_settings(&lib).unwrap();
    let session = ProviderSession {
        cookies: "cf_clearance=x".into(),
        user_agent: "Mozilla/5.0".into(),
        saved_at: None,
    };
    settings.provider_sessions.insert("comick".into(), session);
    library::set_settings(&lib, &settings).unwrap();
    home
}

fn providers(home: &Path, bases: &[(&str, &str)], extra: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mangas-cli"));
    cmd.arg("providers").args(extra);
    for (id, base) in bases {
        cmd.arg("--base").arg(format!("{id}={base}"));
    }
    cmd.env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn states(report: &Value) -> Vec<(String, String)> {
    let rows = report["providers"].as_array().unwrap();
    let field = |r: &Value, k: &str| r[k].as_str().unwrap().to_string();
    rows.iter()
        .map(|r| (field(r, "id"), field(r, "state")))
        .collect()
}

#[test]
fn a_partial_outage_exits_0() {
    let home = home();
    let (ok, gone) = (up(), down());
    let bases: Vec<_> = IDS
        .iter()
        .map(|&id| (id, if id == "mangadex" { &ok } else { &gone }.as_str()))
        .collect();
    let out = providers(home.path(), &bases, &["--json"]);
    assert!(out.status.success(), "{out:?}");
    let report = json(&out);
    assert_eq!(report["all_unreachable"], false);
    let state = |id| {
        if id == "mangadex" {
            "ready"
        } else {
            "unreachable"
        }
    };
    let want = IDS.map(|id| (id.to_string(), state(id).to_string()));
    assert_eq!(states(&report), want);
    assert!(
        report["providers"][1]["reason"]
            .as_str()
            .unwrap()
            .contains("network"),
        "{report}"
    );
    let text = providers(home.path(), &bases, &[]);
    assert!(text.status.success(), "{text:?}");
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert_eq!(stdout.lines().count(), 6, "{stdout}");
    assert!(
        stdout.starts_with("mangadex   MangaDex   https://mangadex.org     ready\n"),
        "{stdout}"
    );
}

#[test]
fn every_provider_unreachable_exits_1_with_the_rows_still_printed() {
    let home = home();
    let gone = down();
    let bases: Vec<_> = IDS.iter().map(|&id| (id, gone.as_str())).collect();
    let out = providers(home.path(), &bases, &["--json"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let report = json(&out);
    assert_eq!(report["all_unreachable"], true);
    assert_eq!(
        states(&report),
        IDS.map(|id| (id.to_string(), "unreachable".to_string()))
    );
    let text = providers(home.path(), &bases, &[]);
    assert_eq!(text.status.code(), Some(1), "{text:?}");
    assert_eq!(String::from_utf8_lossy(&text.stdout).lines().count(), 6);
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains("no provider is reachable"), "{stderr}");
}

#[test]
fn a_session_wall_is_not_an_outage() {
    let home = tempfile::tempdir().unwrap();
    let gone = down();
    let bases: Vec<_> = IDS.iter().map(|&id| (id, gone.as_str())).collect();
    let out = providers(home.path(), &bases, &["--json"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(json(&out)["all_unreachable"], false);
}

#[test]
fn no_tls_certificates_exits_1() {
    let home = home();
    let no_certs = home.path().join("no-certs");
    std::fs::create_dir_all(&no_certs).unwrap();
    std::fs::write(no_certs.join("none.pem"), "").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .arg("providers")
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("SSL_CERT_FILE", no_certs.join("none.pem"))
        .env("SSL_CERT_DIR", &no_certs)
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.matches("unreachable: ").count(), 6, "{stdout}");
}

#[test]
fn an_unknown_base_id_is_refused_instead_of_probing_the_live_site() {
    let home = home();
    let out = providers(
        home.path(),
        &[("mangadx", "http://127.0.0.1:9")],
        &["--json"],
    );
    assert!(!out.status.success(), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown provider \"mangadx\""), "{stderr}");
}

#[test]
fn listing_flags_are_refused_before_a_subcommand() {
    let home = home();
    let base = "mangadex=http://127.0.0.1:9";
    for args in [
        vec!["--json", "check", "mangadex"],
        vec!["--base", base, "check", "mangadex"],
    ] {
        let out = providers(home.path(), &[], &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
    }
}
