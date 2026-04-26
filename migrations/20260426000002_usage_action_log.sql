-- ChefOS Wallet — full audit trail for spend events.
--
-- Until now spends were only reflected as aggregated daily counters in
-- `user_usage`. The wallet history UI requires per-event records, so
-- every successful `perform_action` / `perform_batch` call now writes
-- one row here. Credits stay in `usage_purchases` (already exists).
--
-- The two tables together form a unified ledger:
--   credits → usage_purchases    (source = 'iap' | 'welcome_bonus' | 'weekly_bonus' | 'promo')
--   debits  → usage_action_log   (action_type + paid_from = 'free' | 'purchased')

CREATE TABLE IF NOT EXISTS usage_action_log (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type  TEXT NOT NULL,        -- 'generate_plan' | 'create_recipe' | 'scan_receipt' | 'optimize_day' | 'ai_chat'
    cost         INT  NOT NULL,        -- actions deducted (0 if served from free tier)
    paid_from    TEXT NOT NULL,        -- 'free' | 'purchased'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usage_action_log_user_time
    ON usage_action_log (user_id, created_at DESC);
