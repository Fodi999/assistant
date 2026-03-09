-- ══════════════════════════════════════════════════════════════════════════════
-- Fix fruits category + add missing fruits + full seasonality for fruits/vegetables
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Fix apple: wrong category (Dairy) → Fruits, product_type → fruit ──────
UPDATE catalog_ingredients
SET category_id    = 'd4a64b25-a187-4ec0-9518-3e8954a138fa',
    product_type   = 'fruit',
    availability_model = 'seasonal_calendar'
WHERE slug = 'apple';

-- ── 2. Add missing fruits ─────────────────────────────────────────────────────

INSERT INTO catalog_ingredients
    (id, name_en, name_pl, name_ru, name_uk, category_id, default_unit,
     calories_per_100g, protein_per_100g, fat_per_100g, carbs_per_100g,
     product_type, availability_model,
     description_en, description_pl, description_ru, description_uk,
     seasons, allergens,
     availability_months)
VALUES
-- Pear
(gen_random_uuid(), 'Pear', 'Gruszka', 'Груша', 'Груша',
 'd4a64b25-a187-4ec0-9518-3e8954a138fa', 'kilogram',
 57, 0.4, 0.1, 15.0,
 'fruit', 'seasonal_calendar',
 'Sweet and juicy fruit, rich in fiber and vitamins C and K.',
 'Słodki i soczysty owoc, bogaty w błonnik oraz witaminy C i K.',
 'Сладкий и сочный фрукт, богатый клетчаткой и витаминами C и K.',
 'Солодкий і соковитий фрукт, багатий на клітковину та вітаміни C і K.',
 ARRAY['Summer','Autumn']::season_type[], ARRAY[]::allergen_type[],
 ARRAY[false,false,false,false,false,false,false,true,true,true,true,false]),

-- Cherry
(gen_random_uuid(), 'Cherry', 'Wiśnia', 'Вишня', 'Вишня',
 'd4a64b25-a187-4ec0-9518-3e8954a138fa', 'kilogram',
 63, 1.1, 0.2, 16.0,
 'fruit', 'seasonal_calendar',
 'Sweet or sour stone fruit, rich in antioxidants and vitamins.',
 'Słodki lub kwaśny owoc pestkowy, bogaty w antyoksydanty i witaminy.',
 'Сладкий или кислый косточковый фрукт, богатый антиоксидантами и витаминами.',
 'Солодкий або кислий кісточковий фрукт, багатий на антиоксиданти та вітаміни.',
 ARRAY['Summer']::season_type[], ARRAY[]::allergen_type[],
 ARRAY[false,false,false,false,false,true,true,false,false,false,false,false]),

-- Plum
(gen_random_uuid(), 'Plum', 'Śliwka', 'Слива', 'Слива',
 'd4a64b25-a187-4ec0-9518-3e8954a138fa', 'kilogram',
 46, 0.7, 0.3, 11.0,
 'fruit', 'seasonal_calendar',
 'Sweet stone fruit, excellent for jams, desserts and drying.',
 'Słodki owoc pestkowy, doskonały do dżemów, deserów i suszenia.',
 'Сладкий косточковый фрукт, отлично подходит для джемов, десертов и сушки.',
 'Солодкий кісточковий фрукт, чудово підходить для варення, десертів і сушіння.',
 ARRAY['Summer','Autumn']::season_type[], ARRAY[]::allergen_type[],
 ARRAY[false,false,false,false,false,false,false,true,true,true,false,false]),

-- Peach
(gen_random_uuid(), 'Peach', 'Brzoskwinia', 'Персик', 'Персик',
 'd4a64b25-a187-4ec0-9518-3e8954a138fa', 'kilogram',
 39, 0.9, 0.3, 10.0,
 'fruit', 'seasonal_calendar',
 'Juicy summer stone fruit with sweet aromatic flesh.',
 'Soczysty letni owoc pestkowy o słodkim aromatycznym miąższu.',
 'Сочный летний косточковый фрукт со сладкой ароматной мякотью.',
 'Соковитий літній кісточковий фрукт із солодкою ароматною м''якоттю.',
 ARRAY['Summer']::season_type[], ARRAY[]::allergen_type[],
 ARRAY[false,false,false,false,false,false,true,true,false,false,false,false]),

