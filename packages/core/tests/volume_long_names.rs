//! #221: titles too long for the file system get a shortened file name, cut at a character
//! boundary, with room for the ` (n)` suffix and the extension.

use mangas_core::domain::{Chapter, ChapterId, Manga};
use mangas_core::export::volume::{
    file_name, file_name_as, one_file_each, plan, written_names, Grouping, MAX_STEM_BYTES,
};
use mangas_core::export::Format;

/// The longest file name common file systems take (ext4, APFS, NTFS in UTF-16 units).
const NAME_MAX: usize = 255;

fn ch(number: &str, volume: Option<&str>) -> Chapter {
    Chapter {
        id: ChapterId(format!("c{number}")),
        number: number.into(),
        volume: volume.map(Into::into),
        language: "en".into(),
        local_path: Some(format!("/lib/ch{number}")),
        ..Default::default()
    }
}

fn manga(title: &str) -> Manga {
    Manga {
        title: title.into(),
        ..Default::default()
    }
}

fn stem_bytes(name: &str) -> usize {
    name.rsplit_once('.')
        .map_or(name.len(), |(stem, _)| stem.len())
}

#[test]
fn a_long_series_is_shortened_and_the_range_stays() {
    let series = "A".repeat(300);
    let chapters = [ch("1", Some("3")), ch("2", None), ch("3", None)];
    let plans = plan(&manga(&series), &chapters, Grouping::ByVolume, None, None);
    let names: Vec<&str> = plans.iter().map(|p| p.file_name.as_str()).collect();
    assert!(names[0].ends_with("A… v3.epub"), "{names:?}");
    assert!(names[1].ends_with("A… ch 2-3.epub"), "{names:?}");
    for (p, name) in plans.iter().zip(&names) {
        assert!(stem_bytes(name) <= MAX_STEM_BYTES, "{name}");
        assert_eq!(
            p.title,
            format!("{series} {}", &p.title[301..]),
            "title stays whole"
        );
    }
}

#[test]
fn multi_byte_titles_are_cut_on_a_character_boundary() {
    for title in [
        "進撃の巨人".repeat(60),
        "🦊é".repeat(90),
        format!("{} x", "ü".repeat(150)),
    ] {
        let name = file_name(&title);
        assert!(stem_bytes(&name) <= MAX_STEM_BYTES, "{name}");
        assert!(name.ends_with("….epub"), "{name}");
        assert!(title.starts_with(name.trim_end_matches("….epub")), "{name}");
    }
}

#[test]
fn short_names_are_untouched_and_the_cut_name_has_room_for_suffix_and_format() {
    assert_eq!(file_name("Solo v1"), "Solo v1.epub");
    let exact = "b".repeat(MAX_STEM_BYTES);
    assert_eq!(file_name(&exact), format!("{exact}.epub"));
    let long = format!("{}.", "b".repeat(MAX_STEM_BYTES + 1));
    let name = file_name_as(&long, Format::Cbz);
    assert_eq!(name, format!("{}….cbz", "b".repeat(MAX_STEM_BYTES - 3)));
    let numbered = name.replace("….cbz", "… (9999).epub.part");
    assert!(numbered.len() < NAME_MAX, "{}", numbered.len());
}

#[test]
fn a_series_too_long_to_keep_any_of_it_cuts_the_end() {
    let series = "S".repeat(50);
    let template = format!("{{series}} {}", "x".repeat(250));
    let plans = plan(
        &manga(&series),
        &[ch("1", None)],
        Grouping::Single,
        Some(&template),
        None,
    );
    let name = &plans[0].file_name;
    assert!(stem_bytes(name) <= MAX_STEM_BYTES, "{name}");
    assert!(
        name.starts_with(&series) && name.ends_with("x….epub"),
        "{name}"
    );
}

#[test]
fn a_long_template_without_the_longer_series_cuts_the_end() {
    let series = "S".repeat(300);
    let template = format!("Book {}", "x".repeat(250));
    let plans = plan(
        &manga(&series),
        &[ch("1", None)],
        Grouping::Single,
        Some(&template),
        None,
    );
    let name = &plans[0].file_name;
    assert_eq!(
        *name,
        format!("Book {}….epub", "x".repeat(MAX_STEM_BYTES - 8))
    );
}

#[test]
fn written_names_number_the_shortened_name() {
    let dir = tempfile::tempdir().unwrap();
    let series = "Ω".repeat(200);
    let plans = plan(
        &manga(&series),
        &[ch("1", None)],
        Grouping::Single,
        None,
        None,
    );
    let wanted = written_names(&plans, dir.path(), "pdf", false).unwrap();
    std::fs::write(dir.path().join(&wanted[0]), "taken").unwrap();
    let next = written_names(&plans, dir.path(), "pdf", false).unwrap();
    assert_eq!(next[0], wanted[0].replace(".pdf", " (2).pdf"));
    std::fs::write(dir.path().join(&next[0]), "fits").unwrap();
    let replaced = written_names(&one_file_each(plans, true), dir.path(), "pdf", true).unwrap();
    assert_eq!(replaced, wanted);
}
