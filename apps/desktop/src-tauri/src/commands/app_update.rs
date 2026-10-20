//! App self-update: check the release channel and install in place with `tauri-plugin-updater`.
//! Channel URLs and the version rule come from `mangas_core::app_update`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mangas_core::app_update::{self, Channel, Status, Version};
use mangas_core::library::Network;
use mangas_core::net;
use serde::Serialize;
use tauri::ipc::Channel as Progress;
use tauri::{AppHandle, State, Url};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use super::errors::to_ipc;
use super::settings_or_default;

/// The update the last check found, kept for `install_app_update`.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

/// Download progress sent to the banner while an update installs.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallProgress {
    Downloading { received: u64, total: Option<u64> },
    Installing,
}

/// Bytes between two progress messages, so a download doesn't flood the IPC channel.
const PROGRESS_STEP: u64 = 256 * 1024;

/// The whole installer download. The updater reuses the check's timeout for it, and 20 s is
/// too short for an installer on a slow link.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn check_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    channel: Channel,
) -> Result<Status, String> {
    let installed = app.package_info().version.clone();
    let current = installed.to_string();
    let pubkey = configured_pubkey(&app)?;
    let offered = Arc::new(Mutex::new(None));
    let found = updater(&app, channel, offered.clone())?
        .check()
        .await
        .map_err(update_error)?;
    let (status, found) = match found {
        Some(update) if !app_update::signed_by(&update.signature, &pubkey) => {
            let version = update.version.clone();
            let status = Status::InvalidSignature {
                current,
                version,
                channel,
            };
            (status, None)
        }
        Some(mut update) => {
            update.timeout = Some(DOWNLOAD_TIMEOUT);
            let status = Status::Available {
                current,
                version: update.version.clone(),
                notes: update.body.clone().filter(|n| !n.trim().is_empty()),
                date: update.date.map(|d| d.to_string()),
                channel,
            };
            (status, Some(update))
        }
        None => match offered.lock().map_err(to_ipc)?.take() {
            Some(remote) if app_update::is_older(&installed, &remote) => {
                let version = remote.to_string();
                (
                    Status::Older {
                        current,
                        version,
                        channel,
                    },
                    None,
                )
            }
            _ => (Status::UpToDate { current, channel }, None),
        },
    };
    *pending.0.lock().map_err(to_ipc)? = found;
    Ok(status)
}

/// `plugins.updater.pubkey` from the app's config: the key updates must be signed with.
fn configured_pubkey(app: &AppHandle) -> Result<String, String> {
    let updater = app.config().plugins.0.get("updater");
    let key = updater
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str());
    key.map(str::to_string)
        .ok_or_else(|| to_ipc("this build has no updater public key"))
}

/// Downloads and installs `version`, found by the last check, then relaunches the app. When
/// checks overlapped, the pending update may not be the one shown: then nothing installs.
#[tauri::command]
#[tracing::instrument(level = "error", skip_all)]
pub async fn install_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    version: String,
    on_progress: Progress<InstallProgress>,
) -> Result<(), String> {
    let update = {
        let mut slot = pending.0.lock().map_err(to_ipc)?;
        match slot.take() {
            Some(update) if update.version == version => update,
            other => {
                *slot = other;
                return Err(to_ipc(format!(
                    "v{version} is no longer the update found; check for updates again"
                )));
            }
        }
    };
    let (mut received, mut sent) = (0u64, 0u64);
    let installed = update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                if received - sent >= PROGRESS_STEP || Some(received) == total {
                    sent = received;
                    let _ = on_progress.send(InstallProgress::Downloading { received, total });
                }
            },
            || {
                let _ = on_progress.send(InstallProgress::Installing);
            },
        )
        .await;
    if let Err(e) = installed {
        *pending.0.lock().map_err(to_ipc)? = Some(update);
        return Err(update_error(e));
    }
    tracing::info!("installed v{}; relaunching", update.version);
    app.restart()
}

/// The updater's error, naming missing root certificates like every other connection does.
fn update_error(e: tauri_plugin_updater::Error) -> String {
    if mangas_core::domain::missing_root_certificates(&e) {
        to_ipc(mangas_core::Error::NoRootCertificates)
    } else {
        to_ipc(e)
    }
}

/// The updater for `channel`; the version it is offered lands in `offered`, so a check that
/// finds nothing to install can still tell an older release from the same one.
fn updater(
    app: &AppHandle,
    channel: Channel,
    offered: Arc<Mutex<Option<Version>>>,
) -> Result<Updater, String> {
    let urls = app_update::endpoints(channel, app_update::override_url().as_deref())
        .iter()
        .map(|u| Url::parse(u))
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_ipc)?;
    let builder = app.updater_builder();
    let builder = match net::effective(&settings_or_default().network()) {
        Network::System => builder,
        Network::Direct => builder.no_proxy(),
        Network::Proxy(url) => builder.proxy(Url::parse(&url).map_err(to_ipc)?),
    };
    builder
        .endpoints(urls)
        .map_err(to_ipc)?
        .version_comparator(move |current, remote| {
            if let Ok(mut slot) = offered.lock() {
                *slot = Some(remote.version.clone());
            }
            app_update::should_update(&current, &remote.version, channel)
        })
        .timeout(app_update::CHECK_TIMEOUT)
        .build()
        .map_err(to_ipc)
}
