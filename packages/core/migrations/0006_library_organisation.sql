-- User categories (many-to-many with manga), the user's reading status, when a sync last found a
-- new chapter, and when a chapter was last exported (the app has no reader: exported = read).
CREATE TABLE IF NOT EXISTS category (
    id       INTEGER PRIMARY KEY,
    name     TEXT NOT NULL UNIQUE COLLATE NOCASE,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS manga_category (
    source      TEXT NOT NULL,
    manga_id    TEXT NOT NULL,
    category_id INTEGER NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    PRIMARY KEY (source, manga_id, category_id),
    FOREIGN KEY (source, manga_id) REFERENCES manga (source, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS manga_category_by_category ON manga_category (category_id);

ALTER TABLE manga ADD COLUMN reading_status TEXT NOT NULL DEFAULT 'reading';
ALTER TABLE manga ADD COLUMN updated_at TEXT;
UPDATE manga SET updated_at = added_at;
ALTER TABLE chapter ADD COLUMN exported_at TEXT;
