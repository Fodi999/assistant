-- Extend product_allergens with 7 remaining EU allergens
ALTER TABLE product_allergens
    ADD COLUMN IF NOT EXISTS peanuts  BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS sesame   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS celery   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS mustard  BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS sulfites BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS lupin    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS molluscs BOOLEAN NOT NULL DEFAULT false;

-- Add multilingual descriptions to products
ALTER TABLE products
    ADD COLUMN IF NOT EXISTS description_en TEXT,
    ADD COLUMN IF NOT EXISTS description_ru TEXT,
    ADD COLUMN IF NOT EXISTS description_pl TEXT,
    ADD COLUMN IF NOT EXISTS description_uk TEXT;

-- Migrate existing single description → description_en (only if column still exists)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'products' AND column_name = 'description'
    ) THEN
        UPDATE products SET description_en = description WHERE description IS NOT NULL;
        ALTER TABLE products DROP COLUMN description;
    END IF;
END $$;
