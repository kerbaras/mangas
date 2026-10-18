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
    /// The chapter is listed but cannot be downloaded; `reason` comes from
    /// [`crate::domain::Availability::reason`].
    #[error("chapter {number} can't be downloaded: {reason}")]
    Unavailable { number: String, reason: String },
    /// A page or cover host resolves to a loopback, private or metadata address. Final: the
    /// answer will not change on a retry.
    #[error("refused: {host} resolves to a non-public address {ip}")]
    BlockedAddress { host: String, ip: std::net::IpAddr },
    #[error("library: {0}")]
    Library(#[from] rusqlite::Error),
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    /// A local import found nothing it could use; the message is meant for the user as is.
    #[error("{0}")]
    Import(String),
    /// Email or USB delivery failed; the message is meant for the user as is.
    #[error("{0}")]
    Send(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
