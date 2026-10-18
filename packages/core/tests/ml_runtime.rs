//! ONNX Runtime download: pinned specs, archive extraction, install, status, remove.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;

use mangas_core::ml::cache::{model_path, sha256_hex};
use mangas_core::ml::runtime::{
    ensure, extract_member, for_target, lib_path, root, status_of, Archive, RuntimeSpec,
    RuntimeStatus, RUNTIMES,
};
use mangas_core::ml::{is_cancelled, missing_bytes, remove, MODELS};

const RELEASE_TARGETS: [(&str, &str); 6] = [
    ("macos", "aarch64"),
    ("macos", "x86_64"),
    ("windows", "x86_64"),
    ("windows", "aarch64"),
    ("linux", "x86_64"),
    ("linux", "aarch64"),
];

#[test]
fn every_release_target_has_an_official_runtime() {
    for (os, arch) in RELEASE_TARGETS {
        let spec = for_target(os, arch).unwrap_or_else(|| panic!("no runtime for {os}-{arch}"));
        let base = format!(
            "https://github.com/microsoft/onnxruntime/releases/download/v{}/",
            spec.version
        );
        assert!(spec.url.starts_with(&base), "{}", spec.url);
        assert!(spec.member.contains(spec.version));
        let minor: u32 = spec.version.split('.').nth(1).unwrap().parse().unwrap();
        assert!(minor >= 17, "ort rc.13 needs C API 17");
        let lib = match os {
            "windows" => "onnxruntime.dll",
            "macos" => "libonnxruntime.dylib",
            _ => "libonnxruntime.so",
        };
        assert_eq!(spec.lib_name, lib);
        assert_eq!(
            spec.archive,
            if os == "windows" {
                Archive::Zip
            } else {
                Archive::Tgz
            }
        );
        assert_eq!(spec.sha256.len(), 64);
        assert_eq!(spec.lib_sha256.len(), 64);
    }
    assert_eq!(RUNTIMES.len(), RELEASE_TARGETS.len());
    assert_eq!(for_target("linux", "riscv64"), None);
}

fn tgz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(gz);
    for (path, data) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, path, *data).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (path, data) in entries {
        w.start_file(*path, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

#[test]
fn extracts_the_library_from_tgz_and_zip() {
    let t = tgz(&[
        ("./ort-1/lib/notes.txt", b"x"),
        ("./ort-1/lib/libonnxruntime.so.1", b"LIB"),
    ]);
    assert_eq!(
        extract_member(&t, Archive::Tgz, "ort-1/lib/libonnxruntime.so.1").unwrap(),
        b"LIB"
    );
    let z = zip(&[("ort/lib/onnxruntime.dll", b"DLL")]);
    assert_eq!(
        extract_member(&z, Archive::Zip, "ort/lib/onnxruntime.dll").unwrap(),
        b"DLL"
    );
    let err = extract_member(&z, Archive::Zip, "ort/lib/missing.dll").unwrap_err();
    assert!(err.to_string().contains("missing.dll"), "{err}");
    assert!(extract_member(b"not gzip", Archive::Tgz, "x").is_err());
}

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

/// Serves `body` to `requests` connections, then stops.
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
    let path = ensure(dir.path(), &spec, &mut |done, _| seen = done, &never)
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
    ensure(dir.path(), &spec, &mut |_, _| {}, &never)
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
    let err = ensure(dir.path(), &spec, &mut |_, _| {}, &cancel)
        .await
        .unwrap_err();
    assert!(is_cancelled(&err), "{err}");
    spec.sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
    let never = AtomicBool::new(false);
    let err = ensure(dir.path(), &spec, &mut |_, _| {}, &never)
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

/// Downloads all six official archives (~220 MB): `cargo test --test ml_runtime -- --ignored`.
#[tokio::test]
#[ignore]
async fn official_assets_match_their_checksums() {
    for spec in RUNTIMES {
        let dir = tempfile::tempdir().unwrap();
        let never = AtomicBool::new(false);
        let path = ensure(dir.path(), &spec, &mut |_, _| {}, &never)
            .await
            .unwrap();
        assert!(path.is_file(), "{} {}", spec.os, spec.arch);
    }
}
