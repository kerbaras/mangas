//! Webtoons ids and the site paths derived from them. Title numbers are per language and per
//! kind (originals vs canvas) and every URL needs both, so a title id is
//! `{lang}-{webtoon|canvas}-{titleNo}` (`en-webtoon-95`); an episode is `{title id}-{episodeNo}`.

use mangas_core::domain::{ChapterId, Error, MangaId, Result};

use super::SOURCE_NAME;

/// Language of a bare title number (`95`), as on the site's default edition.
const BARE_LANGUAGE: &str = "en";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub language: String,
    /// A canvas (user-published) title rather than an original.
    pub canvas: bool,
    pub number: u64,
}

impl Title {
    pub fn original(language: &str, number: u64) -> Self {
        let language = language.to_string();
        Self {
            language,
            canvas: false,
            number,
        }
    }

    pub fn canvas(language: &str, number: u64) -> Self {
        let language = language.to_string();
        Self {
            language,
            canvas: true,
            number,
        }
    }

    /// Parses [`Title::id`]; a bare number is an English original.
    pub fn parse(id: &MangaId) -> Result<Self> {
        if let Ok(number) = id.0.parse() {
            return Ok(Self::original(BARE_LANGUAGE, number));
        }
        let mut parts = id.0.rsplitn(3, '-');
        let (Some(number), Some(kind), Some(language)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(malformed("title", &id.0));
        };
        let number = number.parse().map_err(|_| malformed("title", &id.0))?;
        match kind {
            _ if language.is_empty() => Err(malformed("title", &id.0)),
            "webtoon" => Ok(Self::original(language, number)),
            "canvas" => Ok(Self::canvas(language, number)),
            _ => Err(malformed("title", &id.0)),
        }
    }

    pub fn id(&self) -> MangaId {
        MangaId(format!("{}-{}-{}", self.language, self.kind(), self.number))
    }

    fn kind(&self) -> &'static str {
        if self.canvas {
            "canvas"
        } else {
            "webtoon"
        }
    }

    /// Episode list page; the site redirects it to the canonical `/{lang}/{genre}/{slug}/list`.
    pub fn list_path(&self) -> &'static str {
        if self.canvas {
            "/challenge/episodeList"
        } else {
            "/episodeList"
        }
    }

    pub fn list_query(&self) -> Vec<(&'static str, String)> {
        vec![("titleNo", self.number.to_string())]
    }

    /// Mobile API path listing the episodes.
    pub fn episodes_path(&self) -> String {
        format!("/api/v1/{}/{}/episodes", self.kind(), self.number)
    }

    /// Episode list query: canvas titles also name their reading language.
    pub fn episodes_query(
        &self,
        page_size: usize,
        cursor: Option<u64>,
    ) -> Vec<(&'static str, String)> {
        let mut query = vec![("pageSize", page_size.to_string())];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor.to_string()));
        }
        if self.canvas {
            query.push(("readingLanguageCode", self.language.clone()));
        }
        query
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub title: Title,
    pub number: u64,
}

impl Episode {
    pub fn parse(id: &ChapterId) -> Result<Self> {
        let (title, number) =
            id.0.rsplit_once('-')
                .ok_or_else(|| malformed("episode", &id.0))?;
        Ok(Self {
            title: Title::parse(&MangaId(title.to_string()))?,
            number: number.parse().map_err(|_| malformed("episode", &id.0))?,
        })
    }

    pub fn id(&self) -> ChapterId {
        ChapterId(format!("{}-{}", self.title.id(), self.number))
    }

    /// Viewer page. Any slug works: the site redirects to the canonical one.
    pub fn viewer_path(&self) -> String {
        let lang = &self.title.language;
        if self.title.canvas {
            format!("/{lang}/canvas/a/a/viewer")
        } else {
            format!("/{lang}/a/a/a/viewer")
        }
    }

    pub fn viewer_query(&self) -> Vec<(&'static str, String)> {
        vec![
            ("title_no", self.title.number.to_string()),
            ("episode_no", self.number.to_string()),
        ]
    }
}

fn malformed(what: &str, id: &str) -> Error {
    Error::Source {
        name: SOURCE_NAME.to_string(),
        message: format!("malformed {what} id {id:?}"),
    }
}
