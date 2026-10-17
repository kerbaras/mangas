//! Page URLs from the MangaDex@Home server and the uploads mirror.

use mangas_core::sources::PageUrls;
use url::Url;

use super::api::AtHome;
use super::mapping::parse;
use mangas_core::domain::Error;

/// Origin serving every chapter file; used when an MD@Home node fails.
pub const UPLOADS_ORIGIN: &str = "https://uploads.mangadex.org";

/// `GET /at-home/server/{chapter}` body -> page URLs on the assigned node, compressed
/// (`data-saver`) when `data_saver` is set and the chapter has them.
pub fn parse_at_home_response(body: &str, data_saver: bool) -> Result<PageUrls, Error> {
    let at_home: AtHome = parse(body)?;
    let base = at_home.base_url.trim_end_matches('/');
    let chapter = at_home.chapter;
    let (dir, files) = if data_saver && !chapter.data_saver.is_empty() {
        ("data-saver", chapter.data_saver)
    } else {
        ("data", chapter.data)
    };
    let hash = chapter.hash;
    Ok(PageUrls(
        files
            .iter()
            .map(|file| format!("{base}/{dir}/{hash}/{file}"))
            .collect(),
    ))
}

/// The same page on [`UPLOADS_ORIGIN`], for a page URL from [`parse_at_home_response`].
/// `None` for anything else (already on uploads, not a chapter file).
pub fn uploads_mirror(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let uploads = Url::parse(UPLOADS_ORIGIN).ok()?;
    let path = chapter_path(parsed.path())?;
    (parsed.host() != uploads.host()).then(|| format!("{UPLOADS_ORIGIN}{path}"))
}

/// `/data/...` or `/data-saver/...`; MD@Home nodes may prefix it with a token
/// (`/{token}/data/...`) that uploads does not take.
fn chapter_path(path: &str) -> Option<&str> {
    ["/data/", "/data-saver/"]
        .iter()
        .filter_map(|dir| path.find(dir))
        .min()
        .map(|i| &path[i..])
}
