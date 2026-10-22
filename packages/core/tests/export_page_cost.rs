//! An AZW3 page over 128 KiB also stores a standard copy, so estimates and parts count it.

use mangas_core::export::kindle::{page_cost, split_points};
use mangas_core::export::{Format, PageInfo};

const KIB: u64 = 1024;

fn page(bytes: u64) -> PageInfo {
    PageInfo {
        width: 1236,
        height: 1648,
        bytes: bytes as usize,
    }
}

#[test]
fn an_azw3_page_over_128_kib_costs_a_standard_copy_more_than_in_an_epub() {
    let (small, big) = (100 * KIB, 400 * KIB);
    assert_eq!(
        page_cost(Format::Azw3, small),
        page_cost(Format::Epub, small)
    );
    assert_eq!(
        page_cost(Format::Azw3, big),
        page_cost(Format::Epub, big) + 128 * KIB
    );
    assert_eq!(page_cost(Format::Cbz, big), page_cost(Format::Epub, big));
}

#[test]
fn azw3_parts_hold_fewer_big_pages_so_each_stays_under_the_part_size() {
    let pages: Vec<PageInfo> = (0..40).map(|_| page(400 * KIB)).collect();
    let max = 5_000_000;
    let epub = split_points(&pages, None, max, Format::Epub);
    let azw3 = split_points(&pages, None, max, Format::Azw3);
    assert!(azw3.len() > epub.len(), "{azw3:?} vs {epub:?}");
    for range in &azw3 {
        let bytes: u64 = pages[range.clone()]
            .iter()
            .map(|p| page_cost(Format::Azw3, p.bytes as u64))
            .sum();
        assert!(bytes <= max, "{range:?} costs {bytes}");
    }
}
