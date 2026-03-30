-- ══════════════════════════════════════════════════════════════════════════════
-- FIX: add 'condiment' and 'poultry' to product_type CHECK constraints
--
-- Code infer_product_type() returns 'condiment' for soy sauce, vinegar, etc.
-- and 'poultry' for chicken, turkey, duck — but these are not in CHECK.
-- This causes: "violates check constraint catalog_ingredients_product_type_check"
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Fix products table ──
ALTER TABLE products DROP CONSTRAINT IF EXISTS products_product_type_check;
ALTER TABLE products ADD CONSTRAINT products_product_type_check
    CHECK (product_type IN (
        'seafood','fish','vegetable','fruit','meat','poultry','grain','dairy',
        'spice','herb','legume','nut','mushroom','oil','beverage',
        'egg','sweetener','sauce','condiment','pasta','bread','other'
    ));

-- ── 2. Fix catalog_ingredients table ──
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
          AND pg_get_constraintdef(con.oid) ILIKE '%product_type%'
    LOOP
        EXECUTE 'ALTER TABLE catalog_ingredients DROP CONSTRAINT ' || r.conname;
    END LOOP;
END $$;

ALTER TABLE catalog_ingredients ADD CONSTRAINT catalog_ingredients_product_type_check
    CHECK (product_type IN (
        'seafood','fish','vegetable','fruit','meat','poultry','grain','dairy',
        'spice','herb','legume','nut','mushroom','oil','beverage',
        'egg','sweetener','sauce','condiment','pasta','bread','other'
    ));

-- ── 3. Fix existing 'condiment' products stuck as 'other' ──
-- (soy sauce was auto-fixed to 'condiment' but DB rejected it, so it's still 'other')
UPDATE catalog_ingredients
SET product_type = 'condiment'
WHERE product_type = 'other'
  AND (LOWER(name_en) LIKE '%soy sauce%'
    OR LOWER(name_en) LIKE '%vinegar%'
    OR LOWER(name_en) LIKE '%ketchup%'
    OR LOWER(name_en) LIKE '%mayonnaise%'
    OR LOWER(name_en) LIKE '%mustard%'
    OR LOWER(name_en) LIKE '%worcestershire%');
