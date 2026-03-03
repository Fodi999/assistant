-- Add image_url to dishes table
ALTER TABLE dishes
ADD COLUMN IF NOT EXISTS image_url TEXT;
