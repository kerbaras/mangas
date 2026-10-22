use super::*;

#[test]
fn a_full_disk_or_kindle_is_classed_no_space() {
    assert_eq!(
        error_class("not enough space in /out: 3 MB needed"),
        Some("no_space")
    );
    assert_eq!(error_class("io: Permission denied (os error 13)"), None);
    assert_eq!(error_class("io: /not enough space in x: denied"), None);
}

#[test]
fn an_unsent_book_is_classed_by_its_error_not_its_path() {
    use crate::commands::deliver::{not_sent, SendArg};
    let book = Path::new("/a/B.epub was written but not sent: x/B.epub");
    let full = anyhow::anyhow!("not enough space on Kindle at /media/k");
    let unsent = not_sent(book, SendArg::Usb, None, &full);
    assert_eq!(unsent.class, Some("no_space"));
    let tricky = Path::new("/not enough space on k/B.epub");
    let denied = anyhow::anyhow!("io: Permission denied (os error 13)");
    assert_eq!(not_sent(tricky, SendArg::Usb, None, &denied).class, None);
}
