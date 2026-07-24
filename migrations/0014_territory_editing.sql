-- T1 Territory Map Editing (planning view), schema layer
-- (constraint 6 — ADDITIVE ONLY: column adds, trigger adds, grant adds.
-- No drops, no type rewrites, no data destroyed.)
--
-- ── DISCLOSED POSTURE CHANGE: geography stops being app-read-only ───────────
-- 0013 granted plenum_app SELECT only on territory_states ("the app reads it
-- and never writes it"). T1 introduces a VP/admin planning-edit surface, so
-- the app role gains row DML on BOTH territory config tables. These tables
-- carry no RLS (they are config, not ledger — the 0013/P4 doctrine: org
-- chart and geography are config; money is what scope guards). Their defense
-- stack, stated the way 0013 states its rationale:
--   · handler role gate — every write endpoint requires session role
--     vp|admin (the generate-signals precedent), 403 otherwise, 401 unauth;
--   · immutable audit trail — the 0006 audit_row_change trigger is attached
--     to both tables below; audit_log is app-immutable (0011 REVOKE
--     UPDATE/DELETE), so every geography/territory mutation is a permanent
--     record with the acting user's id (the app.user_id GUC);
--   · typed errors — unknown state 404, bad input 422, never a silent write.
-- (territories already carried app DML via 0007's blanket GRANT ON ALL
-- TABLES — it existed at 0007 time; the explicit grant below makes T1's
-- intended posture visible in one place either way. territory_states, born
-- in 0013 after the blanket, is the real change.)

-- ── territory_states gains a uuid id — for the audit trigger ONLY ───────────
-- audit_row_change() (0006) audits any table with a uuid `id` column
-- (entity_id uuid NOT NULL). territory_states is keyed by state_code — the
-- deliberate 0013 truncate-proof design — so the id exists SOLELY to give
-- the trigger an entity_id. state_code STAYS the PK and the join key; no
-- read path, no FK, no join touches this column.
ALTER TABLE territory_states
    ADD COLUMN id uuid NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE territory_states
    ADD CONSTRAINT territory_states_id_key UNIQUE (id);

-- ── audit both territory tables (the 0006 trigger, unchanged) ───────────────
CREATE TRIGGER territory_states_audit
AFTER INSERT OR UPDATE OR DELETE ON territory_states
FOR EACH ROW EXECUTE FUNCTION audit_row_change();

CREATE TRIGGER territories_audit
AFTER INSERT OR UPDATE OR DELETE ON territories
FOR EACH ROW EXECUTE FUNCTION audit_row_change();

-- NOTE the seed consequence, disclosed: the seed regenerates territories
-- (8 INSERTs — TRUNCATE itself fires no row triggers) and restores canonical
-- geography (66 DELETEs + 66 INSERTs, the T1 seed step), so the audit_log
-- count at the seed's printed summary rises from the historical 17 to 157.
-- Deterministic, actor NULL on superuser dev seeds (the P0 semantics).

-- ── territories.color_token — the planning palette hook ─────────────────────
-- Nullable text naming a --color-terr-plan-* token (tokens.css is the only
-- palette source; the API validates against the same 8-name list the client
-- renders). Canonical territories keep NULL → the frontend falls back to
-- today's territoryFill mapping, so the canonical map is byte-identical.
ALTER TABLE territories
    ADD COLUMN color_token text;

-- ── the write grants (the posture change itself) ────────────────────────────
GRANT INSERT, UPDATE, DELETE ON territory_states TO plenum_app;
GRANT INSERT, UPDATE, DELETE ON territories TO plenum_app;
