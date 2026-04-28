-- Laboratory v2 — pivot: Photo → 3D Model.
--
-- New entities (replaces lab_projects/ingredients/steps/analysis pipeline):
--   • laboratory_images   — uploaded source photos
--   • laboratory_3d_assets — generated 3D models (one per image, 1:1 for MVP)
--
-- Legacy `laboratory_projects` and friends are intentionally kept in place;
-- they will be dropped in a separate cleanup migration once v2 is stable.

-- ─────────────────────────────────────────────────────────────────────────────
-- laboratory_images
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS laboratory_images (
    id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID         NOT NULL,
    -- Reserved for future B2B scoping. MVP queries by user_id only.
    tenant_id         UUID         NULL,
    image_url         TEXT         NOT NULL,
    mime_type         TEXT         NOT NULL,
    original_filename TEXT         NULL,
    byte_size         BIGINT       NULL,
    width_px          INTEGER      NULL,
    height_px         INTEGER      NULL,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_laboratory_images_user
    ON laboratory_images(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_laboratory_images_tenant
    ON laboratory_images(tenant_id) WHERE tenant_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- laboratory_3d_assets
-- ─────────────────────────────────────────────────────────────────────────────
-- One asset per (image, generation attempt). A fresh /generate-model on the
-- same image creates a NEW asset row — old ones stay for history/debugging.
--
-- status lifecycle:
--   pending → analyzing_image → generating_model → ready
--                                                ↘ failed (any earlier stage)
--
-- provider:
--   "chefos_procedural" — built-in OBJ/GLB generators (flat_card, sauce_in_bowl, …)
--   future: "meshy", "triposr", "luma" …

CREATE TABLE IF NOT EXISTS laboratory_3d_assets (
    id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    image_id          UUID         NOT NULL REFERENCES laboratory_images(id) ON DELETE CASCADE,
    user_id           UUID         NOT NULL,
    tenant_id         UUID         NULL,

    status            TEXT         NOT NULL DEFAULT 'pending'
                      CHECK (status IN (
                          'pending',
                          'analyzing_image',
                          'generating_model',
                          'ready',
                          'failed'
                      )),
    provider          TEXT         NOT NULL DEFAULT 'chefos_procedural',

    -- Vision output (Product3DSpec JSON). Populated after analyzing_image.
    object_type       TEXT         NULL,
    object_spec_json  JSONB        NULL,

    -- Generated model artefact. Populated after generating_model → ready.
    model_format      TEXT         NULL,   -- "obj" | "glb" | "gltf"
    model_url         TEXT         NULL,
    thumbnail_url     TEXT         NULL,

    error_message     TEXT         NULL,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_laboratory_3d_assets_user
    ON laboratory_3d_assets(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_laboratory_3d_assets_image
    ON laboratory_3d_assets(image_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_laboratory_3d_assets_status
    ON laboratory_3d_assets(status)
    WHERE status IN ('pending', 'analyzing_image', 'generating_model');

-- updated_at trigger (reuses helper from earlier migrations if present;
-- create lazily so this migration is self-contained).
CREATE OR REPLACE FUNCTION laboratory_3d_assets_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_laboratory_3d_assets_updated_at ON laboratory_3d_assets;
CREATE TRIGGER trg_laboratory_3d_assets_updated_at
    BEFORE UPDATE ON laboratory_3d_assets
    FOR EACH ROW
    EXECUTE FUNCTION laboratory_3d_assets_set_updated_at();
