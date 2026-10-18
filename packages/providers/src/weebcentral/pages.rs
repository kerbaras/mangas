//! `/chapters/{id}/images?reading_style=long_strip`: the chapter's page `<img alt="Page N">`
//! in reading order. Images toggled by Alpine (`x-show`) belong to other reading modes.

use mangas_core::domain::Result;
use mangas_core::sources::PageUrls;

use super::SITE;
use crate::html;

pub fn parse_pages(body: &str) -> Result<PageUrls> {
    let doc = html::parse(body);
    let images = html::select(html::root(&doc), r#"img[alt*="Page"]:not([x-show])"#)?;
    let urls = images
        .into_iter()
        .filter_map(|img| html::attr(img, "src"))
        .filter_map(|src| html::absolute(SITE, &src))
        .collect();
    Ok(PageUrls(urls))
}
