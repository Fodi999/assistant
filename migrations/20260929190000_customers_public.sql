-- C6: customers (clients), the public profile of a business, and platform
-- moderation. Only businesses that a platform admin approved AND whose owner
-- published the profile are visible to the public.
--
-- Isolation notes
--   * A client belongs to exactly one business (tenant RLS on business_id).
--   * A customer's identity is a user session (registered or guest). Phone and
--     e-mail are contact data only, never used to find "who someone is".
--   * Public visibility is a row policy on `business` itself (no subquery into
--     other tenant tables), so it cannot recurse and cannot leak other rows.
--   * moderation_status can only be changed by a platform admin (trigger).

-- ---------------------------------------------------------------------------
-- Platform admins (created by an operator with SQL / the make_admin binary)
-- ---------------------------------------------------------------------------
CREATE TABLE platform_admin (
    user_id    uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT ON platform_admin TO beauty_app;
ALTER TABLE platform_admin ENABLE ROW LEVEL SECURITY;
-- Everybody can only see their own row, so "am I an admin?" is the only
-- question the application role can ask.
CREATE POLICY platform_admin_self ON platform_admin FOR SELECT TO beauty_app
    USING (user_id = app_user_id());

CREATE FUNCTION is_platform_admin() RETURNS boolean
    LANGUAGE sql STABLE
AS $$ SELECT EXISTS (SELECT 1 FROM platform_admin WHERE user_id = app_user_id()) $$;

-- ---------------------------------------------------------------------------
-- Public profile and moderation state on the business
-- ---------------------------------------------------------------------------
ALTER TABLE business
    ADD COLUMN moderation_status text NOT NULL DEFAULT 'pending'
        CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'suspended')),
    ADD COLUMN moderated_at timestamptz,
    -- The owner's switch: "show me in the public catalog once approved".
    ADD COLUMN is_published boolean NOT NULL DEFAULT false,
    ADD COLUMN city      text CHECK (city IS NULL OR length(btrim(city)) BETWEEN 1 AND 80),
    ADD COLUMN headline  text CHECK (headline IS NULL OR length(btrim(headline)) BETWEEN 1 AND 140),
    ADD COLUMN about     text CHECK (about IS NULL OR length(about) <= 2000),
    ADD COLUMN instagram text CHECK (instagram IS NULL OR instagram ~ '^[A-Za-z0-9._]{1,30}$');
CREATE INDEX business_public_idx ON business (lower(city), name)
    WHERE moderation_status = 'approved' AND is_published AND status = 'active' AND deleted_at IS NULL;

-- Guards the application role only: an operator working in SQL as the schema
-- owner (or a superuser fixture) is not the application and may set anything.
CREATE FUNCTION business_guard_moderation() RETURNS trigger
    LANGUAGE plpgsql
AS $$
DECLARE
    is_app boolean;
BEGIN
    SELECT pg_has_role(current_user, 'beauty_app', 'MEMBER') AND NOT r.rolsuper
      INTO is_app
      FROM pg_roles r WHERE r.rolname = current_user;
    IF NOT COALESCE(is_app, false) OR is_platform_admin() THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.moderation_status <> 'pending' THEN
            RAISE EXCEPTION 'moderation_status is set by the platform' USING ERRCODE = '42501';
        END IF;
    ELSIF NEW.moderation_status IS DISTINCT FROM OLD.moderation_status
       OR NEW.moderated_at IS DISTINCT FROM OLD.moderated_at THEN
        RAISE EXCEPTION 'moderation_status is set by the platform' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER business_guard_moderation BEFORE INSERT OR UPDATE ON business
    FOR EACH ROW EXECUTE FUNCTION business_guard_moderation();

-- Public catalog: approved + published + active businesses are readable by any
-- session (also anonymous). The application selects only whitelisted columns.
CREATE POLICY business_public_select ON business FOR SELECT TO beauty_app
    USING (moderation_status = 'approved' AND is_published
           AND status = 'active' AND deleted_at IS NULL);
-- Platform admins see every business (the moderation queue).
CREATE POLICY business_admin_select ON business FOR SELECT TO beauty_app
    USING (is_platform_admin());
