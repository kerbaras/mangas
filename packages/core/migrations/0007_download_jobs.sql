-- Persistent download queue: one row per chapter. `owner` + `heartbeat_at` lease a running job
-- to one process (desktop app or CLI); a stale heartbeat means that process is gone.
-- Times are unix milliseconds.
CREATE TABLE IF NOT EXISTS download_jobs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    source       TEXT NOT NULL,
    chapter_id   TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'queued',
    attempts     INTEGER NOT NULL DEFAULT 0,
    error        TEXT,
    pages_done   INTEGER NOT NULL DEFAULT 0,
    pages_total  INTEGER NOT NULL DEFAULT 0,
    not_before   INTEGER NOT NULL DEFAULT 0,
    owner        TEXT,
    heartbeat_at INTEGER NOT NULL DEFAULT 0,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    UNIQUE (source, chapter_id),
    FOREIGN KEY (source, chapter_id) REFERENCES chapter (source, id) ON DELETE CASCADE
);
