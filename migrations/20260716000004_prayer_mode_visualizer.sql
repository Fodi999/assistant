-- "Prayer mode" immersive WebGL visualizer settings, attached per-prayer
-- (each language row of a prayer can have its own image/settings, same
-- convention already used for audio_url/image_url on church_prayers).

ALTER TABLE church_prayers
    ADD COLUMN IF NOT EXISTS visualizer_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS visualizer_image_url TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS particle_count_desktop INTEGER NOT NULL DEFAULT 50000,
    ADD COLUMN IF NOT EXISTS particle_count_mobile INTEGER NOT NULL DEFAULT 16000,
    ADD COLUMN IF NOT EXISTS particle_size REAL NOT NULL DEFAULT 2.0,
    ADD COLUMN IF NOT EXISTS particle_color_mode TEXT NOT NULL DEFAULT 'silver_gold',
    ADD COLUMN IF NOT EXISTS background_color TEXT NOT NULL DEFAULT '#000000',
    ADD COLUMN IF NOT EXISTS audio_reactivity REAL NOT NULL DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS scene_timeline JSONB NOT NULL DEFAULT
        '{"idle":2000,"assemble":2500,"reveal":1500,"dissolve":2000}'::jsonb,
    ADD COLUMN IF NOT EXISTS subtitle_cues JSONB NOT NULL DEFAULT '[]'::jsonb;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'church_prayers_particle_color_mode_check'
    ) THEN
        ALTER TABLE church_prayers
            ADD CONSTRAINT church_prayers_particle_color_mode_check
            CHECK (particle_color_mode IN ('silver_gold', 'gold', 'silver', 'warm_white'));
    END IF;
END $$;
