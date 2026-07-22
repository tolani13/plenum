# PLENUM — Production Conversion Map

This demo runs on synthetic data by design. The architecture underneath it is
production-shaped on purpose. This page is the honest map from demo to
deployment.

## Already production-grade (no rework)

- Security boundary: every row a user sees is filtered by Postgres
  Row-Level Security under a non-privileged database role — not by
  application code. Scope rules live in one view.
- Governance: discount thresholds, signal thresholds, and territory
  geography are configuration rows, not code. The audit trail is append-only
  at the database-permission level.
- Signal engine: deterministic, idempotent, re-runnable — safe to schedule.
  Signals derive from data alone; no demo scripting.
- AI posture: one vendor seam, feature-flagged, key server-side only; every
  AI answer carries its receipts (the SQL, the comparables, the math). A
  missing key degrades gracefully — no screen errors.
- Money: integer cents everywhere; gross and net computed from line-level
  facts; every rollup reconciles to the ledger.

## What swaps to go live

1. Data in: the seed generator hands off at a marked importer seam
   (crates/seed/src/main.rs). Replace it with an ERP-extract loader (CSV or
   API) for accounts, sites, installed units, products, and order history.
   Everything downstream — metrics, signals, AI — runs unchanged on real
   rows.
2. Telemetry in: the filter-life ingest endpoint
   (crates/api/src/routes/telemetry.rs) is the template for the live sensor
   feed; add gateway authentication in front of it.
3. Sessions: in-memory session store swaps for a durable one (same session
   framework, different backing store).
4. Identity: SSO (OIDC) slots in front of the existing session auth; roles
   map to the same four the system already enforces.
5. Operations: TLS termination, scheduled rollup refresh + signal
   generation, backups, log shipping, secret manager.

## Realistic sequence

Phase 1 — data-mapping workshop + extract loader against a copy of the
customer's ERP data. Phase 2 — parallel-run: PLENUM's numbers reconciled
against existing reports until trusted. Phase 3 — pilot territory live
(signals + quotes). Phase 4 — rollout + AI features enabled per policy. The
demo you just watched is Phase 0, and nothing in it is throwaway.
