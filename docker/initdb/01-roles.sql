-- Local development roles. Runs once, when the docker volume is first created.
--   beauty      (POSTGRES_USER) superuser: runs migrations and the RLS tests
--   beauty_api  limited login used by the running app, member of beauty_app,
--               so row-level security applies to it exactly as in production
DO $$
BEGIN
    CREATE ROLE beauty_app NOLOGIN NOBYPASSRLS;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

CREATE ROLE beauty_api LOGIN PASSWORD 'beauty_api' IN ROLE beauty_app;
GRANT CONNECT ON DATABASE beauty TO beauty_api;
