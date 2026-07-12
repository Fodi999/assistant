-- Source attribution fields and two extra prayer types (velichanie, modern).

ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT '';
ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS source_url TEXT NOT NULL DEFAULT '';
ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS note TEXT NOT NULL DEFAULT '';

ALTER TABLE church_prayers DROP CONSTRAINT IF EXISTS church_prayers_type_check;
ALTER TABLE church_prayers ADD CONSTRAINT church_prayers_type_check
    CHECK (prayer_type = ANY (ARRAY['prayer', 'akathist', 'troparion', 'kontakion', 'velichanie', 'modern']));
