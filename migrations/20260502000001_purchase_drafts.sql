-- Purchase drafts — заготовки заказов поставщикам, создаются Copilot-ом
-- Жизненный цикл: draft → sent → received | cancelled

CREATE TABLE IF NOT EXISTS purchase_drafts (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    tenant_id       UUID NOT NULL,
    supplier_name   TEXT,
    delivery_date   DATE,
    note            TEXT,
    status          TEXT NOT NULL DEFAULT 'draft',  -- draft|sent|received|cancelled
    total_cost_cents BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_purchase_drafts_tenant   ON purchase_drafts (tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_purchase_drafts_user     ON purchase_drafts (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS purchase_draft_items (
    id                      UUID PRIMARY KEY,
    draft_id                UUID NOT NULL REFERENCES purchase_drafts(id) ON DELETE CASCADE,
    catalog_ingredient_id   UUID,
    ingredient_name         TEXT NOT NULL,
    quantity                DOUBLE PRECISION NOT NULL,
    unit                    TEXT NOT NULL DEFAULT 'kg',
    price_per_unit_cents    BIGINT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_purchase_draft_items_draft ON purchase_draft_items (draft_id);
