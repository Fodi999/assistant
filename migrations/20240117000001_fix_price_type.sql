-- Change price type to NUMERIC for proper Decimal handling in Rust
-- Safe: if column doesn't exist or already NUMERIC, no error
DO $$
BEGIN
    -- Check if column exists and is not already NUMERIC
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'catalog_ingredients' 
        AND column_name = 'price'
        AND data_type != 'numeric'
    ) THEN
        ALTER TABLE catalog_ingredients ALTER COLUMN price TYPE NUMERIC(10, 2);
    END IF;
END $$;
