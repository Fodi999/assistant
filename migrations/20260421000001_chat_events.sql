-- ══════════════════════════════════════════════════════════════════════════════
-- CHAT EVENTS — telemetry for ChefOS Chat (Step 4)
--
-- Stream of user interactions with chat cards/actions. Serves as:
--   1. Observability — what actions get tapped, which cards get dismissed
--   2. Preference learning — nightly job derives likes/dislikes from events
--   3. Ranking signal — A/B future changes to card ordering
--
-- Designed append-only; no updates. Retention policy handled separately.
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS chat_events (
    id           BIGSERIAL PRIMARY KEY,
    user_id      UUID REFERENCES users(id) ON DELETE CASCADE,
    -- Nullable to support anonymous / not-yet-signed-in sessions.
    session_id   TEXT,

    -- Event type — small enum, keep tight.
    --   'query_sent'         — user sent a message
    --   'card_shown'         — card rendered on screen
    --   'card_dismissed'     — user swiped away or explicit close
    --   'action_clicked'     — tapped a card action button
    --   'suggestion_clicked' — tapped a follow-up suggestion chip
    event_type   TEXT NOT NULL,

    -- Card / action context — all optional; shape depends on event_type.
    card_type    TEXT,       -- 'product' | 'recipe' | 'nutrition' | 'conversion'
    card_slug    TEXT,       -- product slug or recipe slug
    action_type  TEXT,       -- 'add_to_plan' | 'add_to_shopping' | 'start_cooking' | ...
    intent       TEXT,       -- detected intent for this turn
    query        TEXT,       -- raw user input (truncated at ingestion)
    lang         TEXT,       -- 'ru' | 'en' | 'pl' | 'uk'

    -- Free-form JSON for event-specific metadata (position, reason, etc.).
    metadata     JSONB DEFAULT '{}'::jsonb,

    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Hot-path indexes.
CREATE INDEX IF NOT EXISTS idx_chat_events_user_time
    ON chat_events (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_chat_events_slug_type
    ON chat_events (card_slug, event_type)
    WHERE card_slug IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_chat_events_type_time
    ON chat_events (event_type, created_at DESC);
