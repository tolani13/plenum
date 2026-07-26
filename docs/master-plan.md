# PLENUM — Master Plan
Product-level plan. All phase units reference this file and the spec
(docs/plenum-crm-01.md, v01). Newest [LATEST] block wins; history below.

## [LATEST] 2026-07-26 — B-1: the collector demo is in the repo, de-branded,
served by PLENUM at /collector as a tenth screen and a fifth lazy route
(three.js + drei ride their own 982 kB lazy chunk; the main chunk is
428.39 kB, still under the 500 kB law). B-2 adds the telemetry push into the
reorder branch — an admin-gated write, deliberately its own unit. Vendor and
product-line names are gone; the industry vocabulary stays.

## 2026-07-25 — D-1/D-2 fixed: the Items leaderboard paints in under
2 s and the client no longer lies about why a request failed. Live demo path
is clear end to end. Remaining audition-track items: collector (Artifact 1)
into the repo at /collector + the telemetry bridge, live demo rehearsal,
tag demo-live-20260725. Platform track (multi-tenancy et al.) is unapproved
and forks to a private repo at the multi-tenancy migration, not before.

## 2026-07-23 — T1 Territory Map Editing (planning view): built
- T1 territory editing (planning view) shipped: VP/admin redraw the map
  (paint + drag), define territories, audited, planning-only; realignment
  commit + state splits + Canada editing recorded as future units in
  territory-realignment-prep.md.
- First post-ladder unit (NOT a phase), on `t1-territory-editing` from
  main 5367a5f. /map gains an Edit mode for vp/admin only: click-to-paint
  (primary) + drag-to-row (secondary) state reassignment, territory
  create/rename/recolor/delete (delete refused with a reason unless
  completely empty), standing planning-view banner. Write surface
  PUT /api/territory-states/:code · POST/PATCH/DELETE/GET /api/territories,
  all vp|admin-gated and audited (0014: territory_states.id + audit
  triggers on both territory tables + write GRANTs — disclosed posture
  change from app-read-only geography). PLANNING-VIEW LAW tested: the
  Territory Board feed is byte-identical across a reassignment while
  /api/metrics/states regroups live from territory_states. Seed now
  restores canonical Census geography (66 rows single-sourced from the
  0013 text). Tripwire 75 layout + 7 scope. Anchors intact
  (17353/11556020473 · mv 120/195/1699/614 · ledger 2467089087).

## 2026-07-22 — P5 Polish + demo hardening + Territory Map: built
- FINAL phase, on `p5-polish-map` from main 8bfe7c7. Shipped surface: the
  Territory Map (/map — the Board projected on the continent: committed CC0
  US-states SVG rendered as typed React paths, territory_states geography
  config seeded in migration 0013 along Census division lines, per-state
  dollars on hover per the PRE-2 verdict, click-through territory panel
  with TM/RM staffing + board figures, global basis flip, rep view =
  dimmed foreign silhouettes with zero foreign dollars in the DOM);
  Leakage (/leakage — distribution bar, the outlier feed running the
  discount_anomaly generator's exact math via the disclosed
  `outliers=policy` mode so rows and signal chips agree 1:1, and the rep ×
  family heat table banded by NEW heat tokens with Wes Turner reading
  worst at VP); Data Quality (/data-quality — pure-SQL finders landing
  exactly on the seeded trio at VP view, "clean book" designed empty in a
  rep's scope); signal auto-expiry (0013: 'expired' status + expired_at;
  generate_signals() re-created with a per-type expiry step + expired
  count + reopen-on-return; humans never machine-touched; same-day double
  run stays 0/0/0 with zero audit delta); perf (ONE measured index —
  orders(site_id, ordered_on DESC, id DESC) — plus a seed post-load
  ANALYZE: enriched list 1,187→25 ms, generate_signals 2,876→72 ms); lane
  pagination (25 + show-more); bundle split (main 773.63→423.29 kB, no
  Vite warning); states pass + Command tall-viewport rhythm; + New
  account modal (server-validated, typed 422 inline); param/LIMIT dedups;
  scripts/run-all.ps1 (+ fresh-clone dev-.env materialization) +
  scripts/demo-reset.ps1; PRODUCTION.md verbatim; README rewritten around
  the 3-command quickstart + §13 script; /api/metrics/states +
  /api/data-quality (disclosed, RLS-scoped, adversarial-tested).
