-- Update checks: when each chapter was first stored, which chapter languages of a title were
-- synced before (a chapter is only "new" in a language seen before), when the user last opened
-- the title, and whether its new chapters download by themselves.
-- Times use 'YYYY-MM-DD HH:MM:SS.SSS' (UTC) so they compare as text.
ALTER TABLE chapter ADD COLUMN first_seen_at TEXT;
ALTER TABLE manga ADD COLUMN auto_download INTEGER NOT NULL DEFAULT 0;
ALTER TABLE manga ADD COLUMN last_viewed_at TEXT;
UPDATE manga SET last_viewed_at = strftime('%Y-%m-%d %H:%M:%f', 'now');

CREATE TABLE IF NOT EXISTS manga_language_sync (
    source   TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    language TEXT NOT NULL,
    PRIMARY KEY (source, manga_id, language),
    FOREIGN KEY (source, manga_id) REFERENCES manga (source, id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO manga_language_sync (source, manga_id, language)
    SELECT DISTINCT c.source, c.manga_id, c.language FROM chapter c
    JOIN manga m ON m.source = c.source AND m.id = c.manga_id;