-- Apricot
(gen_random_uuid(), 'Apricot', 'Morela', 'Абрикос', 'Абрикос',
 'd4a64b25-a187-4ec0-9518-3e8954a138fa', 'kilogram',
 48, 1.4, 0.4, 11.0,
 'fruit', 'seasonal_calendar',
 'Sweet orange stone fruit, excellent fresh or dried. Rich in beta-carotene.',
 'Słodki pomarańczowy owoc pestkowy, doskonały świeży lub suszony. Bogaty w beta-karoten.',
 'Сладкий оранжевый косточковый фрукт, прекрасен свежим или сушёным. Богат бета-каротином.',
 'Солодкий помаранчевий кісточковий фрукт, чудовий свіжим або сушеним. Багатий на бета-каротин.',
 ARRAY['Summer']::season_type[], ARRAY[]::allergen_type[],
 ARRAY[false,false,false,false,false,true,true,false,false,false,false,false])
ON CONFLICT (slug) DO NOTHING;

-- ── 3. Seasonality for fruits → catalog_product_seasonality ──────────────────
-- Migrate availability_months for new and existing fruits

INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT
    ci.id,
    'GLOBAL',
    gs.month,
    CASE
        WHEN ci.availability_months[gs.month] THEN 'good'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1, 12) AS gs(month)
WHERE ci.product_type = 'fruit'
  AND ci.availability_months IS NOT NULL
ON CONFLICT (product_id, region_code, month) DO UPDATE
    SET status = EXCLUDED.status;

-- ── 4. Update apple's availability_months (was wrong/missing after category fix)
UPDATE catalog_ingredients
SET availability_months = ARRAY[
  false, false, false, false, false, false, false, true, true, true, false, false
]
WHERE slug = 'apple';

-- Re-insert seasonality for apple after fixing months
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'GLOBAL', gs.month,
    CASE WHEN gs.month IN (8,9,10) THEN 'good' ELSE 'off' END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1, 12) AS gs(month)
WHERE ci.slug = 'apple'
ON CONFLICT (product_id, region_code, month) DO UPDATE
    SET status = EXCLUDED.status;

-- ── 5. Seasonality for ALL vegetables ────────────────────────────────────────
-- Set availability_months for each vegetable then sync to seasonality table

-- tomato: Jun-Sep peak, May/Oct good
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,true,true,true,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'tomato';

-- cucumber: Jun-Aug peak, May/Sep good
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'cucumber';

-- carrot: all year, best Aug-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'carrot';

-- potato: all year, best Jul-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'potato';

-- onion: all year
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'onion';

-- garlic: all year
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'garlic';

-- spinach: Mar-May, Sep-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,true,true,true,false,false,false,true,true,true,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'spinach';

-- broccoli: Apr-Jun, Sep-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,false,false,true,true,true,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'broccoli';

-- zucchini: Jun-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,true,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'zucchini';

-- eggplant: Jul-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'eggplant';

-- bell-pepper: Jul-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'bell-pepper';

-- cabbage: all year, best Aug-Feb
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,false,false,false,false,false,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'cabbage';

-- cauliflower: Apr-Jun, Sep-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,false,false,true,true,true,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'cauliflower';

-- lettuce: Apr-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'lettuce';

-- corn: Jul-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'corn';

-- green-peas: May-Jul
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,false,false,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'green-peas';

-- porcini-mushroom: Jul-Oct
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,true,true,true,true,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'porcini-mushroom';

-- button-mushroom: all year
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model = 'seasonal_calendar'
WHERE slug = 'button-mushroom';

-- artichoke: Apr-Jun, Oct-Nov
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,false,false,false,true,true,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'artichoke';

-- ── 6. Seasonality for fruits ─────────────────────────────────────────────────

-- strawberry: May-Jul
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,false,false,false,false,false]
WHERE slug = 'strawberry';

-- raspberry: Jun-Aug
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,true,true,true,false,false,false,false]
WHERE slug = 'raspberry';

-- blueberry: Jun-Aug
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,true,true,true,false,false,false,false]
WHERE slug = 'blueberry';

-- watermelon: Jul-Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,true,true,true,false,false,false]
WHERE slug = 'watermelon';

-- grape: Aug-Oct
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,false,false,false,true,true,true,false,false]
WHERE slug = 'grape';

-- orange: Nov-Mar (winter citrus)
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,false,false,false,false,false,false,false,true,true]
WHERE slug = 'orange';

-- lemon: all year
UPDATE catalog_ingredients SET
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true]
WHERE slug = 'lemon';

-- banana: all year (tropical)
UPDATE catalog_ingredients SET
    availability_months  = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model   = 'all_year'
WHERE slug = 'banana';

-- pineapple: all year (tropical)
UPDATE catalog_ingredients SET
    availability_months  = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model   = 'all_year'
WHERE slug = 'pineapple';

-- avocado: all year (imported)
UPDATE catalog_ingredients SET
    availability_months  = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true],
    availability_model   = 'all_year'
WHERE slug = 'avocado';

-- ── 7. Sync ALL vegetable + fruit availability_months → seasonality table ─────

