//! Settings edited in one process, saved over what another process stored meanwhile.

use serde_json::{Map, Value};

use super::Settings;
use crate::domain::{Error, Result};

/// Three-way merge. The fields `edited` changed relative to `base` (the copy the edit started
/// from) are applied to `current` (what is stored now); every other field keeps the stored
/// value, so saving an edit never reverts what the CLI or a restore wrote in the meantime.
/// Provider sessions always come from `current`: they are connected and disconnected
/// elsewhere, never edited on a copy.
pub fn merge_edit(base: &Settings, current: &Settings, edited: &Settings) -> Result<Settings> {
    let (base, edited) = (fields(base)?, fields(edited)?);
    let mut merged = fields(current)?;
    let keys = base
        .keys()
        .chain(edited.keys())
        .filter(|k| *k != "provider_sessions");
    for key in keys {
        if base.get(key) == edited.get(key) {
            continue;
        }
        match edited.get(key) {
            Some(value) => merged.insert(key.clone(), value.clone()),
            None => merged.remove(key),
        };
    }
    serde_json::from_value(Value::Object(merged)).map_err(json_error)
}

fn fields(settings: &Settings) -> Result<Map<String, Value>> {
    match serde_json::to_value(settings).map_err(json_error)? {
        Value::Object(map) => Ok(map),
        other => Err(Error::Other(format!("settings: not an object: {other}"))),
    }
}

fn json_error(e: serde_json::Error) -> Error {
    Error::Other(format!("settings: {e}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::library::ProviderSession;

    fn base() -> Settings {
        Settings {
            default_device: Some("kindle-paperwhite12".into()),
            preferred_languages: vec!["en".into()],
            update_check_hours: Some(6),
            ..Default::default()
        }
    }

    #[test]
    fn an_edit_keeps_what_another_process_changed_meanwhile() {
        let current = Settings {
            preferred_languages: vec!["es".into(), "en".into()],
            data_dir: Some("/mnt/manga".into()),
            ..base()
        };
        let edited = Settings {
            default_device: Some("kindle-scribe".into()),
            ..base()
        };
        let merged = merge_edit(&base(), &current, &edited).unwrap();
        assert_eq!(
            merged,
            Settings {
                default_device: Some("kindle-scribe".into()),
                ..current
            }
        );
    }

    #[test]
    fn the_edit_wins_on_a_field_both_changed_and_can_clear_a_field() {
        let current = Settings {
            default_device: Some("kindle-oasis".into()),
            update_check_hours: Some(24),
            ..base()
        };
        let edited = Settings {
            default_device: Some("kindle-scribe".into()),
            update_check_hours: None,
            ..base()
        };
        let merged = merge_edit(&base(), &current, &edited).unwrap();
        assert_eq!(merged.default_device.as_deref(), Some("kindle-scribe"));
        assert_eq!(merged.update_check_hours, None);
    }

    #[test]
    fn an_unchanged_copy_saves_nothing_and_sessions_stay_stored() {
        let session = ProviderSession {
            cookies: "cf=1".into(),
            ..Default::default()
        };
        let current = Settings {
            provider_sessions: BTreeMap::from([("comick".to_string(), session)]),
            default_preset: Some(crate::smartcut::Preset::NoShrink),
            ..base()
        };
        assert_eq!(merge_edit(&base(), &current, &base()).unwrap(), current);
        let edited = Settings {
            provider_sessions: BTreeMap::new(),
            ..base()
        };
        assert_eq!(merge_edit(&base(), &current, &edited).unwrap(), current);
    }
}
