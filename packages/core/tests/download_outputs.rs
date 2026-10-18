use mangas_core::domain::{Chapter, ChapterId, MangaId};
use std::path::{Path, PathBuf};

use mangas_core::download::{cbz_path, chapter_dir, parse_selection, select_chapters, write_cbz};

fn chapter(number: &str) -> Chapter {
    Chapter {
        id: ChapterId(format!("ch-{number}")),
        manga_id: MangaId("m".into()),
        title: None,
        language: "en".into(),
        volume: None,
        number: number.into(),
        local_path: None,
        group: None,
        ..Default::default()
    }
}

#[test]
fn cbz_stores_pages_in_order() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "0002.jpg",
        "0001.png",
        "0003.png.part",
        "0010.jpg",
        "ComicInfo.xml",
        ".DS_Store",
    ] {
        std::fs::write(dir.path().join(name), name).unwrap();
    }
    let out = dir.path().join("out").join("ch.cbz");
    assert_eq!(write_cbz(dir.path(), &out).unwrap(), 3);
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    assert_eq!(names, ["0001.png", "0002.jpg", "0010.jpg"]);
    assert_eq!(
        archive.by_index(0).unwrap().compression(),
        zip::CompressionMethod::Stored
    );
}

#[test]
fn selection_parses_numbers_and_ranges() {
    assert_eq!(
        parse_selection("1-5,7").unwrap(),
        vec![(1.0, 5.0), (7.0, 7.0)]
    );
    assert_eq!(
        parse_selection(" 2.5 , 10-12 ").unwrap(),
        vec![(2.5, 2.5), (10.0, 12.0)]
    );
    assert!(parse_selection("5-1").is_err());
    assert!(parse_selection("abc").is_err());
    assert!(parse_selection("").is_err() || parse_selection("").unwrap().is_empty());
}

#[test]
fn select_chapters_by_published_number() {
    let all: Vec<Chapter> = ["1", "2", "2.5", "3", "7", "Oneshot"]
        .iter()
        .map(|n| chapter(n))
        .collect();
    let picked: Vec<String> = select_chapters(&all, "1-2,7")
        .unwrap()
        .into_iter()
        .map(|c| c.number)
        .collect();
    assert_eq!(picked, ["1", "2", "7"]);
    assert!(select_chapters(&all, "x").is_err());
    assert!(select_chapters(&all, "").is_err());
}

#[test]
fn cbz_path_keeps_decimal_chapter_names() {
    assert_eq!(cbz_path(Path::new("/x/m/2.5")), Path::new("/x/m/2.5.cbz"));
    assert_eq!(cbz_path(Path::new("/x/m/12")), Path::new("/x/m/12.cbz"));
}

#[test]
fn unnumbered_chapters_get_their_own_directory() {
    let root = Path::new("/dl");
    assert_eq!(
        chapter_dir(root, "mangadex", &chapter("2.5")),
        Path::new("/dl/mangadex/m/2.5")
    );
    let a = chapter_dir(root, "mangadex", &chapter(""));
    let mut other = chapter("");
    other.id = mangas_core::domain::ChapterId("other".into());
    assert_eq!(a, Path::new("/dl/mangadex/m/ch-"));
    assert_ne!(a, chapter_dir(root, "mangadex", &other));
}

#[test]
fn chapter_dirs_cannot_escape_the_root() {
    let root = Path::new("/dl");
    let mut evil = chapter("..");
    evil.manga_id = mangas_core::domain::MangaId("../../etc".into());
    let dir = chapter_dir(root, "mangadex", &evil);
    assert!(dir.starts_with("/dl/mangadex"));
    assert!(!dir.components().any(|c| c.as_os_str() == ".."));
    assert_eq!(dir, Path::new("/dl/mangadex/.._.._etc/_"));
}

#[test]
fn same_number_chapters_get_distinct_dirs() {
    use mangas_core::download::unique_chapter_dir;
    let root = Path::new("/dl");
    let first = chapter("12");
    let mut second = chapter("12");
    second.id = mangas_core::domain::ChapterId("abcdef0123456789".into());
    let dir = chapter_dir(root, "mangadex", &first);
    let mut stored = first.clone();
    stored.local_path = Some(dir.to_string_lossy().into_owned());

    assert_eq!(
        unique_chapter_dir(dir.clone(), &first, &[stored.clone()]),
        dir
    );
    assert_eq!(
        unique_chapter_dir(dir.clone(), &second, &[stored]),
        Path::new("/dl/mangadex/m/12-abcdef0123456789")
    );
    assert_eq!(unique_chapter_dir(dir.clone(), &second, &[]), dir);

    let mut evil = chapter("../x");
    evil.id = mangas_core::domain::ChapterId("zz".into());
    let evil_dir = chapter_dir(root, "mangadex", &evil);
    let suffixed = unique_chapter_dir(evil_dir.clone(), &evil, &[stored_at(&evil_dir)]);
    assert_eq!(suffixed.parent(), evil_dir.parent());
    assert!(!suffixed.components().any(|c| c.as_os_str() == ".."));
}

fn stored_at(dir: &Path) -> Chapter {
    let mut c = chapter("other");
    c.local_path = Some(dir.to_string_lossy().into_owned());
    c
}

#[test]
fn page_urls_must_be_public_http() {
    use mangas_core::download::is_public_http_url;
    assert!(is_public_http_url(
        "https://uploads.mangadex.org/data/x/1.png"
    ));
    assert!(is_public_http_url("http://93.184.216.34/p.jpg"));
    for bad in [
        "http://localhost:8080/x",
        "http://foo.localhost/x",
        "http://127.0.0.1/x",
        "http://10.0.0.5/x",
        "http://192.168.1.1/x",
        "http://169.254.169.254/latest/meta-data",
        "http://100.64.0.1/x",
        "http://0.0.0.1/x",
        "http://198.18.0.1/x",
        "http://192.0.0.8/x",
        "http://192.0.2.1/x",
        "http://240.0.0.1/x",
        "http://[::1]/x",
        "http://[fd00::1]/x",
        "http://[::ffff:127.0.0.1]/x",
        "file:///etc/passwd",
        "ftp://example.com/x",
        "not a url",
    ] {
        assert!(!is_public_http_url(bad), "{bad}");
    }
}

#[test]
fn ipv4_compatible_ipv6_is_judged_by_its_ipv4() {
    use mangas_core::download::is_public_ip;
    for bad in ["::127.0.0.1", "::10.0.0.1", "::ffff:169.254.169.254"] {
        assert!(!is_public_ip(bad.parse().unwrap()), "{bad}");
    }
    assert!(is_public_ip("2606:4700::1111".parse().unwrap()));
}

#[test]
fn releases_with_sequential_ids_get_their_own_dirs() {
    use mangas_core::download::unique_chapter_dir;
    let base = chapter_dir(Path::new("/dl"), "mangadot", &chapter("4"));
    let release = |id: &str| Chapter {
        id: ChapterId(id.into()),
        ..chapter("4")
    };
    // An earlier download under the old 8-character name keeps its stored path.
    let old = PathBuf::from("/dl/mangadot/m/4-u_183386");
    let mut legacy = release("u:1833866");
    legacy.local_path = Some(old.to_string_lossy().into_owned());
    let known = [stored_at(&base), legacy];
    let next = unique_chapter_dir(base.clone(), &release("u:1833868"), &known);
    assert_eq!(next, Path::new("/dl/mangadot/m/4-u_1833868"));
    assert_ne!(next, old);
}
