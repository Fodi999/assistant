-- Identity, tenancy and audit foundation.
-- Requirements: docs/PRODUCT_SPEC.md §10 (models), §17 (roles), §18 (GDPR).
--
-- Tenant isolation model
--   * The application connects as a LOGIN role that is a member of `beauty_app`
--     (never as the table owner, never as a superuser).
--   * Every request runs in a transaction that sets two transaction-local
--     settings: app.user_id (who) and app.business_id (which business).
--   * Business tables carry business_id and have row-level security (RLS) that
--     compares it with app.business_id. No context => no rows (fail closed).
--   * Identity tables (users, auth_identity, device, refresh_token) are reached
--     by the auth module. Lookups that happen BEFORE a user is known (login,
--     OTP) will go through SECURITY DEFINER functions added with the auth stage.
--   * Role-based rules (who may change roles, etc.) live in the application;
--     RLS is the second line of defence against cross-tenant leaks.

-- ---------------------------------------------------------------------------
-- Application role
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    CREATE ROLE beauty_app NOLOGIN NOBYPASSRLS;
EXCEPTION
    WHEN duplicate_object OR unique_violation THEN NULL;
END
$$;

GRANT USAGE ON SCHEMA public TO beauty_app;

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------
CREATE FUNCTION app_user_id() RETURNS uuid
    LANGUAGE sql STABLE
AS $$ SELECT NULLIF(current_setting('app.user_id', true), '')::uuid $$;

CREATE FUNCTION app_business_id() RETURNS uuid
    LANGUAGE sql STABLE
AS $$ SELECT NULLIF(current_setting('app.business_id', true), '')::uuid $$;

CREATE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END
$$;

-- ---------------------------------------------------------------------------
-- Users and authentication (identity tables)
-- ---------------------------------------------------------------------------
CREATE TABLE users (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email        text,
    phone_e164   text,
    display_name text,
    locale       text NOT NULL DEFAULT 'pl' CHECK (locale IN ('pl', 'en', 'ru', 'uk')),
    avatar_url   text,
    status       text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled', 'deleted')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    deleted_at   timestamptz,
    CONSTRAINT users_email_lowercase CHECK (email IS NULL OR email = lower(email)),
    CONSTRAINT users_phone_e164 CHECK (phone_e164 IS NULL OR phone_e164 ~ '^\+[1-9][0-9]{6,14}$')
);
CREATE UNIQUE INDEX users_email_key ON users (email) WHERE email IS NOT NULL AND deleted_at IS NULL;
CREATE UNIQUE INDEX users_phone_key ON users (phone_e164) WHERE phone_e164 IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE auth_identity (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider         text NOT NULL CHECK (provider IN ('apple', 'google', 'email', 'phone')),
    provider_subject text NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_subject)
);
CREATE INDEX auth_identity_user_idx ON auth_identity (user_id);

