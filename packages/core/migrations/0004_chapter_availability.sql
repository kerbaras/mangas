-- Whether a chapter can be downloaded: 'available', 'locked' or 'external' (then `external_url`
-- says where it is published). Rows from before this migration are available.
ALTER TABLE chapter ADD COLUMN availability TEXT NOT NULL DEFAULT 'available';
ALTER TABLE chapter ADD COLUMN external_url TEXT;
