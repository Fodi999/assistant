-- Prayer sheet photo, independent of any linked icon image.

ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS image_url TEXT NOT NULL DEFAULT '';
