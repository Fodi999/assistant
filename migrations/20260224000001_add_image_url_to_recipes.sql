-- Add image_url to recipes table
-- Date: 2026-02-24

ALTER TABLE recipes 
ADD COLUMN IF NOT EXISTS image_url TEXT;

COMMENT ON COLUMN recipes.image_url IS 'URL of the recipe image (optional)';

-- Verification
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'recipes' AND column_name = 'image_url'
    ) THEN
        RAISE EXCEPTION 'Column image_url was not added to recipes table';
    END IF;
END $$;
