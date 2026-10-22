use super::*;

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
        levers_line(&l),
        "to fit: --every 8 (fewer chapters per book), --split-mb 180 (parts), --quality 80 \
         (lower JPEG quality)"
    );
}

#[test]
fn the_json_fields_are_flat_and_stable() {
    let p = PlannedBook {
        path: "/o/b.epub".into(),
        title: "b".into(),
        chapters: 10,
        budget: book(false, 200_000_000, None),
    };
    let v = serde_json::to_value(&p).unwrap_or_default();
    for key in [
        "estimated_bytes",
        "budget_bytes",
        "over_budget",
        "jpeg_quality",
        "files",
    ] {
        assert!(v.get(key).is_some(), "{key} missing in {v}");
    }
    assert!(v.get("levers").is_none());
}
