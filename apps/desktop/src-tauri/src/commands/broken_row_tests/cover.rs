//! #199: with the settings row broken, a volume export still embeds the series cover that is
//! already cached in the default data folder.

use std::path::{Path, PathBuf};

use mangas_core::covers::{CoverCache, Limits};
use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::export::volume::Grouping;
use mangas_core::library;

use super::super::open_library;
use super::super::volumes::export_volumes;
use super::chapter_dir;
use crate::paths;

const URL: &str = "https://uploads.mangadex.org/covers/covered/cover.png";

pub(super) async fn the_export_embeds_the_cached_series_cover(home: &Path) {
    let lib = open_library().unwrap();
    let manga = Manga {
        id: MangaId("covered".into()),
        source: "mangadex".into(),
        title: "Covered".into(),
        cover_url: Some(URL.into()),
        ..Default::default()
    };
    let chapter = Chapter {
        id: ChapterId("covered-1".into()),
        manga_id: manga.id.clone(),
        language: "en".into(),
        number: "1".into(),
        ..Default::default()
    };
    library::upsert_manga(&lib, &manga).unwrap();
    let chapters = std::slice::from_ref(&chapter);
    library::upsert_chapters(&lib, "mangadex", &manga.id, "en", chapters).unwrap();
    let dir = chapter_dir(home, "covered");
    library::mark_downloaded(&lib, "mangadex", &chapter.id, &dir).unwrap();
    let covers = paths::covers().unwrap();
    assert_eq!(
        covers,
        home.join("mangas/covers"),
        "the default data folder"
    );
    let cache = CoverCache::new(covers, Limits::default());
    cache.store("mangadex", "covered", URL, &red_png()).unwrap();

    let written = export_volumes(
        tauri::test::mock_app().handle().clone(),
        "mangadex".into(),
        "covered".into(),
        vec!["covered-1".into()],
        Grouping::Single,
        None,
        None,
        Some(mangas_core::export::Format::Epub),
        None,
        false,
        Some(PathBuf::from("covered")),
        None,
        None,
        None,
    )
    .await
    .unwrap()
    .written;
    let book = std::fs::read(&written[0].output).unwrap();
    let entry = b"OEBPS/images/cover.jpg";
    assert!(
        book.windows(entry.len()).any(|w| w == entry),
        "the EPUB has the series cover"
    );
}

fn red_png() -> Vec<u8> {
    let cover = image::RgbImage::from_pixel(60, 90, image::Rgb([200, 30, 30]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    cover.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}
