-- Confirmed appointments: who the client is, when it was confirmed, and how it
-- was cancelled. No payments here (deposits and Stripe come later); no health
-- information belongs in `note`.
ALTER TABLE appointment
    ADD COLUMN client_name         text
        CHECK (client_name IS NULL OR length(btrim(client_name)) BETWEEN 1 AND 120),
    ADD COLUMN client_phone        text
        CHECK (client_phone IS NULL OR client_phone ~ '^\+[1-9][0-9]{6,14}$'),
    ADD COLUMN note                text CHECK (note IS NULL OR length(note) <= 500),
    ADD COLUMN confirmed_at        timestamptz,
    ADD COLUMN cancelled_at        timestamptz,
    ADD COLUMN cancelled_by_user_id uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN cancel_reason       text CHECK (cancel_reason IS NULL OR length(cancel_reason) <= 500),
    -- Cancelled inside the free-cancellation window (policy only; no money yet).
    ADD COLUMN cancel_late         boolean,
    ADD CONSTRAINT appointment_confirmed_has_client
        CHECK (status <> 'confirmed' OR (client_name IS NOT NULL AND confirmed_at IS NOT NULL)),
    ADD CONSTRAINT appointment_cancelled_has_time
        CHECK (status <> 'cancelled' OR cancelled_at IS NOT NULL);
