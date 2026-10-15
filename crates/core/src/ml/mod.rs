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
    pub sha256: Option<&'static str>,
}

pub const BUBBLE_TEXT_DETECTOR: ModelSpec = ModelSpec {
    id: "comic-text-and-bubble-detector",
    url: "https://huggingface.co/ogkalu/comic-text-and-bubble-detector/resolve/main/detector.onnx",
    licence: "Apache-2.0",
    sha256: None,
};

pub const FACE_DETECTOR: ModelSpec = ModelSpec {
    id: "anime-face-detect-v0-n",
    url: "https://huggingface.co/deepghs/anime_face_detection/resolve/main/face_detect_v0_n/model.onnx",
    licence: "MIT",
    sha256: None,
};
