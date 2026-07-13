ALTER TABLE church_info
    ADD COLUMN IF NOT EXISTS gallery_images TEXT[] NOT NULL DEFAULT '{}';
