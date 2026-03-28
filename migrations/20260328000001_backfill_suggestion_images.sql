-- Backfill image_url inside smart_response JSON for existing lab_combo_pages.
-- Two arrays live inside smart_response: "suggestions" and "pairings".
-- Both contain objects with { slug, image_url } where image_url may be null
-- even though catalog_ingredients has the real photo.

-- 1) Backfill suggestions[].image_url
UPDATE lab_combo_pages lcp
SET smart_response = (
    SELECT jsonb_set(
        lcp.smart_response::jsonb,
        '{suggestions}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN (elem->>'image_url') IS NULL
                             AND (elem->>'slug') IS NOT NULL
                             AND ci.image_url IS NOT NULL
                        THEN elem || jsonb_build_object('image_url', ci.image_url)
                        ELSE elem
                    END
                    ORDER BY idx
                )
                FROM jsonb_array_elements(lcp.smart_response::jsonb->'suggestions')
                     WITH ORDINALITY AS t(elem, idx)
                LEFT JOIN catalog_ingredients ci
                    ON ci.slug = elem->>'slug'
                   AND COALESCE(ci.is_active, true) = true
            ),
            '[]'::jsonb
        )
    )
)
WHERE smart_response::jsonb ? 'suggestions'
  AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements(lcp.smart_response::jsonb->'suggestions') AS s(elem)
      WHERE (s.elem->>'image_url') IS NULL
        AND (s.elem->>'slug') IS NOT NULL
  );

-- 2) Backfill pairings[].image_url
UPDATE lab_combo_pages lcp
SET smart_response = (
    SELECT jsonb_set(
        lcp.smart_response::jsonb,
        '{pairings}',
        COALESCE(
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN (elem->>'image_url') IS NULL
                             AND (elem->>'slug') IS NOT NULL
                             AND ci.image_url IS NOT NULL
                        THEN elem || jsonb_build_object('image_url', ci.image_url)
                        ELSE elem
                    END
                    ORDER BY idx
                )
                FROM jsonb_array_elements(lcp.smart_response::jsonb->'pairings')
                     WITH ORDINALITY AS t(elem, idx)
                LEFT JOIN catalog_ingredients ci
                    ON ci.slug = elem->>'slug'
                   AND COALESCE(ci.is_active, true) = true
            ),
            '[]'::jsonb
        )
    )
)
WHERE smart_response::jsonb ? 'pairings'
  AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements(lcp.smart_response::jsonb->'pairings') AS s(elem)
      WHERE (s.elem->>'image_url') IS NULL
        AND (s.elem->>'slug') IS NOT NULL
  );
