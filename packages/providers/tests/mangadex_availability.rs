use mangas_core::domain::{Availability, Chapter, MangaId};
use mangas_providers::mangadex::{dedupe_chapters, parse_feed_response, secure_link, FeedChapter};

const EMINENCE: &str = "91d928a1-7237-4d03-abc3-4b1440cc7d55";

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/mangadex/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn dedupe_and_sort(entries: Vec<FeedChapter>) -> Vec<Chapter> {
    dedupe_chapters(entries, &[])
}

#[test]
fn hosted_upload_beats_an_external_one_with_the_same_number() {
    let id = MangaId(EMINENCE.into());
    let mut entries = parse_feed_response(&fixture("feed_hosted.json"), &id, "en")
        .unwrap()
        .chapters;
    let newest_ch1 = entries
        .iter_mut()
        .filter(|e| e.chapter.number == "1")
        .max_by(|a, b| a.published_at.cmp(&b.published_at))
        .unwrap();
    newest_ch1.chapter.availability = Availability::External {
        url: Some("https://mangaplus.shueisha.co.jp/viewer/1".into()),
    };
    let unique = dedupe_and_sort(entries);
    let ch1 = unique.iter().find(|c| c.number == "1").unwrap();
    assert!(ch1.is_available(), "the newer external upload must not win");
}

#[test]
fn external_chapter_alone_keeps_its_number_in_the_list() {
    let id = MangaId(EMINENCE.into());
    let mut entries = parse_feed_response(&fixture("feed_hosted.json"), &id, "en")
        .unwrap()
        .chapters;
    for e in entries.iter_mut().filter(|e| e.chapter.number == "2.2") {
        e.chapter.availability = Availability::External {
            url: Some("https://mangaplus.shueisha.co.jp/viewer/2".into()),
        };
    }
    let numbers: Vec<_> = dedupe_and_sort(entries)
        .into_iter()
        .map(|c| {
            let ok = c.is_available();
            (c.number, ok)
        })
        .collect();
    assert_eq!(
        numbers,
        [
            ("1".to_string(), true),
            ("2.1".to_string(), true),
            ("2.2".to_string(), false)
        ]
    );
}

#[test]
fn secure_link_upgrades_http_and_drops_other_schemes() {
    let https = "https://mangaplus.shueisha.co.jp/viewer/1";
    assert_eq!(secure_link(https).as_deref(), Some(https));
    assert_eq!(
        secure_link("HTTP://mangaplus.shueisha.co.jp/viewer/1").as_deref(),
        Some(https)
    );
    for bad in [
        "javascript://alert(1)",
        "file:///etc/passwd",
        "mailto:x@y.z",
        "http://",
        "https://exa mple.com/1",
        "https://[::1/viewer",
        "x",
    ] {
        assert_eq!(secure_link(bad), None, "{bad}");
    }
}

const WEBNOVEL: &str = "https://www.webnovel.com/comic/15227640605485101/45196186038101142";

/// The fixture is raw API output, which escapes `/` as `\/`.
fn escaped(url: &str) -> String {
    url.replace('/', "\\/")
}

fn first_availability_with_url(raw: &str) -> Availability {
    let json = fixture("feed.json").replace(&escaped(WEBNOVEL), &escaped(raw));
    let id = MangaId("32d76d19-8a05-4db0-9fc2-e0b0648fe9d0".into());
    let page = parse_feed_response(&json, &id, "en").unwrap();
    page.chapters[0].chapter.availability.clone()
}

#[test]
fn feed_upgrades_http_links_and_keeps_others_external_without_a_link() {
    let http = WEBNOVEL.replacen("https", "http", 1);
    assert_eq!(
        first_availability_with_url(&http),
        Availability::External {
            url: Some(WEBNOVEL.into())
        }
    );
    assert_eq!(
        first_availability_with_url("ftp://www.webnovel.com/comic/1"),
        Availability::External { url: None }
    );
}

#[test]
fn external_chapter_with_a_link_beats_one_without() {
    let id = MangaId(EMINENCE.into());
    let mut entries = parse_feed_response(&fixture("feed_hosted.json"), &id, "en")
        .unwrap()
        .chapters;
    entries.retain(|e| e.chapter.number == "1");
    assert!(
        entries.len() > 1,
        "fixture has duplicate uploads of chapter 1"
    );
    let newest = entries
        .iter()
        .map(|e| e.published_at.clone())
        .max()
        .unwrap();
    let link = "https://mangaplus.shueisha.co.jp/1";
    for e in entries.iter_mut() {
        let url = (e.published_at != newest).then(|| link.to_string());
        e.chapter.availability = Availability::External { url };
    }
    let unique = dedupe_and_sort(entries);
    let url = Some(link.to_string());
    assert_eq!(unique[0].availability, Availability::External { url });
}
