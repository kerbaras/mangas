//! `mangas update --check`: whether a newer mangas is published on the app's release channel.
//! Same manifests, version rule and signing key as Settings → About in the app.

use anyhow::Context;
use mangas_core::app_update::{self, pubkey_from_config, Channel, Status, RELEASES};
use serde_json::json;

use super::stored_settings;
use crate::exit::Exit;
use crate::output::outln;

/// The desktop app's config: its `plugins.updater.pubkey` is the key releases are signed with.
const TAURI_CONFIG: &str = include_str!("../../../desktop/src-tauri/tauri.conf.json");

pub async fn run(channel: Option<&str>, json: bool) -> anyhow::Result<()> {
    match check(channel).await {
        Ok(status) => report(&status, json),
        Err(e) if json => {
            let message = crate::exit::describe(&e);
            outln!("{}", json!({ "kind": "error", "message": message }))?;
            Err(Exit { code: 1, message }.into())
        }
        Err(e) => Err(e),
    }
}

async fn check(channel: Option<&str>) -> anyhow::Result<Status> {
    let settings = stored_settings();
    let channel = match channel {
        Some(name) => Channel::parse(name)
            .ok_or_else(|| anyhow::anyhow!("unknown channel '{name}' (try: stable, rc)"))?,
        None => settings.app_update_channel.unwrap_or_default(),
    };
    let pubkey = pubkey_from_config(TAURI_CONFIG)
        .ok_or_else(|| anyhow::anyhow!("this build has no updater public key"))?;
    app_update::check(
        env!("CARGO_PKG_VERSION"),
        channel,
        &pubkey,
        &settings.network(),
    )
    .await
    .context("couldn't check for updates")
}

fn report(status: &Status, json: bool) -> anyhow::Result<()> {
    if json {
        outln!("{}", serde_json::to_string_pretty(status)?)?;
    } else if !status.is_refusal() {
        outln!("{}", status.line())?;
    }
    if status.is_refusal() {
        return Err(Exit {
            code: 1,
            message: status.line(),
        }
        .into());
    }
    if matches!(status, Status::Available { .. }) && !json {
        outln!("Install it from the app (Settings → About) or {RELEASES}")?;
    }
    Ok(())
}
