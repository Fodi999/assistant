-- ════════════════════════════════════════════════════════════════════════
-- Fix food_pairing FK: drop products FK, add catalog_ingredients FK
-- This allows pairings for products created via admin catalog
-- (catalog_ingredients is the source of truth for admin CRUD)
-- ════════════════════════════════════════════════════════════════════════

-- 1. Drop old FK constraints referencing products(id)
ALTER TABLE food_pairing DROP CONSTRAINT IF EXISTS food_pairing_ingredient_a_fkey;
ALTER TABLE food_pairing DROP CONSTRAINT IF EXISTS food_pairing_ingredient_b_fkey;

-- 2. Add new FK constraints referencing catalog_ingredients(id)
ALTER TABLE food_pairing
    ADD CONSTRAINT food_pairing_ingredient_a_fkey
    FOREIGN KEY (ingredient_a) REFERENCES catalog_ingredients(id) ON DELETE CASCADE;

ALTER TABLE food_pairing
    ADD CONSTRAINT food_pairing_ingredient_b_fkey
    FOREIGN KEY (ingredient_b) REFERENCES catalog_ingredients(id) ON DELETE CASCADE;

-- 3. Sync: ensure all catalog_ingredients IDs also exist in products table
-- (for backward compat with nutrition queries that JOIN products)
INSERT INTO products (id, slug, name_en, name_pl, name_ru, name_uk, category_id, product_type, unit, image_url)
SELECT ci.id, ci.slug, ci.name_en, ci.name_pl, ci.name_ru, ci.name_uk,
       ci.category_id, COALESCE(ci.product_type, 'other'), ci.default_unit::text, ci.image_url
FROM catalog_ingredients ci
WHERE ci.is_active = true
  AND NOT EXISTS (SELECT 1 FROM products p WHERE p.id = ci.id)
ON CONFLICT (id) DO NOTHING;
