//! `mangas providers --base ID=URL`: probe a provider at a mock server instead of its site.

use std::collections::HashMap;
use url::Host;

/// `--base ID=URL` pairs. Refuses an unknown id rather than probing the live site instead, and a
/// URL off this machine: the probe sends the saved browser session (Comick cookies) to it.
pub fn parse_bases(bases: &[String]) -> anyhow::Result<HashMap<String, String>> {
    let known = mangas_providers::providers();
    bases
        .iter()
        .map(|b| match b.split_once('=') {
            Some((id, _)) if !known.iter().any(|p| p.id == id) => {
                anyhow::bail!("--base: unknown provider {id:?}")
            }
            Some((_, url)) if !is_local(url) => {
                anyhow::bail!("--base: {url} is not a local mock server (localhost or loopback IP)")
            }
            Some((id, url)) => Ok((id.to_string(), url.to_string())),
            None => anyhow::bail!("--base takes ID=URL"),
        })
        .collect()
}

/// Whether `raw` is http(s) on `localhost` or a loopback address, read by the same URL parser
/// the HTTP client uses (so `\\`, `@` and other authority tricks resolve the same way).
fn is_local(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    let host_is_local = match url.host() {
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    matches!(url.scheme(), "http" | "https") && host_is_local
}

#[cfg(test)]
mod tests {
    use super::is_local;

    #[test]
    fn only_this_machine_is_local() {
        let local = [
            "http://127.0.0.1:9",
            "http://localhost:8/x",
            "https://[::1]:7/",
            "http://localhost\\@evil.example",
        ];
        for url in local {
            assert!(is_local(url), "{url}");
        }
        let remote = [
            "https://example.com",
            "http://127.0.0.1@evil.example",
            "http://evil.example\\@localhost",
            "http://10.0.0.1:9",
            "http://localhost.evil.example",
            "ftp://127.0.0.1",
            "127.0.0.1:9",
        ];
        for url in remote {
            assert!(!is_local(url), "{url}");
        }
    }
}
