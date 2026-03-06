-- Migration: add image_url for existing fish entries

UPDATE catalog_ingredients SET image_url = 'https://cdn.dima-fomin.pl/ingredients/salmon.webp'
WHERE slug = 'salmon';

UPDATE catalog_ingredients SET image_url = 'https://cdn.dima-fomin.pl/ingredients/cod.webp'
WHERE slug = 'cod';

UPDATE catalog_ingredients SET image_url = 'https://cdn.dima-fomin.pl/ingredients/herring.webp'
WHERE slug = 'herring';

UPDATE catalog_ingredients SET image_url = 'https://cdn.dima-fomin.pl/ingredients/tuna.webp'
WHERE slug = 'canned-tuna';

UPDATE catalog_ingredients SET image_url = 'https://cdn.dima-fomin.pl/ingredients/shrimp.webp'
WHERE slug = 'shrimp';
