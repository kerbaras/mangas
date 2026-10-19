use std::process::ExitStatus;

use super::*;
use crate::ml::ModelStatus;

fn installed() -> Status {
    Status {
        enabled: true,
        downloaded: true,
        runtime: RuntimeStatus::Downloaded {
            version: "1.28.3".into(),
        },
        models: vec![ModelStatus {
            id: "m".into(),
            downloaded: true,
            licence: "MIT".into(),
        }],
        execution_provider: Some("cpu".into()),
        download_bytes: 0,
        disk_bytes: 5,
    }
}

#[cfg(unix)]
fn exit(code: i32) -> ExitStatus {
    std::os::unix::process::ExitStatusExt::from_raw(code << 8)
}

#[cfg(windows)]
fn exit(code: i32) -> ExitStatus {
    std::os::windows::process::ExitStatusExt::from_raw(code as u32)
}

fn output(code: i32, stdout: &str, stderr: &str) -> std::io::Result<Output> {
    Ok(Output {
        status: exit(code),
        stdout: stdout.into(),
        stderr: stderr.into(),
    })
}

#[test]
fn the_childs_answer_is_the_status() {
    let mut child = installed();
    child.runtime = RuntimeStatus::Failed {
        version: "1.28.3".into(),
        reason: "onnx runtime /x.so: failed to map segment from shared object".into(),
    };
    child.downloaded = false;
    let json = serde_json::to_string_pretty(&child).unwrap();
    assert_eq!(from_child(installed(), output(0, &json, "")), child);
}

#[test]
fn a_child_that_fails_or_garbles_marks_the_runtime_failed() {
    let reason = |s: Status| match s.runtime {
        RuntimeStatus::Failed { reason, .. } if !s.downloaded => reason,
        other => panic!("{other:?}"),
    };
    let crashed = reason(from_child(installed(), output(3, "", "error: boom\n")));
    assert!(
        crashed.starts_with("detector status check failed ("),
        "{crashed}"
    );
    assert!(crashed.ends_with("): boom"), "{crashed}");
    let garbled = reason(from_child(installed(), output(0, "{", "")));
    assert!(garbled.contains("unreadable answer"), "{garbled}");
    let timed_out = std::io::Error::new(std::io::ErrorKind::TimedOut, "no answer in 60 s");
    assert_eq!(
        reason(from_child(installed(), Err(timed_out))),
        "detector status check: no answer in 60 s"
    );
}

#[test]
fn the_arguments_are_the_clis_json_status_of_the_folder() {
    let args = args(Path::new("/data/models"));
    let args: Vec<_> = args.iter().map(|a| a.to_string_lossy()).collect();
    assert_eq!(
        args,
        ["ml", "status", "--json", "--models-dir", "/data/models"]
    );
}

#[test]
fn nothing_installed_starts_no_child() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-exe");
    assert_eq!(probed_status(dir.path(), &missing), status(dir.path()));
}

#[cfg(unix)]
#[test]
fn a_chatty_child_cant_block_on_a_full_pipe() {
    let child = Command::new("sh")
        .args(["-c", "head -c 3000000 /dev/zero >&2; echo done"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let out = wait(child, Duration::from_secs(10)).unwrap();
    assert!(out.status.success());
    assert_eq!(out.stderr.len() as u64, KEPT, "kept only the first MiB");
    assert_eq!(out.stdout, b"done\n");
}
