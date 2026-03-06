-- Add per-language description columns to catalog_ingredients
-- Existing `description` column (single-lang) stays for backward compat
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS description_en TEXT,
ADD COLUMN IF NOT EXISTS description_pl TEXT,
ADD COLUMN IF NOT EXISTS description_ru TEXT,
ADD COLUMN IF NOT EXISTS description_uk TEXT;

-- Copy existing generic description into English column where it's not already set
UPDATE catalog_ingredients
SET description_en = description
WHERE description IS NOT NULL AND description_en IS NULL;
