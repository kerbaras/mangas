//! A server certificate that failed verification: the host answered, so the error names the
//! certificate problem instead of suggesting the connection is down.

use std::error::Error as StdError;
use std::fmt;

use rustls::CertificateError;

/// Why a certificate was refused, worded for the user. The same words for the HTTP client
/// (rustls) and the Connect window (the webview's own checks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertProblem {
    /// Self-signed, or signed by an authority the system doesn't trust.
    UnknownIssuer,
    Expired,
    NotYetValid,
    /// Issued for another host name.
    WrongHost,
    Revoked,
    /// Anything else, as the verifier put it.
    Other(String),
}

impl fmt::Display for CertProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CertProblem::UnknownIssuer => f.write_str("self-signed or from an unknown authority"),
            CertProblem::Expired => f.write_str("expired"),
            CertProblem::NotYetValid => f.write_str("not valid yet"),
            CertProblem::WrongHost => f.write_str("issued for another site"),
            CertProblem::Revoked => f.write_str("revoked"),
            CertProblem::Other(text) => f.write_str(text),
        }
    }
}

impl From<&CertificateError> for CertProblem {
    fn from(e: &CertificateError) -> Self {
        use CertificateError as E;
        match e {
            E::UnknownIssuer | E::BadSignature => CertProblem::UnknownIssuer,
            E::Expired | E::ExpiredContext { .. } => CertProblem::Expired,
            E::NotValidYet | E::NotValidYetContext { .. } => CertProblem::NotYetValid,
            E::NotValidForName | E::NotValidForNameContext { .. } => CertProblem::WrongHost,
            E::Revoked => CertProblem::Revoked,
            // webpki's word for a self-signed certificate made with `openssl req -x509`.
            E::Other(o) if o.to_string().contains("CaUsedAsEndEntity") => {
                CertProblem::UnknownIssuer
            }
            E::Other(o) => CertProblem::Other(o.to_string()),
            other => CertProblem::Other(other.to_string()),
        }
    }
}

/// The certificate problem behind `err`, if any error in its chain is rustls refusing the
/// server's certificate. `io::Error` hides its inner error from `source()`, so it is opened too.
pub fn certificate_problem(err: &(dyn StdError + 'static)) -> Option<CertProblem> {
    let mut next = Some(err);
    while let Some(e) = next {
        if let Some(problem) = refused(e) {
            return Some(problem);
        }
        let inner = e
            .downcast_ref::<std::io::Error>()
            .and_then(|io| io.get_ref());
        if let Some(problem) = inner.and_then(|i| refused(i)) {
            return Some(problem);
        }
        next = inner
            .map(|i| i as &(dyn StdError + 'static))
            .or_else(|| e.source());
    }
    None
}

fn refused(e: &(dyn StdError + 'static)) -> Option<CertProblem> {
    match e.downcast_ref::<rustls::Error>()? {
        rustls::Error::InvalidCertificate(cert) => Some(CertProblem::from(cert)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod webpki_like {
        #[derive(Debug, thiserror::Error)]
        #[error("CaUsedAsEndEntity")]
        pub struct CaUsedAsEndEntity;
    }

    #[derive(Debug, thiserror::Error)]
    #[error("error sending request")]
    struct Outer(#[source] std::io::Error);

    #[test]
    fn finds_the_refused_certificate_inside_an_io_error() {
        let tls = rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer);
        let err = Outer(std::io::Error::other(tls));
        assert_eq!(certificate_problem(&err), Some(CertProblem::UnknownIssuer));
        let refused = Outer(std::io::Error::from(std::io::ErrorKind::ConnectionRefused));
        assert_eq!(certificate_problem(&refused), None);
        let other = Outer(std::io::Error::other(rustls::Error::DecryptError));
        assert_eq!(certificate_problem(&other), None);
    }

    #[test]
    fn words_each_problem_for_the_user() {
        let words = |e: CertificateError| CertProblem::from(&e).to_string();
        assert_eq!(
            words(CertificateError::UnknownIssuer),
            "self-signed or from an unknown authority"
        );
        assert_eq!(words(CertificateError::Expired), "expired");
        assert_eq!(words(CertificateError::NotValidYet), "not valid yet");
        assert_eq!(
            words(CertificateError::NotValidForName),
            "issued for another site"
        );
        assert_eq!(words(CertificateError::Revoked), "revoked");
        let self_signed_ca =
            rustls::OtherError(std::sync::Arc::new(webpki_like::CaUsedAsEndEntity));
        assert_eq!(
            words(CertificateError::Other(self_signed_ca)),
            "self-signed or from an unknown authority"
        );
    }
}
