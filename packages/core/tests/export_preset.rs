//! Per-series export presets: storage round trip and explicit > series > defaults precedence.

use std::path::Path;

use mangas_core::domain::{Chapter, ChapterId, Manga, MangaId};
use mangas_core::export::{ExportPreset, Format};
use mangas_core::library::{
    clear_export_preset, get_export_preset, mark_downloaded, open_in_memory, preset_for_dirs,
    remove_manga, series_of_dirs, set_export_preset, upsert_chapters, upsert_manga, Library,
};
use mangas_core::smartcut::Preset;

fn follow(lib: &Library, id: &str, chapters: &[&str]) {
    let manga = Manga {
        id: MangaId(id.into()),
        source: "mangadex".into(),
        title: id.into(),
        ..Default::default()
    };
    upsert_manga(lib, &manga).unwrap();
    let list: Vec<Chapter> = chapters
        .iter()
        .map(|cid| Chapter {
            id: ChapterId((*cid).into()),
            manga_id: manga.id.clone(),
            language: "en".into(),
            number: (*cid).into(),
            ..Default::default()
        })
        .collect();
    upsert_chapters(lib, "mangadex", &manga.id, "en", &list).unwrap();
    for cid in chapters {
        let dir = format!("/lib/{id}/{cid}");
        mark_downloaded(lib, "mangadex", &ChapterId((*cid).into()), Path::new(&dir)).unwrap();
    }
}

fn dirs(d: &[&'static str]) -> Vec<&'static Path> {
    d.iter().map(|s| Path::new(*s)).collect()
}

fn tuned() -> ExportPreset {
    ExportPreset {
        cut_preset: Some(Preset::FewestCuts),
        overlap: Some(0.1),
        page_height: Some(0.6),
        trim_margins: Some(true),
        contrast: Some(1.4),
        format: Some(Format::Cbz),
        profile: Some("kobo-clara".into()),
        ..Default::default()
    }
}

#[test]
fn a_preset_round_trips_and_clears() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "a", &["1"]);
    assert_eq!(get_export_preset(&lib, "mangadex", "a").unwrap(), None);
    set_export_preset(&lib, "mangadex", "a", &tuned()).unwrap();
    assert_eq!(
        get_export_preset(&lib, "mangadex", "a").unwrap(),
        Some(tuned())
    );
    let replaced = ExportPreset {
        max_shrink: Some(1.0),
        ..Default::default()
    };
    set_export_preset(&lib, "mangadex", "a", &replaced).unwrap();
    assert_eq!(
        get_export_preset(&lib, "mangadex", "a").unwrap(),
        Some(replaced)
    );
    assert!(clear_export_preset(&lib, "mangadex", "a").unwrap());
    assert!(!clear_export_preset(&lib, "mangadex", "a").unwrap());
    assert_eq!(get_export_preset(&lib, "mangadex", "a").unwrap(), None);
}

#[test]
fn saving_an_empty_preset_clears_it_and_bad_values_are_refused() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "a", &["1"]);
    set_export_preset(&lib, "mangadex", "a", &tuned()).unwrap();
    set_export_preset(&lib, "mangadex", "a", &ExportPreset::default()).unwrap();
    assert_eq!(get_export_preset(&lib, "mangadex", "a").unwrap(), None);
    for bad in [
        ExportPreset {
            overlap: Some(0.9),
            ..Default::default()
        },
        ExportPreset {
            page_height: Some(0.1),
            ..Default::default()
        },
        ExportPreset {
            gamma: Some(f64::NAN),
            ..Default::default()
        },
        ExportPreset {
            profile: Some("etch-a-sketch".into()),
            ..Default::default()
        },
    ] {
        assert!(
            set_export_preset(&lib, "mangadex", "a", &bad).is_err(),
            "{bad:?}"
        );
    }
    assert!(set_export_preset(&lib, "mangadex", "unknown", &tuned()).is_err());
}

#[test]
fn removing_the_series_removes_its_preset() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "a", &["1"]);
    set_export_preset(&lib, "mangadex", "a", &tuned()).unwrap();
    remove_manga(&lib, "mangadex", &MangaId("a".into())).unwrap();
    follow(&lib, "a", &["1"]);
    assert_eq!(get_export_preset(&lib, "mangadex", "a").unwrap(), None);
}

#[test]
fn chapter_folders_find_their_series_only_when_they_share_one() {
    let lib = open_in_memory().unwrap();
    follow(&lib, "a", &["a1", "a2"]);
    follow(&lib, "b", &["b1"]);
    set_export_preset(&lib, "mangadex", "a", &tuned()).unwrap();
    let a = Some(("mangadex".to_string(), "a".to_string()));
    assert_eq!(
        series_of_dirs(&lib, &dirs(&["/lib/a/a1", "/lib/a/a2/"])).unwrap(),
        a
    );
    assert_eq!(
        series_of_dirs(&lib, &dirs(&["/lib/a/a1", "/lib/b/b1"])).unwrap(),
        None
    );
    assert_eq!(
        series_of_dirs(&lib, &dirs(&["/lib/a/a1", "/elsewhere"])).unwrap(),
        None
    );
    assert_eq!(series_of_dirs(&lib, &[]).unwrap(), None);
    assert_eq!(
        preset_for_dirs(&lib, &dirs(&["/lib/a/a2"])).unwrap(),
        Some(tuned())
    );
    assert_eq!(preset_for_dirs(&lib, &dirs(&["/lib/b/b1"])).unwrap(), None);
}

#[test]
fn explicit_options_beat_the_series_preset_which_beats_the_defaults() {
    let defaults = ExportPreset {
        cut_preset: Some(Preset::NoShrink),
        format: Some(Format::Pdf),
        profile: Some("kindle-paperwhite12".into()),
        ..Default::default()
    };
    let explicit = ExportPreset {
        overlap: Some(0.0),
        format: Some(Format::Epub),
        ..Default::default()
    };
    let r = ExportPreset::resolve(&explicit, Some(&tuned()), &defaults);
    assert_eq!(r.format, Some(Format::Epub), "explicit");
    assert_eq!(r.overlap, Some(0.0), "explicit");
    assert_eq!(r.cut_preset, Some(Preset::FewestCuts), "series");
    assert_eq!(r.profile.as_deref(), Some("kobo-clara"), "series");
    assert_eq!(r.page().height, Some(0.6), "series");
    assert!(r.page().trim_margins, "series");
    let cut = r.cut();
    assert_eq!(cut.max_shrink, Preset::FewestCuts.options().max_shrink);
    assert_eq!(cut.overlap, 0.0);

    let none = ExportPreset::resolve(&explicit, None, &defaults);
    assert_eq!(none.cut_preset, Some(Preset::NoShrink), "defaults");
    assert_eq!(none.format, Some(Format::Epub));
    assert_eq!(
        none.profile_or("kobo-clara").unwrap().id,
        "kindle-paperwhite12"
    );
    assert!(!none.page().trim_margins);
    assert_eq!(none.page(), Default::default());
    let bare = ExportPreset::default();
    assert_eq!(bare.cut(), Preset::Balanced.options());
    assert_eq!(bare.profile_or("kobo-clara").unwrap().id, "kobo-clara");
}
