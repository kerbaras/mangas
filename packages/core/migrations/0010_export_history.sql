-- Books written by an export and where they were sent: one row per export, stamped when the
-- file is emailed or copied to a Kindle. A file sent without an export here gets a row with no
-- `exported_at`.
CREATE TABLE export_history (
    id          INTEGER PRIMARY KEY,
    file        TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    device      TEXT NOT NULL DEFAULT '',
    exported_at TEXT,
    sent_via    TEXT,
    sent_to     TEXT,
    sent_at     TEXT
);
CREATE INDEX export_history_file ON export_history (file);
