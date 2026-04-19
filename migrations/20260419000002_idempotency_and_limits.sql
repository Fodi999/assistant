-- Idempotency keys for safe retries
CREATE TABLE idempotency_keys (
    key         TEXT NOT NULL,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    response    JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (key, user_id)
);

-- Auto-expire old keys (keep 24h)
CREATE INDEX idx_idempotency_created ON idempotency_keys (created_at);

-- Server-driven limits (changeable without deploy)
CREATE TABLE usage_limits (
    id              INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- singleton row
    plans           INT NOT NULL DEFAULT 2,
    recipes         INT NOT NULL DEFAULT 2,
    scans           INT NOT NULL DEFAULT 1,
    optimize        INT NOT NULL DEFAULT 1,
    chats           INT NOT NULL DEFAULT 10,
    cost_plan       INT NOT NULL DEFAULT 5,
    cost_recipe     INT NOT NULL DEFAULT 3,
    cost_scan       INT NOT NULL DEFAULT 2,
    cost_optimize   INT NOT NULL DEFAULT 4,
    cost_chat       INT NOT NULL DEFAULT 1,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default limits
INSERT INTO usage_limits (id) VALUES (1);
