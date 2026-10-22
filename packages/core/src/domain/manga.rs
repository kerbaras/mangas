use serde::{Deserialize, Serialize};

/// Source-scoped identifier of a manga (e.g. the MangaDex UUID).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MangaId(pub String);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MangaStatus {
    Ongoing,
    Completed,
    Hiatus,
    Cancelled,
    #[default]
    Unknown,
}

/// Which way a title's pages turn: manga read right to left, most others left to right.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingDirection {
    #[default]
    Ltr,
    Rtl,
}

impl ReadingDirection {
    pub fn is_rtl(self) -> bool {
        self == Self::Rtl
    }
}

/// A title as a source describes it. The credits, `year` and `tags` are optional: sources fill
/// what they publish and leave the rest empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manga {
    pub id: MangaId,
    pub source: String,
    pub title: String,
    pub description: String,
    pub cover_url: Option<String>,
    pub status: MangaStatus,
    /// Story credits, in the source's order.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Art credits, in the source's order.
    #[serde(default)]
    pub artists: Vec<String>,
    /// Year of first publication.
    #[serde(default)]
    pub year: Option<u16>,
    /// Genres and themes as the source labels them.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Which way its pages turn, when the source says (or its kind makes it plain).
    #[serde(default)]
    pub direction: Option<ReadingDirection>,
}

impl Manga {
    /// Who a book of this title credits: the authors, then artists not already credited, each
    /// once and trimmed. Empty when the source names nobody; never a placeholder.
    pub fn creators(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for name in self.authors.iter().chain(&self.artists) {
            let name = name.trim();
            if !name.is_empty() && !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
        out
    }
}

impl std::fmt::Display for MangaId {
    /// `pad` keeps width and alignment (`{:<36}`) working in CLI tables.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Manga;

    #[test]
    fn creators_are_authors_then_new_artists() {
        let manga = Manga {
            authors: vec!["Azuma Kiyohiko".into(), " ".into()],
            artists: vec![" Azuma Kiyohiko ".into(), "Studio".into()],
            ..Manga::default()
        };
        assert_eq!(manga.creators(), ["Azuma Kiyohiko", "Studio"]);
        assert!(Manga::default().creators().is_empty());
    }
}
