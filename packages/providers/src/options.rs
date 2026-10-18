//! Provider options from the command line: `key=value` strings checked against the provider's
//! effective options (each value takes the JSON type of the current one) and stored as
//! `Settings.provider_options[id]`, the same entry the desktop settings edit.

use mangas_core::domain::{Error, Result};
use mangas_core::library::Settings;
use serde_json::{Map, Value};

use crate::registry::{find, unknown};

/// Effective options of provider `id`, sorted by key; empty when it has none.
pub fn options(id: &str, settings: &Settings) -> Result<Map<String, Value>> {
    let provider = find(id).ok_or_else(|| unknown(id))?;
    Ok(match provider.options(settings) {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    })
}

/// `settings` with `key=value` assignments applied to provider `id`'s options.
pub fn set_options(id: &str, settings: &Settings, assignments: &[String]) -> Result<Settings> {
    let mut current = options(id, settings)?;
    let mut assigned = Vec::new();
    for assignment in assignments {
        let (key, raw) = split_assignment(assignment)?;
        let old = current
            .get(key)
            .ok_or_else(|| unknown_key(id, key, &current))?;
        let new = coerce(old, raw).ok_or_else(|| invalid(key, raw, old))?;
        current.insert(key.to_string(), new);
        assigned.push((key, raw));
    }
    let mut next = settings.clone();
    next.provider_options
        .insert(id.to_string(), Value::Object(current.clone()));
    // The provider reads values back normalised; anything it changed was not accepted as given.
    let read = options(id, &next)?;
    for (key, raw) in assigned {
        if read.get(key) != current.get(key) {
            let shown = read.get(key).map(display_value).unwrap_or_default();
            return Err(Error::Other(format!(
                "{id} does not accept {key}={raw} (it would read as '{shown}')"
            )));
        }
    }
    Ok(next)
}

/// `settings` without provider `id`'s stored options (back to its defaults).
pub fn reset_options(id: &str, settings: &Settings) -> Result<Settings> {
    find(id).ok_or_else(|| unknown(id))?;
    let mut next = settings.clone();
    next.provider_options.remove(id);
    Ok(next)
}

/// `true`, `["a","b"]` -> `"true"`, `"a,b"`: the form [`set_options`] accepts back.
pub fn display_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(display_value)
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn split_assignment(assignment: &str) -> Result<(&str, &str)> {
    assignment
        .split_once('=')
        .map(|(k, v)| (k.trim(), v.trim()))
        .filter(|(k, _)| !k.is_empty())
        .ok_or_else(|| Error::Other(format!("expected key=value, got '{assignment}'")))
}

/// `raw` parsed as the same JSON type as `old`; lists are comma-separated strings.
fn coerce(old: &Value, raw: &str) -> Option<Value> {
    match old {
        Value::Bool(_) => match raw.to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => Some(Value::Bool(true)),
            "false" | "no" | "off" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
        Value::Array(_) => Some(Value::Array(
            raw.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_string()))
                .collect(),
        )),
        Value::Number(_) => serde_json::from_str::<serde_json::Number>(raw)
            .ok()
            .map(Value::Number),
        Value::String(_) => Some(Value::String(raw.to_string())),
        _ => serde_json::from_str(raw).ok(),
    }
}

fn unknown_key(id: &str, key: &str, current: &Map<String, Value>) -> Error {
    let keys: Vec<&str> = current.keys().map(String::as_str).collect();
    match keys.is_empty() {
        true => Error::Other(format!("{id} has no options")),
        false => Error::Other(format!(
            "unknown {id} option '{key}' (try: {})",
            keys.join(", ")
        )),
    }
}

fn invalid(key: &str, raw: &str, old: &Value) -> Error {
    Error::Other(format!(
        "'{raw}' is not a valid value for {key} (now {old})"
    ))
}
