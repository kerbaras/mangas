//! Query strings in debug logs: search terms are what a user typed, so `-vv` output (often
//! pasted into bug reports) shows parameter names only. `MANGAS_LOG=trace` logs them in full.

/// `[("title", "one piece"), ("limit", "10")]` -> `"?title=…&limit=…"`; empty for no query.
pub fn redact_query(query: &[(&str, String)]) -> String {
    let pairs: Vec<String> = query.iter().map(|(key, _)| format!("{key}=…")).collect();
    match pairs.is_empty() {
        true => String::new(),
        false => format!("?{}", pairs.join("&")),
    }
}
