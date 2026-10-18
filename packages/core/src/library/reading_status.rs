use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::Library;
use crate::domain::{Error, MangaId, Result};

/// Where the user is with a followed title. Separate from the publication status the source
/// reports: a completed series can still be `PlanToRead`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    #[default]
    Reading,
    PlanToRead,
    OnHold,
    Completed,
    Dropped,
}

impl ReadingStatus {
    pub const ALL: [ReadingStatus; 5] = [
        Self::Reading,
        Self::PlanToRead,
        Self::OnHold,
        Self::Completed,
        Self::Dropped,
    ];

    /// Column / wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::PlanToRead => "plan_to_read",
            Self::OnHold => "on_hold",
            Self::Completed => "completed",
            Self::Dropped => "dropped",
        }
    }

    /// Unknown values read as the default so a bad row never hides a title.
    pub fn parse(s: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|r| r.as_str() == s)
            .unwrap_or_default()
    }
}

/// Sets the reading status of a followed title.
pub fn set_reading_status(
    lib: &Library,
    source: &str,
    id: &MangaId,
    status: ReadingStatus,
) -> Result<()> {
    let changed = lib.conn()?.execute(
        "UPDATE manga SET reading_status = ?3 WHERE source = ?1 AND id = ?2",
        params![source, id.0, status.as_str()],
    )?;
    if changed == 0 {
        return Err(Error::NotFound(format!(
            "{source} manga {id} is not followed"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_round_trips_and_unknown_reads_as_reading() {
        for s in ReadingStatus::ALL {
            assert_eq!(ReadingStatus::parse(s.as_str()), s);
            let json = serde_json::to_string(&s).unwrap();
            assert_eq!(json, format!("\"{}\"", s.as_str()));
        }
        assert_eq!(ReadingStatus::parse("binge"), ReadingStatus::Reading);
    }
}
