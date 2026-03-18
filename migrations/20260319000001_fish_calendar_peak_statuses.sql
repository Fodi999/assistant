-- ══════════════════════════════════════════════════════════════════════════════
-- FISH CALENDAR QUALITY: Set proper peak/limited/good/off for all fish
--
-- Currently bream, catfish, crucian-carp, eel have only good/off.
-- This migration adds realistic peak & limited statuses.
-- Also fixes canned-tuna (should be all_year, not seasonal calendar).
-- ══════════════════════════════════════════════════════════════════════════════

-- Helper: update status for a fish in ALL regions at once
-- We'll update GLOBAL, PL, EU, UA simultaneously

-- ── BREAM (Лящ) — freshwater ──────────────────────────────────────────────
-- Peak: Jun–Aug (summer warmth, most active)
-- Good: Apr–May, Sep–Oct (shoulder seasons)
-- Off: Nov–Mar (winter, ice)
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'bream')
  AND month IN (1, 2, 3, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'bream')
  AND month IN (4, 5, 9, 10);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'bream')
  AND month IN (6, 7, 8);

-- ── CATFISH (Сом) — freshwater ────────────────────────────────────────────
-- Peak: Jun–Aug (warm water, most active feeding)
-- Good: May, Sep–Oct
-- Off: Nov–Apr
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'catfish')
  AND month IN (1, 2, 3, 4, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'catfish')
  AND month IN (5, 9, 10);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'catfish')
  AND month IN (6, 7, 8);

-- ── CRUCIAN CARP (Карась) — freshwater ────────────────────────────────────
-- Peak: May–Jul (spawning + feeding)
-- Good: Apr, Aug–Sep
-- Limited: Oct
-- Off: Nov–Mar
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'crucian-carp')
  AND month IN (1, 2, 3, 11, 12);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'crucian-carp')
  AND month IN (10);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'crucian-carp')
  AND month IN (4, 8, 9);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'crucian-carp')
  AND month IN (5, 6, 7);

-- ── EEL (Вугор) — migratory ──────────────────────────────────────────────
-- Peak: Jun–Aug (summer migration, best catches)
-- Good: May, Sep
-- Off: Oct–Apr
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'eel')
  AND month IN (1, 2, 3, 4, 10, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'eel')
  AND month IN (5, 9);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'eel')
  AND month IN (6, 7, 8);

-- ── CANNED TUNA — available all year, no peak ────────────────────────────
-- Shelf-stable product, always available. Mark as all_year.
UPDATE catalog_ingredients
SET availability_model = 'all_year',
    availability_months = ARRAY[true,true,true,true,true,true,true,true,true,true,true,true]
WHERE slug = 'canned-tuna';

-- Remove from seasonal calendar (it's all_year)
DELETE FROM catalog_product_seasonality
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'canned-tuna');

-- ── PIKE (Щука) — freshwater, improve peak data ──────────────────────────
-- Peak: Sep–Nov (autumn feeding frenzy before winter)
-- Good: Jan–Apr, Dec
-- Limited: May (spawning ban in many countries)
-- Off: Jun–Aug (warm water, lethargic)
UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'pike')
  AND month IN (1, 2, 3, 4, 12);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'pike')
  AND month IN (5);

UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'pike')
  AND month IN (6, 7, 8);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'pike')
  AND month IN (9, 10, 11);

-- ── COD (Тріска) — sea, improve peak data ────────────────────────────────
-- Peak: Jan–Mar (winter/spring, North Sea + Baltic)
-- Good: Oct–Dec
-- Limited: Apr
-- Off: May–Sep
UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'cod')
  AND month IN (1, 2, 3);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'cod')
  AND month IN (4);

UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'cod')
  AND month IN (5, 6, 7, 8, 9);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'cod')
  AND month IN (10, 11, 12);

