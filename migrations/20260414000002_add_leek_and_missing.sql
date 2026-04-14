-- Add Leek, Bay leaf, and Noodles to catalog_ingredients
-- Fixes: unresolved "leek" from LLM recipes
-- Date: 2026-04-14

-- ── Leek ─────────────────────────────────────────────────────────────────────
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
    'Por', 'Leek', 'Порей', 'Лук-порей',
    'leek',
    'piece', 10,
    ARRAY[]::allergen_type[], 61, ARRAY['Autumn','Winter','AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Vegetables'),
    'vegetable',
    1.5, 0.3, 14.2,
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
    product_type = EXCLUDED.product_type;

-- ── Bay leaf ─────────────────────────────────────────────────────────────────
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
    'Liść laurowy', 'Bay leaf', 'Лавровий лист', 'Лавровый лист',
    'bay-leaf',
    'piece', 365,
    ARRAY[]::allergen_type[], 313, ARRAY['AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Spices & Herbs'),
    'spice',
    7.6, 8.4, 48.7,
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
    product_type = EXCLUDED.product_type;

-- ── Noodles / Vermicelli ─────────────────────────────────────────────────────
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
    'Makaron nitki', 'Noodles', 'Локшина', 'Вермишель',
    'noodles',
    'gram', 365,
    ARRAY['Wheat']::allergen_type[], 138, ARRAY['AllYear']::season_type[],
    (SELECT id FROM catalog_categories WHERE name_en = 'Grains & Pasta'),
    'grain',
    4.5, 0.4, 25.0,
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
    product_type = EXCLUDED.product_type;
