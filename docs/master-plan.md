# PLENUM — Master Plan
Product-level plan. All phase units reference this file and the spec
(docs/plenum-crm-01.md, v01). Newest [LATEST] block wins; history below.

## [LATEST] 2026-07-18 — P1 Metrics core: built, pending D.'s acceptance
- Derived layer: v_order_facts + v_unit_facts (security_invoker — definer
  views over RLS tables would bypass RLS; plenum_admin is superuser), four
  mv_* rollups keyed (entity, territory_id, quarter_start), scoped read
  views = 0005 predicate verbatim + live-quarter UNION (boundary pair: mv
  rows < current quarter, live rows >= current quarter — refresh staleness
  can never double-count), refresh_rollups() SECURITY DEFINER gated
  role=admin. plenum_app has NO grant on raw mv_*.
- Surface: /api/metrics/{territories,leaderboard,items,customers,leakage,
  coverage,defection} + POST /api/admin/refresh-rollups. Dual-basis in
  every payload; basis picks ranking only. period=YYYY|YYYY-Qn|cumulative|
  ttm; kind=capital|consumable|all; limit max 200; 422 on garbage;
  empty=200.
- Architect resolution: metric 7 = GET /metrics/defection (§10 "all 7
  groups" governs over §7's six-name list); P4 signals will reuse the view.
- Verification: Tier 3 — adversarial matrix (rep cross-territory denial on
  every endpoint, no-GUC zero rows, mv grant denial as plenum_app) + P1-1/
  P1-2 + rollup-vs-live equivalence + determinism anchors unchanged.
- Amendment 2026-07-18 (D.'s order, pre-acceptance): API port 8080 → 5777,
  default bind 127.0.0.1 — the parked port-move decision executed; bank
  demo keeps 8080, no contention. (The P0 entry's "API on 8080" line below
  is dated history and stands as written.)
- Update at merge: [CC appends acceptance + merge line on D.'s "merge"]

## 2026-07-17 — P0 Foundation: built, audited, accepted
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
