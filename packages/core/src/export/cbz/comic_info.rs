//! `ComicInfo.xml` (the Anansi/ComicRack schema 2.0), built as a plain string.

use crate::export::epub::PageImage;
use crate::export::output::xml_escape;
use crate::export::{Meta, Profile};

/// Book metadata and one `<Page>` per image; a chapter's first page carries its title as a
/// `Bookmark`, which readers show as a table of contents. `Series` falls back to the export
/// title when the chapters aren't one library series, so readers still group the file.
pub fn comic_info_xml(pages: &[PageImage], meta: &Meta, profile: &Profile) -> String {
    let labels = &meta.labels;
    let chapter_title = labels.title.as_deref().filter(|_| !meta.title_explicit);
    let title = xml_escape(chapter_title.unwrap_or(&meta.title));
    let series = xml_escape(labels.series.as_deref().unwrap_or(&meta.title));
    let optional: String = [("Number", &labels.number), ("Volume", &labels.volume)]
        .into_iter()
        .filter_map(|(tag, v)| {
            v.as_deref()
                .map(|v| format!("  <{tag}>{}</{tag}>\n", xml_escape(v)))
        })
        .collect();
    let manga = if meta.right_to_left {
        "YesAndRightToLeft"
    } else {
        "Yes"
    };
    let bw = if profile.grayscale { "Yes" } else { "No" };
    let page_list: String = pages
        .iter()
        .enumerate()
        .map(|(i, p)| page_xml(i, p, meta))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <Title>{title}</Title>
  <Series>{series}</Series>
{optional}  <PageCount>{count}</PageCount>
  <LanguageISO>{lang}</LanguageISO>
  <Manga>{manga}</Manga>
  <BlackAndWhite>{bw}</BlackAndWhite>
  <Notes>Exported by mangas for {device}</Notes>
  <Pages>
{page_list}  </Pages>
</ComicInfo>
"#,
        count = pages.len(),
        lang = xml_escape(&meta.language),
        device = xml_escape(profile.name),
    )
}

fn page_xml(index: usize, page: &PageImage, meta: &Meta) -> String {
    let cover = if index == 0 {
        r#" Type="FrontCover""#
    } else {
        ""
    };
    let bookmark = meta
        .chapters
        .iter()
        .find(|c| c.first_page == index)
        .map(|c| format!(r#" Bookmark="{}""#, xml_escape(&c.title)))
        .unwrap_or_default();
    format!(
        r#"    <Page Image="{index}"{cover} ImageSize="{size}" ImageWidth="{w}" ImageHeight="{h}"{bookmark} />
"#,
        size = page.jpeg.len(),
        w = page.width,
        h = page.height,
    )
}