- Rulings digest: R1 σ-from-config byte-identical (SHA-proven) + policy
  outlier mode + disclosed payload additions · R2 finders, no extensions ·
  R3 CC0 asset + code-keyed territory_states (FK would cascade-wipe on
  reseed) + roster disclosure + drawer-link omitted per "if cheap" · R4
  expiry + disclosed reopen-on-return (check 8 rehearsable) · R5
  measured index (prompt candidates moot) · R6 lane slices · R7 lazy
  routes · R8 states inventory + height-gated rhythm · R9 owed trio ·
  R10 PRODUCTION.md · R11 README/scripts · R12 tripwire 70+5 · R13 the
  repairs below.
- Verification: Tier 3. check.sh ALL CHECKS PASSED (60 tests = 56 prior
  untouched + 4 p5_http: expiry matrix, states scope + roster + grammar,
  R1 equivalence + policy parity + Wes-worst pin, data-quality trio);
  PRE-1…6 pasted; two seed runs byte-identical INCLUDING post-0013;
  tripwire 70/70 + 5 scope PASS; npm build clean; perf before/after
  pasted; browser-driven internal walk (map panel, leakage chips, DQ
  trio, new-account 422).
- Anchors: frozen set unchanged (orders 17353/11556020473 · order_lines
  25497/−166812187229 · opportunities 16/3367519569 · mv 120/195/1699/614
  · audit 17 at seed · $24,670,890.87 · serena 293778300/278301715 · Wes
  28% quote 13158000/9698040). PRE-2: 100.00% alignment × 8 territories →
  per-state dollars shipped, seed untouched. Signal counts clock-drifting
  (build-day 39/12/28/172 = 251).
- New dependencies: NONE (zero npm, zero crates). New asset: Blank US Map
  (states only).svg — Heitordp, Wikimedia Commons, CC0 1.0 — committed
  with provenance header.
- Accepted + merged: D.'s literal "merge" reply 2026-07-22 (merge =
  approval per the unit's pre-authorized PHASE 2; no in-session per-check
  walk — the 14-check observation pass stays owed to D.'s own hands,
  recorded in the HANDOFF-LOG entry; run it before the demo rehearsal);
  merge commit 924da62 (--no-ff), staleness check passed. Repo remains
  local-only; branch p5-polish-map kept. FINAL phase — the P0→P5 ladder
  is complete.

## 2026-07-20 — P4 Signals + AI: built, merged 2026-07-21
- Shipped surface: signals become REAL — four deterministic generators
  (reorder_due incl. a telemetry branch, defection_risk from metric 7's view
  verbatim, conquest via the filter_fits cross-reference, discount_anomaly
  from per-family order-line statistics) derive the queue from table data
  alone via generate_signals(), an idempotent invoker-rights job (dedupe_key
  identity; reruns never duplicate, never touch worked cards, zero audit
  noise unchanged) run by the seed post-refresh and by POST
  /api/admin/generate-signals. Thresholds live in signal_policy (seeded
  config, survives reseed); v_defection_risk now reads its multiplier from
  it (byte-identical output at the 1.50 default). API: GET /api/signals
  (enriched, envelope, active-default) + assign/action/dismiss write-backs
  (scope-checked assignee, required reason/outcome, terminal states, RLS
  404s, audit via the P0 trigger) + summary + assignees (disclosed
  additions) + the R13 telemetry stub (admin-gated inbound-feed template).
  AI behind one seam (crates/api/src/ai/): Ask PLENUM — NL → SQL via the
  vendor, sqlparser AST validation against the six-view whitelist, executed
  in the caller's READ-ONLY RLS transaction (5s timeout, LIMIT 500 wrap),
  SQL receipts always returned; discount recommender — cohort comparables
  (family × industry × size band) under the caller's scope, narrative only
  with a key, degraded never erroring; flags + key env-only
  (ANTHROPIC_API_KEY never committed/logged/client-side); /api/ai/status
  gates the UI. Web: Signals queue (4 lanes, receipts on cards,
  draft-quote-from-signal composing P3 machinery, log-call, dismiss+reason),
  Command KPI 4 → OPEN SIGNALS + per-tile counts, Ask screen (table +
  recharts bar + SQL receipts + always-on saved-question library), builder
  COMPS panel + signal prefill, Account 360 signals fill, nav + Cmd-K.
- Rulings digest: R1 generators-read-the-world (fixture-proven on invented
  names in a rolled-back tx) · R2 idempotent dedupe-keyed job, no
  auto-expiry (P5) · R3 signal_policy config (window laddered at PRE-5:
  90d) · R4 exact math, 30.44 days/month · R5 write surface + two disclosed
  reads · R6 client-composed draft-from-signal · R7 Command rewire
  (kpi-signals; defection drawer fetch moved into DrillDrawer) · R8 one
  vendor seam, 503 ai_unavailable, 15s timeouts · R9 AST-or-nothing
  validation + read-only scoped execution + receipts · R10 recommender
  degradation contract · R11 identity single-seam held · R12 seed importer
  markers · R13 telemetry stub BUILT · R14 360 fill. Flagged: check-7's
  "no comps button" wording vs R8/R10 (flag gates the button, key gates
  the narrative) — shipped per rulings.
