-- Master schedules: weekly working hours, breaks, time off and one-day exceptions.
-- Requirements: docs/PRODUCT_SPEC.md §10.3, §12.1, §12.3, §17.
--
-- Time model
--   * Working hours, breaks and exceptions are "wall clock" times in the
--     business time zone (business.timezone, an IANA name, Europe/Warsaw at the
--     start). Storing them this way keeps the schedule right across daylight
--     saving changes; they are converted to UTC only when slots are computed.
--   * Time off is an absolute UTC interval (timestamptz), like appointments.
--   * weekday: 0 = Monday ... 6 = Sunday.
--   * Intervals do not cross midnight (end is later than start on the same day).
--
-- Every row carries business_id. The composite foreign key (staff_id,
-- business_id) makes it impossible to attach a schedule to another business's
-- staff member, and row-level security fences every read and write.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Minutes since midnight, for overlap checks on time-of-day ranges.
CREATE FUNCTION local_minutes(t time) RETURNS integer
    LANGUAGE sql IMMUTABLE STRICT
AS $$ SELECT (EXTRACT(HOUR FROM t) * 60 + EXTRACT(MINUTE FROM t))::integer $$;

-- ---------------------------------------------------------------------------
-- Weekly working hours. A day may have several intervals (split shift); they
-- may not overlap while both are valid. valid_from/valid_to (inclusive, local
-- dates) let a new week pattern take over on a given date.
-- ---------------------------------------------------------------------------
CREATE TABLE working_schedule (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id    uuid NOT NULL,
    weekday     smallint NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    start_local time NOT NULL,
    end_local   time NOT NULL,
    valid_from  date,
    valid_to    date,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT working_schedule_order CHECK (start_local < end_local),
    CONSTRAINT working_schedule_validity CHECK (
        valid_from IS NULL OR valid_to IS NULL OR valid_from <= valid_to
    ),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id) ON DELETE CASCADE,
    CONSTRAINT working_schedule_no_overlap EXCLUDE USING gist (
        staff_id WITH =,
        weekday WITH =,
        daterange(valid_from, valid_to, '[]') WITH &&,
        int4range(local_minutes(start_local), local_minutes(end_local)) WITH &&
    )
);
CREATE INDEX working_schedule_staff_idx ON working_schedule (staff_id, weekday);

-- Breaks inside the working day (lunch, cleaning).
CREATE TABLE schedule_break (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id    uuid NOT NULL,
    weekday     smallint NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    start_local time NOT NULL,
    end_local   time NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT schedule_break_order CHECK (start_local < end_local),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id) ON DELETE CASCADE,
    CONSTRAINT schedule_break_no_overlap EXCLUDE USING gist (
        staff_id WITH =,
        weekday WITH =,
        int4range(local_minutes(start_local), local_minutes(end_local)) WITH &&
    )
);
CREATE INDEX schedule_break_staff_idx ON schedule_break (staff_id, weekday);

-- Vacation, sick leave, blocked time. Absolute (UTC) interval.
-- rrule is reserved for recurring blocks; the API does not accept it yet.
CREATE TABLE time_off (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id    uuid NOT NULL,
    start_at    timestamptz NOT NULL,
    end_at      timestamptz NOT NULL,
    kind        text NOT NULL CHECK (kind IN ('vacation', 'sick', 'blocked', 'break')),
    rrule       text,
    note        text CHECK (note IS NULL OR length(note) <= 500),
    created_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT time_off_order CHECK (start_at < end_at),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id) ON DELETE CASCADE
);
CREATE INDEX time_off_staff_idx ON time_off (staff_id, start_at);

-- One-day override of the weekly pattern: a day off, or different hours.
CREATE TABLE schedule_exception (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id    uuid NOT NULL,
    date_local  date NOT NULL,
    kind        text NOT NULL CHECK (kind IN ('day_off', 'custom_hours')),
    start_local time,
    end_local   time,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT schedule_exception_hours CHECK (
        (kind = 'day_off' AND start_local IS NULL AND end_local IS NULL)
        OR (kind = 'custom_hours' AND start_local IS NOT NULL AND end_local IS NOT NULL
            AND start_local < end_local)
    ),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id) ON DELETE CASCADE,
    UNIQUE (staff_id, date_local)
);

-- ---------------------------------------------------------------------------
-- Triggers, privileges, row-level security
-- ---------------------------------------------------------------------------
CREATE TRIGGER working_schedule_set_updated_at BEFORE UPDATE ON working_schedule
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER schedule_break_set_updated_at BEFORE UPDATE ON schedule_break
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER time_off_set_updated_at BEFORE UPDATE ON time_off
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER schedule_exception_set_updated_at BEFORE UPDATE ON schedule_exception
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

GRANT SELECT, INSERT, UPDATE, DELETE ON working_schedule, schedule_break, schedule_exception,
                                        time_off TO beauty_app;

ALTER TABLE working_schedule   ENABLE ROW LEVEL SECURITY;
ALTER TABLE working_schedule   FORCE  ROW LEVEL SECURITY;
ALTER TABLE schedule_break     ENABLE ROW LEVEL SECURITY;
ALTER TABLE schedule_break     FORCE  ROW LEVEL SECURITY;
ALTER TABLE time_off           ENABLE ROW LEVEL SECURITY;
ALTER TABLE time_off           FORCE  ROW LEVEL SECURITY;
ALTER TABLE schedule_exception ENABLE ROW LEVEL SECURITY;
ALTER TABLE schedule_exception FORCE  ROW LEVEL SECURITY;

CREATE POLICY working_schedule_tenant ON working_schedule FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY schedule_break_tenant ON schedule_break FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY time_off_tenant ON time_off FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
CREATE POLICY schedule_exception_tenant ON schedule_exception FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());
