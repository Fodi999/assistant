-- ══════════════════════════════════════════════════════════════════════════════
-- FIX: products.water_type CHECK constraint mismatch
--
-- products table: CHECK (water_type IN ('fresh','salt','both'))
-- catalog_ingredients: CHECK (water_type IN ('sea','freshwater','both'))
-- admin frontend sends: 'freshwater','saltwater','brackish'
-- → NONE match → 500 error on save
--
-- Solution: unify to ('sea','freshwater','brackish','both')
-- ══════════════════════════════════════════════════════════════════════════════

-- 1. Drop old constraint on products
ALTER TABLE products DROP CONSTRAINT IF EXISTS products_water_type_check;

-- 2. Convert existing data from old naming
UPDATE products SET water_type = 'sea'        WHERE water_type = 'salt';
UPDATE products SET water_type = 'freshwater' WHERE water_type = 'fresh';

-- 3. Add unified constraint
ALTER TABLE products ADD CONSTRAINT products_water_type_check
    CHECK (water_type IN ('sea', 'freshwater', 'brackish', 'both'));

-- 4. Also update catalog_ingredients constraint to include 'brackish'
-- The inline CHECK from migration 20260309000007 has auto-generated name.
-- Drop ALL check constraints on water_type column, then re-add.
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT con.conname
        FROM pg_constraint con
        JOIN pg_class rel ON rel.oid = con.conrelid
        JOIN pg_namespace nsp ON nsp.oid = rel.relnamespace
        WHERE rel.relname = 'catalog_ingredients'
          AND con.contype = 'c'
          AND pg_get_constraintdef(con.oid) ILIKE '%water_type%'
    LOOP
        EXECUTE 'ALTER TABLE catalog_ingredients DROP CONSTRAINT ' || r.conname;
    END LOOP;
END $$;

ALTER TABLE catalog_ingredients ADD CONSTRAINT catalog_ingredients_water_type_check
    CHECK (water_type IN ('sea', 'freshwater', 'brackish', 'both'));