CREATE TABLE device (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    platform     text NOT NULL CHECK (platform IN ('ios', 'android', 'web')),
    push_token   text,
    app_version  text,
    locale       text,
    timezone     text,
    last_seen_at timestamptz,
    revoked_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX device_user_idx ON device (user_id);

-- One row per issued refresh token. Tokens rotate on every use; all tokens of a
-- login share a family_id, so reuse of an already-rotated token revokes the
-- whole family (theft detection). Only the SHA-256 hash of the token is stored.
CREATE TABLE refresh_token (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    device_id   uuid REFERENCES device (id) ON DELETE CASCADE,
    family_id   uuid NOT NULL,
    token_hash  text NOT NULL UNIQUE,
    expires_at  timestamptz NOT NULL,
    revoked_at  timestamptz,
    replaced_by uuid REFERENCES refresh_token (id),
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX refresh_token_user_idx ON refresh_token (user_id);
CREATE INDEX refresh_token_family_idx ON refresh_token (family_id);

-- ---------------------------------------------------------------------------
-- Business (the tenant), memberships, staff
-- ---------------------------------------------------------------------------
CREATE TABLE business (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name           text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 255),
    slug           text NOT NULL,
    description    text,
    country        char(2) NOT NULL DEFAULT 'PL',
    timezone       text NOT NULL DEFAULT 'Europe/Warsaw',
    currency       char(3) NOT NULL DEFAULT 'PLN',
    default_locale text NOT NULL DEFAULT 'pl' CHECK (default_locale IN ('pl', 'en', 'ru', 'uk')),
    status         text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    deleted_at     timestamptz,
    CONSTRAINT business_slug_format CHECK (
        slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$' AND length(slug) BETWEEN 3 AND 63
    )
);
CREATE UNIQUE INDEX business_slug_key ON business (slug) WHERE deleted_at IS NULL;

-- A user's role inside a business. Customers are not members: a customer is a
-- `client` of a business (added with the CRM stage).
CREATE TABLE membership (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    user_id     uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role        text NOT NULL CHECK (role IN ('owner', 'manager', 'reception', 'employee')),
    permissions jsonb NOT NULL DEFAULT '{}'::jsonb,
    status      text NOT NULL DEFAULT 'active' CHECK (status IN ('invited', 'active', 'suspended', 'removed')),
    invited_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (business_id, user_id)
);
CREATE INDEX membership_user_idx ON membership (user_id);

-- A bookable person shown to clients. Usually linked to a membership; may exist
-- without one (a master who does not use the app).
CREATE TABLE staff_member (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id   uuid NOT NULL REFERENCES business (id) ON DELETE CASCADE,
    membership_id uuid REFERENCES membership (id) ON DELETE SET NULL,
    display_name  text NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 120),
    photo_url     text,
    bio           text,
    color         text CHECK (color IS NULL OR color ~ '^#[0-9A-Fa-f]{6}$'),
    is_bookable   boolean NOT NULL DEFAULT true,
    sort_order    integer NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX staff_member_business_idx ON staff_member (business_id, sort_order);

-- ---------------------------------------------------------------------------
-- Audit log and consents (append-only)
-- ---------------------------------------------------------------------------
CREATE TABLE audit_log (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id   uuid REFERENCES business (id),
    actor_user_id uuid,
    action        text NOT NULL,
    entity        text NOT NULL,
    entity_id     uuid,
    ip_hash       text,
    meta          jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_log_business_idx ON audit_log (business_id, created_at DESC);

-- Proof of consent. Every grant or withdrawal is a new row (never updated).
-- Kept when a user is anonymised, hence RESTRICT rather than CASCADE.
CREATE TABLE consent (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    business_id  uuid REFERENCES business (id),
    type         text NOT NULL CHECK (type IN (
        'terms', 'privacy', 'dpa', 'marketing_sms', 'marketing_email',
        'marketing_push', 'health_data', 'photo_use'
    )),
    granted      boolean NOT NULL,
    text_version text NOT NULL,
    source       text NOT NULL DEFAULT 'app',
    ip_hash      text,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX consent_user_idx ON consent (user_id, type, created_at DESC);

-- ---------------------------------------------------------------------------
-- updated_at triggers
-- ---------------------------------------------------------------------------
CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER device_set_updated_at BEFORE UPDATE ON device
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER business_set_updated_at BEFORE UPDATE ON business
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER membership_set_updated_at BEFORE UPDATE ON membership
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER staff_member_set_updated_at BEFORE UPDATE ON staff_member
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Privileges: least privilege for the application role
-- ---------------------------------------------------------------------------
GRANT SELECT, INSERT, UPDATE         ON users, business                        TO beauty_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_identity, device, refresh_token,
                                        membership, staff_member               TO beauty_app;
GRANT SELECT, INSERT                 ON audit_log, consent                     TO beauty_app;

-- ---------------------------------------------------------------------------
-- Row-level security: tenant tables (FORCE: even the owner is subject to it)
-- ---------------------------------------------------------------------------
ALTER TABLE business     ENABLE ROW LEVEL SECURITY;
ALTER TABLE business     FORCE  ROW LEVEL SECURITY;
ALTER TABLE membership   ENABLE ROW LEVEL SECURITY;
ALTER TABLE membership   FORCE  ROW LEVEL SECURITY;
ALTER TABLE staff_member ENABLE ROW LEVEL SECURITY;
ALTER TABLE staff_member FORCE  ROW LEVEL SECURITY;
ALTER TABLE audit_log    ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log    FORCE  ROW LEVEL SECURITY;

-- business: visible inside its own context, or to any active member (so a user
-- can list "my businesses" right after login, before choosing one). A new
-- business is created after the app sets app.business_id to the new id.
CREATE POLICY business_select ON business FOR SELECT TO beauty_app
    USING (
        id = app_business_id()
        OR EXISTS (
            SELECT 1 FROM membership m
            WHERE m.business_id = business.id
              AND m.user_id = app_user_id()
              AND m.status = 'active'
        )
    );
CREATE POLICY business_insert ON business FOR INSERT TO beauty_app
    WITH CHECK (id = app_business_id());
CREATE POLICY business_update ON business FOR UPDATE TO beauty_app
    USING (id = app_business_id())
    WITH CHECK (id = app_business_id());

-- membership: tenant policy, plus every user may read their own memberships.
CREATE POLICY membership_tenant ON membership FOR ALL TO beauty_app
    USING (business_id = app_business_id())
    WITH CHECK (business_id = app_business_id());
CREATE POLICY membership_self_read ON membership FOR SELECT TO beauty_app
    USING (user_id = app_user_id());

CREATE POLICY staff_member_tenant ON staff_member FOR ALL TO beauty_app
    USING (business_id = app_business_id())
    WITH CHECK (business_id = app_business_id());

CREATE POLICY audit_log_select ON audit_log FOR SELECT TO beauty_app
    USING (business_id = app_business_id());
CREATE POLICY audit_log_insert ON audit_log FOR INSERT TO beauty_app
    WITH CHECK (
        business_id = app_business_id()
        OR (business_id IS NULL AND actor_user_id = app_user_id())
    );

-- ---------------------------------------------------------------------------
-- Row-level security: identity tables (own rows only)
-- ---------------------------------------------------------------------------
ALTER TABLE users         ENABLE ROW LEVEL SECURITY;
ALTER TABLE auth_identity ENABLE ROW LEVEL SECURITY;
ALTER TABLE device        ENABLE ROW LEVEL SECURITY;
ALTER TABLE refresh_token ENABLE ROW LEVEL SECURITY;
ALTER TABLE consent       ENABLE ROW LEVEL SECURITY;

-- users: yourself, and members of the business you are currently working in.
CREATE POLICY users_select ON users FOR SELECT TO beauty_app
    USING (
        id = app_user_id()
        OR EXISTS (
            SELECT 1 FROM membership m
            WHERE m.user_id = users.id
              AND m.business_id = app_business_id()
        )
    );
CREATE POLICY users_insert ON users FOR INSERT TO beauty_app
    WITH CHECK (id = app_user_id());
CREATE POLICY users_update ON users FOR UPDATE TO beauty_app
    USING (id = app_user_id())
    WITH CHECK (id = app_user_id());

CREATE POLICY auth_identity_own ON auth_identity FOR ALL TO beauty_app
    USING (user_id = app_user_id())
    WITH CHECK (user_id = app_user_id());
CREATE POLICY device_own ON device FOR ALL TO beauty_app
    USING (user_id = app_user_id())
    WITH CHECK (user_id = app_user_id());
CREATE POLICY refresh_token_own ON refresh_token FOR ALL TO beauty_app
    USING (user_id = app_user_id())
    WITH CHECK (user_id = app_user_id());
CREATE POLICY consent_own ON consent FOR ALL TO beauty_app
    USING (user_id = app_user_id())
    WITH CHECK (user_id = app_user_id());
