use super::*;

const fn est(pages: usize, bytes: u64) -> Estimate {
    Estimate { pages, bytes }
}

#[test]
fn email_is_held_to_50_mb_and_everything_else_to_200_mb() {
    assert_eq!(Delivery::Email.budget(), 50_000_000);
    assert_eq!(Delivery::Other.budget(), 200_000_000);
    assert_eq!(Delivery::Email.target(), super::super::PART_SIZE);
    assert_eq!(Delivery::Other.target(), 180_000_000);
}

#[test]
fn a_split_book_is_held_to_its_part_size() {
    let big = est(304, 212_000_000);
    assert_eq!(biggest_file(&big, None), 212_000_000);
    assert_eq!(biggest_file(&big, Some(45_000_000)), 45_000_000);
    assert_eq!(files(&big, None), 1);
    assert_eq!(files(&big, Some(45_000_000)), 5);
    assert_eq!(files(&est(2, 212_000_000), Some(45_000_000)), 2);
}

#[test]
fn fewer_chapters_per_book_scale_with_the_estimate() {
    assert_eq!(chapters_per_book(10, 212_000_000, 180_000_000), Some(8));
    assert_eq!(chapters_per_book(10, 212_000_000, 45_000_000), Some(2));
    assert_eq!(chapters_per_book(10, 2_000_000_000, 45_000_000), Some(1));
    assert_eq!(chapters_per_book(1, 212_000_000, 45_000_000), None);
}
