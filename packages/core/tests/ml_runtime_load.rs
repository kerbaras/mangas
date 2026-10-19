//! #200/#201: a runtime that won't load says why, and leaves the detectors off: no `ort` session
//! is built, and previews and exports cut without them.
#![cfg(feature = "ml")]

use std::path::Path;

use image::{Rgba, RgbaImage};
use mangas_core::export::kindle::{device, export, preview, ExportRequest, PreviewRequest};
use mangas_core::export::{Format, PageSettings};
use mangas_core::ml::cache::model_path;
use mangas_core::ml::{
    self, runtime, Detectors, RuntimeStatus, BUBBLE_TEXT_DETECTOR, FACE_DETECTOR,
};
use mangas_core::smartcut::Preset;

fn garbage_runtime(models: &Path) -> std::path::PathBuf {
    let spec = runtime::current().unwrap();
    let lib = runtime::lib_path(models, &spec);
    std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
    std::fs::write(&lib, b"not a library").unwrap();
    for m in [&BUBBLE_TEXT_DETECTOR, &FACE_DETECTOR] {
        let path = model_path(models, m);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"not a model").unwrap();
    }
    lib
}

#[cfg(unix)]
#[test]
fn a_library_that_wont_open_reports_the_loaders_message() {
    let dir = tempfile::tempdir().unwrap();
    let lib = dir.path().join("libonnxruntime.so");
    std::fs::write(&lib, b"not a library").unwrap();
    let err = runtime::probe(&lib).unwrap_err().to_string();
    // libloading alone says "dlopen failed"; the system's dlerror() text follows it.
    let (_, dlerror) = err.split_once("dlopen failed: ").expect(&err);
    assert!(dlerror.contains("libonnxruntime.so"), "{err}");
}

#[test]
fn a_failed_load_is_recorded_and_no_session_is_built() {
    let dir = tempfile::tempdir().unwrap();
    garbage_runtime(dir.path());
    let err = runtime::ensure_loaded(dir.path()).unwrap_err().to_string();
    match ml::status(dir.path()).runtime {
        RuntimeStatus::Failed { reason, .. } => assert_eq!(reason, err),
        other => panic!("{other:?}"),
    }
    assert!(!runtime::is_loaded());
    // Before the fix this reached `ort` and panicked (OrtGetApiBase must be present).
    let loaded = Detectors::load(
        &model_path(dir.path(), &BUBBLE_TEXT_DETECTOR),
        &model_path(dir.path(), &FACE_DETECTOR),
    );
    let Err(e) = loaded else {
        panic!("detectors loaded without a runtime")
    };
    assert!(e.to_string().contains("not loaded"), "{e}");
}

#[test]
fn the_status_check_skips_a_runtime_without_models_and_one_removed_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    let lib = garbage_runtime(dir.path());
    std::fs::remove_file(model_path(dir.path(), &FACE_DETECTOR)).unwrap();
    let runtime = ml::checked_status(dir.path()).runtime;
    assert!(
        matches!(runtime, RuntimeStatus::Downloaded { .. }),
        "{runtime:?}"
    );
    std::fs::remove_file(&lib).unwrap();
    assert!(runtime::ensure_loaded(dir.path()).is_err());
    let runtime = ml::status(dir.path()).runtime;
    assert!(
        matches!(runtime, RuntimeStatus::NotDownloaded { .. }),
        "{runtime:?}"
    );
}

fn chapter(root: &Path) -> std::path::PathBuf {
    let dir = root.join("Chapter 1");
    std::fs::create_dir_all(&dir).unwrap();
    let img = RgbaImage::from_fn(800, 3000, |_, y| {
        Rgba(if (y / 100) % 2 == 0 {
            [30; 4]
        } else {
            [250; 4]
        })
    });
    img.save(dir.join("001.png")).unwrap();
    dir
}

#[tokio::test]
async fn preview_and_export_cut_without_detectors_that_cant_run() {
    // No network: the runtime can't be (re)fetched, like an offline first use.
    std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:9");
    let models = tempfile::tempdir().unwrap();
    garbage_runtime(models.path());
    let detection = ml::penalizer_or_reason(
        models.path(),
        &mangas_core::library::Network::default(),
        &mut |_, _, _| {},
    )
    .await;
    assert!(detection.penalizer.is_none());
    let reason = detection.error.unwrap();
    assert!(matches!(
        ml::status(models.path()).runtime,
        RuntimeStatus::Failed { reason: r, .. } if r == reason
    ));

    let root = tempfile::tempdir().unwrap();
    let dir = chapter(root.path());
    let req = PreviewRequest {
        chapter_dir: dir.clone(),
        device: device("kindle-paperwhite12").unwrap(),
        cut: Preset::Balanced.options(),
        page: PageSettings::default(),
        limit: 4,
        out_dir: root.path().join("preview"),
    };
    let shown = preview(&req, detection.penalizer.clone(), &|| false).unwrap();
    assert!(!shown.pages.is_empty());
    let book = ExportRequest {
        chapter_dirs: vec![dir],
        device: device("kindle-paperwhite12").unwrap(),
        format: Format::Cbz,
        cut: Preset::Balanced.options(),
        page: PageSettings::default(),
        title: "t".into(),
        title_explicit: false,
        output: root.path().join("book.cbz"),
        replace: false,
        meta: Default::default(),
        labels: Default::default(),
    };
    assert!(
        export(&book, detection.penalizer, &mut |_| {})
            .unwrap()
            .pages
            > 0
    );
}
