use super::*;
use crate::export::kindle::levers::most_that_fit;

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
fn fewer_chapters_per_book_fit_the_longest_run_of_chapters() {
    let even = [30; 10];
    assert_eq!(most_that_fit(&even, 700_000, 180_000_000), Some(8));
    assert_eq!(most_that_fit(&even, 700_000, 45_000_000), Some(2));
    // The longest run of chapters decides it, not the average.
    let front = [32, 32, 32, 32, 32, 32, 31, 31, 25, 25];
    assert_eq!(most_that_fit(&front, 900_000, 180_000_000), Some(6));
    assert_eq!(most_that_fit(&even, 7_000_000, 45_000_000), None);
    assert_eq!(most_that_fit(&[300], 700_000, 45_000_000), None);
}
