-- Link language versions of a prayer through a shared translation group.

ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS translation_group_id UUID;

-- Existing rows that share (site_id, slug) are translations of each other.
UPDATE church_prayers p
SET translation_group_id = g.gid
FROM (
    SELECT site_id, slug, gen_random_uuid() AS gid
    FROM church_prayers
    GROUP BY site_id, slug
) g
WHERE p.site_id = g.site_id
  AND p.slug = g.slug
  AND p.translation_group_id IS NULL;

ALTER TABLE church_prayers ALTER COLUMN translation_group_id SET DEFAULT gen_random_uuid();
ALTER TABLE church_prayers ALTER COLUMN translation_group_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_church_prayers_translation_group
    ON church_prayers(translation_group_id, language);
