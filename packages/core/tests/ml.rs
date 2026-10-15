//! Detector post-processing that does not need ONNX Runtime: NMS, tiling, row penalties, cache.

use mangas_core::ml::cache::{is_downloaded, model_path, sha256_hex};
use mangas_core::ml::nms::{nms, Class, Detection};
use mangas_core::ml::penalty::row_penalties;
use mangas_core::ml::tiles::windows;
use mangas_core::ml::{status, BUBBLE_TEXT_DETECTOR, ENABLED, MODELS};

fn det(class: Class, score: f32, y0: f32, y1: f32) -> Detection {
    Detection {
        class,
        score,
        x0: 0.0,
        y0,
        x1: 100.0,
        y1,
    }
}

#[test]
fn nms_keeps_best_box_per_overlapping_cluster() {
    let dets = vec![
        det(Class::Text, 0.9, 0.0, 100.0),
        det(Class::Text, 0.8, 5.0, 105.0),
        det(Class::Text, 0.7, 300.0, 400.0),
        det(Class::Bubble, 0.6, 0.0, 100.0),
    ];
    let kept = nms(dets, 0.5);
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[0].score, 0.9);
    assert!(
        kept.iter().any(|d| d.class == Class::Bubble),
        "different classes are not suppressed"
    );
}

#[test]
fn tiles_cover_the_strip_with_overlap() {
    assert_eq!(windows(500, 1024, 128), vec![(0, 500)]);
    let w = windows(3000, 1024, 128);
    assert_eq!(w.first(), Some(&(0, 1024)));
    assert_eq!(w.last().map(|t| t.1), Some(3000));
    for pair in w.windows(2) {
        assert!(pair[1].0 <= pair[0].1 - 128, "{pair:?}");
        assert_eq!(pair[1].1 - pair[1].0, 1024);
    }
    assert!(windows(0, 1024, 128).is_empty());
}

#[test]
fn row_penalties_use_go_cost_weights() {
    let dets = vec![
        det(Class::Bubble, 0.9, 10.0, 20.0),
        det(Class::Text, 0.9, 15.0, 25.5),
        det(Class::Face, 0.9, 40.0, 45.0),
    ];
    let rows = row_penalties(&dets, 50);
    assert_eq!(rows[9], 0.0);
    assert_eq!(rows[10], 30.0);
    assert_eq!(rows[15], 60.0, "text wins over bubble");
    assert_eq!(rows[25], 60.0, "ceil of y1");
    assert_eq!(rows[26], 0.0);
    assert_eq!(rows[42], 60.0);
    assert_eq!(rows[49], 0.0);
}

#[test]
fn cache_paths_and_status() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_downloaded(dir.path(), &BUBBLE_TEXT_DETECTOR));
    assert_eq!(
        model_path(dir.path(), &BUBBLE_TEXT_DETECTOR),
        dir.path().join("comic-text-and-bubble-detector.onnx")
    );
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    for m in &MODELS {
        assert!(m.sha256.is_some_and(|h| h.len() == 64));
    }
    let s = status(dir.path());
    assert_eq!(s.enabled, ENABLED);
    assert!(!s.downloaded);
    assert_eq!(s.models.len(), 2);
    assert_eq!(s.execution_provider.is_some(), ENABLED);
}
