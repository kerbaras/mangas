//! #330: `--eject` after a copy ejects the Kindle that holds the books, not the first one listed.

use std::path::{Path, PathBuf};

use mangas_core::send::usb::{choose_holding, on_kindle, KindleScan, KindleVolume};

fn kindle(root: &Path, name: &str) -> KindleVolume {
    let root = root.join(name);
    std::fs::create_dir_all(root.join("documents")).unwrap();
    KindleVolume {
        name: name.to_string(),
        documents: root.join("documents"),
        root,
        ejectable: true,
    }
}

#[test]
fn picks_the_kindle_holding_the_books() {
    let tmp = tempfile::tempdir().unwrap();
    let (first, second) = (kindle(tmp.path(), "A"), kindle(tmp.path(), "B"));
    std::fs::write(second.documents.join("book.azw3"), b"x").unwrap();
    let books = [PathBuf::from("/out/book.azw3")];
    let scan = || KindleScan {
        volumes: vec![first.clone(), second.clone()],
        mtp: Vec::new(),
    };
    assert_eq!(choose_holding(None, scan(), &books).unwrap(), second);
    assert_eq!(
        on_kindle(&second, &books),
        [second.documents.join("book.azw3")]
    );
    // An explicit mount wins; with no book on any Kindle, the first one listed.
    assert_eq!(
        choose_holding(Some(&first.root), scan(), &books)
            .unwrap()
            .root,
        first.root
    );
    assert_eq!(choose_holding(None, scan(), &[]).unwrap(), first);
}
