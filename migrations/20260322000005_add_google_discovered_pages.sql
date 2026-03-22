-- Add google_discovered_pages to seo_settings
-- This is the baseline metric from Google Search Console
-- Manually updated via admin UI → PUT /api/admin/intent-pages/settings

INSERT INTO seo_settings (key, value)
VALUES ('google_discovered_pages', '7192')
ON CONFLICT (key) DO NOTHING;
