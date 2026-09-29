-- Adding an existing account to a business (until invitations exist).
-- RLS hides other people's user rows, so the lookup is a SECURITY DEFINER
-- function that reveals only the id and display name of an ACTIVE account.
-- Only owners/managers reach it (checked in the API).
CREATE FUNCTION find_active_user_by_email(p_email text)
    RETURNS TABLE (user_id uuid, display_name text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = public, pg_temp
AS $$
    SELECT u.id, u.display_name
    FROM users u
    WHERE u.email = lower(btrim(p_email))
      AND u.status = 'active'
      AND u.deleted_at IS NULL
$$;

REVOKE ALL ON FUNCTION find_active_user_by_email(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION find_active_user_by_email(text) TO beauty_app;
