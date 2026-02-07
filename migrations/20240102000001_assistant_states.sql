-- Assistant States (персональный state для каждого пользователя)
CREATE TABLE assistant_states (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    current_step TEXT NOT NULL DEFAULT 'Start',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Индекс для быстрого поиска по tenant
CREATE INDEX idx_assistant_states_tenant ON assistant_states(tenant_id);

-- Автообновление updated_at
CREATE OR REPLACE FUNCTION update_assistant_state_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER assistant_state_updated_at
    BEFORE UPDATE ON assistant_states
    FOR EACH ROW
    EXECUTE FUNCTION update_assistant_state_timestamp();
