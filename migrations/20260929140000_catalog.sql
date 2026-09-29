-- Service catalog: categories, services, price/duration variants, and which
-- staff member performs which service.
-- Requirements: docs/PRODUCT_SPEC.md §10.2, §7.1, §17.
--
-- Conventions (same for every tenant table below)
--   * Money is an integer in minor units (grosze) plus an ISO 4217 currency.
--   * Names and descriptions are i18n objects: {"pl": "...", "en": "..."}.
--   * Every row carries business_id; row-level security compares it with the
--     request scope (fail closed, FORCE so even the owner role is subject to it).
--   * Cross-table references are COMPOSITE (id, business_id). A plain foreign key
--     is checked without row-level security, so it would let one business point
--     at another business's row; the composite key makes that impossible.
--   * Soft delete (deleted_at) for rows that bookings will reference later;
--     `version` increases on every update (optimistic locking).

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------
CREATE FUNCTION bump_version() RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    NEW.version = OLD.version + 1;
    RETURN NEW;
END
$$;

-- An i18n text: a non-empty object whose keys are supported locales and whose
-- values are non-empty strings of at most max_len characters.
CREATE FUNCTION i18n_text_ok(v jsonb, max_len integer) RETURNS boolean
    LANGUAGE sql IMMUTABLE
AS $$
    SELECT jsonb_typeof(v) = 'object'
       AND v <> '{}'::jsonb
       AND NOT EXISTS (
            SELECT 1
            FROM jsonb_each(v) AS e
            WHERE e.key NOT IN ('pl', 'en', 'ru', 'uk')
               OR jsonb_typeof(e.value) <> 'string'
               OR length(btrim(e.value #>> '{}')) = 0
               OR length(e.value #>> '{}') > max_len
       )
$$;

-- Target of composite foreign keys.
ALTER TABLE staff_member ADD CONSTRAINT staff_member_id_business_key UNIQUE (id, business_id);

-- ---------------------------------------------------------------------------
-- Tables
-- ---------------------------------------------------------------------------
CREATE TABLE service_category (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    name        jsonb NOT NULL CHECK (i18n_text_ok(name, 120)),
    sort_order  integer NOT NULL DEFAULT 0,
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    deleted_at  timestamptz,
    UNIQUE (id, business_id)
);
CREATE INDEX service_category_business_idx ON service_category (business_id, sort_order)
    WHERE deleted_at IS NULL;

CREATE TABLE service (
    id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id          uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    category_id          uuid,
    name                 jsonb NOT NULL CHECK (i18n_text_ok(name, 255)),
    description          jsonb CHECK (description IS NULL OR i18n_text_ok(description, 2000)),
    is_active            boolean NOT NULL DEFAULT true,
    is_online_bookable   boolean NOT NULL DEFAULT true,
    -- Slot grid for online booking (minutes between offered start times).
    booking_step_minutes integer NOT NULL DEFAULT 15
        CHECK (booking_step_minutes IN (5, 10, 15, 20, 30, 60)),
    -- Cleaning/preparation time after the service; blocks the calendar too.
    buffer_after_min     integer NOT NULL DEFAULT 0 CHECK (buffer_after_min BETWEEN 0 AND 240),
    min_notice_min       integer NOT NULL DEFAULT 120 CHECK (min_notice_min BETWEEN 0 AND 43200),
    max_advance_days     integer NOT NULL DEFAULT 90 CHECK (max_advance_days BETWEEN 1 AND 365),
    intake_questions     jsonb NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(intake_questions) = 'array'),
    sort_order           integer NOT NULL DEFAULT 0,
    version              integer NOT NULL DEFAULT 1,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),
    deleted_at           timestamptz,
    UNIQUE (id, business_id),
    FOREIGN KEY (category_id, business_id) REFERENCES service_category (id, business_id)
);
CREATE INDEX service_business_idx ON service (business_id, sort_order) WHERE deleted_at IS NULL;
CREATE INDEX service_category_idx ON service (category_id) WHERE deleted_at IS NULL;

-- One row per bookable option of a service (e.g. Classic 120 min / 250 PLN).
CREATE TABLE service_variant (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    service_id   uuid NOT NULL,
    -- NULL: the variant is shown under the service's own name.
    name         jsonb CHECK (name IS NULL OR i18n_text_ok(name, 255)),
    duration_min integer NOT NULL CHECK (duration_min BETWEEN 5 AND 720),
    price_minor  bigint NOT NULL CHECK (price_minor BETWEEN 0 AND 100000000),
    price_type   text NOT NULL DEFAULT 'fixed' CHECK (price_type IN ('fixed', 'from')),
    currency     char(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    is_active    boolean NOT NULL DEFAULT true,
    sort_order   integer NOT NULL DEFAULT 0,
    version      integer NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    deleted_at   timestamptz,
    UNIQUE (id, business_id),
    FOREIGN KEY (service_id, business_id) REFERENCES service (id, business_id)
);
CREATE INDEX service_variant_service_idx ON service_variant (service_id, sort_order)
    WHERE deleted_at IS NULL;

-- Which staff member performs which service. Per-variant price/duration
-- overrides are added with the booking stage, where they are first needed.
CREATE TABLE staff_service (
    staff_id    uuid NOT NULL,
    service_id  uuid NOT NULL,
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (staff_id, service_id),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id) ON DELETE CASCADE,
    FOREIGN KEY (service_id, business_id) REFERENCES service (id, business_id) ON DELETE CASCADE
);
CREATE INDEX staff_service_service_idx ON staff_service (service_id);

-- ---------------------------------------------------------------------------
-- Triggers
-- ---------------------------------------------------------------------------
CREATE TRIGGER service_category_set_updated_at BEFORE UPDATE ON service_category
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER service_set_updated_at BEFORE UPDATE ON service
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER service_variant_set_updated_at BEFORE UPDATE ON service_variant
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER service_category_bump_version BEFORE UPDATE ON service_category
    FOR EACH ROW EXECUTE FUNCTION bump_version();
CREATE TRIGGER service_bump_version BEFORE UPDATE ON service
    FOR EACH ROW EXECUTE FUNCTION bump_version();
CREATE TRIGGER service_variant_bump_version BEFORE UPDATE ON service_variant
    FOR EACH ROW EXECUTE FUNCTION bump_version();

-- ---------------------------------------------------------------------------
-- Privileges and row-level security
-- ---------------------------------------------------------------------------
GRANT SELECT, INSERT, UPDATE ON service_category, service, service_variant TO beauty_app;
GRANT SELECT, INSERT, DELETE ON staff_service TO beauty_app;

ALTER TABLE service_category ENABLE ROW LEVEL SECURITY;
ALTER TABLE service_category FORCE  ROW LEVEL SECURITY;
ALTER TABLE service          ENABLE ROW LEVEL SECURITY;
ALTER TABLE service          FORCE  ROW LEVEL SECURITY;
ALTER TABLE service_variant  ENABLE ROW LEVEL SECURITY;
ALTER TABLE service_variant  FORCE  ROW LEVEL SECURITY;
ALTER TABLE staff_service    ENABLE ROW LEVEL SECURITY;
ALTER TABLE staff_service    FORCE  ROW LEVEL SECURITY;

CREATE POLICY service_category_tenant ON service_category FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY service_tenant ON service FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY service_variant_tenant ON service_variant FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY staff_service_tenant ON staff_service FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