-- With granular peak/good/limited logic for vegetables
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'GLOBAL', gs.month,
    CASE
        -- tomato
        WHEN ci.slug = 'tomato' AND gs.month IN (7,8,9) THEN 'peak'
        WHEN ci.slug = 'tomato' AND gs.month IN (5,6,10) THEN 'good'
        -- cucumber
        WHEN ci.slug = 'cucumber' AND gs.month IN (6,7,8) THEN 'peak'
        WHEN ci.slug = 'cucumber' AND gs.month IN (5,9) THEN 'good'
        -- strawberry
        WHEN ci.slug = 'strawberry' AND gs.month IN (6) THEN 'peak'
        WHEN ci.slug = 'strawberry' AND gs.month IN (5,7) THEN 'good'
        -- raspberry
        WHEN ci.slug = 'raspberry' AND gs.month IN (7) THEN 'peak'
        WHEN ci.slug = 'raspberry' AND gs.month IN (6,8) THEN 'good'
        -- blueberry
        WHEN ci.slug = 'blueberry' AND gs.month IN (7) THEN 'peak'
        WHEN ci.slug = 'blueberry' AND gs.month IN (6,8) THEN 'good'
        -- cherry
        WHEN ci.slug = 'cherry' AND gs.month IN (6,7) THEN 'peak'
        -- peach
        WHEN ci.slug = 'peach' AND gs.month IN (7,8) THEN 'peak'
        -- apricot
        WHEN ci.slug = 'apricot' AND gs.month IN (6,7) THEN 'peak'
        -- watermelon
        WHEN ci.slug = 'watermelon' AND gs.month IN (7,8) THEN 'peak'
        WHEN ci.slug = 'watermelon' AND gs.month = 9 THEN 'good'
        -- pear
        WHEN ci.slug = 'pear' AND gs.month IN (9,10) THEN 'peak'
        WHEN ci.slug = 'pear' AND gs.month IN (8,11) THEN 'good'
        -- plum
        WHEN ci.slug = 'plum' AND gs.month IN (8,9) THEN 'peak'
        WHEN ci.slug = 'plum' AND gs.month = 10 THEN 'good'
        -- grape
        WHEN ci.slug = 'grape' AND gs.month IN (9,10) THEN 'peak'
        WHEN ci.slug = 'grape' AND gs.month = 8 THEN 'good'
        -- spinach peak
        WHEN ci.slug = 'spinach' AND gs.month IN (4,5,10) THEN 'peak'
        -- broccoli peak
        WHEN ci.slug = 'broccoli' AND gs.month IN (5,10) THEN 'peak'
        -- zucchini peak
        WHEN ci.slug = 'zucchini' AND gs.month IN (7,8) THEN 'peak'
        WHEN ci.slug = 'zucchini' AND gs.month IN (6,9) THEN 'good'
        -- eggplant peak
        WHEN ci.slug = 'eggplant' AND gs.month IN (8,9) THEN 'peak'
        WHEN ci.slug = 'eggplant' AND gs.month = 7 THEN 'good'
        -- bell-pepper peak
        WHEN ci.slug = 'bell-pepper' AND gs.month IN (8,9) THEN 'peak'
        WHEN ci.slug = 'bell-pepper' AND gs.month = 7 THEN 'good'
        -- corn peak
        WHEN ci.slug = 'corn' AND gs.month IN (8) THEN 'peak'
        WHEN ci.slug = 'corn' AND gs.month IN (7,9) THEN 'good'
        -- porcini peak
        WHEN ci.slug = 'porcini-mushroom' AND gs.month IN (8,9,10) THEN 'peak'
        WHEN ci.slug = 'porcini-mushroom' AND gs.month = 7 THEN 'good'
        -- default: if availability_months = true → good, else off
        WHEN ci.availability_months IS NOT NULL AND ci.availability_months[gs.month] THEN 'good'
        ELSE 'off'
    END AS status
FROM catalog_ingredients ci
CROSS JOIN generate_series(1, 12) AS gs(month)
WHERE ci.is_active = true
  AND ci.product_type IN ('fruit', 'vegetable')
ON CONFLICT (product_id, region_code, month) DO UPDATE
    SET status = EXCLUDED.status;

-- ── 8. Fix canned-tuna: availability_model → all_year (not seasonal) ─────────
UPDATE catalog_ingredients
SET availability_model = 'all_year'
WHERE slug = 'canned-tuna';

-- Same for other canned/preserved/frozen products
UPDATE catalog_ingredients
SET availability_model = 'all_year'
WHERE product_type = 'other'
  AND category_id IN (
    'e49781ea-2c07-46af-b417-548ac6d3d788', -- Canned & Preserved
    '737707e6-a641-4739-98f7-bad9f18a2e33'  -- Frozen
  );
