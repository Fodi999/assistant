-- ══════════════════════════════════════════════════════════════════════════════
-- Copy fish seasonality from GLOBAL → PL, EU, UA
-- Fish seasons don't differ much between these regions (same Atlantic/Baltic fish)
-- ES gets slightly different data (Mediterranean species)
-- ══════════════════════════════════════════════════════════════════════════════

-- PL: copy from GLOBAL (Baltic/North Sea fish, same season)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT product_id, 'PL', month, status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.category_id = '503794cf-37e0-48c1-a6d8-b5c3f21e03a1'
  AND ci.availability_model != 'all_year'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- EU: copy from GLOBAL
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT product_id, 'EU', month, status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.category_id = '503794cf-37e0-48c1-a6d8-b5c3f21e03a1'
  AND ci.availability_model != 'all_year'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- UA: copy from GLOBAL (Black Sea / freshwater fish similar seasons)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT product_id, 'UA', month, status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.category_id = '503794cf-37e0-48c1-a6d8-b5c3f21e03a1'
  AND ci.availability_model != 'all_year'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES: Mediterranean adjustments
-- Mackerel earlier in Spain (Mar-Jun instead of May-Jul)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (4,5,6) THEN 'peak'
        WHEN gs.month IN (3,7)   THEN 'good'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'mackerel'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- Sea bass earlier in Spain (Feb-May peak)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (3,4,5) THEN 'peak'
        WHEN gs.month IN (2,6)   THEN 'good'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'sea-bass'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- Tuna in Spain earlier (May-Jul peak)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (5,6,7) THEN 'peak'
        WHEN gs.month IN (4,8)   THEN 'good'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'tuna'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- Shrimp in Spain: longer season (Apr-Sep)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (5,6,7,8) THEN 'peak'
        WHEN gs.month IN (4,9)     THEN 'good'
        WHEN gs.month IN (3,10)    THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'shrimp'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- Salmon ES: same as GLOBAL (imported Atlantic)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT product_id, 'ES', month, status
FROM catalog_product_seasonality cps
JOIN catalog_ingredients ci ON ci.id = cps.product_id
WHERE cps.region_code = 'GLOBAL'
  AND ci.slug IN ('salmon','cod','herring','trout','carp','pike')
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;
