# PLENUM — Master Plan
Product-level plan. All phase units reference this file and the spec
(docs/plenum-crm-01.md, v01). Newest [LATEST] block wins; history below.

## [LATEST] 2026-07-19 — P3 CRM operational core: built, awaiting acceptance
- Shipped surface (branch p3-crm-core): the whole operational loop, all
  territory-scoped by Postgres RLS, all surviving an API restart. API adds
  GET /accounts/:id (360 payload: header + cumulative gross/net/leakage +
  sites + contacts + installed-unit timeline + recent orders + opps +
  activities + signals:[]), POST /accounts; GET/POST /opportunities + PATCH
  /opportunities/:id/stage; GET/POST /quotes + submit/approve/reject + GET
  /quotes/:id + GET /quotes/:id/audit; GET /policy/discount; GET /products;
  GET/POST /activities. Web adds Pipeline (kanban, native DnD + Move-to
  fallback, Won-books confirm + toast), Quotes (My/Approvals, builder with
  live verdict, detail with role-gated actions + audit trail), Account 360
  (installed-base timeline hero), the activity log, rail nav (Pipeline +
  Quotes) and customer-row → 360 links.
- Architect rulings: R1 — dragging to Won BOOKS a real order (copies the
  most-recent approved quote's lines verbatim, rep_id = opp owner, site =
  MIN(id); quote → accepted; won/lost terminal; won needs an approved quote,
  lost needs a reason). R2 — a deterministic opportunity book (14 + the
  Ridgeline win-back beat) on an ISOLATED RNG stream so every frozen anchor
  is byte-identical. R3 — discount thresholds become seed-config
  (discount_policy, 10/25); submit computes the worst-line verdict
  (self-approve ≤10 / manager 10–25 / VP >25) and the approve/reject HANDLER
  enforces the role tier. R4 — audit_log is app-immutable (REVOKE
  UPDATE/DELETE from plenum_app) and read ONLY through /api/quotes/:id/audit
  (scoped by the quote's RLS visibility; no generic audit feed).
- Migration 0011 is additive only: quotes gains
  discount_policy_result/submitted_at/decided_at/decision_reason;
  discount_policy created + seeded; the audit REVOKE; grants.
- Verification: Tier 3 (money + scope + authz write surface). check.sh ALL
  CHECKS PASSED; 34 tests (12 domain unit + 13 prior HTTP untouched + 9 new
  crm_http adversarial); tripwire 45/45 layout + command-scope + pipeline-
  scope PASS; P3-1 + P3-2 round-trips proven over HTTP (VP audit actor;
  booking delta == quote net; refresh invariance).
- New anchors: opportunities 16 (lead 3 / qualified 5 / quoted 4 /
  negotiation 4), opp checksum 3367519569, quotes still 1. Frozen anchors
  unchanged (orders 17353/11556020473, order_lines 25497/-166812187229,
  mv 120/195/1699/614, customers CUM NET footer $24,670,890.87).

## 2026-07-19 — P2 Command + Leaderboards UI: built, accepted, merged
- web/ (React 19 + Vite 7 + Tailwind 4 + TanStack Query/Table + react-router
  7, TS strict): tokens-first §8 design system (graphite control room,
  nameplates, tabular numerals), login + shell, Command (Territory Board
  4×2 cartogram, 4 KPIs, gross/net FLIP re-rank, drill drawer), Leaderboards
  (reps/items/customers, period scrubber 2023–2026/Q/CUM/TTM, basis, kind,
  group, footer totals, client CSV). Serving = Vite dev proxy →
  127.0.0.1:5777; API and all Rust/SQL untouched (zero-diff proven).
- Architect resolutions: defection-risk KPI until P4 signals; basis-
  invariant figures stay put by metric definition (attainment always net);
  drawer-drill (no server territory filter); react-router added; relative
  leakage LED bands. Signal-count surfaces land P4.
- Gate amendment (architect ruling 2026-07-19): frozen seed yields no
  territory re-rank at 2026/cumulative and no rep-#1 flip; P2-1's re-rank
  observable → customers tab (P1-proven — Vantage Metalworks Coastal drops
  out of net top-10 at 2025, check 3b); board toggle = every-dollar flip
  (2026 territory margins near-uniform — honest reading, no synthetic
  motion); leakage beat = worst-leakage-at-#1 (Wes Turner).
- Port amendment (D.'s call 2026-07-19): web dev server on 127.0.0.1:5177
  (5173 held by another tenant); API unchanged on 5777.
- Responsive doctrine + Playwright tripwire (5 widths × 5 screens +
  rep-scope assertion) land here per spec — P2-2's automated half (25/25 +
  scope PASS).
- Verification: Tier 3 — tripwire scope assertion, cross-login cache purge,
  rep CSV scope, anchors on-screen ($24,670,890.87 cumulative net footer).
- Accepted + merged 2026-07-19: D.'s acceptance run (all checks PASS —
  1–6 + 3b under D.'s hands; 7–8 Cowork-driven in D.'s browser under D.'s
  observation, export files opened + verified; 9 D.'s terminal; 10 amended
  desktop window-resize form, real-tablet deferred to P5) + literal "merge";
  merge commit de0be08 (--no-ff). Repo remains local-only.

## 2026-07-18 — P1 Metrics core: built, accepted, merged
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
- Accepted + merged 2026-07-18: D.'s "merge" order (= approval per unit
  protocol) after D.'s acceptance run against 127.0.0.1:5777; merge commit
  2f610ba (--no-ff). Repo remains local-only.

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
- P1 Metrics core — DONE (this entry).
- P2 Command + Leaderboards UI — DONE.
- P3 CRM operational core — BUILT on p3-crm-core (pending D.'s acceptance):
  Account 360 + installed-base timeline, pipeline kanban with Won-books-order,
  quote builder + approval state machine + audit UI, activities.
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
- 2026-07-18 — P1 built on p1-metrics (626d920, port amendment 2b34203);
  merged 2f610ba.
- 2026-07-19 — P2 built on p2-command-ui (d0741e8; gate + 5177 port
  amendments); merged de0be08.
- 2026-07-19 — P3 built on p3-crm-core (migration 0011, opp book, CRM
  routes, Pipeline/Quotes/Account 360, crm_http suite); awaiting acceptance.
