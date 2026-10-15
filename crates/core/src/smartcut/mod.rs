//! Smart cutting of long vertical strips into device pages (port of the Go implementation):
//! row features -> cut costs -> dynamic-programming page boundaries -> rendered pages.

mod bubbles;
pub mod components;
pub mod costs;
pub mod features;
mod morphology;
pub mod paginate;
mod paginator;
pub mod raster;
pub mod render;
mod rows;
mod text;

use serde::{Deserialize, Serialize};

pub use components::PixelBox;
pub use features::{RowSpan, StripFeatures};
pub use paginate::PageSpan;
pub use paginator::{capacity, is_long_strip, Page, RowPenalizer, StripPaginator, ANALYSIS_WIDTH};

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
