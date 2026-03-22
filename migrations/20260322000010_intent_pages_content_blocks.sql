-- Add structured content_blocks to intent_pages
-- Each block: {"type": "heading"|"text"|"image", "content"?: "...", "level"?: 2, "key"?: "hero", "alt"?: "...", "src"?: "url"}
-- Images stored in R2: assets/seo/{page_id}/{key}.webp

ALTER TABLE intent_pages
ADD COLUMN IF NOT EXISTS content_blocks JSONB NOT NULL DEFAULT '[]';

COMMENT ON COLUMN intent_pages.content_blocks IS
  'Structured article blocks: heading, text, image. Images have key (hero/benefits/nutrition/cooking) and alt text.';
