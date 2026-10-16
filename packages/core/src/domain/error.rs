/// Core error type. Binaries wrap it with `anyhow`; the Tauri layer serialises `to_string()`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not implemented yet")]
    NotImplemented(&'static str),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("source {name}: {message}")]
    Source { name: String, message: String },
    /// The provider sits behind a browser verification (Cloudflare); the app must open `site`
    /// in a window, let the user pass it and store the resulting session.
    #[error("challenge: {name} needs a browser session for {site}")]
    Challenge { name: String, site: String },
    #[error("library: {0}")]
    Library(#[from] rusqlite::Error),
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
