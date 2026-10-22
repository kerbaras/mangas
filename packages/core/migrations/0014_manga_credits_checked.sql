-- When the update check last re-fetched an uncredited title's record, so it does so once.
ALTER TABLE manga ADD COLUMN credits_checked_at TEXT;
