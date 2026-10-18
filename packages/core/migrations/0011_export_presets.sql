-- Export preset of a series: smart-cut options, page tweaks, format and profile as JSON
-- (`export::ExportPreset`, every field optional). Exports of the series use it by default.
CREATE TABLE export_preset (
    source     TEXT NOT NULL,
    manga_id   TEXT NOT NULL,
    preset     TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (source, manga_id),
    FOREIGN KEY (source, manga_id) REFERENCES manga (source, id) ON DELETE CASCADE
);
