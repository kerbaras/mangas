//! Domain types shared by every module.

mod availability;
mod causes;
mod certificate;
mod chapter;
mod chapter_group;
mod chapter_tag;
mod error;
mod io_at;
mod manga;
mod season_label;
mod tls;

pub use availability::Availability;
pub use causes::with_causes;
pub use certificate::{certificate_problem, CertProblem};
pub use chapter::{compare_numbers, parse_number, sort_by_number, Chapter, ChapterId};
pub use chapter_group::{
    already_have, collapse_groups, groups_of, in_group, normalize_group, same_slot, GroupCount,
};
pub use chapter_tag::{chapter_tag, season_tag, toc_label};
pub use error::{Error, Result};
pub(crate) use io_at::{copy_at, placed_at, unplaced};
pub use io_at::{io_at, IoAt};
pub use manga::{Manga, MangaId, MangaStatus, ReadingDirection};
pub use season_label::season_label;
pub use tls::{
    missing_root_certificates, system_has_root_certificates, webview_tls_failure,
    NO_ROOT_CERTIFICATES,
};
