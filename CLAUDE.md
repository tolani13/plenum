# PLENUM — repo working memory
Source of truth: docs/plenum-crm-01.md (spec v01). Do not re-ask what it answers.
Phase state: P0 merged to main d4f512d 2026-07-17 (D. acceptance 7/7 PASS).
P1 Metrics core merged to main 2f610ba 2026-07-18 on D.'s "merge".
P2 Command+Leaderboards UI merged to main de0be08 2026-07-19 on D.'s "merge".
P3 CRM core merged to main c8936ec 2026-07-20 on D.'s "merge" (11/11 PASS).
P4 Signals+AI merged to main 56cdd9b 2026-07-21 on D.'s "merge"; the
12-check walk ran the same evening, 12/12 PASS (acceptance record 8bfe7c7).
P5 (FINAL) merged to main 924da62 2026-07-22 on D.'s "merge" (per the
unit's pre-authorized PHASE 2; the 14-check observation walk stays OWED to
D.'s own hands — run it before the demo rehearsal). The P0→P5 ladder is
COMPLETE; any further work is a new unit from a fresh Cowork session, not
a phase. Phase gate discipline stands for any such unit: D.'s explicit
pass, never assumed.
Non-negotiables: RLS in Postgres (API connects ONLY as plenum_app; admin conn is
seed/migrations only) · money = BIGINT cents · typed errors 401/403/404/422, empty
result ≠ error · pagination max 200 · no secrets in repo or client · sqlx
compile-checked (.sqlx committed) · clippy -D warnings clean.
P3 write surface: opportunities/quotes/activities/accounts POST + stage/approval
actions; Won books an order from the most recent approved quote (quote →
accepted); discount thresholds live in discount_policy (seed-config 10/25);
audit_log is app-immutable (REVOKE UPDATE/DELETE) and read only via
/api/quotes/:id/audit.
P4 signals live: signals are DERIVED, never seeded/hand-created —
generate_signals() (0012; invoker-rights plpgsql, EXECUTE plenum_app only)
runs post-seed and via POST /api/admin/generate-signals (role=admin).
Dedupe/no-clobber law: dedupe_key UNIQUE identity per R2; upserts touch only
open rows and only when score/reasons changed — reruns never duplicate,
never resurrect dismissed, zero audit rows when nothing changed. Thresholds
in signal_policy (seeded in 0012, NOT truncated by seed);
v_defection_risk reads defection_multiplier from it (byte-identical at the
1.50 default). Signal write surface: GET /api/signals (+/summary,
+/assignees — disclosed) and assign/action/dismiss (scope-checked assignee,
required reason/outcome, actioned/dismissed terminal). Command's 4th KPI =
OPEN SIGNALS (kpi-signals; tiles carry per-territory counts);
/metrics/defection still exists (DrillDrawer fetches it lazily).
P4 AI: env keys ANTHROPIC_API_KEY (env-only secret — never in repo, client,
or logs; only presence logged), ANTHROPIC_MODEL (default claude-sonnet-5),
AI_ASK_ENABLED, AI_DISCOUNT_ENABLED. One-seam rule: crates/api/src/ai/client.rs
is the ONLY module naming reqwest/the vendor; vendor failure = typed 503
ai_unavailable, never a 500 or an error screen. Ask: sqlparser AST
validation (single SELECT over the six whitelisted views only) + execution
in the caller's READ-ONLY rls tx (rls_readonly_tx) with 5s timeout + LIMIT
500 wrap; the validated SQL is always returned (receipts). Recommender:
flag gates endpoint, key gates narrative (degrades to comparables).
Telemetry stub: POST /api/telemetry/filter-life (role=admin, 422/404 typed)
feeds the reorder telemetry branch on next generation.
P5 surface: three new screens/routes — /map (Territory Map: committed CC0
US-states SVG at web/src/map/, rendered from the derived typed module;
geography config in territory_states, seeded IN migration 0013 along
Census lines, keyed by territory CODE — an id FK would be cascade-wiped by
the seed's TRUNCATE territories CASCADE; NOT in the truncate list), /leakage
(distribution + outlier feed + rep×family heat; heat/territory palettes are
tokens.css entries ONLY), /data-quality (read-only pure-SQL finders; the
seeded trio is complete only at VP view). New reads (disclosed, RLS-scoped):
GET /api/metrics/states (per-state money via sites.state ⋈ territory_states
+ the config-level TM/RM/state_codes roster) and GET /api/data-quality.
/metrics/leakage: σ now reads signal_policy.discount_sigma (byte-identical
at 2.00 — proven); `outliers=policy` (disclosed param) serves the
discount_anomaly generator's exact math so feed rows match signal chips
1:1; outlier rows carry order_line_id/account_id/territory_code/signal_id;
payload gains heat cells. Signal auto-expiry law (0013): generate_signals()
expires OPEN machine-keyed cards whose dedupe_key is not re-emitted
(status='expired' + expired_at; per-type expired count in the return);
assigned/actioned/dismissed NEVER machine-touched; an expired key re-emitted
REOPENS the same card (expired is machine state); write-backs on expired =
422; same-day double run stays 0/0/0 with zero audit delta. Perf (0013):
idx_orders_site_ordered ON orders(site_id, ordered_on DESC, id DESC) — the
measured fix (v_unit_facts' last_paid lateral + JIT threshold); the seed
runs a post-load ANALYZE (stats stale after truncate-reload). Queue lanes
render 25 cards + Show-more (client slice); status filter gains Expired
(active stays open ∪ assigned). Web bundle: Ask/Map/Leakage/DataQuality are
lazy routes (main chunk < 500 kB; ASK_FOCUS_EVENT lives in lib/events.ts —
never import from a lazy chunk into the Shell). lib/fetchAll.ts owns
FETCH_LIMIT/q; routes/common.rs owns parse_page for accounts+metrics too.
Scripts: scripts/run-all.ps1 (API + web, materializes a dev .env on fresh
clones — committed dev values only) · scripts/demo-reset.ps1 (reseed
one-liner; in-memory sessions mean re-login after). PRODUCTION.md exists at
the repo root (the demo→deployment map; keep truthful).
T1 Territory editing (planning view) merged: /map Edit mode (vp/admin
ONLY; rep/manager DOM carries zero edit affordances — tripwire-asserted).
Write surface: PUT /api/territory-states/:state_code · POST/PATCH/DELETE
/api/territories · GET /api/territories (vp/admin). All app-gated
vp|admin + audited via audit_row_change (0014 adds territory_states.id
uuid + triggers + write GRANTs — geography is no longer app-read-only;
config tables' defense = role gate + immutable audit, no RLS). Colors:
territories.color_token (nullable) resolved client-side through
territoryFill; planning palette = --terr-plan-* tokens.css entries ONLY.
PLANNING-VIEW LAW: map grouping/planning sums are site-attributed config;
Command/Leaderboards/Board figures and RLS scope are order-attributed and
UNTOUCHED by map edits (tested). Canada blocks locked in v1. Seed now
restores canonical Census geography (demo reset = canon; runtime
territories + their state rows wiped). Realignment/state-split path:
docs/territory-realignment-prep.md.
D-1/D-2 fix merged (branch fix-items-perf from 81eba71): /api/metrics/items
attach-rate is SET-BASED — `page`/`fits`/`served`/`att` CTEs reading
v_unit_facts ONCE and v_order_facts ONCE. The old LEFT JOIN LATERAL + per-row
EXISTS is GONE and must not come back: it drove the planner's cost estimate to
0.69M–12.05M, past jit_optimize_above_cost/jit_inline_above_cost (500 000), so
every request LLVM-compiled ~500 functions before executing — 83–97% of a
34–39 s live response — and three concurrent requests killed the free-tier
Postgres backends (the typed 500 behind D-2). Post-fix every estimate is
~20.1k–20.7k, below jit_above_cost (100 000): no plan carries a JIT section.
COST-ESTIMATE LAW for any new metrics query: if EXPLAIN puts it over 100 000
on the frozen seed, the shape is wrong — fix the shape, never the knob.
Scoping unchanged (same security_invoker facts views, same rls_tx); 53
response bodies byte-identical before/after. Client failure honesty: api.ts
exports NetworkError + describeError, and ErrorPanel names the server's typed
code/message — ONLY a transport failure may say the API was unreachable; its
no-argument default is cause-agnostic. Ten other ErrorPanel call sites still
pass no error (reported, not fixed — one line each).
Tripwire is now 75 layout (15 screens × 5 widths) + 7 scope
(command/pipeline/signals/leakage/map — map = no foreign-territory dollars
in a rep's DOM — plus rep AND manager zero-edit-affordance on /map) + 3
honest-error specs (web/honest-errors.spec.ts).
BLANK-SCREEN LAW: the app root and every lazily-loaded route sit inside an error boundary. A
failed chunk download or any uncaught render error must render a named panel with a retry
inside the surviving app shell — never an empty document. A new lazy route without a boundary
above it is an incomplete change.
D-3 implements it: web/src/components/ErrorBoundary.tsx (the only class component in the
app — getDerivedStateFromError is React's sole render-time mechanism) at THREE layers —
root (main.tsx, backstop above the router), screen (Shell's <Outlet/>, so the nav survives
every routed screen), and per-lazy-route (LazyRoute.tsx). LazyRoute owns the four route
loaders; adding a lazy route means adding a screenLoader + <LazyRoute>, never a bare
lazy()/<Suspense>. Retry law, measured not assumed: React.lazy memoizes its rejection AND
the browser's module map caches a failed module URL forever (a second import() of the same
specifier issues no request at all), so a retry re-imports under a one-time ?d3-retry=N URL
parsed from the error and same-origin-checked. When the failed screen's SHARED dep chunk is
poisoned too (whole-network outage), no in-document re-import can win — the panel escalates
to an explicit "Reload PLENUM" (second rung only, never automatic; a document reload keeps
the session and the URL — the MemoryStore is server-side). Test hooks, permanent:
plenum-test-render-error (screen boundary) and plenum-test-root-error (root boundary) —
both ship in the production bundle, mounted unconditionally (accepted: D. needs the first
for the live acceptance check).
ROUTE-IDENTITY LAW (D-4): LazyRoute keys its inner component on the pathname, applied
inside LazyRoute so a new lazy route cannot forget it, and its lazy() components come
from the module-level lazyFor cache — NEVER built during render. D-3 shipped one
component type serving four routes with lazy() inside a useMemo, and a suspending render
never commits, so every retry rebuilt the memo, made a new lazy(), re-imported and
suspended again: measured 8 572 renders / 4 286 loader calls in 4 s, still climbing at
22 506 / 11 253, while React Router's transition kept <Suspense> showing the PREVIOUS
screen. URL right, no error, wrong screen, hot loop. Never build a lazy() (or anything
whose identity matters) during a render that can suspend. Screens report themselves in
body[data-screen] via useScreenReady(ready, screen) — required arg; assert navigation on
that marker, never on the URL, which was correct throughout the D-4 defect.
Metrics layer: plenum_app has NO grant on raw mv_* rollups; all metric reads
go through security_invoker facts views or scoped views carrying the
v_user_scope fail-closed predicate; refresh only via refresh_rollups()
behind a role=admin handler gate. Definer views over RLS tables = leak.
Seed: deterministic (seed 20260717); rerun = truncate + regenerate; login password
for all demo users: demo-plenum-2026. Story beats (Ridgeline Grain silence, 28%
pending quote, MT-1 conquest prospect, leakage rep, data mess) are seeded ON
PURPOSE — they are demo script material, not bugs to fix.
Discipline: no-proof-no-run — report phases by walking acceptance checks, stating
what D. will observe; never report done on internal tests alone.
History: §14 devil's-case gate waived by D. 2026-07-17 (execute order = lock/go).
Deploy merged to main 2e67b23 2026-07-23 on D.'s "merge"; live service
tracks MAIN per the blueprint (README de-branded for the public repo the
same day, D.'s order).
Deploy (2026-07-22, branch deploy-render): repo now has origin =
github.com/tolani13/plenum (private) — the ONE sanctioned remote; pushes
only per deploy-unit protocol. Prod artifacts: Dockerfile (3-stage: SPA →
rust 1.95 release api+seed → bookworm-slim, non-root), docker/entrypoint.sh
(PORT→BIND_ADDR), render.yaml blueprint (free Postgres 16 plenum-db + free
Docker web service plenum, oregon, healthCheckPath /api/health, autoDeploy
off, AI flags false, NO key in prod). Env-gated code (dev byte-identical):
GET /api/health (unauth 200); MIGRATE_ON_BOOT=true → ensure plenum_app role
(NOLOGIN) + embedded migrations on boot + empty-world serves-with-warning;
WEB_DIST static tier (tower-http fs, the one permitted dep, MIT) — SPA deep
links 200 via ServeDir::fallback, unknown /api/* stays typed JSON 404.
Seed in prod = explicit local run over TLS (external URL + ?sslmode=require
+ the DB's IP allowlist — 74.124.184.78/32 on file), never part of a
deploy; `render jobs create` is paid-plan-only ("free tier plans are not
supported for jobs"). The seed pins the seeded ADMIN's RLS identity on
NON-superuser (managed) connections only — managed owners sit under the
FORCEd RLS; local superuser seeds are byte-identical, NULL audit actor
included. Live: https://plenum.onrender.com · svc srv-d9goii4vikkc739qverg
· db dpg-d9go6b3bc2fs738vcm00-a (free PG16 expires ~30 days unless
upgraded; blueprint + one seed restores). Render CLI v2.15.1 is the
authenticated surface (workspace tea-d5ufur7fte5s73eaj0e0); its key funds
REST calls for what the CLI lacks; free plans only.
Machine note: DB host port is 5434 (native PostgreSQL owns 5432/5433 on D.'s
machine — D.'s call 2026-07-17); container-internal port stays 5432, so every
`docker compose exec db psql` command is unaffected. API port 5777, bind
127.0.0.1 (D.'s call 2026-07-18, executing the parked port-move decision):
PLENUM owns 5777; the Local-Secure-Ops bank demo keeps 8080 — no contention.
The never-touch rule for other agents' processes and folders stands (the bank
demo is Codex's active project; Grok Build also works this machine).
Web (P2): web/ = Vite dev server 127.0.0.1:5177 (strictPort; 5173 was held by
another tenant — D.'s call 2026-07-19), proxy /api -> 127.0.0.1:5777; npm run
dev | dev:lan (iPad checks; API stays loopback) | build (tsc -b) | tripwire
(Playwright 5-width overflow gate + rep-scope). Frontend consumes payloads
verbatim; client-side scope widening = breach. Coverage takes basis only;
defection takes limit/offset only (422 otherwise by design). Signals table empty
until P4 (Command's 4th KPI stays defection-risk); activities are user-writable
from P3 (seeded activities remain absent by design). Gate amendment 2026-07-19: the frozen seed holds
territory order at 2026/cumulative (near-uniform margins), so the gross/net
re-rank observable lives on the customers tab (P1-1's proven surface); the
Command toggle proves the every-dollar flip. No seed/SQL/Rust change in P2.
