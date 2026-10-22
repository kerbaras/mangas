use super::*;
use crate::domain::{Chapter, ChapterId};

fn plan(file: &str, ids: &[&str]) -> VolumePlan {
    let chapter = |id: &&str| Chapter {
        id: ChapterId(id.to_string()),
        ..Default::default()
    };
    VolumePlan {
        number: None,
        title: file.into(),
        file_name: file.into(),
        chapters: ids.iter().map(chapter).collect(),
        authors: Vec::new(),
    }
}

fn kept(books: &[(&[&str], &str)]) -> Vec<KeptBook> {
    let one = |(ids, file): &(&[&str], &str)| KeptBook {
        chapters: ids.iter().map(|id| id.to_string()).collect(),
        file: file.to_string(),
    };
    books.iter().map(one).collect()
}

fn files(plans: &[VolumePlan]) -> Vec<&str> {
    plans.iter().map(|p| p.file_name.as_str()).collect()
}

#[test]
fn skips_the_written_books_even_when_titles_repeat() {
    let plans = vec![
        plan("Big ch 5", &["a5"]),
        plan("Big ch 5", &["b5"]),
        plan("Big ch 6", &["a6"]),
    ];
    let left = skip_written(plans.clone(), &kept(&[(&["a5"], "Big ch 5.epub")]));
    assert_eq!(files(&left), ["Big ch 5", "Big ch 6"]);
    assert_eq!(skip_written(plans.clone(), &[]), plans);
}

#[test]
fn a_book_planned_ahead_of_the_kept_one_since_the_stop_is_written() {
    // The stopped run wrote v1 (ch 1-2) and failed on v2; ch 0 finished downloading after.
    let now = vec![
        plan("v0", &["c0"]),
        plan("v1", &["c2", "c1"]),
        plan("v2", &["c3"]),
    ];
    let left = skip_written(now, &kept(&[(&["c1", "c2"], "v1.epub")]));
    assert_eq!(files(&left), ["v0", "v2"]);
}

#[test]
fn a_kept_book_that_gained_a_chapter_is_a_different_book() {
    let now = vec![plan("v1", &["c1", "c2", "c2b"]), plan("v2", &["c3"])];
    let left = skip_written(now, &kept(&[(&["c1", "c2"], "v1.epub"), (&[], "x.epub")]));
    assert_eq!(files(&left), ["v1", "v2"]);
}

#[test]
fn replacing_never_numbers_a_book_onto_a_kept_file() {
    // Same-titled releases: the stopped run kept "X ch 1.epub" (b1) and failed on "X ch 1 (2)"
    // (c1); a1 landed since and plans first.
    let now = vec![
        plan("X ch 1", &["a1"]),
        plan("X ch 1", &["b1"]),
        plan("X ch 1", &["c1"]),
    ];
    let keep = kept(&[(&["b1"], "X ch 1.epub")]);
    assert_eq!(
        files(&retry_plans(now.clone(), &keep, "epub", true)),
        ["X ch 1 (2)", "X ch 1 (3)"]
    );
    // Without Replace the writer picks free names on disk, so the planned names stay.
    assert_eq!(
        files(&retry_plans(now, &keep, "epub", false)),
        ["X ch 1", "X ch 1"]
    );
}

#[test]
fn a_kept_split_book_keeps_its_name_too() {
    let now = vec![plan("X ch 1", &["a1"]), plan("X ch 1", &["b1"])];
    let keep = kept(&[(&["b1"], "X ch 1 (part 1).epub")]);
    assert_eq!(
        files(&retry_plans(now, &keep, "epub", true)),
        ["X ch 1 (2)"]
    );
}

#[test]
fn with_nothing_kept_replacing_numbers_like_one_file_each() {
    let now = vec![plan("X ch 1", &["a1"]), plan("X ch 1", &["b1"])];
    let expect = super::super::one_file_each(now.clone(), true);
    assert_eq!(retry_plans(now, &[], "epub", true), expect);
}
