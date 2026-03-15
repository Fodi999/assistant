-- Add water_percent column to catalog_ingredients
-- Used by rule_bot to calculate accurate drying/cooking nutrition transforms
ALTER TABLE catalog_ingredients
    ADD COLUMN IF NOT EXISTS water_percent DECIMAL(5,2);

-- Backfill water content based on product_type (USDA averages)
-- Fruits & vegetables: high water
UPDATE catalog_ingredients SET water_percent = 86.0 WHERE product_type = 'fruit'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 90.0 WHERE product_type = 'vegetable' AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 92.0 WHERE product_type = 'leafy'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 91.0 WHERE product_type = 'salad'     AND water_percent IS NULL;

-- Meat & fish: moderate water
UPDATE catalog_ingredients SET water_percent = 65.0 WHERE product_type = 'meat'      AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 68.0 WHERE product_type = 'fish'      AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 78.0 WHERE product_type = 'seafood'   AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 62.0 WHERE product_type = 'poultry'   AND water_percent IS NULL;

-- Dairy & eggs
UPDATE catalog_ingredients SET water_percent = 87.0 WHERE product_type = 'dairy'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 76.0 WHERE product_type = 'egg'       AND water_percent IS NULL;

-- Dry goods: low water
UPDATE catalog_ingredients SET water_percent = 12.0 WHERE product_type = 'grain'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 12.0 WHERE product_type = 'cereal'    AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 11.0 WHERE product_type = 'pasta'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 10.0 WHERE product_type = 'flour'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 12.0 WHERE product_type = 'legume'    AND water_percent IS NULL;

-- Nuts & seeds: very low water
UPDATE catalog_ingredients SET water_percent = 4.0  WHERE product_type = 'nut'       AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 5.0  WHERE product_type = 'seed'      AND water_percent IS NULL;

-- Oils & fats: negligible water
UPDATE catalog_ingredients SET water_percent = 0.0  WHERE product_type = 'oil'       AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 16.0 WHERE product_type = 'butter'    AND water_percent IS NULL;

-- Spices & herbs: low water
UPDATE catalog_ingredients SET water_percent = 10.0 WHERE product_type = 'spice'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 80.0 WHERE product_type = 'herb'      AND water_percent IS NULL;

-- Sweeteners & condiments
UPDATE catalog_ingredients SET water_percent = 17.0 WHERE product_type = 'sweetener' AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 60.0 WHERE product_type = 'sauce'     AND water_percent IS NULL;
UPDATE catalog_ingredients SET water_percent = 50.0 WHERE product_type = 'condiment' AND water_percent IS NULL;

-- Beverages
UPDATE catalog_ingredients SET water_percent = 95.0 WHERE product_type = 'beverage'  AND water_percent IS NULL;

-- Default fallback for remaining products
UPDATE catalog_ingredients SET water_percent = 70.0 WHERE water_percent IS NULL;

-- Known specific overrides (USDA values)
UPDATE catalog_ingredients SET water_percent = 94.0 WHERE slug = 'tomato';
UPDATE catalog_ingredients SET water_percent = 86.0 WHERE slug = 'apple';
UPDATE catalog_ingredients SET water_percent = 79.0 WHERE slug = 'potato';
UPDATE catalog_ingredients SET water_percent = 89.0 WHERE slug = 'onion';
UPDATE catalog_ingredients SET water_percent = 92.0 WHERE slug = 'zucchini';
UPDATE catalog_ingredients SET water_percent = 96.0 WHERE slug = 'cucumber';
UPDATE catalog_ingredients SET water_percent = 96.0 WHERE slug = 'lettuce';
UPDATE catalog_ingredients SET water_percent = 91.0 WHERE slug = 'watermelon';
UPDATE catalog_ingredients SET water_percent = 64.0 WHERE slug = 'salmon';
UPDATE catalog_ingredients SET water_percent = 81.0 WHERE slug = 'cod';
UPDATE catalog_ingredients SET water_percent = 75.0 WHERE slug = 'chicken-breast';
UPDATE catalog_ingredients SET water_percent = 61.0 WHERE slug = 'beef';
UPDATE catalog_ingredients SET water_percent = 4.0  WHERE slug = 'walnut';
UPDATE catalog_ingredients SET water_percent = 3.0  WHERE slug = 'almond';
UPDATE catalog_ingredients SET water_percent = 2.0  WHERE slug = 'cashew';
UPDATE catalog_ingredients SET water_percent = 6.0  WHERE slug = 'peanut';
UPDATE catalog_ingredients SET water_percent = 12.0 WHERE slug = 'rice';
UPDATE catalog_ingredients SET water_percent = 11.0 WHERE slug = 'oat';
UPDATE catalog_ingredients SET water_percent = 13.0 WHERE slug = 'wheat';
UPDATE catalog_ingredients SET water_percent = 100.0 WHERE slug = 'mineral-water';
UPDATE catalog_ingredients SET water_percent = 100.0 WHERE slug = 'water';
