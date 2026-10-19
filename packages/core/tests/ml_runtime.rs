//! ONNX Runtime pins: one official, checksummed archive per release target, and extracting the
//! library from it. Download, install, status and removal are in `ml_runtime_install.rs`.

mod ml_support;

use std::io::Write;
use std::sync::atomic::AtomicBool;

use mangas_core::ml::runtime::{ensure, extract_member, for_target, Archive, RUNTIMES};
use ml_support::tgz;

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

/// Downloads all six official archives (~220 MB): `cargo test --test ml_runtime -- --ignored`.
#[tokio::test]
#[ignore]
async fn official_assets_match_their_checksums() {
    for spec in RUNTIMES {
        let dir = tempfile::tempdir().unwrap();
        let never = AtomicBool::new(false);
        let path = ensure(
            dir.path(),
            &spec,
            &Default::default(),
            &mut |_, _| {},
            &never,
        )
        .await
        .unwrap();
        assert!(path.is_file(), "{} {}", spec.os, spec.arch);
    }
}
