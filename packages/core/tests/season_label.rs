use mangas_core::domain::season_label;

/// (site title, expected label); titles from Tower of God (Webtoons title 95) and other
/// season-numbered originals.
const FIXTURES: &[(&str, Option<&str>)] = &[
    ("[Season 1] Ep. 0", Some("S1 Ep. 0")),
    (
        "[Season 1] Ep. 1 - 1F.Headon's Floor",
        Some("S1 Ep. 1 - 1F.Headon's Floor"),
    ),
    ("[Season 2] Ep. 1", Some("S2 Ep. 1")),
    (
        "[Season 3] Ep. 235 (Season 3 Finale)",
        Some("S3 Ep. 235 (Season 3 Finale)"),
    ),
    ("[Season 2] Ep. 79 + Bonus", Some("S2 Ep. 79 + Bonus")),
    (
        "(Season 2) Episode 10: The Return",
        Some("S2 Ep. 10 - The Return"),
    ),
    ("Season 02 - Ep 007", Some("S2 Ep. 7")),
    ("season 4, episode 12.5", Some("S4 Ep. 12.5")),
    ("[S3] Ep.4", Some("S3 Ep. 4")),
    ("S2 Ep. 5 | Homecoming", Some("S2 Ep. 5 - Homecoming")),
    ("[Season 2] Prologue", Some("S2 Prologue")),
    ("[Season 2] Epilogue", Some("S2 Epilogue")),
    ("  [Season 1]  ", Some("S1")),
    ("Ep. 1 - The Floor of Tests", None),
    ("Episode 5", None),
    ("Seasons Greetings", None),
    ("Season Finale", None),
    ("Spring 2 Ep. 1", None),
    ("S2E5", None),
    ("Special: Season 2 Q&A", None),
    ("", None),
];

#[test]
fn season_titles_become_compact_labels() {
    for (title, want) in FIXTURES {
        assert_eq!(season_label(title).as_deref(), *want, "title {title:?}");
    }
}
