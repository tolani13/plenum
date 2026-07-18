-- DEV-ONLY bootstrap. Runs once, on first `docker compose up` against a fresh
-- volume. Creates the application role the API connects as.
--
-- plenum_app is the ONLY role the API ever uses. It is deliberately powerless:
-- no superuser, no DDL, no role creation, and critically NO BYPASSRLS — every
-- row it sees passes through the Row-Level Security policies. The plenum_admin
-- superuser (created by the container itself) is reserved for migrations and
-- the seed binary.
--
-- The password is a dev-only value, paired with docker-compose.yml.
CREATE ROLE plenum_app
    LOGIN
    NOSUPERUSER
    NOCREATEDB
    NOCREATEROLE
    NOBYPASSRLS
    PASSWORD 'plenum_dev_app_pw';
