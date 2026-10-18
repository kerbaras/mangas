-- A job queued with `force` downloads every page again, overwriting the files already on disk
-- (the user's "Download again", CLI `download --force`).
ALTER TABLE download_jobs ADD COLUMN force INTEGER NOT NULL DEFAULT 0;
