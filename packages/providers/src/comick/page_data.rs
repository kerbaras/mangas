//! Reading the JSON comick.live inlines in its HTML pages, and turning its HTML descriptions
//! into plain text.

/// Body of `<script ... id="{id}" ...>…</script>`, trimmed.
pub fn script_json<'a>(html: &'a str, id: &str) -> Option<&'a str> {
    let marker = format!("id=\"{id}\"");
    let at = html.find(&marker)?;
    let tag_start = html[..at].rfind("<script")?;
    let body_start = tag_start + html[tag_start..].find('>')? + 1;
    let body_end = body_start + html[body_start..].find("</script>")?;
    Some(html[body_start..body_end].trim()).filter(|s| !s.is_empty())
}

/// Plain text of an HTML fragment: tags dropped (block ends become line breaks), common
/// entities decoded, blank runs collapsed.
pub fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = &rest[open..];
            break;
        };
        let tag = rest[open + 1..open + close].to_ascii_lowercase();
        if tag.starts_with("/p") || tag.starts_with("br") || tag.starts_with("/li") {
            text.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    text.push_str(rest);
    collapse(&decode_entities(&text))
}

fn decode_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Spaces collapsed inside lines, empty lines dropped.
fn collapse(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
