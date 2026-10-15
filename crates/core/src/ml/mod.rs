//! Optional ONNX detectors (speech bubbles, text, faces) that sharpen smart cutting.
//! Models are downloaded on first use, never bundled. Inference lands in the
//! "export + smartcut + ml" PR behind the `ml` feature.

/// A downloadable detector. Licences are permissive on purpose (the GPL YOLOv8 bubble model is
/// deliberately not listed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: &'static str,
    pub url: &'static str,
    pub licence: &'static str,
    /// Hex SHA-256 of the file at `url`, checked after download.
    pub sha256: Option<&'static str>,
}

pub const BUBBLE_TEXT_DETECTOR: ModelSpec = ModelSpec {
    id: "comic-text-and-bubble-detector",
    url: "https://huggingface.co/ogkalu/comic-text-and-bubble-detector/resolve/main/detector.onnx",
    licence: "Apache-2.0",
    sha256: Some("065744e91c0594ad8663aa8b870ce3fb27222942eded5a3cc388ce23421bd195"),
};

pub const FACE_DETECTOR: ModelSpec = ModelSpec {
    id: "anime-face-detect-v0-n",
    url: "https://huggingface.co/deepghs/anime_face_detection/resolve/main/face_detect_v0_n/model.onnx",
    licence: "MIT",
    sha256: Some("60d40c22cd57a2d129ff913227e7abcb8db9a3b98ebb8ec4fa93a72bee0dcfab"),
};
