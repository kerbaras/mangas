use serde::{Deserialize, Serialize};

/// Whether a chapter can be downloaded from its source. Sources list locked and externally
/// hosted chapters too, so the chapter list keeps the site's numbering without gaps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Availability {
    #[default]
    Available,
    /// Behind a paywall, wait-until-free or early access on the site.
    Locked,
    /// Published on another site (e.g. MANGA Plus), readable at `url`. `None` when the
    /// source gave no https link to show.
    External { url: Option<String> },
}

impl Availability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Why the chapter cannot be downloaded, or `None` when it can.
    pub fn reason(&self) -> Option<String> {
        match self {
            Self::Available => None,
            Self::Locked => {
                Some("it is locked on the site (paid, wait-until-free or early access)".into())
            }
            Self::External { url: Some(url) } => {
                Some(format!("it is hosted on another site: {url}"))
            }
            Self::External { url: None } => Some("it is hosted on another site".into()),
        }
    }

    /// Storage form: the `availability` column and the optional `external_url` column.
    pub fn to_columns(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Available => ("available", None),
            Self::Locked => ("locked", None),
            Self::External { url } => ("external", url.as_deref()),
        }
    }

    /// Inverse of [`Availability::to_columns`]. Unknown kinds read as available, so a bad row
    /// never hides a chapter.
    pub fn from_columns(kind: &str, url: Option<String>) -> Self {
        match (kind, url) {
            ("locked", _) => Self::Locked,
            ("external", url) => Self::External {
                url: url.filter(|u| !u.is_empty()),
            },
            _ => Self::Available,
        }
    }
}
