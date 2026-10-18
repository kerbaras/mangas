-- The scanlation group a followed title is narrowed to (#160): Details shows only its rows and
-- downloads take only its chapters. NULL means every group.
ALTER TABLE manga ADD COLUMN chapter_group TEXT;
