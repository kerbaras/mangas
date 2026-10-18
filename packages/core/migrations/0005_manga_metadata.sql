-- Optional title metadata. Credit and tag lists are JSON arrays of strings.
ALTER TABLE manga ADD COLUMN authors TEXT NOT NULL DEFAULT '[]';
ALTER TABLE manga ADD COLUMN artists TEXT NOT NULL DEFAULT '[]';
ALTER TABLE manga ADD COLUMN year INTEGER;
ALTER TABLE manga ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';
