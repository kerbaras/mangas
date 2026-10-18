//! The HTML steps: Open Graph tags of the title page, canvas search cards and viewer images.
//! Plain string scanning over the server-rendered markup; no DOM.

use url::Url;

/// One canvas title card of `/{lang}/search/canvas`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub number: u64,
    pub title: String,
    pub cover: Option<String>,
}

/// `content` of `<meta property="{property}">`, unescaped.
pub fn og(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{property}\"");
    let start = html.find(&needle)?;
    let tag = &html[start..start + html[start..].find('>')?];
    Some(unescape(attr(tag, "content")?)).filter(|v| !v.is_empty())
}

/// Names on the title page's `.author_area` ("Sangyoung Seong , Sang-A"), before the
/// "author info" button.
pub fn creators(html: &str) -> Vec<String> {
    let Some(area) = between(html, "class=\"author_area\">", "</div>") else {
        return Vec::new();
    };
    let names = area.split("<button").next().unwrap_or_default();
    strip_tags(names)
        .split([',', '/'])
        .map(unescape)
        .filter(|n| !n.is_empty())
        .collect()
}

/// The genre heading of the title page (`<h2 class="genre g_fantasy">Fantasy</h2>`).
pub fn genre(html: &str) -> Option<String> {
    let heading = between(html, "<h2 class=\"genre", "</h2>")?;
    let (_, name) = heading.split_once('>')?;
    Some(unescape(name)).filter(|n| !n.is_empty())
}

fn strip_tags(fragment: &str) -> String {
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
    text
}

/// Canvas cards: links tagged `data-webtoon-type="CHALLENGE"` with their title and thumbnail.
pub fn canvas_cards(html: &str) -> Vec<Card> {
    html.split("data-title-no=\"")
        .skip(1)
        .filter_map(|chunk| {
            let card = &chunk[..chunk.find("</a>").unwrap_or(chunk.len())];
            let (number, rest) = card.split_once('"')?;
            let head = &rest[..rest.find('>').unwrap_or(rest.len())];
            if attr(head, "data-webtoon-type") != Some("CHALLENGE") {
                return None;
            }
            Some(Card {
                number: number.parse().ok()?,
                title: unescape(between(card, "<strong class=\"title\">", "</strong>")?),
                cover: between(card, "<img", ">")
                    .and_then(|img| attr(img, "src"))
                    .map(unescape),
            })
        })
        .filter(|c| !c.title.is_empty())
        .collect()
}

/// Page images of a viewer page, in order: the `_images` of `#_imageList` only (the page also
/// lists every episode thumbnail as `data-url`), without the `type` resize parameter.
pub fn viewer_images(html: &str) -> Vec<String> {
    let Some(start) = html.find("id=\"_imageList\"") else {
        return Vec::new();
    };
    let list = &html[start..];
    let list = &list[..list.find("</div>").unwrap_or(list.len())];
    list.split("<img")
        .skip(1)
        .map(|img| &img[..img.find('>').unwrap_or(img.len())])
        .filter(|img| attr(img, "class").is_some_and(|c| c.split(' ').any(|c| c == "_images")))
        .filter_map(|img| attr(img, "data-url"))
        .map(|url| full_size(&unescape(url)))
        .collect()
}

/// The motion viewer animates layered assets instead of listing images.
pub fn is_motion_toon(html: &str) -> bool {
    html.contains("id=\"ozViewer\"")
}

/// Drops `?type=q90`-style resize parameters so the CDN serves the original image.
pub fn full_size(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw) else {
        return raw.to_string();
    };
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "type")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
    url.to_string()
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    Some(&text[start..start + text[start..].find(close)?])
}

/// Decodes the entities the site emits (named ones it uses, and numeric ones) and trims.
pub fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest.find(';').filter(|&end| end <= 10).and_then(|end| {
            let c = entity(&rest[1..end])?;
            Some((c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(code);
    }
    Some(match name {
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "lt" => '<',
        "gt" => '>',
        "nbsp" => ' ',
        "mdash" => '—',
        "ndash" => '–',
        "hellip" => '…',
        "middot" => '·',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        _ => return None,
    })
}
