-- Add availability_months column to catalog_ingredients
-- BOOLEAN[12] = per-month availability (index 1=Jan, 2=Feb, ... 12=Dec)
-- Source of truth for fish seasonality calendar

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS availability_months BOOLEAN[12];

-- Fill availability_months for all 11 fish from the old hardcoded FISH_TABLE
-- Each array: {Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec}

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  true,  true,  false, false, false, true,  true,  true,  true,  true,  true
] WHERE slug = 'salmon';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  false, false, false, true,  true,  true,  true,  true,  true,  false, false, false
] WHERE slug = 'tuna';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true
] WHERE slug = 'canned-tuna';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  true,  true,  true,  false, false, false, false, false, true,  true,  true
] WHERE slug = 'cod';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  true,  false, false, false, false, false, false, true,  true,  true,  true
] WHERE slug = 'herring';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  false, false, true,  true,  true,  false, false, false, true,  true,  true,  false
] WHERE slug = 'trout';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  false, false, false, false, true,  true,  true,  true,  true,  true,  false, false
] WHERE slug = 'mackerel';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  false, false, true,  true,  true,  true,  true,  false, false, false, false, false
] WHERE slug = 'sea-bass';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  true,  true,  true,  false, false, false, false, false, false, true,  true
] WHERE slug = 'pike';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  true,  false, false, false, false, false, false, false, false, true,  true,  true
] WHERE slug = 'carp';

UPDATE catalog_ingredients SET availability_months = ARRAY[
  false, false, false, true,  true,  true,  true,  true,  true,  false, false, false
] WHERE slug = 'shrimp';
