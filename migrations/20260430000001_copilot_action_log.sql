-- ChefOS Copilot — audit log table.
--
-- Записывает каждое действие Copilot-а:
--   planned              → план создан
--   awaiting_confirmation → ждёт подтверждения
--   executed             → выполнен
--   cancelled            → отменён
--   failed               → ошибка
--
-- action_payload — полный ActionPlan JSON для воспроизведения при confirm.

CREATE TABLE IF NOT EXISTS copilot_action_log (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id               UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id             UUID        NOT NULL,
    screen                TEXT        NOT NULL,
    input_message         TEXT        NOT NULL,
    intent                TEXT        NOT NULL DEFAULT '',
    used_tools            JSONB       NOT NULL DEFAULT '[]',
    action_payload        JSONB,
    status                TEXT        NOT NULL DEFAULT 'planned',
    requires_confirmation BOOLEAN     NOT NULL DEFAULT FALSE,
    confirmed_at          TIMESTAMPTZ,
    executed_at           TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_copilot_log_user_time
    ON copilot_action_log (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_copilot_log_status
    ON copilot_action_log (status)
    WHERE status IN ('awaiting_confirmation', 'planned');
