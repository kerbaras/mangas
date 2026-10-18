//! Minimal string helpers over Tapas' server-rendered HTML. Pure functions; no DOM.

/// The value of `name="..."` in `tag` (a slice holding one start tag), entity-decoded.
pub fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(unescape(&tag[start..end]))
}

/// Every start tag `<name ...>` in `html`, in document order.
pub fn tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name}");
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(&open) {
        let tail = &rest[start..];
        let Some(end) = tail.find('>') else { break };
        let after = tail.as_bytes().get(open.len()).copied();
        if matches!(after, Some(b' ' | b'\n' | b'\t' | b'\r' | b'/' | b'>')) {
            found.push(&tail[..=end]);
        }
        rest = &tail[end + 1..];
    }
    found
}

/// Text of the first element whose start tag contains `marker`, up to `close`, with nested
/// tags stripped, entities decoded and whitespace collapsed.
pub fn text_after(html: &str, marker: &str, close: &str) -> Option<String> {
    let at = html.find(marker)?;
    let body_start = html[at..].find('>')? + at + 1;
    let body_end = html[body_start..].find(close)? + body_start;
    Some(clean_text(&html[body_start..body_end])).filter(|t| !t.is_empty())
}

/// `content` of `<meta property="{property}">`.
pub fn meta(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{property}\"");
    tags(html, "meta")
        .into_iter()
        .find(|t| t.contains(&needle))
        .and_then(|t| attr(t, "content"))
        .filter(|v| !v.is_empty())
}

pub fn clean_text(fragment: &str) -> String {
    let mut text = String::with_capacity(fragment.len());
    let mut in_tag = false;
    for c in fragment.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    unescape(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
