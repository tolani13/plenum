-- P3 CRM operational core, schema layer (architect ruling R9 — ADDITIVE ONLY).
--
-- Nothing here is destructive: four nullable columns on quotes, one new
-- config table, one grant, and one REVOKE that TIGHTENS a P0 over-grant. No
-- column is dropped, no type rewritten, no data touched. sqlx migrate runs
-- each file in its own transaction.

-- ── quotes: the approval state machine's bookkeeping ────────────────────────
-- All four nullable so every existing row (the seeded Wes quote) stays valid;
-- the seed backfills discount_policy_result + submitted_at on that one row.
--   discount_policy_result — the R3 verdict receipts (worst-line discount, the
--                            two thresholds, the tier the verdict landed in).
--   submitted_at           — when the quote left draft.
--   decided_at             — when it was approved/rejected (or self-approved).
--   decision_reason        — reject reason (R3: reject requires a reason).
ALTER TABLE quotes
    ADD COLUMN discount_policy_result jsonb,
    ADD COLUMN submitted_at           timestamptz,
    ADD COLUMN decided_at             timestamptz,
    ADD COLUMN decision_reason        text;

-- ── discount_policy: the thresholds become seed-config (R3) ─────────────────
-- Exactly one row. Discount governance (self ≤10%, manager 10–25%, VP >25%)
-- is READ per request from here, never hardcoded. numeric(5,2) to match the
-- discount_pct columns it is compared against. A single-row guard keeps the
-- table honest (config, not a log).
CREATE TABLE discount_policy (
    id              boolean PRIMARY KEY DEFAULT true,
    self_max_pct    numeric(5, 2) NOT NULL CHECK (self_max_pct >= 0 AND self_max_pct <= 100),
    manager_max_pct numeric(5, 2) NOT NULL CHECK (manager_max_pct >= 0 AND manager_max_pct <= 100),
    CONSTRAINT discount_policy_singleton CHECK (id),
    CONSTRAINT discount_policy_ordered CHECK (self_max_pct <= manager_max_pct)
);

-- The one config row, seeded here so the table is never empty (the seed binary
-- does NOT truncate discount_policy — it is config, not generated world, and
-- survives a reseed). Defaults are spec §14's 10/25 thresholds.
INSERT INTO discount_policy (id, self_max_pct, manager_max_pct)
VALUES (true, 10.00, 25.00);

-- ── audit_log becomes app-immutable (R4) ────────────────────────────────────
-- 0007 blanket-granted plenum_app SELECT+INSERT+UPDATE+DELETE on ALL TABLES,
-- which handed the app role the power to TAMPER with the audit trail. The
-- triggers (audit_row_change) run as plenum_app and INSERT, and the UI SELECTs
-- through /api/quotes/:id/audit — so INSERT and SELECT stay. UPDATE and DELETE
-- are revoked: once written, an audit row is immutable to every path the API
-- can reach (only the plenum_admin migration/seed connection could touch it,
-- and it never does outside a truncate-and-reseed).
REVOKE UPDATE, DELETE ON audit_log FROM plenum_app;

-- ── grants for the new table ────────────────────────────────────────────────
-- 0007's grant predates this table and does not cover it (no ALTER DEFAULT
-- PRIVILEGES exists). SELECT only — the policy is seed/admin-owned config; the
-- app reads it (GET /api/policy/discount, submit verdict) and never writes it.
GRANT SELECT ON discount_policy TO plenum_app;
