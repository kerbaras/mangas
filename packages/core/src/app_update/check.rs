//! The network edge: fetch the channel's manifest and evaluate it.

use std::time::Duration;

use super::{current_target, endpoints, evaluate, parse_manifest, Channel, Status, OVERRIDE_ENV};
use crate::domain::{Error, Result};
use crate::library::Network;
use crate::net;

/// How long a check waits for the release feed before it gives up.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(20);

/// The manifest URL set in [`OVERRIDE_ENV`], if any.
pub fn override_url() -> Option<String> {
    std::env::var(OVERRIDE_ENV)
        .ok()
        .filter(|u| !u.trim().is_empty())
}

/// Checks `channel` for a release newer than `current`, signed by `pubkey`, through `network`
/// (the proxy setting). Endpoints are tried in order; the error of the last one is returned
/// when none answers.
pub async fn check(
    current: &str,
    channel: Channel,
    pubkey: &str,
    network: &Network,
) -> Result<Status> {
    let builder = reqwest::Client::builder()
        .timeout(CHECK_TIMEOUT)
        .user_agent(format!("mangas/{current}"));
    let client = net::apply(builder, network)?.build()?;
    let mut last = Error::Other("no update endpoint".into());
    for url in endpoints(channel, override_url().as_deref()) {
        match fetch(&client, &url).await {
            Ok(body) => {
                let manifest = parse_manifest(&body)?;
                return evaluate(current, &manifest, channel, &current_target(), pubkey);
            }
            Err(e) => {
                tracing::debug!("update manifest {url}: {e}");
                last = e;
            }
        }
    }
    Err(last)
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::Other(format!(
            "update manifest: HTTP {status} from {url}"
        )));
    }
    Ok(response.text().await?)
}
