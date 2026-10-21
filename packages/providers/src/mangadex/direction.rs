//! Reading direction of a MangaDex title, from its original language and format tags.

use mangas_core::domain::ReadingDirection;

use super::api::MangaAttributes;

/// Japanese manga read right to left; Korean, Chinese and long-strip (webtoon) titles left to
/// right. Other languages: the site doesn't say.
pub(super) fn direction(attrs: &MangaAttributes) -> Option<ReadingDirection> {
    let long_strip = attrs.tags.iter().any(|t| {
        t.attributes
            .name
            .get("en")
            .is_some_and(|n| n == "Long Strip")
    });
    match attrs.original_language.as_deref()? {
        _ if long_strip => Some(ReadingDirection::Ltr),
        "ja" => Some(ReadingDirection::Rtl),
        "ko" | "zh" | "zh-hk" => Some(ReadingDirection::Ltr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(language: Option<&str>, tags: &[&str]) -> MangaAttributes {
        let tags: Vec<_> = tags
            .iter()
            .map(|t| serde_json::json!({"id": t, "attributes": {"name": {"en": t}}}))
            .collect();
        serde_json::from_value(serde_json::json!({"originalLanguage": language, "tags": tags}))
            .unwrap()
    }

    #[test]
    fn japanese_manga_read_right_to_left_and_strips_left_to_right() {
        assert_eq!(
            direction(&attrs(Some("ja"), &["Action"])),
            Some(ReadingDirection::Rtl)
        );
        assert_eq!(
            direction(&attrs(Some("ja"), &["Long Strip"])),
            Some(ReadingDirection::Ltr)
        );
        assert_eq!(
            direction(&attrs(Some("ko"), &[])),
            Some(ReadingDirection::Ltr)
        );
        assert_eq!(
            direction(&attrs(Some("zh"), &[])),
            Some(ReadingDirection::Ltr)
        );
        assert_eq!(direction(&attrs(Some("en"), &[])), None);
        assert_eq!(direction(&attrs(None, &[])), None);
    }
}
