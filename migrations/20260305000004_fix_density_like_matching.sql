-- Fix density values that were not populated because name_en doesn't match exactly
-- (e.g. DB has "Wheat flour" but migration used WHERE name_en = 'flour')

-- Use LIKE matching to fill missing densities
UPDATE catalog_ingredients SET density_g_per_ml = 0.53  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%flour%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.03  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%milk%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%butter%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.95  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%egg%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.09  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cheese%' AND LOWER(name_en) NOT LIKE '%cream cheese%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.05  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%yogurt%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.01  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%sour cream%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.01  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cream%' AND LOWER(name_en) NOT LIKE '%sour%' AND LOWER(name_en) NOT LIKE '%ice%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.85  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%sugar%' AND LOWER(name_en) NOT LIKE '%brown%' AND LOWER(name_en) NOT LIKE '%powdered%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.83  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%brown sugar%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%powdered sugar%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.05  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%salmon%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.04  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%chicken%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.05  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%beef%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.05  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%pork%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.03  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%tuna%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.03  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cod%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.04  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%shrimp%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.77  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%potato%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.95  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%tomato%' AND LOWER(name_en) NOT LIKE '%paste%' AND LOWER(name_en) NOT LIKE '%sauce%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.33  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%tomato paste%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.06  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%tomato sauce%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.74  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%onion%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.72  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%garlic%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.64  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%carrot%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.40  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%broccoli%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.25  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%spinach%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.00  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%lemon%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.96  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%avocado%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%ginger%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.60  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%pepper%' AND LOWER(name_en) NOT LIKE '%black pepper%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.55  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%mushroom%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.60  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%zucchini%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.60  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%eggplant%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.60  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cucumber%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.30  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%lettuce%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.30  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%basil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.30  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%parsley%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.30  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%dill%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.30  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cilantro%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.77  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%rice%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%pasta%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.43  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%oat%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.55  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%breadcrumb%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.91  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%olive oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%coconut oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%sesame oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%sunflower oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%vegetable oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%canola oil%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.52  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%walnut%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.55  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%almond%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.42  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%honey%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.26  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%salt%' AND LOWER(name_en) NOT LIKE '%sauce%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.00  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%water%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.08  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%soy sauce%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.01  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%vinegar%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.15  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%ketchup%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.91  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%mayonnaise%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.05  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%mustard%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cinnamon%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.50  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%black pepper%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.50  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%paprika%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.45  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cumin%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.67  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%turmeric%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.72  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%cocoa%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.90  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%baking powder%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.10  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%baking soda%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.04  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%vanilla%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.20  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%parmesan%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.09  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%peanut butter%';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%corn starch%';
UPDATE catalog_ingredients SET density_g_per_ml = 1.32  WHERE density_g_per_ml IS NULL AND LOWER(name_en) LIKE '%maple syrup%';

-- Also populate protein/fat/carbs via LIKE for items that were missed
UPDATE catalog_ingredients SET protein_per_100g = 10.0, fat_per_100g = 1.0, carbs_per_100g = 76.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%flour%';

UPDATE catalog_ingredients SET protein_per_100g = 20.0, fat_per_100g = 13.0, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%salmon%';

UPDATE catalog_ingredients SET protein_per_100g = 31.0, fat_per_100g = 3.6, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%chicken%';

UPDATE catalog_ingredients SET protein_per_100g = 26.0, fat_per_100g = 15.0, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%beef%';

UPDATE catalog_ingredients SET protein_per_100g = 18.0, fat_per_100g = 10.0, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%pork%';

UPDATE catalog_ingredients SET protein_per_100g = 23.0, fat_per_100g = 1.0, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%tuna%';

UPDATE catalog_ingredients SET protein_per_100g = 18.0, fat_per_100g = 0.7, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%cod%';

UPDATE catalog_ingredients SET protein_per_100g = 24.0, fat_per_100g = 0.3, carbs_per_100g = 0.2
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%shrimp%';

UPDATE catalog_ingredients SET protein_per_100g = 3.1, fat_per_100g = 0.3, carbs_per_100g = 3.3
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%mushroom%';

UPDATE catalog_ingredients SET protein_per_100g = 1.2, fat_per_100g = 0.3, carbs_per_100g = 3.1
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%zucchini%';

UPDATE catalog_ingredients SET protein_per_100g = 1.0, fat_per_100g = 0.2, carbs_per_100g = 6.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%eggplant%';

UPDATE catalog_ingredients SET protein_per_100g = 0.7, fat_per_100g = 0.1, carbs_per_100g = 3.6
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%cucumber%';

UPDATE catalog_ingredients SET protein_per_100g = 1.4, fat_per_100g = 0.2, carbs_per_100g = 2.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%lettuce%';

UPDATE catalog_ingredients SET protein_per_100g = 3.2, fat_per_100g = 0.6, carbs_per_100g = 2.7
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%basil%';

UPDATE catalog_ingredients SET protein_per_100g = 3.0, fat_per_100g = 0.8, carbs_per_100g = 6.3
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%parsley%';

UPDATE catalog_ingredients SET protein_per_100g = 3.5, fat_per_100g = 1.1, carbs_per_100g = 7.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%dill%';

UPDATE catalog_ingredients SET protein_per_100g = 2.1, fat_per_100g = 0.5, carbs_per_100g = 3.7
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%cilantro%';

UPDATE catalog_ingredients SET protein_per_100g = 3.6, fat_per_100g = 0.4, carbs_per_100g = 6.3
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%pepper%' AND LOWER(name_en) NOT LIKE '%black pepper%';

UPDATE catalog_ingredients SET protein_per_100g = 0.0, fat_per_100g = 100.0, carbs_per_100g = 0.0
WHERE protein_per_100g IS NULL AND LOWER(name_en) LIKE '%oil%' AND LOWER(name_en) NOT LIKE '%sesame oil%';
