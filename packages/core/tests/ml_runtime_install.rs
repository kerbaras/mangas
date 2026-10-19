//! ONNX Runtime download: install from a local server, checksum and cancel, status, remove.

mod ml_support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;

use mangas_core::ml::cache::{model_path, sha256_hex};
use mangas_core::ml::runtime::{
    ensure, lib_path, root, status_of, Archive, RuntimeSpec, RuntimeStatus, RUNTIMES,
};
use mangas_core::ml::{is_cancelled, missing_bytes, remove, MODELS};
use ml_support::tgz;

fn spec_for(url: String, archive: &[u8], lib: &[u8]) -> RuntimeSpec {
    RuntimeSpec {
        os: "linux",
        arch: "x86_64",
        version: "9.9.9",
        url: Box::leak(url.into_boxed_str()),
        archive: Archive::Tgz,
        sha256: Box::leak(sha256_hex(archive).into_boxed_str()),
        size: archive.len() as u64,
        member: "ort/lib/libonnxruntime.so.9",
        lib_name: "libonnxruntime.so",
        lib_sha256: Box::leak(sha256_hex(lib).into_boxed_str()),
    }
}

fn serve(body: Vec<u8>, requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/ort.tgz", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().take(requests) {
            let mut s = stream.unwrap();
            let mut buf = [0u8; 4096];
            let _ = s.read(&mut buf);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            s.write_all(head.as_bytes()).unwrap();
            s.write_all(&body).unwrap();
        }
    });
    url
}

#[tokio::test]
async fn installs_verified_library_once() {
    let lib = b"fake onnx runtime".to_vec();
    let archive = tgz(&[("ort/lib/libonnxruntime.so.9", &lib)]);
    let spec = spec_for(serve(archive.clone(), 1), &archive, &lib);
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root(dir.path()).join("1.0.0")).unwrap();
    let mut seen = 0;
    let never = AtomicBool::new(false);
    let path = ensure(
        dir.path(),
        &spec,
        &Default::default(),
        &mut |done, _| seen = done,
        &never,
    )
    .await
    .unwrap();
    assert_eq!(path, dir.path().join("runtime/9.9.9/libonnxruntime.so"));
    assert_eq!(std::fs::read(&path).unwrap(), lib);
    assert_eq!(seen, archive.len() as u64);
    assert!(
        !root(dir.path()).join("1.0.0").exists(),
        "older runtimes are pruned"
    );
    // The server is gone: a present, verified library is not downloaded again.
    ensure(
        dir.path(),
        &spec,
        &Default::default(),
        &mut |_, _| {},
        &never,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn rejects_checksum_mismatch_and_honours_cancel() {
    let lib = b"lib".to_vec();
    let archive = tgz(&[("ort/lib/libonnxruntime.so.9", &lib)]);
    let mut spec = spec_for(serve(archive.clone(), 2), &archive, &lib);
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(true);
    let err = ensure(
        dir.path(),
        &spec,
        &Default::default(),
        &mut |_, _| {},
        &cancel,
    )
    .await
    .unwrap_err();
    assert!(is_cancelled(&err), "{err}");
    spec.sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
    let never = AtomicBool::new(false);
    let err = ensure(
        dir.path(),
        &spec,
        &Default::default(),
        &mut |_, _| {},
        &never,
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("sha256 mismatch"), "{err}");
    assert!(!lib_path(dir.path(), &spec).exists());
}

#[test]
fn status_reports_absent_present_and_failed() {
    let dir = tempfile::tempdir().unwrap();
    let spec = RUNTIMES[0];
    let version = spec.version.to_string();
    assert_eq!(
        status_of(dir.path(), &spec, None),
        RuntimeStatus::NotDownloaded {
            version: version.clone()
        }
    );
    let path = lib_path(dir.path(), &spec);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"lib").unwrap();
    assert_eq!(
        status_of(dir.path(), &spec, None),
        RuntimeStatus::Downloaded {
            version: version.clone()
        }
    );
    let failed = status_of(dir.path(), &spec, Some("bad".into()));
    assert_eq!(
        failed,
        RuntimeStatus::Failed {
            version,
            reason: "bad".into()
        }
    );
    let json = serde_json::to_value(&failed).unwrap();
    assert_eq!(json["state"], "failed");
}

#[test]
fn sizes_and_removes_runtime_with_models() {
    let dir = tempfile::tempdir().unwrap();
    let spec = RUNTIMES[0];
    let all = spec.size + MODELS.iter().map(|m| m.size).sum::<u64>();
    assert_eq!(missing_bytes(dir.path(), Some(&spec)), all);
    let path = lib_path(dir.path(), &spec);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"12345").unwrap();
    std::fs::write(model_path(dir.path(), &MODELS[0]), b"123").unwrap();
    assert_eq!(missing_bytes(dir.path(), Some(&spec)), MODELS[1].size);
    assert_eq!(missing_bytes(dir.path(), None), MODELS[1].size);
    assert_eq!(remove(dir.path()).unwrap(), 8);
    assert!(!root(dir.path()).exists());
    assert!(!model_path(dir.path(), &MODELS[0]).exists());
    assert_eq!(remove(dir.path()).unwrap(), 0);
}
