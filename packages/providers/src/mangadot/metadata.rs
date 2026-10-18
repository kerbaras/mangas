//! Metadata fields of a MangaDot record, read leniently: the API stores the credit lists as
//! JSON-encoded strings (`"[\"MIURA Kentaro\"]"`) and any field may be null or missing.

use serde_json::Value;

/// Credit names from an encoded string or a plain array; null or garbage is no names.
pub fn names(raw: Option<&Value>) -> Vec<String> {
    match raw {
        Some(Value::String(encoded)) => serde_json::from_str::<Value>(encoded)
            .map(|v| strings(&v))
            .unwrap_or_default(),
        Some(v) => strings(v),
        None => Vec::new(),
    }
}

/// A positive year from a number or a numeric string.
pub fn year(raw: Option<&Value>) -> Option<u16> {
    let year = match raw? {
        Value::Number(n) => n.as_u64()?,
        Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    u16::try_from(year).ok().filter(|y| *y > 0)
}

/// `genres` then the names in `tags[].tags[]`, keeping the first spelling of each label.
pub fn tags(genres: Option<&Value>, groups: Option<&Value>) -> Vec<String> {
    let nested = groups
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|group| group.get("tags")?.as_array())
        .flatten()
        .filter_map(|tag| tag.get("name"))
        .cloned()
        .collect::<Vec<_>>();
    let all = genres.map(strings).unwrap_or_default();
    all.into_iter().chain(strings(&Value::Array(nested))).fold(
        Vec::new(),
        |mut kept: Vec<String>, label| {
            if !kept.iter().any(|k| k.eq_ignore_ascii_case(&label)) {
                kept.push(label);
            }
            kept
        },
    )
}

/// Trimmed, non-empty strings of a JSON array, in order.
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}