-- ── HERRING (Оселедець) — sea ────────────────────────────────────────────
-- Peak: Oct–Dec (autumn herring, fattest)
-- Good: Jan–Feb, Aug–Sep (spring herring, summer)
-- Off: Mar–Jul
UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'herring')
  AND month IN (1, 2, 8, 9);

UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'herring')
  AND month IN (3, 4, 5, 6, 7);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'herring')
  AND month IN (10, 11, 12);

-- ── MACKEREL (Скумбрія) — sea ────────────────────────────────────────────
-- Peak: Jun–Aug (summer, fattest Atlantic mackerel)
-- Good: Apr–May, Sep
-- Off: Oct–Mar
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'mackerel')
  AND month IN (1, 2, 3, 10, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'mackerel')
  AND month IN (4, 5, 9);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'mackerel')
  AND month IN (6, 7, 8);

-- ── SEA BASS (Морський окунь) — sea ──────────────────────────────────────
-- Peak: Mar–May (spring, pre-spawning, fattened)
-- Good: Feb, Jun–Jul
-- Off: Aug–Jan
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'sea-bass')
  AND month IN (1, 8, 9, 10, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'sea-bass')
  AND month IN (2, 6, 7);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'sea-bass')
  AND month IN (3, 4, 5);

-- ── TROUT (Форель) — freshwater ──────────────────────────────────────────
-- Peak: Apr–May (spring, cleanest water)
-- Good: Mar, Sep–Nov
-- Limited: Jun
-- Off: Jul–Aug (warm water), Dec–Feb (winter)
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'trout')
  AND month IN (1, 2, 7, 8, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'trout')
  AND month IN (3, 9, 10, 11);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'trout')
  AND month IN (6);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'trout')
  AND month IN (4, 5);

-- ── SHRIMP (Креветки) — sea ──────────────────────────────────────────────
-- Peak: Jun–Sep (warm waters, Atlantic + Baltic)
-- Good: Mar–May, Oct
-- Off: Nov–Feb
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'shrimp')
  AND month IN (1, 2, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'shrimp')
  AND month IN (3, 4, 5, 10);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'shrimp')
  AND month IN (6, 7, 8, 9);

-- ── TUNA (Тунець) — sea ─────────────────────────────────────────────────
-- Peak: Jun–Sep (Mediterranean + Atlantic migration)
-- Good: Mar–May
-- Limited: Oct
-- Off: Nov–Feb
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'tuna')
  AND month IN (1, 2, 11, 12);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'tuna')
  AND month IN (3, 4, 5);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'tuna')
  AND month IN (10);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'tuna')
  AND month IN (6, 7, 8, 9);

-- ── CARP (Короп) — freshwater ────────────────────────────────────────────
-- Peak: Sep–Nov (autumn, fattest before winter, traditional Christmas carp)
-- Good: Aug, Dec–Feb
-- Limited: Mar
-- Off: Apr–Jul (spawning season)
UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'carp')
  AND month IN (4, 5, 6, 7);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'carp')
  AND month IN (3);

UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'carp')
  AND month IN (8, 12, 1, 2);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'carp')
  AND month IN (9, 10, 11);

-- ── SALMON (Лосось) — sea ────────────────────────────────────────────────
-- Already has peak/limited from original migration, but let's normalize:
-- Peak: Jul–Sep (summer run, Atlantic + Baltic)
-- Good: Jan–Mar, Oct–Dec (farmed always available)
-- Limited: Apr
-- Off: May–Jun (spawning transition)
UPDATE catalog_product_seasonality SET status = 'good'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'salmon')
  AND month IN (1, 2, 3, 10, 11, 12);

UPDATE catalog_product_seasonality SET status = 'limited'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'salmon')
  AND month IN (4);

UPDATE catalog_product_seasonality SET status = 'off'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'salmon')
  AND month IN (5, 6);

UPDATE catalog_product_seasonality SET status = 'peak'
WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = 'salmon')
  AND month IN (7, 8, 9);
