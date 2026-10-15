//! Smart cutting of long vertical strips into device pages (port of the Go implementation:
//! row features -> cut costs -> dynamic-programming page boundaries). Implemented in the
//! "export + smartcut + ml" PR; the options type is fixed here so the UI and CLI can be built.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Options {
    /// 1.10 = a page may hold 10% more rows than fit at native zoom; 1.0 = never shrink.
    pub max_shrink: f64,
    /// Cost of shrinking a page by 100% of its capacity.
    pub shrink_penalty: f64,
    /// Rows repeated after a cut through artwork, as a fraction of page capacity.
    pub overlap: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Balanced,
    FewestCuts,
    NoShrink,
}

impl Preset {
    pub fn options(self) -> Options {
        match self {
            Preset::Balanced => Options {
                max_shrink: 1.10,
                shrink_penalty: 30.0,
                overlap: 0.075,
            },
            Preset::FewestCuts => Options {
                max_shrink: 1.25,
                shrink_penalty: 10.0,
                overlap: 0.075,
            },
            Preset::NoShrink => Options {
                max_shrink: 1.0,
                shrink_penalty: 30.0,
                overlap: 0.075,
            },
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Preset::Balanced.options()
    }
}
