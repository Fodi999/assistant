-- Add missing translations for catalog ingredients
-- Products that have English names but missing Polish/Ukrainian translations

-- Pineapple (Ананас)
UPDATE catalog_ingredients 
SET 
    name_ru = 'Ананас',
    name_pl = 'Ananas',
    name_uk = 'Ананас'
WHERE name_en = 'Pineapple';

-- Ketchup (already has RU/UK, add PL)
UPDATE catalog_ingredients 
SET name_pl = 'Ketchup'
WHERE name_en = 'Ketchup' AND name_pl = 'Ketchup';

-- Oregano (already has RU/UK, add PL)  
UPDATE catalog_ingredients 
SET name_pl = 'Oregano'
WHERE name_en = 'Oregano' AND name_pl = 'Oregano';

-- Verify translations
SELECT name_en, name_ru, name_pl, name_uk 
FROM catalog_ingredients 
WHERE name_en IN ('Pineapple', 'Ketchup', 'Oregano')
ORDER BY name_en;
