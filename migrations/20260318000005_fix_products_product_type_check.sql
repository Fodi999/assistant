-- ══════════════════════════════════════════════════════════════════════════════
-- FIX: products.product_type CHECK constraint — add missing types
--
-- Current CHECK (both tables):
--   'seafood','vegetable','fruit','meat','grain','dairy','spice','herb',
--   'legume','nut','mushroom','other'
--
-- Admin frontend also uses: 'fish','oil','beverage'
-- → 500 error when saving product_type = 'fish'
--
-- Solution: add 'fish','oil','beverage','egg','sweetener','sauce','pasta','bread'
-- to both tables for future-proofing
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Fix products table ──
ALTER TABLE products DROP CONSTRAINT IF EXISTS products_product_type_check;
ALTER TABLE products ADD CONSTRAINT products_product_type_check
    CHECK (product_type IN (
        'seafood','fish','vegetable','fruit','meat','grain','dairy',
        'spice','herb','legume','nut','mushroom','oil','beverage',
        'egg','sweetener','sauce','pasta','bread','other'
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
        'seafood','fish','vegetable','fruit','meat','grain','dairy',
        'spice','herb','legume','nut','mushroom','oil','beverage',
        'egg','sweetener','sauce','pasta','bread','other'
    ));
