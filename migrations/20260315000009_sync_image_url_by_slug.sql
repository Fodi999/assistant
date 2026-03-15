-- ══════════════════════════════════════════════════════════════════════════════
-- Sync image_url from catalog_ingredients → products (by slug)
-- Previous migration matched by id, which may differ.
-- This one matches by slug as a fallback.
-- ══════════════════════════════════════════════════════════════════════════════

UPDATE products p
SET    image_url  = ci.image_url,
       updated_at = now()
FROM   catalog_ingredients ci
WHERE  ci.slug       = p.slug
  AND  ci.image_url IS NOT NULL
  AND  (p.image_url IS NULL OR p.image_url = '');
