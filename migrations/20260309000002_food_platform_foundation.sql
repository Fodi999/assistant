-- ══════════════════════════════════════════════════════════════════════════════
-- Food Platform Foundation Migration
-- Adds: product_type, availability_model, kitchen calculator fields,
--       extra nutrients, seasonality table, substitutions table,
--       and migrates availability_months → catalog_product_seasonality
-- ══════════════════════════════════════════════════════════════════════════════

-- ── ШАГИ 1 & 3 & 4: Новые колонки в catalog_ingredients ─────────────────────

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS product_type TEXT
    CHECK (product_type IN (
        'seafood','vegetable','fruit','meat','grain',
        'dairy','spice','herb','legume','nut','mushroom','other'
    ))
    DEFAULT 'other';

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS availability_model TEXT
    CHECK (availability_model IN (
        'seasonal_calendar','all_year','event_driven','static'
    ))
    DEFAULT 'all_year';

-- Kitchen calculator fields
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS shelf_life_days INTEGER;

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS edible_yield_percent NUMERIC;

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS typical_portion_g NUMERIC;

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS substitution_group TEXT;

-- Extra nutrients per 100g
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS fiber_per_100g NUMERIC;

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS sugar_per_100g NUMERIC;

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS salt_per_100g NUMERIC;

-- ── ШАГ 2: Таблица сезонности ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS catalog_product_seasonality (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id  UUID NOT NULL
                    REFERENCES catalog_ingredients(id)
                    ON DELETE CASCADE,
    region_code TEXT NOT NULL DEFAULT 'GLOBAL',
    month       SMALLINT NOT NULL CHECK (month BETWEEN 1 AND 12),
    status      TEXT NOT NULL CHECK (status IN ('peak','good','limited','off')),
    note        TEXT,
    created_at  TIMESTAMPTZ DEFAULT now(),
    UNIQUE(product_id, region_code, month)
);

CREATE INDEX IF NOT EXISTS idx_seasonality_product_id
    ON catalog_product_seasonality(product_id);

CREATE INDEX IF NOT EXISTS idx_seasonality_region_month
    ON catalog_product_seasonality(region_code, month);

-- ── ШАГ 5: Таблица замен ─────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS catalog_product_substitutions (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id           UUID NOT NULL
                             REFERENCES catalog_ingredients(id)
                             ON DELETE CASCADE,
    substitute_product_id UUID NOT NULL
                             REFERENCES catalog_ingredients(id)
                             ON DELETE CASCADE,
    note                 TEXT,
    UNIQUE(product_id, substitute_product_id)
);

CREATE INDEX IF NOT EXISTS idx_substitutions_product_id
    ON catalog_product_substitutions(product_id);

-- ── ШАГ 10: Заполнить product_type и availability_model для всех 106 продуктов

-- Fish & Seafood (category 503794cf)
UPDATE catalog_ingredients
SET product_type = 'seafood',
    availability_model = 'seasonal_calendar'
WHERE category_id = '503794cf-37e0-48c1-a6d8-b5c3f21e03a1';

-- Vegetables (5a841ce0)
UPDATE catalog_ingredients
SET product_type = 'vegetable',
    availability_model = 'seasonal_calendar'
WHERE category_id = '5a841ce0-2ea5-4230-a1f7-011fa445afdc';

-- Fruits (d4a64b25)
UPDATE catalog_ingredients
SET product_type = 'fruit',
    availability_model = 'seasonal_calendar'
WHERE category_id = 'd4a64b25-a187-4ec0-9518-3e8954a138fa';

-- Meat & Poultry (eb707494)
UPDATE catalog_ingredients
SET product_type = 'meat',
    availability_model = 'all_year'
WHERE category_id = 'eb707494-78f8-427f-9408-9c297a882ae0';

-- Dairy & Eggs (b33520f3)
UPDATE catalog_ingredients
SET product_type = 'dairy',
    availability_model = 'all_year'
WHERE category_id = 'b33520f3-e788-40a1-9f27-186cad5d96da';

-- Grains & Pasta (d532ac04)
UPDATE catalog_ingredients
SET product_type = 'grain',
    availability_model = 'all_year'
WHERE category_id = 'd532ac04-0d29-4a76-ab6e-9d08e183119c';

-- Spices & Herbs (40ce05d1)
UPDATE catalog_ingredients
SET product_type = 'spice',
    availability_model = 'all_year'
WHERE category_id = '40ce05d1-70c1-4766-b697-45ac6c857d4a';

-- Legumes (9d882580)
UPDATE catalog_ingredients
SET product_type = 'legume',
    availability_model = 'all_year'
WHERE category_id = '9d882580-9b21-42cc-b731-56d78cd779bc';

-- Nuts & Seeds (1e9fdeb2)
UPDATE catalog_ingredients
SET product_type = 'nut',
    availability_model = 'all_year'
WHERE category_id = '1e9fdeb2-4f7a-4013-8fa7-0abb16573a0a';

-- Oils & Fats (415c59fd)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'all_year'
WHERE category_id = '415c59fd-ce2c-41eb-9312-131e055049ba';

