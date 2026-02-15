-- Create ingredient dictionary for hybrid translation cache
-- This table stores translations to minimize AI API calls

CREATE TABLE IF NOT EXISTS ingredient_dictionary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name_en TEXT NOT NULL,
    name_pl TEXT NOT NULL,
    name_ru TEXT NOT NULL,
    name_uk TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Prevent duplicate English names (case-insensitive)
    CONSTRAINT check_name_en_not_empty CHECK (TRIM(name_en) != '')
);

-- Case-insensitive unique index on English name for fast lookups
CREATE UNIQUE INDEX IF NOT EXISTS idx_dictionary_lower_en
ON ingredient_dictionary (LOWER(TRIM(name_en)));

-- Index for created_at for potential cleanup queries
CREATE INDEX IF NOT EXISTS idx_dictionary_created_at
ON ingredient_dictionary (created_at DESC);

-- Add comment for documentation
COMMENT ON TABLE ingredient_dictionary IS 
'Hybrid translation cache. Stores ingredient names in 4 languages to avoid repeated AI API calls.
Strategy: Check this table first (0$), only call Groq if not found.
Each translation is cached permanently after first Groq call.';

COMMENT ON COLUMN ingredient_dictionary.name_en IS 'English ingredient name (unique, case-insensitive)';
COMMENT ON COLUMN ingredient_dictionary.name_pl IS 'Polish translation (cached from Groq)';
COMMENT ON COLUMN ingredient_dictionary.name_ru IS 'Russian translation (cached from Groq)';
COMMENT ON COLUMN ingredient_dictionary.name_uk IS 'Ukrainian translation (cached from Groq)';
