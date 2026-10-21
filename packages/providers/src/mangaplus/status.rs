//! A title's publication status from its details. The title list carries none, so search
//! results stay `Unknown` (shown without a label).

use mangas_core::domain::MangaStatus;

use super::proto::TitleDetailView;

const COMPLETED: i32 = 8;
const ONE_SHOT: i32 = 9;

/// Completed when the schedule, genre or note says so, on hiatus when the note says so,
/// ongoing only when a release schedule or a next chapter date says so.
pub fn status(detail: &TitleDetailView) -> MangaStatus {
    let schedule = detail
        .title_labels
        .as_ref()
        .map_or(0, |l| l.release_schedule);
    let note = detail.non_appearance_info.to_lowercase();
    let one_shot = detail.genre_list.iter().any(|g| g.slug == "one-shot");
    let completed_note = ["completado", "complete", "completo"]
        .iter()
        .any(|w| note.contains(w));
    match schedule {
        COMPLETED | ONE_SHOT => MangaStatus::Completed,
        _ if one_shot || completed_note => MangaStatus::Completed,
        _ if note.contains("hiatus") => MangaStatus::Hiatus,
        1..=6 => MangaStatus::Ongoing,
        _ if detail.next_time_stamp > 0 => MangaStatus::Ongoing,
        _ => MangaStatus::Unknown,
    }
}
