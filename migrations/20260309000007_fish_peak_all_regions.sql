-- ══════════════════════════════════════════════════════════════════════════════
-- Full granular peak/good/limited for all fish, all regions (PL/EU/UA/ES/GLOBAL)
-- Also adds: water_type (sea/freshwater/both), wild_farmed (wild/farmed/both)
-- ══════════════════════════════════════════════════════════════════════════════

-- ── Add metadata columns ─────────────────────────────────────────────────────
ALTER TABLE catalog_ingredients
    ADD COLUMN IF NOT EXISTS water_type   TEXT CHECK (water_type   IN ('sea','freshwater','both')),
    ADD COLUMN IF NOT EXISTS wild_farmed  TEXT CHECK (wild_farmed  IN ('wild','farmed','both')),
    ADD COLUMN IF NOT EXISTS sushi_grade  BOOLEAN DEFAULT false;

-- ── Fill fish metadata ────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='both',   sushi_grade=true  WHERE slug='salmon';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=true  WHERE slug='tuna';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=false WHERE slug='cod';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=false WHERE slug='herring';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=false WHERE slug='mackerel';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=true  WHERE slug='sea-bass';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='both',   sushi_grade=false WHERE slug='shrimp';
UPDATE catalog_ingredients SET water_type='freshwater',  wild_farmed='both',   sushi_grade=false WHERE slug='trout';
UPDATE catalog_ingredients SET water_type='freshwater',  wild_farmed='wild',   sushi_grade=false WHERE slug='carp';
UPDATE catalog_ingredients SET water_type='freshwater',  wild_farmed='wild',   sushi_grade=false WHERE slug='pike';
UPDATE catalog_ingredients SET water_type='sea',         wild_farmed='wild',   sushi_grade=false WHERE slug='canned-tuna';

-- ══════════════════════════════════════════════════════════════════════════════
-- GLOBAL — granular peak/good/limited for all 10 fish
-- ══════════════════════════════════════════════════════════════════════════════

-- SALMON: peak Jul-Sep, good Oct-Mar, limited Apr, off May-Jun
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (7,8,9)          THEN 'peak'
         WHEN month IN (10,11,12,1,2,3) THEN 'good'
         WHEN month = 4                 THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='salmon');

-- COD: peak Oct-Dec, good Jan-Mar, limited Sep
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (10,11,12) THEN 'peak'
         WHEN month IN (1,2,3)    THEN 'good'
         WHEN month = 4           THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='cod');

-- HERRING: peak Sep-Nov, good Dec-Feb, limited Aug
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (9,10,11) THEN 'peak'
         WHEN month IN (12,1,2)  THEN 'good'
         WHEN month = 8          THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='herring');

-- MACKEREL: peak May-Jul, good Aug-Sep, limited Apr
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (5,6,7) THEN 'peak'
         WHEN month IN (8,9)   THEN 'good'
         WHEN month = 4        THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='mackerel');

-- TUNA: peak Jun-Aug, good Apr-May Sep, limited Mar
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (6,7,8)  THEN 'peak'
         WHEN month IN (4,5,9)  THEN 'good'
         WHEN month = 3         THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='tuna');

-- TROUT: peak Apr-May, good Mar Sep-Oct, limited Nov
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (4,5)     THEN 'peak'
         WHEN month IN (3,9,10)  THEN 'good'
         WHEN month = 11         THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='trout');

-- SHRIMP: peak May-Jul, good Apr Aug-Sep, limited Mar Oct
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (5,6,7)   THEN 'peak'
         WHEN month IN (4,8,9)   THEN 'good'
         WHEN month IN (3,10)    THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='shrimp');

-- SEA BASS: peak Apr-Jun, good Mar Jul, limited Feb
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (4,5,6) THEN 'peak'
         WHEN month IN (3,7)   THEN 'good'
         WHEN month = 2        THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='sea-bass');

-- CARP: peak Oct-Jan, good Sep Feb, limited Aug
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (10,11,12,1) THEN 'peak'
         WHEN month IN (9,2)        THEN 'good'
         WHEN month = 8             THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='carp');

-- PIKE: peak Jan-Mar, good Nov-Dec Apr, limited May
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (1,2,3)   THEN 'peak'
         WHEN month IN (11,12,4) THEN 'good'
         WHEN month = 5          THEN 'limited'
         ELSE 'off' END
WHERE region_code='GLOBAL'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='pike');

-- ══════════════════════════════════════════════════════════════════════════════
-- Sync PL / EU / UA from updated GLOBAL
-- ══════════════════════════════════════════════════════════════════════════════
UPDATE catalog_product_seasonality cps_target
SET status = cps_src.status
FROM catalog_product_seasonality cps_src
JOIN catalog_ingredients ci ON ci.id = cps_src.product_id
WHERE cps_src.region_code = 'GLOBAL'
  AND cps_target.product_id = cps_src.product_id
  AND cps_target.month      = cps_src.month
  AND cps_target.region_code IN ('PL','EU','UA')
  AND ci.slug IN ('salmon','cod','herring','mackerel','tuna','trout','shrimp','sea-bass','carp','pike');

-- ══════════════════════════════════════════════════════════════════════════════
-- ES — Mediterranean adjustments (different from GLOBAL)
-- ══════════════════════════════════════════════════════════════════════════════

-- Mackerel ES: peak Apr-Jun (month earlier than PL)
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (4,5,6) THEN 'peak'
         WHEN month IN (3,7)   THEN 'good'
         WHEN month = 8        THEN 'limited'
         ELSE 'off' END
WHERE region_code='ES'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='mackerel');

-- Tuna ES: peak May-Jul (Mediterranean bluefin)
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (5,6,7) THEN 'peak'
         WHEN month IN (4,8)   THEN 'good'
         WHEN month IN (3,9)   THEN 'limited'
         ELSE 'off' END
WHERE region_code='ES'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='tuna');

-- Sea bass ES: peak Mar-May
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (3,4,5) THEN 'peak'
         WHEN month IN (2,6)   THEN 'good'
         WHEN month = 7        THEN 'limited'
         ELSE 'off' END
WHERE region_code='ES'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='sea-bass');

-- Shrimp ES: longer season, peak May-Aug
UPDATE catalog_product_seasonality SET status =
    CASE WHEN month IN (5,6,7,8) THEN 'peak'
         WHEN month IN (4,9)     THEN 'good'
         WHEN month IN (3,10)    THEN 'limited'
         ELSE 'off' END
WHERE region_code='ES'
  AND product_id=(SELECT id FROM catalog_ingredients WHERE slug='shrimp');
