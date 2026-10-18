//! `mangas … 2> log.txt` must write plain text: no ANSI colour codes when stderr is not a TTY.

use std::process::Command;

#[test]
fn redirected_stderr_has_no_colour_codes() {
    let dir = std::env::temp_dir().join(format!("mangas-log-colours-{}", std::process::id()));
    let chapter = dir.join("chapter");
    std::fs::create_dir_all(&chapter).unwrap();
    std::fs::write(chapter.join("notes.txt"), "not a page").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mangas-cli"))
        .arg("kindle")
        .arg(&chapter)
        .arg("-o")
        .arg(dir.join("out.epub"))
        .env_remove("NO_COLOR")
        .env_remove("MANGAS_LOG")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("skipping unsupported file"),
        "no warning logged: {stderr}"
    );
    assert!(
        !stderr.contains('\u{1b}'),
        "ANSI escape in redirected stderr: {stderr:?}"
    );
}
