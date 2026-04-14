-- Add plain Water, Celery, and Sour cream to catalog_ingredients
-- Fixes: unresolved "water" in auto_insert_implicit, unresolved "celery" from Gemini
-- Date: 2026-04-14
-- Fix: include category_id (NOT NULL constraint)

-- ── Water (plain) ────────────────────────────────────────────────────────────
INSERT INTO catalog_ingredients (
    name_pl, name_en, name_uk, name_ru,
    slug,
    default_unit, default_shelf_life_days,
    allergens, calories_per_100g, seasons,
    category_id,
    product_type,
    protein_per_100g, fat_per_100g, carbs_per_100g,
    density_g_per_ml,
    is_active
) VALUES (
    'Woda', 'Water', 'Вода', 'Вода',
    'water',
    'liter', 365,
    ARRAY[]::allergen_type[], 0, ARRAY['AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Beverages'),
    'beverage',
    0.0, 0.0, 0.0,
    1.0,
    true
)
ON CONFLICT (slug) WHERE is_active = true DO UPDATE SET
    name_pl = EXCLUDED.name_pl,
    name_en = EXCLUDED.name_en,
    name_uk = EXCLUDED.name_uk,
    name_ru = EXCLUDED.name_ru,
    calories_per_100g = EXCLUDED.calories_per_100g,
    protein_per_100g = EXCLUDED.protein_per_100g,
    fat_per_100g = EXCLUDED.fat_per_100g,
    carbs_per_100g = EXCLUDED.carbs_per_100g,
    product_type = EXCLUDED.product_type,
    density_g_per_ml = EXCLUDED.density_g_per_ml;

-- ── Celery ───────────────────────────────────────────────────────────────────
INSERT INTO catalog_ingredients (
    name_pl, name_en, name_uk, name_ru,
    slug,
    default_unit, default_shelf_life_days,
    allergens, calories_per_100g, seasons,
    category_id,
    product_type,
    protein_per_100g, fat_per_100g, carbs_per_100g,
    density_g_per_ml,
    is_active
) VALUES (
    'Seler', 'Celery', 'Селера', 'Сельдерей',
    'celery',
    'piece', 14,
    ARRAY['Celery']::allergen_type[], 14, ARRAY['AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Vegetables'),
    'vegetable',
    0.7, 0.2, 3.0,
    NULL,
    true
)
ON CONFLICT (slug) WHERE is_active = true DO UPDATE SET
    name_pl = EXCLUDED.name_pl,
    name_en = EXCLUDED.name_en,
    name_uk = EXCLUDED.name_uk,
    name_ru = EXCLUDED.name_ru,
    calories_per_100g = EXCLUDED.calories_per_100g,
    protein_per_100g = EXCLUDED.protein_per_100g,
    fat_per_100g = EXCLUDED.fat_per_100g,
    carbs_per_100g = EXCLUDED.carbs_per_100g,
    product_type = EXCLUDED.product_type,
    allergens = EXCLUDED.allergens;

-- ── Sour cream (explicit entry — "sour-cream" → "cream" rewrite sometimes fails) ──
INSERT INTO catalog_ingredients (
    name_pl, name_en, name_uk, name_ru,
    slug,
    default_unit, default_shelf_life_days,
    allergens, calories_per_100g, seasons,
    category_id,
    product_type,
    protein_per_100g, fat_per_100g, carbs_per_100g,
    density_g_per_ml,
    is_active
) VALUES (
    'Śmietana', 'Sour cream', 'Сметана', 'Сметана',
    'sour-cream',
    'gram', 14,
    ARRAY['Milk']::allergen_type[], 193, ARRAY['AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Dairy & Eggs'),
    'dairy',
    2.4, 19.0, 3.4,
    1.05,
    true
)
ON CONFLICT (slug) WHERE is_active = true DO UPDATE SET
    name_pl = EXCLUDED.name_pl,
    name_en = EXCLUDED.name_en,
    name_uk = EXCLUDED.name_uk,
    name_ru = EXCLUDED.name_ru,
    calories_per_100g = EXCLUDED.calories_per_100g,
    protein_per_100g = EXCLUDED.protein_per_100g,
    fat_per_100g = EXCLUDED.fat_per_100g,
    carbs_per_100g = EXCLUDED.carbs_per_100g,
    product_type = EXCLUDED.product_type;
