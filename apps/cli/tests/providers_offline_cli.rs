//! #219: with the network down, `mangas providers` exits 1 even though Comick has no saved
//! session: a provider that needs one is still probed, and `needs_session` is only reported
//! when its host answers.

use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};

use mangas_core::library::{self, ProxyMode};
use serde_json::Value;

/// A fresh home (no Comick session) whose provider requests go to `proxy`, when given.
fn home(proxy: Option<String>) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("data/mangas/mangas.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let lib = library::open(&db).unwrap();
    let mut settings = library::get_settings(&lib).unwrap();
    settings.proxy_mode = proxy.as_ref().map(|_| ProxyMode::Custom);
    settings.proxy_url = proxy;
    library::set_settings(&lib, &settings).unwrap();
    home
}

/// `mangas providers --json` run through `wrapper` (e.g. `unshare -rn`), against the real sites.
fn providers(home: &Path, wrapper: &[&str]) -> Output {
    let bin = env!("CARGO_BIN_EXE_mangas-cli");
    let (program, args) = match wrapper {
        [program, args @ ..] => (*program, [args, &[bin]].concat()),
        [] => (bin, vec![]),
    };
    Command::new(program)
        .args(args)
        .args(["providers", "--json"])
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn assert_all_unreachable(out: &Output) {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["all_unreachable"], true, "{report}");
    let rows = report["providers"].as_array().unwrap();
    assert_eq!(rows.len(), 7, "{report}");
    for row in rows {
        assert_eq!(row["state"], "unreachable", "{row}");
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no provider is reachable"), "{stderr}");
}

#[test]
fn a_dead_proxy_makes_every_provider_unreachable_comick_included() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let home = home(Some(dead));
    assert_all_unreachable(&providers(home.path(), &[]));
}

/// The QA recipe: no network at all (a fresh network namespace). Skipped where unprivileged
/// user namespaces are disabled (GitHub's Ubuntu 24.04 runners).
#[test]
fn no_network_makes_every_provider_unreachable_comick_included() {
    let unshare = ["unshare", "--user", "--map-root-user", "--net"];
    let available = Command::new(unshare[0])
        .args(&unshare[1..])
        .arg("true")
        .output()
        .is_ok_and(|o| o.status.success());
    if !available {
        eprintln!("skipped: unshare can't create a network namespace here");
        return;
    }
    let home = home(None);
    assert_all_unreachable(&providers(home.path(), &unshare));
}

/// The issue's recipe: no root certificates, so no TLS client can be built.
#[test]
fn no_tls_certificates_exits_1() {
    let home = home(None);
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
    assert_eq!(stdout.matches("unreachable: ").count(), 7, "{stdout}");
}
