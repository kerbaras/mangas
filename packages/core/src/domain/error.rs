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
    /// The host answered with a certificate that failed verification. Final: a retry gets the
    /// same certificate.
    #[error("network: the certificate of {host} couldn't be verified ({problem})")]
    Certificate {
        host: String,
        problem: super::CertProblem,
    },
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
    /// A file or folder operation failed on `path`; the message names it once.
    #[error("io: {}: {source}", path.display())]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    /// A local import found nothing it could use; the message is meant for the user as is.
    #[error("{0}")]
    Import(String),
    /// A Kindle (`device`, its USB product name) is on the bus but has no mounted folder: it
    /// talks MTP only, which a USB copy can't use. Email still works.
    #[error("{device} {}", crate::send::usb::MTP_HINT)]
    KindleOverMtp { device: String },
    /// A USB copy the format table refuses, or warns about and the user didn't confirm: the
    /// verdict's reason names the format and the device and what to do instead.
    #[error("{}", .0.reason.as_deref().unwrap_or("this format can't be copied to a Kindle"))]
    NotOnKindle(Box<crate::send::usb::UsbVerdict>),
    /// The user stopped a USB copy: no partial book is left and a book already there stays.
    #[error("copy cancelled; the Kindle keeps what it had")]
    Cancelled,
    /// The Kindle (`device`) is on the bus but another process holds it (macOS `ptpcamerad`,
    /// Android File Transfer): `holder` names it, `fix` is what the user runs or quits.
    #[error("{device} is in use by {holder}: {fix}")]
    KindleHeld {
        device: String,
        holder: String,
        fix: String,
    },
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
        } else if let Some(problem) = super::certificate::certificate_problem(&e) {
            let host = e.url().and_then(|u| u.host_str()).unwrap_or("the site");
            Error::Certificate {
                host: host.to_string(),
                problem,
            }
        } else {
            Error::Network(e)
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
