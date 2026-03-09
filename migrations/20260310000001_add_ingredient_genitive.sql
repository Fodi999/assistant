-- Add grammar columns for ingredients
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_en_gen TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_ru_gen TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_pl_gen TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_uk_gen TEXT;

ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_en_loc TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_ru_loc TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_pl_loc TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_uk_loc TEXT;

ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_en_dat TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_ru_dat TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_pl_dat TEXT;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS name_uk_dat TEXT;

-- Seed some common data
UPDATE catalog_ingredients SET 
  name_en_gen = 'wheat flour',
  name_ru_gen = 'пшеничной муки',
  name_pl_gen = 'mąki pszennej',
  name_uk_gen = 'пшеничного борошна',
  name_ru_loc = 'пшеничной муке',
  name_ru_dat = 'пшеничной муке'
WHERE slug = 'wheat-flour';

UPDATE catalog_ingredients SET 
  name_en_gen = 'rice',
  name_ru_gen = 'риса',
  name_pl_gen = 'ryżu',
  name_uk_gen = 'рису',
  name_ru_loc = 'рисе',
  name_ru_dat = 'рису'
WHERE slug = 'rice' OR slug = 'white-rice';

UPDATE catalog_ingredients SET 
  name_en_gen = 'butter',
  name_ru_gen = 'сливочного масла',
  name_pl_gen = 'masła',
  name_uk_gen = 'вершкового масла',
  name_ru_loc = 'сливочном масле',
  name_ru_dat = 'сливочному маслу'
WHERE slug = 'butter';

UPDATE catalog_ingredients SET 
  name_en_gen = 'sugar',
  name_ru_gen = 'сахара',
  name_pl_gen = 'cukru',
  name_uk_gen = 'цукру',
  name_ru_loc = 'сахаре',
  name_ru_dat = 'сахару'
WHERE slug = 'sugar';
