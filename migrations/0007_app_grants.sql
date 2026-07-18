-- Grants for the application role. Runs last, after all objects exist.
-- plenum_app gets row-level DML only: no TRUNCATE, no DDL, no ownership.
-- (GRANT ... ON ALL TABLES includes views, so v_user_scope is covered.)

GRANT USAGE ON SCHEMA public TO plenum_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO plenum_app;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO plenum_app;
