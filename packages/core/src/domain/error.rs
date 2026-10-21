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
    Network(#[source] reqwest::Error),
    /// No HTTP or TLS client can be built: the system has no root certificates.
    #[error("network: {}", super::tls::NO_ROOT_CERTIFICATES)]
    NoRootCertificates,
    /// An export ran out of disk space in `folder` (the spool or the file itself). Final: the
    /// export removed what it wrote, and a retry fails the same way until space is freed.
    #[error("not enough space in {}", folder.display())]
    NoSpace { folder: std::path::PathBuf },
    /// A USB copy filled the Kindle; `device` names it and where it is mounted. Final like
    /// [`Error::NoSpace`]: the partial book was removed from the device.
    #[error("not enough space on {device}")]
    DeviceFull { device: String },
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

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        if super::tls::missing_root_certificates(&e) {
            Error::NoRootCertificates
        } else {
            Error::Network(e)
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