- Verification: Tier 3. check.sh ALL CHECKS PASSED (56 tests: 12 domain +
  7 validator + 22 prior HTTP + 15 new adversarial incl. the R1/R2 fixture
  world, generation idempotency + zero audit delta, no-resurrection,
  read-only refusal, LIMIT wrap, 5s timeout as typed 422, recommender
  scope/degradation, telemetry contract); two seed runs byte-identical;
  tripwire 55/55 layout + 3 scope PASS; browser-driven internal walk of
  gates P4-1 (Ridgeline card + prefilled draft + actioned write-back) and
  the P4-2 flag-off half.
- Anchors: frozen set unchanged; signal counts CLOCK-DRIFTING by design
  (build-day: 38/12/28/173 = 251; audit_log +251 after first generation).
- New dependencies (pre-authorized): reqwest 0.12.28 (MIT/Apache-2.0),
  sqlparser 0.62.0 (Apache-2.0). recharts finally in use.
- Accepted + merged 2026-07-21 on D.'s literal "merge" (per the unit's
  pre-authorized PHASE 2). The 12-check walk ran the same evening in the
  Cowork session, 12/12 PASS, recorded in the HANDOFF-LOG acceptance
  record (8bfe7c7) — superseding this block's earlier owed-walk wording.
  Merge commit 56cdd9b (--no-ff). Repo remains local-only; branch
  p4-signals-ai kept.

## 2026-07-20 — P3 CRM operational core: built, accepted, merged
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
  mv 120/195/1699/614, customers CUM NET footer $24,670,890.87). Corrected
  serena cumulative anchor (D., 2026-07-20): $2,937,783.00 gross /
  $2,783,017.15 net (the unit prompt's $12.9M/$10.8M line was a wrong
  reconstruction; never entered the repo).
- Accepted + merged: D.'s acceptance run 2026-07-20, all 11 checks PASS
  (1–6, 8, 9a Cowork-driven in D.'s browser under D.'s observation —
  browser-drive precedent amended by D. for P3 to include writes; 7, 9b,
  10, 11 in D.'s own terminal) + literal "merge"; merge commit c8936ec
  (--no-ff). Known benign: post-booking refresh transiently reports
  mv_product_period 1700 (current-quarter row is read-filtered by the
  boundary; reseed restores 1699). Repo remains local-only.

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
- P3 CRM operational core — DONE (this entry): Account 360 + installed-base
  timeline, pipeline kanban with Won-books-order, quote builder + approval
  state machine + audit UI, activities.
- P4 Signals + AI — DONE: generators + queue with write-backs, Command
  rewire, Ask PLENUM + recommender behind flags, telemetry stub. Merged
  56cdd9b on D.'s "merge" 2026-07-21; 12-check walk 12/12 PASS same
  evening (HANDOFF-LOG record, 8bfe7c7).
- P5 Polish + demo hardening + Territory Map — DONE (this entry): map +
  leakage screen + data-quality panel + signal auto-expiry + perf/bundle/
  states pass + quickstart scripts + PRODUCTION.md + tripwire 70+5.
  Merged 924da62 on D.'s "merge" 2026-07-22; the 14-check observation
  walk stays owed to D.'s hands. LADDER COMPLETE.

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
  routes, Pipeline/Quotes/Account 360, crm_http suite; tip 7ac1e08).
- 2026-07-20 — P3 accepted (D., 11/11 PASS) and merged c8936ec.
- 2026-07-20 — P4 built on p4-signals-ai (migration 0012 + generate_signals,
  signals surface, ai/ seam + validator, telemetry stub, queue/Ask/Command
  rewire, tripwire 55+3).
- 2026-07-21 — P4 merged 56cdd9b on D.'s "merge"; the 12-check walk ran the
  same evening, 12/12 PASS (acceptance record 8bfe7c7).
- 2026-07-22 — P5 built on p5-polish-map (migration 0013, Territory Map +
  Leakage + Data Quality, auto-expiry, perf/bundle, scripts + PRODUCTION.md
  + README, tripwire 70+5) and merged 924da62 on D.'s "merge" the same day
  (14-check observation walk owed). The P0→P5 ladder is complete.
