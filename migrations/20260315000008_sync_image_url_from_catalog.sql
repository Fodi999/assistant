-- ══════════════════════════════════════════════════════════════════════════════
-- Sync image_url from catalog_ingredients → products
-- The original seed migration copied image_url, but photos were uploaded to
-- catalog_ingredients AFTER the seed ran, so products.image_url is NULL.
-- This migration back-fills the missing values.
-- ══════════════════════════════════════════════════════════════════════════════

UPDATE products p
SET    image_url    = ci.image_url,
       updated_at   = now()
FROM   catalog_ingredients ci
WHERE  ci.id         = p.id
  AND  ci.image_url IS NOT NULL
  AND  (p.image_url IS NULL OR p.image_url = '');
