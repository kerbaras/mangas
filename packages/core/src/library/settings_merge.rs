//! Settings edited in one process, saved over what another process stored meanwhile.

use serde_json::{Map, Value};

use super::Settings;
use crate::domain::{Error, Result};

/// Three-way merge. The fields `edited` changed relative to `base` (the copy the edit started
/// from) are applied to `current` (what is stored now); every other field keeps the stored
/// value, so saving an edit never reverts what the CLI or a restore wrote in the meantime.
/// Objects merge key by key at every depth, so editing one provider option keeps another
/// option (or provider) changed elsewhere. Provider sessions always come from `current`: they are
/// connected and disconnected elsewhere, never edited on a copy.
pub fn merge_edit(base: &Settings, current: &Settings, edited: &Settings) -> Result<Settings> {
    let (mut base, mut edited) = (fields(base)?, fields(edited)?);
    base.remove("provider_sessions");
    edited.remove("provider_sessions");
    let merged = merge_map(&base, fields(current)?, &edited);
    serde_json::from_value(Value::Object(merged)).map_err(json_error)
}

/// `current` with the keys `edited` changed relative to `base`; where both sides hold objects
/// (an entry the copy didn't have counts as empty), they merge the same way.
fn merge_map(
    base: &Map<String, Value>,
    mut current: Map<String, Value>,
    edited: &Map<String, Value>,
) -> Map<String, Value> {
    let empty = Map::new();
    let keys: Vec<String> = base.keys().chain(edited.keys()).cloned().collect();
    for key in keys {
        let (was, now) = (base.get(&key), edited.get(&key));
        if was == now {
            continue;
        }
        match (was, now, current.get(&key)) {
            (b, Some(Value::Object(e)), Some(Value::Object(c)))
                if matches!(b, None | Some(Value::Object(_))) =>
            {
                let b = b.and_then(Value::as_object).unwrap_or(&empty);
                let inner = merge_map(b, c.clone(), e);
                current.insert(key, Value::Object(inner))
            }
            (_, Some(value), _) => current.insert(key, value.clone()),
            (_, None, _) => current.remove(&key),
        };
    }
    current
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
    fn editing_one_provider_keeps_another_providers_change() {
        let base = Settings {
            providers_enabled: BTreeMap::from([("tapas".to_string(), false)]),
            ..base()
        };
        let current = Settings {
            providers_enabled: BTreeMap::from([
                ("tapas".to_string(), false),
                ("comick".to_string(), false),
            ]),
            provider_languages: BTreeMap::from([("mangadex".to_string(), vec!["es".into()])]),
            ..base.clone()
        };
        let edited = Settings {
            providers_enabled: BTreeMap::new(),
            ..base.clone()
        };
        let merged = merge_edit(&base, &current, &edited).unwrap();
        assert_eq!(
            merged.providers_enabled,
            BTreeMap::from([("comick".to_string(), false)])
        );
        assert_eq!(merged.provider_languages, current.provider_languages);
    }

    #[test]
    fn editing_one_provider_option_keeps_another_option_set_meanwhile() {
        let options = |v: serde_json::Value| BTreeMap::from([("mangadex".to_string(), v)]);
        let base = Settings {
            provider_options: options(serde_json::json!({"data_saver": true})),
            ..base()
        };
        let current = Settings {
            provider_options: options(
                serde_json::json!({"data_saver": true, "preferred_groups": ["CLI Group"]}),
            ),
            ..base.clone()
        };
        let edited = Settings {
            provider_options: options(
                serde_json::json!({"data_saver": true, "content_ratings": ["safe", "erotica"]}),
            ),
            ..base.clone()
        };
        let merged = merge_edit(&base, &current, &edited).unwrap();
        assert_eq!(
            merged.provider_options["mangadex"],
            serde_json::json!({
                "data_saver": true,
                "preferred_groups": ["CLI Group"],
                "content_ratings": ["safe", "erotica"],
            })
        );
        let fresh = merge_edit(&Settings::default(), &current, &edited).unwrap();
        assert_eq!(fresh.provider_options, merged.provider_options);
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
