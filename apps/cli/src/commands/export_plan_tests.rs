use super::*;
use mangas_core::export::KINDLE_REASON;

fn book(over: bool, budget: u64, levers: Option<Levers>) -> BookBudget {
    BookBudget {
        pages: 304,
        estimated_bytes: 212_000_000,
        budget_bytes: budget,
        over_budget: over,
        jpeg_quality: 90,
        files: 1,
        levers,
    }
}

#[test]
fn the_line_names_the_budget_of_the_delivery_and_the_quality() {
    let web = budget_line(&book(true, 200_000_000, None));
    assert_eq!(
        web,
        "about 212.0 MB for the 200 MB Send-to-Kindle web budget, 304 pages, JPEG quality 90: \
         over budget"
    );
    let email = budget_line(&book(true, 50_000_000, None));
    assert!(
        email.contains("50 MB Send-to-Kindle email budget"),
        "{email}"
    );
}

#[test]
fn an_over_budget_book_lists_every_lever_as_a_flag() {
    let l = Levers {
        split_bytes: 180_000_000,
        chapters_per_book: Some(8),
        jpeg_quality: Some(80),
    };
    assert_eq!(
        levers_line(&l, Fewer::Every),
        "to fit: --every 8 (fewer chapters per book), --split-mb 180 (parts), --quality 80 \
         (lower JPEG quality)"
    );
    assert!(levers_line(&l, Fewer::Ranges).starts_with("to fit: --chapters in ranges of 8 (fewer"));
    assert!(
        levers_line(&l, Fewer::Folders).starts_with("to fit: 8 chapter folders per book (fewer")
    );
    let none = Levers {
        jpeg_quality: None,
        chapters_per_book: None,
        ..l
    };
    assert_eq!(
        levers_line(&none, Fewer::Every),
        "to fit: --split-mb 180 (parts), no JPEG quality down to 50 fits"
    );
}

#[test]
fn the_json_fields_are_flat_and_stable() {
    let p = PlannedBook {
        path: "/o/b.epub".into(),
        title: "b".into(),
        chapters: 10,
        format: Format::Azw3,
        format_reason: Some(KINDLE_REASON),
        budget: book(false, 200_000_000, None),
    };
    let v = serde_json::to_value(&p).unwrap_or_default();
    for key in [
        "estimated_bytes",
        "budget_bytes",
        "over_budget",
        "jpeg_quality",
        "files",
        "format",
        "format_reason",
    ] {
        assert!(v.get(key).is_some(), "{key} missing in {v}");
    }
    assert!(v.get("levers").is_none());
}

#[test]
fn a_failed_book_is_named_as_the_writer_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let book = dir.path().join("Book.epub");
    assert_eq!(planned_book(&book, false), book);
    std::fs::write(&book, b"older").unwrap();
    assert_eq!(planned_book(&book, false), dir.path().join("Book (2).epub"));
    assert_eq!(planned_book(&book, true), book);
}
