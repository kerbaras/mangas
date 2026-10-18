//! Query strings for the MangaDex endpoints (pure, so they are tested without the network).

/// Max page size of `/manga/{id}/feed`.
pub const FEED_PAGE_SIZE: u32 = 500;
/// What `/manga` and `/manga/{id}` expand inline.
const MANGA_INCLUDES: [&str; 3] = ["cover_art", "author", "artist"];

pub type Query = Vec<(&'static str, String)>;

/// `GET /manga`: titles in the allowed `ratings` with chapters in one of `languages`.
pub fn search_query(title: &str, limit: usize, ratings: &[&str], languages: &[String]) -> Query {
    let mut q: Query = vec![
        ("title", title.to_string()),
        ("limit", limit.clamp(1, 100).to_string()),
        ("order[relevance]", "desc".to_string()),
    ];
    q.extend(MANGA_INCLUDES.map(|i| ("includes[]", i.to_string())));
    q.extend(ratings.iter().map(|r| ("contentRating[]", r.to_string())));
    q.extend(
        languages
            .iter()
            .map(|l| ("availableTranslatedLanguage[]", l.clone())),
    );
    q
}

/// `GET /manga/{id}`.
pub fn manga_query() -> Query {
    MANGA_INCLUDES
        .map(|i| ("includes[]", i.to_string()))
        .to_vec()
}

/// One page of `GET /manga/{id}/feed`. Every rating is requested: the reader already picked
/// the title, and MangaDex hides `erotica`/`pornographic` chapters otherwise.
pub fn feed_query(language: &str, offset: u32) -> Query {
    let mut q: Query = vec![
        ("translatedLanguage[]", language.to_string()),
        ("includes[]", "scanlation_group".to_string()),
        ("order[chapter]", "asc".to_string()),
        ("limit", FEED_PAGE_SIZE.to_string()),
        ("offset", offset.to_string()),
    ];
    q.extend(super::options::ALL_CONTENT_RATINGS.map(|r| ("contentRating[]", r.to_string())));
    q
}
