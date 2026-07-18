# PLENUM — Master Plan
Product-level plan. All phase units reference this file and the spec
(docs/plenum-crm-01.md, v01). Newest [LATEST] block wins; history below.

## [LATEST] 2026-07-17 — P0 Foundation: built, audited, accepted
- P0 delivered in 64e4c13 (52 files, +8041): full schema + RLS + audit
  triggers, deterministic seed (ORDERS TOTAL 17353, checksum-stable),
  session auth, RLS middleware, GET /api/accounts. No UI (by design).
- Verification: CC Tier-3 evidence pasted (RLS matrix incl. fail-closed
  hostile cases; curl transcripts; clippy/tests/sqlx clean); Cowork audit
  PASS at evidence tier; D.'s 7-check acceptance run PASS.
- Enforcement mechanism (do not relitigate): non-owner plenum_app
  connection, NOBYPASSRLS, ENABLE+FORCE RLS on the six §4 tables,
  fail-closed NULLIF(current_setting('app.user_id', true)) predicate.
  Zero app-side scoping. Admin connection = seed/migrations only.
- Recorded waiver: spec §14 devil's-case gate waived by D. 2026-07-17.
- Port: API on 8080 — architect's lock; the spec names no port. Serial
  coexistence rule applies if 8080's other tenant is verified real.

## Phase ladder (gate = D.'s pass on prior phase's §11 checks, always)
- P0 Foundation — DONE (this entry).
- P1 Metrics core — NEXT: v_order_facts, materialized rollups + refresh
  endpoint, all 7 metric groups, dual-basis everywhere, pagination.
  Gates P1-1/P1-2 (spec §11).
- P2 Command + Leaderboards UI — login, shell, Territory Board, period/
  basis/kind controls, CSV export, responsive tripwire (Playwright, five
  widths) lands here.
- P3 CRM operational core — Account 360 + installed-base timeline,
  pipeline kanban, quote builder + approval state machine + audit UI.
- P4 Signals + AI — deterministic generators + queue with write-back;
  Ask PLENUM + discount recommender behind flags; telemetry stub stretch.
- P5 Polish + demo hardening — leakage screen, data-quality panel,
  states pass, README demo script.

## Standing constraints (spec §4/§7 shortlist)
BIGINT cents · timestamptz · RLS in Postgres never app code · typed
errors, empty ≠ error · pagination max 200 · no secrets client-side ·
sqlx compile-checked (.sqlx committed) · clippy -D warnings · phases
report by walking acceptance checks, never internal tests alone.

## History
- 2026-07-17 — P0 built (64e4c13); docs/coexistence rule (b93e3d3);
  master-plan created at closeout.