-- ... and the owner's contact data needed to decide.
CREATE POLICY membership_admin_select ON membership FOR SELECT TO beauty_app
    USING (is_platform_admin());
CREATE POLICY users_admin_select ON users FOR SELECT TO beauty_app
    USING (is_platform_admin());

-- Every moderation decision, with the reason. Append-only.
CREATE TABLE business_moderation (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    status      text NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'suspended')),
    note        text CHECK (note IS NULL OR length(note) <= 1000),
    decided_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX business_moderation_business_idx ON business_moderation (business_id, created_at DESC);
GRANT SELECT, INSERT ON business_moderation TO beauty_app;
ALTER TABLE business_moderation ENABLE ROW LEVEL SECURITY;
ALTER TABLE business_moderation FORCE  ROW LEVEL SECURITY;
-- The business sees its own history (so the owner learns why a request was
-- rejected); only a platform admin writes.
CREATE POLICY business_moderation_read ON business_moderation FOR SELECT TO beauty_app
    USING (business_id = app_business_id() OR is_platform_admin());
CREATE POLICY business_moderation_write ON business_moderation FOR INSERT TO beauty_app
    WITH CHECK (is_platform_admin());

-- Portfolio: model only for now (image upload comes with file storage).
CREATE TABLE portfolio_item (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    staff_id     uuid,
    image_url    text NOT NULL CHECK (length(image_url) BETWEEN 1 AND 500),
    caption      text CHECK (caption IS NULL OR length(caption) <= 200),
    sort_order   integer NOT NULL DEFAULT 0,
    is_published boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (staff_id, business_id) REFERENCES staff_member (id, business_id)
);
CREATE INDEX portfolio_item_business_idx ON portfolio_item (business_id, sort_order);
GRANT SELECT, INSERT, UPDATE, DELETE ON portfolio_item TO beauty_app;
ALTER TABLE portfolio_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE portfolio_item FORCE  ROW LEVEL SECURITY;
CREATE POLICY portfolio_item_tenant ON portfolio_item FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());

-- ---------------------------------------------------------------------------
-- Clients of a business
-- ---------------------------------------------------------------------------
CREATE TABLE client (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    -- The signed-in customer (registered or guest session); NULL for a client
    -- the staff typed in themselves.
    user_id     uuid REFERENCES users (id) ON DELETE SET NULL,
    -- staff: added by the business; guest: anonymous app session; account: registered user.
    source      text NOT NULL CHECK (source IN ('staff', 'guest', 'account')),
    full_name   text NOT NULL CHECK (length(btrim(full_name)) BETWEEN 1 AND 120),
    phone_e164  text CHECK (phone_e164 IS NULL OR phone_e164 ~ '^\+[1-9][0-9]{6,14}$'),
    email       text CHECK (email IS NULL OR (email = lower(email) AND length(email) <= 254
                                              AND position('@' IN email) > 1)),
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (id, business_id)
);
CREATE UNIQUE INDEX client_business_user_key ON client (business_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX client_business_name_idx ON client (business_id, lower(full_name));
CREATE INDEX client_business_phone_idx ON client (business_id, phone_e164) WHERE phone_e164 IS NOT NULL;
CREATE TRIGGER client_set_updated_at BEFORE UPDATE ON client
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
GRANT SELECT, INSERT, UPDATE ON client TO beauty_app;
ALTER TABLE client ENABLE ROW LEVEL SECURITY;
ALTER TABLE client FORCE  ROW LEVEL SECURITY;
CREATE POLICY client_tenant ON client FOR ALL TO beauty_app
    USING (business_id = app_business_id()) WITH CHECK (business_id = app_business_id());

ALTER TABLE appointment
    ADD COLUMN client_id uuid,
    ADD CONSTRAINT appointment_client_fk
        FOREIGN KEY (client_id, business_id) REFERENCES client (id, business_id);
CREATE INDEX appointment_client_idx ON appointment (business_id, client_id, start_at DESC)
    WHERE client_id IS NOT NULL;
CREATE INDEX appointment_booked_by_idx ON appointment (business_id, booked_by_user_id, start_at DESC)
    WHERE booked_by_user_id IS NOT NULL;
