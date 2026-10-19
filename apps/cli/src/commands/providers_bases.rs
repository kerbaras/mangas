//! `mangas providers --base ID=URL`: probe a provider at a mock server instead of its site.

use std::collections::HashMap;
use std::net::IpAddr;

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

/// Whether `url` is http(s) on `localhost` or a loopback address.
fn is_local(url: &str) -> bool {
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
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
        ];
        for url in local {
            assert!(is_local(url), "{url}");
        }
        let remote = [
            "https://example.com",
            "http://127.0.0.1@evil.example",
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
