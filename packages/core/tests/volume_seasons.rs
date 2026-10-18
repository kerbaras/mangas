//! Volume names and TOC entries of season-numbered (Webtoons) chapters.

use mangas_core::domain::{Chapter, ChapterId, Manga};
use mangas_core::export::volume::{book_meta, plan, Grouping};

fn ch(number: &str, title: &str) -> Chapter {
    Chapter {
        id: ChapterId(format!("c{number}")),
        number: number.into(),
        title: Some(title.into()),
        local_path: Some(format!("/lib/ch{number}")),
        ..Default::default()
    }
}

fn tog() -> Manga {
    Manga {
        title: "Tower of God".into(),
        ..Default::default()
    }
}

fn title_of(chapters: &[Chapter]) -> String {
    plan(&tog(), chapters, Grouping::Single, None, None)
        .remove(0)
        .title
}

#[test]
fn books_without_a_volume_are_named_by_season_tags() {
    assert_eq!(title_of(&[ch("80", "S2 Ep. 0")]), "Tower of God S2 Ep. 0");
    let range = [
        ch("79", "S1 Ep. 78"),
        ch("80", "S2 Ep. 0"),
        ch("81", "S2 Ep. 1"),
    ];
    assert_eq!(title_of(&range), "Tower of God S1 Ep. 78 - S2 Ep. 1");
    assert_eq!(
        title_of(&[ch("79", "S2 Prologue")]),
        "Tower of God S2 Prologue"
    );
    assert_eq!(
        title_of(&[ch("3", "The Return"), ch("4", "S1 Ep. 3")]),
        "Tower of God c3 - S1 Ep. 3"
    );
    assert_eq!(
        title_of(&[ch("3", "The Return"), ch("4", "Home")]),
        "Tower of God c3-4"
    );
}

#[test]
fn toc_entries_use_the_season_label() {
    let chapters = vec![ch("80", "S2 Ep. 0"), ch("81", "S2 Ep. 1 - Return")];
    let book = plan(&tog(), &chapters, Grouping::Single, None, None).remove(0);
    let volume = mangas_core::export::volume::Volume {
        number: book.number,
        chapters: book.chapters,
    };
    let meta = book_meta(&tog(), &volume, None);
    assert_eq!(meta.chapter_titles, ["S2 Ep. 0", "S2 Ep. 1 - Return"]);
}
