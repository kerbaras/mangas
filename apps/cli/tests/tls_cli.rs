//! #192: on a system without root certificates (a minimal Linux install without the
//! `ca-certificates` package) the network errors say what to install. `SSL_CERT_FILE` and
//! `SSL_CERT_DIR` point the system store at nothing, as on such a system.

use std::path::Path;
use std::process::{Command, Output};

const ADVICE: &str = "install the ca-certificates package";

fn mangas(home: &Path, args: &[&str]) -> Output {
    let empty = home.join("no-certs");
    std::fs::create_dir_all(&empty).unwrap();
    Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("SSL_CERT_FILE", empty.join("none.pem"))
        .env("SSL_CERT_DIR", &empty)
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn providers_name_the_missing_certificates() {
    let home = tempfile::tempdir().unwrap();
    let out = mangas(home.path(), &["providers"]);
    let shown = text(&out.stdout);
    assert!(shown.contains("no CA certificates found"), "{out:?}");
    assert!(shown.contains(ADVICE), "{out:?}");
    assert!(!shown.contains("builder error"), "{out:?}");
}

#[test]
fn email_over_tls_names_the_missing_certificates() {
    let home = tempfile::tempdir().unwrap();
    let setup = [
        "settings",
        "email",
        "--to",
        "reader@kindle.com",
        "--from",
        "me@example.test",
        "--host",
        "127.0.0.1",
        "--port",
        "465",
        "--security",
        "tls",
    ];
    assert!(mangas(home.path(), &setup).status.success());
    let out = mangas(home.path(), &["send", "--test"]);
    assert!(!out.status.success(), "{out:?}");
    assert!(text(&out.stderr).contains(ADVICE), "{out:?}");
}
