//! Another group's release of a chapter already stored is not a new chapter.

use mangas_core::domain::{Availability, Chapter, ChapterId};
use mangas_core::library::updates::{new_chapters, Known};

fn chapter(id: &str, number: &str, language: &str) -> Chapter {
    Chapter {
        id: ChapterId(id.into()),
        number: number.into(),
        language: language.into(),
        ..Default::default()
    }
}

#[test]
fn another_release_of_a_stored_chapter_is_not_new() {
    let known = Known {
        chapter_ids: ["a"].map(String::from).into(),
        languages: ["en".to_string()].into(),
        releases: vec![chapter("a", "1.0", "en"), locked("l", "3")],
    };
    let lists = vec![(
        "en".to_string(),
        vec![
            chapter("a", "1.0", "en"),
            chapter("koi-1", "1", "en"),
            chapter("b", "2", "en"),
            locked("l2", "3"),
            chapter("koi-3", "3", "en"),
        ],
    )];
    let new: Vec<String> = new_chapters(&known, &lists)
        .into_iter()
        .map(|c| c.id.0)
        .collect();
    assert_eq!(
        new,
        ["b", "koi-3"],
        "1 = 1.0; the first downloadable 3 is news"
    );
}

fn locked(id: &str, number: &str) -> Chapter {
    Chapter {
        availability: Availability::Locked,
        ..chapter(id, number, "en")
    }
}
