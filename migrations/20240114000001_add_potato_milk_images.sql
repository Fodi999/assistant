-- Add image URLs to potato and milk in catalog

-- Картофель (Potato)
UPDATE catalog_ingredients
SET image_url = 'https://i.postimg.cc/x8dz4b9r/fodifood-single-whole-potato-top-view-flat-lay-food-photography-2d1571c0-5cbe-4c2c-bd13-b43968b3db68.png'
WHERE name_en = 'Potato';

-- Молоко (Pasteurized milk)
UPDATE catalog_ingredients
SET image_url = 'https://i.postimg.cc/0QPm7B4H/fodifood_single_glass_bottle_filled_with_milk_one_liter_top_vie_8f61ce21_9e27_47e0_8da5_75da410bd79d.png'
WHERE name_en = 'Pasteurized milk';
