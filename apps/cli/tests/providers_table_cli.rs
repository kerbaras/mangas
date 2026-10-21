//! #235: `mangas providers` sizes the id, name and site columns to their longest value, so the
//! status column lines up on every row. The probes go through a dead proxy (a closed local port),
//! so the test needs no network and every status is known not to be `ready`.

use std::net::TcpListener;
use std::process::Command;

/// The id, name and site columns as printed (the status column starts one space after).
const TABLE: &str = "\
mangadex    MangaDex    https://mangadex.org
mangadot    MangaDot    https://mangadot.net
weebcentral WeebCentral https://weebcentral.com
tapas       Tapas       https://tapas.io
comick      Comick      https://comick.live
webtoons    Webtoons    https://www.webtoons.com
mangaplus   MANGA Plus  https://mangaplus.shueisha.co.jp";

fn closed_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn every_row_lines_up_with_the_longest_provider() {
    let home = tempfile::tempdir().unwrap();
    let proxy = format!("http://127.0.0.1:{}", closed_port());
    let out = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .args(["--proxy", &proxy, "providers"])
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    let shown = String::from_utf8_lossy(&out.stdout);
    let width = TABLE.lines().map(str::len).max().unwrap() + 1;
    let columns: Vec<&str> = shown
        .lines()
        .map(|l| l[..width.min(l.len())].trim_end())
        .collect();
    assert_eq!(columns.join("\n"), TABLE, "{shown}");
    for line in shown.lines() {
        let status = &line[width..];
        assert!(
            status.starts_with("unreachable") || status.starts_with("needs a browser session"),
            "status starts right after the site column: {line:?}"
        );
    }
}
