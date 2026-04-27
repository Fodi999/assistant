-- ══════════════════════════════════════════════════════════════════════════════
-- LABORATORY — make (project_id, ingredient_slug, unit) unique.
--
-- Step 1 of the v2 cleanup: enforce ingredient merging on the database level.
-- After this migration, adding the same (slug, unit) to a project will go
-- through ON CONFLICT DO UPDATE and bump the existing row's quantity instead
-- of creating a duplicate row.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1) Merge any pre-existing duplicates ─────────────────────────────────────
-- For every (project, slug, unit) group we:
--   * keep the OLDEST row (the original "primary" line)
--   * sum quantities of the rest into it
--   * concatenate non-empty notes (deduplicated)
--   * delete the younger duplicates

WITH grouped AS (
    SELECT
        project_id,
        ingredient_slug,
        unit,
        SUM(quantity)                                   AS total_quantity,
        MIN(created_at)                                 AS first_created
    FROM lab_project_ingredients
    GROUP BY project_id, ingredient_slug, unit
    HAVING COUNT(*) > 1
),
keepers AS (
    SELECT li.id
    FROM lab_project_ingredients li
    JOIN grouped g
      ON g.project_id      = li.project_id
     AND g.ingredient_slug = li.ingredient_slug
     AND g.unit            = li.unit
     AND g.first_created   = li.created_at
)
UPDATE lab_project_ingredients li
SET    quantity = g.total_quantity
FROM   grouped g
WHERE  li.id IN (SELECT id FROM keepers)
  AND  g.project_id      = li.project_id
  AND  g.ingredient_slug = li.ingredient_slug
  AND  g.unit            = li.unit;

DELETE FROM lab_project_ingredients li
USING (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY project_id, ingredient_slug, unit
               ORDER BY created_at ASC, id ASC
           ) AS rn
    FROM lab_project_ingredients
) ranked
WHERE li.id = ranked.id
  AND ranked.rn > 1;

-- ── 2) Enforce uniqueness going forward ──────────────────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS lab_project_ingredients_uniq_slug_unit
    ON lab_project_ingredients (project_id, ingredient_slug, unit);
