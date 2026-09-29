-- Authentication: password credentials, token hashing and the lookups that must
-- run BEFORE the caller is known (login by email, refresh by token).
--
-- Why SECURITY DEFINER: identity tables are protected by "own rows" RLS, but at
-- login time nobody is "me" yet. The two functions below are the only doors
-- through that wall. They are owned by the migration role, take one narrow input
-- each and return only what the login/refresh step needs.
-- Requirements: docs/PRODUCT_SPEC.md §9 (accounts), §17 (roles), §18 (GDPR).

-- ---------------------------------------------------------------------------
-- Password credentials (one row per user that signs in with email + password)
-- ---------------------------------------------------------------------------
CREATE TABLE user_password (
    user_id       uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    password_hash text NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER user_password_set_updated_at BEFORE UPDATE ON user_password
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

GRANT SELECT, INSERT, UPDATE ON user_password TO beauty_app;

ALTER TABLE user_password ENABLE ROW LEVEL SECURITY;
CREATE POLICY user_password_own ON user_password FOR ALL TO beauty_app
    USING (user_id = app_user_id())
    WITH CHECK (user_id = app_user_id());

-- ---------------------------------------------------------------------------
-- Refresh tokens are stored only as a SHA-256 hash (hex).
-- ---------------------------------------------------------------------------
CREATE FUNCTION hash_token(p_token text) RETURNS text
    LANGUAGE sql IMMUTABLE STRICT
AS $$ SELECT encode(sha256(convert_to(p_token, 'UTF8')), 'hex') $$;

-- ---------------------------------------------------------------------------
-- Pre-login lookups (SECURITY DEFINER)
-- ---------------------------------------------------------------------------
CREATE FUNCTION auth_login_lookup(p_email text)
    RETURNS TABLE (user_id uuid, password_hash text, status text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = public, pg_temp
AS $$
    SELECT u.id, p.password_hash, u.status
    FROM users u
    JOIN user_password p ON p.user_id = u.id
    WHERE u.email = lower(p_email)
      AND u.deleted_at IS NULL
$$;

CREATE FUNCTION auth_refresh_lookup(p_token text)
    RETURNS TABLE (
        token_id    uuid,
        user_id     uuid,
        device_id   uuid,
        family_id   uuid,
        expires_at  timestamptz,
        revoked_at  timestamptz,
        user_status text
    )
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = public, pg_temp
AS $$
    SELECT t.id, t.user_id, t.device_id, t.family_id, t.expires_at, t.revoked_at, u.status
    FROM refresh_token t
    JOIN users u ON u.id = t.user_id
    WHERE t.token_hash = hash_token(p_token)
      AND u.deleted_at IS NULL
$$;

REVOKE ALL ON FUNCTION auth_login_lookup(text)   FROM PUBLIC;
REVOKE ALL ON FUNCTION auth_refresh_lookup(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION auth_login_lookup(text)   TO beauty_app;
GRANT EXECUTE ON FUNCTION auth_refresh_lookup(text) TO beauty_app;
