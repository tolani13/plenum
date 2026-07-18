# PLENUM — repo working memory
Source of truth: docs/plenum-crm-01.md (spec v01). Do not re-ask what it answers.
Phase state: P0 built and ACCEPTED by D. 2026-07-17 (evidence-based pass; hands-on
checks waived). p0-foundation NOT yet merged to main — merge runs only on D.'s
literal "merge". P1+ NOT started. Phase gate: D.'s explicit pass on the prior
phase's acceptance checks — never start a phase without it.
Non-negotiables: RLS in Postgres (API connects ONLY as plenum_app; admin conn is
seed/migrations only) · money = BIGINT cents · typed errors 401/403/404/422, empty
result ≠ error · pagination max 200 · no secrets in repo or client · sqlx
compile-checked (.sqlx committed) · clippy -D warnings clean.
Seed: deterministic (seed 20260717); rerun = truncate + regenerate; login password
for all demo users: demo-plenum-2026. Story beats (Ridgeline Grain silence, 28%
pending quote, MT-1 conquest prospect, leakage rep, data mess) are seeded ON
PURPOSE — they are demo script material, not bugs to fix.
Discipline: no-proof-no-run — report phases by walking acceptance checks, stating
what D. will observe; never report done on internal tests alone.
History: §14 devil's-case gate waived by D. 2026-07-17 (execute order = lock/go).
Machine note: DB host port is 5434 (native PostgreSQL owns 5432/5433 on D.'s
machine — D.'s call 2026-07-17); container-internal port stays 5432, so every
`docker compose exec db psql` command is unaffected. API port 8080 — shared
serially with the Local-Secure-Ops bank demo (Codex's active project: HANDS
OFF, never stop its process; other agents e.g. Grok Build also work this
machine). "cannot bind 0.0.0.0:8080" = the bank demo is up, not a bug.