-- Condiments & Sauces (ec31941e)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'all_year'
WHERE category_id = 'ec31941e-8ec6-41d7-9485-73ed9006d34d';

-- Beverages (102d4138)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'all_year'
WHERE category_id = '102d4138-7137-4de7-8ef3-853c1662305d';

-- Sweets & Baking (85ea8da9)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'all_year'
WHERE category_id = '85ea8da9-236a-4bb7-906f-cc4fe2e0c47f';

-- Canned & Preserved (e49781ea)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'static'
WHERE category_id = 'e49781ea-2c07-46af-b417-548ac6d3d788';

-- Frozen (737707e6)
UPDATE catalog_ingredients
SET product_type = 'other',
    availability_model = 'static'
WHERE category_id = '737707e6-a641-4739-98f7-bad9f18a2e33';

-- ── ШАГ 10: Миграция availability_months (BOOLEAN[12]) → catalog_product_seasonality
-- true  → 'good'
-- false → 'off'

INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT
    id AS product_id,
    'GLOBAL' AS region_code,
    gs.month,
    CASE WHEN availability_months[gs.month] THEN 'good' ELSE 'off' END AS status
FROM catalog_ingredients
CROSS JOIN generate_series(1, 12) AS gs(month)
WHERE availability_months IS NOT NULL
ON CONFLICT (product_id, region_code, month) DO NOTHING;

-- ── Kitchen data for fish (shelf_life, yield, portion, substitution_group) ──

UPDATE catalog_ingredients SET
    shelf_life_days        = 3,
    edible_yield_percent   = 60,
    typical_portion_g      = 180,
    substitution_group     = 'fatty_fish'
WHERE slug = 'salmon';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 65,
    typical_portion_g      = 160,
    substitution_group     = 'lean_fish'
WHERE slug = 'tuna';

UPDATE catalog_ingredients SET
    shelf_life_days        = 730,
    edible_yield_percent   = 100,
    typical_portion_g      = 185,
    substitution_group     = 'canned_fish'
WHERE slug = 'canned-tuna';

UPDATE catalog_ingredients SET
    shelf_life_days        = 3,
    edible_yield_percent   = 50,
    typical_portion_g      = 200,
    substitution_group     = 'lean_fish'
WHERE slug = 'cod';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 60,
    typical_portion_g      = 150,
    substitution_group     = 'fatty_fish'
WHERE slug = 'herring';

UPDATE catalog_ingredients SET
    shelf_life_days        = 3,
    edible_yield_percent   = 60,
    typical_portion_g      = 180,
    substitution_group     = 'fatty_fish'
WHERE slug = 'trout';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 55,
    typical_portion_g      = 180,
    substitution_group     = 'fatty_fish'
WHERE slug = 'mackerel';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 55,
    typical_portion_g      = 200,
    substitution_group     = 'lean_fish'
WHERE slug = 'sea-bass';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 45,
    typical_portion_g      = 250,
    substitution_group     = 'freshwater_fish'
WHERE slug = 'pike';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 50,
    typical_portion_g      = 250,
    substitution_group     = 'freshwater_fish'
WHERE slug = 'carp';

UPDATE catalog_ingredients SET
    shelf_life_days        = 2,
    edible_yield_percent   = 65,
    typical_portion_g      = 120,
    substitution_group     = 'shellfish'
WHERE slug = 'shrimp';

-- ── Fish substitutions ───────────────────────────────────────────────────────

INSERT INTO catalog_product_substitutions (product_id, substitute_product_id, note)
SELECT a.id, b.id, 'Both fatty fish, similar omega-3 content'
FROM catalog_ingredients a, catalog_ingredients b
WHERE a.slug = 'salmon' AND b.slug = 'trout'
ON CONFLICT DO NOTHING;

INSERT INTO catalog_product_substitutions (product_id, substitute_product_id, note)
SELECT a.id, b.id, 'Both fatty fish'
FROM catalog_ingredients a, catalog_ingredients b
WHERE a.slug = 'salmon' AND b.slug = 'mackerel'
ON CONFLICT DO NOTHING;

INSERT INTO catalog_product_substitutions (product_id, substitute_product_id, note)
SELECT a.id, b.id, 'Both fatty fish, similar omega-3 content'
FROM catalog_ingredients a, catalog_ingredients b
WHERE a.slug = 'trout' AND b.slug = 'salmon'
ON CONFLICT DO NOTHING;

INSERT INTO catalog_product_substitutions (product_id, substitute_product_id, note)
SELECT a.id, b.id, 'Both lean white fish'
FROM catalog_ingredients a, catalog_ingredients b
WHERE a.slug = 'cod' AND b.slug = 'sea-bass'
ON CONFLICT DO NOTHING;

INSERT INTO catalog_product_substitutions (product_id, substitute_product_id, note)
SELECT a.id, b.id, 'Both freshwater fish'
FROM catalog_ingredients a, catalog_ingredients b
WHERE a.slug = 'pike' AND b.slug = 'carp'
ON CONFLICT DO NOTHING;
