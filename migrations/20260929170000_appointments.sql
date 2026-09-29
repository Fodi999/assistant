-- Appointments: slot holds now, confirmed bookings, cancellation and history in
-- the next steps of the booking stage (PRODUCT_SPEC §12-13).
--
-- Times are absolute instants (timestamptz, UTC). A row blocks its master's
-- calendar over [start_at, blocked_end): the service itself plus the buffer
-- after it. Only `held` and `confirmed` rows block; PostgreSQL guarantees that
-- two of them never overlap for one master, whatever the application does.

CREATE TABLE appointment (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id         uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id            uuid NOT NULL,
    status              text NOT NULL DEFAULT 'held'
        CHECK (status IN ('held', 'confirmed', 'cancelled', 'expired', 'completed', 'no_show')),
    start_at            timestamptz NOT NULL,
    -- End of the service itself.
    end_at              timestamptz NOT NULL,
    -- End of the blocked time: end_at + the service buffer.
    blocked_end         timestamptz NOT NULL,
    -- Slot hold: the row stops counting once this instant has passed and is
    -- marked `expired` by the next booking attempt on that calendar.
    hold_expires_at     timestamptz,
    source              text NOT NULL DEFAULT 'manual' CHECK (source IN ('app', 'web', 'manual')),
    booked_by_user_id   uuid REFERENCES users (id) ON DELETE SET NULL,
    -- Client-supplied key that makes retrying a request safe (per booking user).
    idempotency_key     text CHECK (idempotency_key IS NULL OR length(idempotency_key) BETWEEN 8 AND 100),
    -- What the key was first used for, to reject a reused key with other data.
    request_fingerprint text,
    version             integer NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT appointment_times_ok CHECK (start_at < end_at AND end_at <= blocked_end),
    CONSTRAINT appointment_hold_has_expiry CHECK (status <> 'held' OR hold_expires_at IS NOT NULL),
    UNIQUE (id, business_id),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id),
    -- No double booking: two blocking rows of one master never overlap.
    CONSTRAINT appointment_no_double_booking EXCLUDE USING gist (
        staff_id WITH =,
        tstzrange(start_at, blocked_end) WITH &&
    ) WHERE (status IN ('held', 'confirmed'))
);
CREATE INDEX appointment_calendar_idx ON appointment (business_id, staff_id, start_at);
CREATE UNIQUE INDEX appointment_idempotency_key
    ON appointment (business_id, booked_by_user_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

-- What was booked, with the price and names as they were at booking time.
CREATE TABLE appointment_item (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id    uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    appointment_id uuid NOT NULL,
    service_id     uuid NOT NULL,
    variant_id     uuid NOT NULL,
    service_name   jsonb NOT NULL,
    variant_name   jsonb,
    duration_min   integer NOT NULL CHECK (duration_min BETWEEN 5 AND 720),
    price_minor    bigint NOT NULL CHECK (price_minor >= 0),
    currency       char(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    created_at     timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (appointment_id, business_id) REFERENCES appointment (id, business_id) ON DELETE CASCADE,
    FOREIGN KEY (service_id, business_id) REFERENCES service (id, business_id),
    FOREIGN KEY (variant_id, business_id) REFERENCES service_variant (id, business_id)
);
CREATE INDEX appointment_item_appointment_idx ON appointment_item (appointment_id);

-- Append-only history of an appointment.
CREATE TABLE appointment_event (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id    uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    appointment_id uuid NOT NULL,
    type           text NOT NULL CHECK (length(type) BETWEEN 1 AND 40),
    actor_user_id  uuid REFERENCES users (id) ON DELETE SET NULL,
    data           jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at     timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (appointment_id, business_id) REFERENCES appointment (id, business_id) ON DELETE CASCADE
);
CREATE INDEX appointment_event_appointment_idx ON appointment_event (appointment_id, created_at);

CREATE TRIGGER appointment_set_updated_at BEFORE UPDATE ON appointment
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER appointment_bump_version BEFORE UPDATE ON appointment
    FOR EACH ROW EXECUTE FUNCTION bump_version();

ALTER TABLE appointment       ENABLE ROW LEVEL SECURITY;
ALTER TABLE appointment       FORCE  ROW LEVEL SECURITY;
ALTER TABLE appointment_item  ENABLE ROW LEVEL SECURITY;
ALTER TABLE appointment_item  FORCE  ROW LEVEL SECURITY;
ALTER TABLE appointment_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE appointment_event FORCE  ROW LEVEL SECURITY;

CREATE POLICY appointment_tenant ON appointment FOR ALL TO beauty_app
    USING (business_id = app_business_id())
    WITH CHECK (business_id = app_business_id());
CREATE POLICY appointment_item_tenant ON appointment_item FOR ALL TO beauty_app
    USING (business_id = app_business_id())
    WITH CHECK (business_id = app_business_id());
CREATE POLICY appointment_event_tenant ON appointment_event FOR ALL TO beauty_app
    USING (business_id = app_business_id())
    WITH CHECK (business_id = app_business_id());

-- Appointments are never deleted; history and items are never rewritten.
GRANT SELECT, INSERT, UPDATE ON appointment TO beauty_app;
GRANT SELECT, INSERT         ON appointment_item, appointment_event TO beauty_app;
