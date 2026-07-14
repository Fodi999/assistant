-- Link language versions of an icon through a shared translation group,
-- mirroring church_prayers.translation_group_id.

ALTER TABLE church_icons ADD COLUMN IF NOT EXISTS translation_group_id UUID;

-- Existing rows that share (site_id, slug) are translations of each other.
UPDATE church_icons i
SET translation_group_id = g.gid
FROM (
    SELECT site_id, slug, gen_random_uuid() AS gid
    FROM church_icons
    GROUP BY site_id, slug
) g
WHERE i.site_id = g.site_id
  AND i.slug = g.slug
  AND i.translation_group_id IS NULL;

ALTER TABLE church_icons ALTER COLUMN translation_group_id SET DEFAULT gen_random_uuid();
ALTER TABLE church_icons ALTER COLUMN translation_group_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_church_icons_translation_group
    ON church_icons(translation_group_id, language);
