-- ChefOS Usage Tracking — server-side source of truth
-- Daily free limits + purchased action balance per user

CREATE TABLE user_usage (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date       DATE NOT NULL DEFAULT CURRENT_DATE,

    -- Daily counters (reset every calendar day)
    plans_used     INT NOT NULL DEFAULT 0,
    recipes_used   INT NOT NULL DEFAULT 0,
    scans_used     INT NOT NULL DEFAULT 0,
    optimize_used  INT NOT NULL DEFAULT 0,
    chats_used     INT NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, date)
);

-- Purchased actions balance (persistent, not daily)
CREATE TABLE user_action_balance (
    user_id            UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    purchased_actions  INT NOT NULL DEFAULT 0,
    total_purchased    INT NOT NULL DEFAULT 0,   -- lifetime total
    total_spent        INT NOT NULL DEFAULT 0,   -- lifetime spent
    welcome_bonus      BOOLEAN NOT NULL DEFAULT FALSE,
    last_weekly_bonus  DATE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Purchase history (audit trail)
CREATE TABLE usage_purchases (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actions     INT NOT NULL,
    source      TEXT NOT NULL DEFAULT 'iap',  -- 'iap', 'welcome_bonus', 'weekly_bonus', 'promo'
    receipt_id  TEXT,                          -- App Store receipt for verification
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_usage_date ON user_usage (date);
CREATE INDEX idx_usage_purchases_user ON usage_purchases (user_id, created_at DESC);
