CREATE TABLE IF NOT EXISTS manga (
    id          TEXT NOT NULL,
    source      TEXT NOT NULL,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    cover_url   TEXT,
    status      TEXT NOT NULL DEFAULT 'unknown',
    added_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (source, id)
);

CREATE TABLE IF NOT EXISTS chapter (
    id         TEXT NOT NULL,
    manga_id   TEXT NOT NULL,
    source     TEXT NOT NULL,
    title      TEXT,
    language   TEXT NOT NULL,
    volume     TEXT,
    number     TEXT NOT NULL,
    local_path TEXT,
    PRIMARY KEY (source, id),
    FOREIGN KEY (source, manga_id) REFERENCES manga (source, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS chapter_by_manga ON chapter (source, manga_id);
