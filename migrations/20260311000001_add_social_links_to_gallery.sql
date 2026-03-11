-- Add social media links to gallery table
ALTER TABLE gallery
  ADD COLUMN IF NOT EXISTS instagram_url TEXT,
  ADD COLUMN IF NOT EXISTS pinterest_url TEXT,
  ADD COLUMN IF NOT EXISTS facebook_url  TEXT,
  ADD COLUMN IF NOT EXISTS tiktok_url    TEXT,
  ADD COLUMN IF NOT EXISTS website_url   TEXT;
