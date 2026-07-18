# PLENUM — repo working memory
Source of truth: docs/plenum-crm-01.md (spec v01). Do not re-ask what it answers.
Phase state: P0 merged to main d4f512d 2026-07-17 (D. acceptance 7/7 PASS).
P1 Metrics core merged to main 2f610ba 2026-07-18 on D.'s "merge".
P2+ NOT started. Phase gate: D.'s explicit pass on the prior phase's
acceptance checks — never start a phase without it.
Non-negotiables: RLS in Postgres (API connects ONLY as plenum_app; admin conn is
seed/migrations only) · money = BIGINT cents · typed errors 401/403/404/422, empty
result ≠ error · pagination max 200 · no secrets in repo or client · sqlx
compile-checked (.sqlx committed) · clippy -D warnings clean.
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
