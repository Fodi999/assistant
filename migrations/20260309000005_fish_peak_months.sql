-- ══════════════════════════════════════════════════════════════════════════════
-- Upgrade fish seasonality: add peak months (was all 'good', now granular)
-- peak  = best time to catch/buy — freshest, most flavourful
-- good  = in season, good quality
-- limited = transitional month, limited availability
-- off   = out of season
-- ══════════════════════════════════════════════════════════════════════════════

-- SALMON: peak Jul-Sep (Atlantic run), good Oct-Mar, off Apr-Jun
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (7,8,9)        THEN 'peak'
    WHEN month IN (1,2,3,10,11,12) THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'salmon' AND cps.region_code = 'GLOBAL';

-- COD (треска): peak Oct-Dec (North Sea), good Jan-Apr
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (10,11,12)  THEN 'peak'
    WHEN month IN (1,2,3,4)   THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'cod' AND cps.region_code = 'GLOBAL';

-- HERRING (сельдь): peak Sep-Nov (autumn herring), good Jan-Feb, Dec
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (9,10,11)   THEN 'peak'
    WHEN month IN (1,2,12)    THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'herring' AND cps.region_code = 'GLOBAL';

-- MACKEREL (скумбрия): peak May-Jul (spring/summer), good Aug-Sep
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (5,6,7)     THEN 'peak'
    WHEN month IN (8,9)       THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'mackerel' AND cps.region_code = 'GLOBAL';

-- TUNA (свежий тунец): peak Jun-Aug, good Apr-May, Sep
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (6,7,8)     THEN 'peak'
    WHEN month IN (4,5,9)     THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'tuna' AND cps.region_code = 'GLOBAL';

-- TROUT (форель): peak Apr-May (spring), good Mar, Sep-Nov
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (4,5)           THEN 'peak'
    WHEN month IN (3,9,10,11)     THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'trout' AND cps.region_code = 'GLOBAL';

-- SHRIMP (креветки): peak May-Jul, good Aug-Sep, Apr
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (5,6,7)     THEN 'peak'
    WHEN month IN (4,8,9)     THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'shrimp' AND cps.region_code = 'GLOBAL';

-- SEA BASS (морской окунь): peak Apr-Jun, good Mar, Jul
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (4,5,6)     THEN 'peak'
    WHEN month IN (3,7)       THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'sea-bass' AND cps.region_code = 'GLOBAL';

-- CARP (карп): peak Oct-Jan (winter carp, best fat content)
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (10,11,12,1)  THEN 'peak'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'carp' AND cps.region_code = 'GLOBAL';

-- PIKE (щука): peak Jan-Apr (pre-spawn), good Nov-Dec
UPDATE catalog_product_seasonality cps
SET status = CASE
    WHEN month IN (1,2,3)     THEN 'peak'
    WHEN month IN (4,11,12)   THEN 'good'
    ELSE 'off'
END
FROM catalog_ingredients ci
WHERE cps.product_id = ci.id AND ci.slug = 'pike' AND cps.region_code = 'GLOBAL';

-- CANNED TUNA: all_year → keep as 'good' all 12 months (no peak for canned)
-- (no change needed, already good)
