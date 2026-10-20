//! #234: an IO failure is reported with its reason once, not `io: X: X`.

use std::path::Path;
use std::process::Command;

/// Runs `mangas kindle <one-page chapter> -o <output>` in `home`; returns stderr's last line.
fn kindle_into(home: &Path, output: &Path) -> String {
    let dir = home.join("Series/1");
    std::fs::create_dir_all(&dir).unwrap();
    let page = image::RgbImage::from_pixel(64, 96, image::Rgb([200, 200, 200]));
    page.save(dir.join("001.png")).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .arg("kindle")
        .arg(&dir)
        .arg("-o")
        .arg(output)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    stderr.lines().last().unwrap_or_default().to_string()
}

fn assert_once(line: &str, reason: &str) {
    assert_eq!(line, format!("error: io: {reason}"), "{line}");
    assert_eq!(line.matches(reason).count(), 1, "{line}");
}

#[test]
fn a_file_name_too_long_is_reported_once() {
    let home = tempfile::tempdir().unwrap();
    let output = home.path().join(format!("{}.epub", "a".repeat(300)));
    let reason = std::io::Error::from_raw_os_error(36).to_string();
    assert_once(&kindle_into(home.path(), &output), &reason);
}

#[cfg(unix)]
#[test]
fn a_permission_denied_is_reported_once() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let locked = home.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::create_dir(locked.join("probe")).is_ok() {
        return; // root ignores permissions
    }
    let reason = std::io::Error::from_raw_os_error(13).to_string();
    assert_once(&kindle_into(home.path(), &locked.join("b.epub")), &reason);
}
