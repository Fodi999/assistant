-- Add media and QR links for church prayers.

ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS audio_url TEXT NOT NULL DEFAULT '';
ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS qr_code_url TEXT NOT NULL DEFAULT '';
