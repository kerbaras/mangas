-- Which way a title's pages turn as its source describes it (#325): 'rtl' or 'ltr'. NULL when the
-- source doesn't say; Kindle exports then read left to right unless the series preset says otherwise.
ALTER TABLE manga ADD COLUMN direction TEXT;
