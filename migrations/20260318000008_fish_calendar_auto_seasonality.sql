-- ══════════════════════════════════════════════════════════════════════════════
-- AUTO-SEASONALITY: Ensure every fish/seafood product gets calendar entries
--
-- 1. Set availability_months for fish that are missing it
-- 2. Set availability_model = 'seasonal_calendar' for all fish/seafood
-- 3. Backfill catalog_product_seasonality from availability_months
-- 4. Copy GLOBAL → PL, EU, UA regions for new fish
-- 5. Create trigger function for auto-generating seasonality on new fish/seafood
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Fill availability_months for fish that don't have it ──────────────────

-- Bream (Лещ) — freshwater, best Apr–Oct
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,true,true,true,true,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'bream' AND availability_months IS NULL;

-- Catfish (Сом) — freshwater, best May–Oct
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,true,true,true,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'catfish' AND availability_months IS NULL;

-- Crucian carp (Карась) — freshwater, best Apr–Oct
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,true,true,true,true,true,true,true,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'crucian-carp' AND availability_months IS NULL;

-- Eel (Угорь) — migratory, best May–Sep
UPDATE catalog_ingredients SET
    availability_months = ARRAY[false,false,false,false,true,true,true,true,true,false,false,false],
    availability_model = 'seasonal_calendar'
WHERE slug = 'eel' AND availability_months IS NULL;

-- ── 2. Ensure all fish/seafood have availability_model = 'seasonal_calendar' ─
UPDATE catalog_ingredients
SET availability_model = 'seasonal_calendar'
WHERE product_type IN ('fish', 'seafood')
  AND availability_model != 'seasonal_calendar';

-- ── 3. Backfill catalog_product_seasonality from availability_months ─────────
-- For GLOBAL region: any fish/seafood with availability_months but no seasonality rows
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT
    ci.id AS product_id,
    'GLOBAL' AS region_code,
    gs.month,
    CASE WHEN ci.availability_months[gs.month] THEN 'good' ELSE 'off' END AS status
FROM catalog_ingredients ci
CROSS JOIN generate_series(1, 12) AS gs(month)
WHERE ci.product_type IN ('fish', 'seafood')
  AND ci.availability_months IS NOT NULL
  AND ci.is_active = true
ON CONFLICT (product_id, region_code, month) DO NOTHING;

-- ── 4. Copy GLOBAL → PL for new fish ────────────────────────────────────────
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT cps.product_id, 'PL', cps.month, cps.status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.product_type IN ('fish', 'seafood')
  AND NOT EXISTS (
      SELECT 1 FROM catalog_product_seasonality x
      WHERE x.product_id = cps.product_id AND x.region_code = 'PL' AND x.month = cps.month
  )
ON CONFLICT (product_id, region_code, month) DO NOTHING;

-- Copy GLOBAL → EU
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT cps.product_id, 'EU', cps.month, cps.status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.product_type IN ('fish', 'seafood')
  AND NOT EXISTS (
      SELECT 1 FROM catalog_product_seasonality x
      WHERE x.product_id = cps.product_id AND x.region_code = 'EU' AND x.month = cps.month
  )
ON CONFLICT (product_id, region_code, month) DO NOTHING;

-- Copy GLOBAL → UA
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT cps.product_id, 'UA', cps.month, cps.status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.product_type IN ('fish', 'seafood')
  AND NOT EXISTS (
      SELECT 1 FROM catalog_product_seasonality x
      WHERE x.product_id = cps.product_id AND x.region_code = 'UA' AND x.month = cps.month
  )
ON CONFLICT (product_id, region_code, month) DO NOTHING;

-- ── 5. Create trigger function: auto-generate seasonality for new fish/seafood ──
-- When a new product is inserted with product_type fish/seafood,
-- auto-generate 12 months of GLOBAL seasonality from availability_months
-- (or default all-year if availability_months is NULL)

