-- ══════════════════════════════════════════════════════════════════════════════
-- Regional seasonality data: PL (Poland) and ES (Spain)
-- Shows how the same product has different seasons by region
-- ══════════════════════════════════════════════════════════════════════════════

-- ── Helper: insert region data for a slug ─────────────────────────────────────
-- PL = Poland (Central Europe), ES = Spain (Mediterranean)

-- ── TOMATO ────────────────────────────────────────────────────────────────────
-- PL: Jul-Sep peak, Jun/Oct good
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month IN (7,8,9) THEN 'peak'
        WHEN gs.month IN (6,10) THEN 'good'
        WHEN gs.month = 5 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'tomato'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES: May-Oct peak, Apr/Nov good (longer Mediterranean season)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (6,7,8,9) THEN 'peak'
        WHEN gs.month IN (5,10) THEN 'good'
        WHEN gs.month IN (4,11) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'tomato'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- EU: average (Jun-Sep peak, May/Oct good)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'EU', gs.month,
    CASE
        WHEN gs.month IN (7,8,9) THEN 'peak'
        WHEN gs.month IN (6,10) THEN 'good'
        WHEN gs.month = 5 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'tomato'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── STRAWBERRY ────────────────────────────────────────────────────────────────
-- PL: Jun peak, May/Jul good
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month = 6 THEN 'peak'
        WHEN gs.month IN (5,7) THEN 'good'
        WHEN gs.month = 4 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'strawberry'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES: Apr peak (much earlier!), Mar/May good
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (4,5) THEN 'peak'
        WHEN gs.month IN (3,6) THEN 'good'
        WHEN gs.month IN (2,7) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'strawberry'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- EU: May peak
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'EU', gs.month,
    CASE
        WHEN gs.month IN (5,6) THEN 'peak'
        WHEN gs.month IN (4,7) THEN 'good'
        WHEN gs.month = 3 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'strawberry'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── CUCUMBER ──────────────────────────────────────────────────────────────────
-- PL
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month IN (7,8) THEN 'peak'
        WHEN gs.month IN (6,9) THEN 'good'
        WHEN gs.month = 5 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'cucumber'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (5,6,7,8) THEN 'peak'
        WHEN gs.month IN (4,9) THEN 'good'
        WHEN gs.month IN (3,10) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'cucumber'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- EU
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'EU', gs.month,
    CASE
        WHEN gs.month IN (6,7,8) THEN 'peak'
        WHEN gs.month IN (5,9) THEN 'good'
        WHEN gs.month = 4 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'cucumber'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── APPLE ─────────────────────────────────────────────────────────────────────
-- PL
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month IN (9,10) THEN 'peak'
        WHEN gs.month IN (8,11) THEN 'good'
        WHEN gs.month IN (7,12) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'apple'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (9,10) THEN 'peak'
        WHEN gs.month IN (8,11) THEN 'good'
        WHEN gs.month = 12 THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'apple'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── SPINACH ───────────────────────────────────────────────────────────────────
-- PL
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month IN (4,5,10) THEN 'peak'
        WHEN gs.month IN (3,6,9,11) THEN 'good'
        WHEN gs.month IN (3) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'spinach'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES (year-round but best Oct-Apr)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (10,11,12,1,2,3) THEN 'peak'
        WHEN gs.month IN (4,9) THEN 'good'
        WHEN gs.month IN (5,8) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'spinach'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── CHERRY ────────────────────────────────────────────────────────────────────
-- PL: Jun-Jul
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'PL', gs.month,
    CASE
        WHEN gs.month IN (6,7) THEN 'peak'
        WHEN gs.month IN (5,8) THEN 'limited'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'cherry'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ES: May-Jun (earlier due to climate)
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT ci.id, 'ES', gs.month,
    CASE
        WHEN gs.month IN (5,6) THEN 'peak'
        WHEN gs.month IN (4,7) THEN 'good'
        ELSE 'off'
    END
FROM catalog_ingredients ci
CROSS JOIN generate_series(1,12) AS gs(month)
WHERE ci.slug = 'cherry'
ON CONFLICT (product_id, region_code, month) DO UPDATE SET status = EXCLUDED.status;

-- ── UA (Ukraine) — same as PL for most products ────────────────────────────────
-- Copy PL data to UA for all products that have PL but not UA
INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
SELECT product_id, 'UA', month, status
FROM catalog_product_seasonality
WHERE region_code = 'PL'
ON CONFLICT (product_id, region_code, month) DO NOTHING;
