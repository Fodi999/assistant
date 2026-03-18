-- ══════════════════════════════════════════════════════════════════════════════
-- DATA FIX: Fill missing water_type/wild_farmed for fish + reclassify fish vs seafood
--
-- Real fish → product_type = 'fish'
-- Shellfish/crustaceans (shrimp) → stays 'seafood'
-- Fill missing water_type and wild_farmed for all fish products
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Reclassify real fish: seafood → fish ──
-- (in both catalog_ingredients and products)

-- catalog_ingredients
UPDATE catalog_ingredients SET product_type = 'fish'
WHERE slug IN (
    'salmon','tuna','cod','herring','mackerel','sea-bass','trout','carp',
    'pike','bream','catfish','crucian-carp','eel','canned-tuna'
) AND product_type = 'seafood';

-- products
UPDATE products SET product_type = 'fish'
WHERE slug IN (
    'salmon','tuna','cod','herring','mackerel','sea-bass','trout','carp',
    'pike','bream','catfish','crucian-carp','eel','canned-tuna'
) AND product_type = 'seafood';

-- ── 2. Fill missing water_type ──

-- catalog_ingredients
UPDATE catalog_ingredients SET water_type = 'freshwater' WHERE slug = 'bream'        AND water_type IS NULL;
UPDATE catalog_ingredients SET water_type = 'freshwater' WHERE slug = 'catfish'      AND water_type IS NULL;
UPDATE catalog_ingredients SET water_type = 'freshwater' WHERE slug = 'crucian-carp' AND water_type IS NULL;
UPDATE catalog_ingredients SET water_type = 'both'       WHERE slug = 'eel'          AND water_type IS NULL;

-- products
UPDATE products SET water_type = 'freshwater' WHERE slug = 'bream'        AND water_type IS NULL;
UPDATE products SET water_type = 'freshwater' WHERE slug = 'catfish'      AND water_type IS NULL;
UPDATE products SET water_type = 'freshwater' WHERE slug = 'crucian-carp' AND water_type IS NULL;
UPDATE products SET water_type = 'both'       WHERE slug = 'eel'          AND water_type IS NULL;

-- ── 3. Fill missing wild_farmed ──

-- catalog_ingredients
UPDATE catalog_ingredients SET wild_farmed = 'wild' WHERE slug = 'bream'        AND wild_farmed IS NULL;
UPDATE catalog_ingredients SET wild_farmed = 'both' WHERE slug = 'catfish'      AND wild_farmed IS NULL;
UPDATE catalog_ingredients SET wild_farmed = 'wild' WHERE slug = 'crucian-carp' AND wild_farmed IS NULL;
UPDATE catalog_ingredients SET wild_farmed = 'wild' WHERE slug = 'eel'          AND wild_farmed IS NULL;

-- products
UPDATE products SET wild_farmed = 'wild' WHERE slug = 'bream'        AND wild_farmed IS NULL;
UPDATE products SET wild_farmed = 'both' WHERE slug = 'catfish'      AND wild_farmed IS NULL;
UPDATE products SET wild_farmed = 'wild' WHERE slug = 'crucian-carp' AND wild_farmed IS NULL;
UPDATE products SET wild_farmed = 'wild' WHERE slug = 'eel'          AND wild_farmed IS NULL;
