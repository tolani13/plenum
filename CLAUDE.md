# PLENUM — repo working memory
Source of truth: docs/plenum-crm-01.md (spec v01). Do not re-ask what it answers.
Phase state: P0 merged to main d4f512d 2026-07-17 (D. acceptance 7/7 PASS).
P1 Metrics core merged to main 2f610ba 2026-07-18 on D.'s "merge".
P2 Command+Leaderboards UI merged to main de0be08 2026-07-19 on D.'s "merge".
P3 CRM core merged to main c8936ec 2026-07-20 on D.'s "merge" (11/11 PASS).
P4 Signals+AI BUILT on branch p4-signals-ai 2026-07-20 (awaiting D.'s
acceptance; merge only on D.'s literal "merge"). P5 NOT started. Phase gate:
D.'s explicit pass on the prior phase's acceptance checks — never start a
phase without it.
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
Tripwire is now 55 layout (11 screens × 5 widths) + 3 scope
(command/pipeline/signals).
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