CREATE OR REPLACE FUNCTION fn_auto_fish_seasonality()
RETURNS TRIGGER AS $$
BEGIN
    -- Only for fish/seafood products
    IF NEW.product_type NOT IN ('fish', 'seafood') THEN
        RETURN NEW;
    END IF;

    -- Generate 12 months of GLOBAL seasonality
    INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
    SELECT
        NEW.id,
        region,
        gs.month,
        CASE
            WHEN NEW.availability_months IS NOT NULL AND NEW.availability_months[gs.month]
                THEN 'good'
            WHEN NEW.availability_months IS NULL
                THEN 'good'  -- Default: all-year if no months specified
            ELSE 'off'
        END AS status
    FROM generate_series(1, 12) AS gs(month)
    CROSS JOIN (VALUES ('GLOBAL'), ('PL'), ('EU'), ('UA')) AS regions(region)
    ON CONFLICT (product_id, region_code, month) DO NOTHING;

    -- Ensure availability_model is set
    IF NEW.availability_model IS NULL OR NEW.availability_model = 'all_year' THEN
        NEW.availability_model := 'seasonal_calendar';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop old trigger if exists, then create
DROP TRIGGER IF EXISTS trg_auto_fish_seasonality ON catalog_ingredients;
CREATE TRIGGER trg_auto_fish_seasonality
    AFTER INSERT ON catalog_ingredients
    FOR EACH ROW
    EXECUTE FUNCTION fn_auto_fish_seasonality();

-- ── 6. Also trigger on UPDATE of product_type (reclassification) ─────────────
-- If a product is reclassified TO fish/seafood, also generate seasonality
CREATE OR REPLACE FUNCTION fn_auto_fish_seasonality_on_update()
RETURNS TRIGGER AS $$
BEGIN
    -- Only if product_type changed TO fish/seafood
    IF NEW.product_type IN ('fish', 'seafood')
       AND (OLD.product_type IS NULL OR OLD.product_type NOT IN ('fish', 'seafood'))
    THEN
        INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
        SELECT
            NEW.id,
            region,
            gs.month,
            CASE
                WHEN NEW.availability_months IS NOT NULL AND NEW.availability_months[gs.month]
                    THEN 'good'
                WHEN NEW.availability_months IS NULL
                    THEN 'good'
                ELSE 'off'
            END AS status
        FROM generate_series(1, 12) AS gs(month)
        CROSS JOIN (VALUES ('GLOBAL'), ('PL'), ('EU'), ('UA')) AS regions(region)
        ON CONFLICT (product_id, region_code, month) DO NOTHING;

        -- Ensure availability_model is set
        UPDATE catalog_ingredients
        SET availability_model = 'seasonal_calendar'
        WHERE id = NEW.id AND (availability_model IS NULL OR availability_model = 'all_year');
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_auto_fish_seasonality_update ON catalog_ingredients;
CREATE TRIGGER trg_auto_fish_seasonality_update
    AFTER UPDATE OF product_type ON catalog_ingredients
    FOR EACH ROW
    EXECUTE FUNCTION fn_auto_fish_seasonality_on_update();

-- ── 7. Also trigger when availability_months is updated ──────────────────────
-- Refreshes seasonality when availability_months changes
CREATE OR REPLACE FUNCTION fn_refresh_seasonality_on_months_change()
RETURNS TRIGGER AS $$
BEGIN
    -- Only for fish/seafood
    IF NEW.product_type NOT IN ('fish', 'seafood') THEN
        RETURN NEW;
    END IF;

    -- Only if availability_months actually changed
    IF NEW.availability_months IS DISTINCT FROM OLD.availability_months THEN
        -- Update existing rows (not insert new — they should already exist)
        UPDATE catalog_product_seasonality cps
        SET status = CASE
            WHEN NEW.availability_months IS NOT NULL AND NEW.availability_months[cps.month]
                THEN 'good'
            WHEN NEW.availability_months IS NULL
                THEN 'good'
            ELSE 'off'
        END
        WHERE cps.product_id = NEW.id;

        -- Insert if missing (safety net)
        INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
        SELECT
            NEW.id,
            region,
            gs.month,
            CASE
                WHEN NEW.availability_months IS NOT NULL AND NEW.availability_months[gs.month]
                    THEN 'good'
                WHEN NEW.availability_months IS NULL
                    THEN 'good'
                ELSE 'off'
            END AS status
        FROM generate_series(1, 12) AS gs(month)
        CROSS JOIN (VALUES ('GLOBAL'), ('PL'), ('EU'), ('UA')) AS regions(region)
        ON CONFLICT (product_id, region_code, month)
        DO UPDATE SET status = EXCLUDED.status;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_refresh_seasonality_months ON catalog_ingredients;
CREATE TRIGGER trg_refresh_seasonality_months
    AFTER UPDATE OF availability_months ON catalog_ingredients
    FOR EACH ROW
    EXECUTE FUNCTION fn_refresh_seasonality_on_months_change();
