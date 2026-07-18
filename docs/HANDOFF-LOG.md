# PLENUM — Handoff Log

One entry per build unit. Newest first.

---

## 2026-07-18 · P1 Metrics core

- **Unit:** P1 Metrics core (v_order_facts + v_unit_facts, four mv_* rollups
  + scoped read views + refresh_rollups(), 7 metric endpoint groups,
  dual-basis, pagination) — branch `p1-metrics` from main a67bb39.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect resolutions recorded:** metric 7 ships as GET /metrics/defection
  (spec §10 "all 7 groups" vs §7 six-route list); matview scoping = grant
  boundary (no plenum_app grant on raw mv_*) + scoped views carrying the 0005
  predicate verbatim; v_order_facts/v_unit_facts are security_invoker
  (plenum_admin is superuser — definer views would bypass RLS); refresh via
  SECURITY DEFINER refresh_rollups() gated by role=admin in the handler;
  cumulative/ttm read v_order_facts directly, quarters/years read rollups.
- **Shipped:**
  - migrations/0008_order_facts.sql — v_order_facts + v_unit_facts, both
    WITH (security_invoker = true); SELECT grants to plenum_app.
  - migrations/0009_rollups.sql — mv_territory_period / mv_rep_period /
    mv_product_period / mv_customer_period, keyed (entity, territory_id,
    quarter_start), WITH NO DATA, unique key indexes, deliberately NO
    plenum_app grants (the enforcement boundary).
  - migrations/0010_scoped_reads.sql — v_territory_period / v_rep_period /
    v_product_period / v_customer_period (definer views: 0005 v_user_scope
    predicate verbatim on BOTH branches + live-current-quarter UNION ALL,
    boundary pair on date_trunc('quarter', now())); v_defection_risk
    (security_invoker, P4 reuses it); refresh_rollups() SECURITY DEFINER
    (search_path pinned, EXECUTE revoked from PUBLIC, granted to
    plenum_app); SELECT grants on the scoped views.
  - crates/domain/src/period.rs — period/basis/kind grammar parser (pure
    logic, 5 unit tests); domain lib.rs/Cargo.toml wiring (chrono from
    workspace deps — no new dependency).
  - crates/api/src/routes/metrics.rs — all 7 metric endpoint groups; static
    sqlx queries only (bind-parameter CASE for basis/by, null-folded
    kind/date filters); rollup path for quarter/year + kind=all, live
    v_order_facts path for cumulative/ttm and kind-filtered slices.
  - crates/api/src/routes/admin.rs — POST /api/admin/refresh-rollups,
    role=admin gate before the definer call; 401/403 typed.
  - crates/api/src/routes/mod.rs — eight new route registrations.
  - crates/api/src/error.rs — Forbidden comment updated (P1 lands the first
    real 403; variant no longer dead code).
  - crates/api/tests/metrics_http.rs — 8 integration tests: rep scope on
    every endpoint, VP/rep cent-equality, gate P1-1, rollup-vs-live year
    sum, kind-slice zeroing, 401 everywhere, 14-case 422 matrix, refresh
    role gate + stability.
  - crates/seed/src/main.rs — ONLY seed change: post-load
    refresh_rollups() call + one console line per matview with row count.
  - README (P1 acceptance section), master-plan, CLAUDE.md, this log;
    .sqlx regenerated (30 new query files).
- **Checks status (internal, output pasted in the session report):**
  clippy -D warnings UNTRIMMED pasted (debt carried from P0 closeout,
  settled) · 22/22 tests (9 domain unit + 5 P0 HTTP + 8 P1 HTTP) · cargo
  sqlx prepare --check clean · seed determinism: two runs, ORDERS TOTAL
  17353 + checksums (orders 17353/11556020473, order_lines
  25497/-166812187229) + matview row counts (120/195/1699/614) identical ·
  adversarial matrix: rep GUC = SE-1-only on all 7 P1 views, no-GUC = 0
  rows everywhere, garbage GUC = 0 rows, mv_* SELECT as plenum_app =
  permission denied ×4, rep/VP SE-1 cent-equality · P1-1 PASS (ORDER
  DIFFERS True, ALL GROSS>=NET True; SAME TOP-10 SET False — stronger form,
  flagged for audit) · P1-2 PASS (2467089087 == 2467089087) ·
  rollup-vs-live equivalence: 0 mismatched rows on all four scoped views ·
  refresh: rep 403 / VP 403 / admin 200 + row counts, P1-2 unchanged after ·
  restart survival PASS (no re-seed).
- **Anchors for the record:** CUMULATIVE NET (all territories, VP view) =
  2467089087 cents; same number from raw order_lines as plenum_admin =
  2467089087 cents.
- **Owed settled:** untrimmed clippy output pasted in this unit's report
  (carried from P0 closeout).
- **Machine note (report, don't fix):** the bank demo binds 127.0.0.1:8080
  specifically, so PLENUM can bind 0.0.0.0:8080 at the same time and
  localhost:8080 traffic still reaches the BANK DEMO. "One API at a time"
  stands; D.'s acceptance run needs the bank demo stopped first. Internal
  verification ran on BIND_ADDR=127.0.0.1:18080 (env override only; no
  config change — the project stays on 8080).
- **Phase gate: P1 pending D.'s acceptance run** (7 checks, README §P1).
- **Commit:** `626d920` on `p1-metrics` (this log line added in the
  immediate follow-up commit).

## 2026-07-17 · P0 Foundation

- **Unit:** P0 Foundation (repo scaffold, schema + RLS + audit triggers,
  deterministic seed, session auth, RLS session middleware, GET /api/accounts)
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Gate record:** §14 devil's-case gate waived by D. 2026-07-17; execute
  order = lock/go. Recorded here; not re-litigated.
- **Machine adaptations (D.'s calls, 2026-07-17, in-session):**
  - DB host port **5434** → container 5432 (native PostgreSQL services own
    5432/5433 on D.'s machine). All in-container psql commands unaffected.
  - Port 8080 freed by stopping `stack-ledger-api.exe` (Local-Secure-Ops
    bank demo) — D. authorized in-session; PLENUM API keeps 8080.
    **Correction, same day:** that demo is Codex's ACTIVE project, not
    leftover cruft. Standing rule from D.: PLENUM sessions leave the bank
    demo (and Grok Build's work) alone — never stop/modify other agents'
    processes or folders. 8080 is shared serially: run one API at a time;
    a "cannot bind 0.0.0.0:8080" from PLENUM means the bank demo is up,
    which is contention, not a P0 failure.
- **Shipped:**
  - Cargo workspace: `crates/domain` (enums, bp-based money math + property
    test vs the SQL CHECK), `crates/api` (axum 0.8, tower-sessions 0.14
    MemoryStore, argon2id, RLS-transaction helper, typed errors, /api/auth/*,
    /api/accounts), `crates/seed` (deterministic engine, seed 20260717).
  - `migrations/0001–0007`: 9 enums, 16 tables (BIGINT-cents price triplet
    CHECK on quote_lines/order_lines), spec + FK indexes, `v_user_scope`
    recursive scope view, RLS ENABLE+FORCE with fail-closed policies on
    accounts/orders/opportunities/quotes/signals/activities, audit trigger
    on quotes/signals/opportunities, grants for `plenum_app`.
  - Docker compose (postgres:16) + initdb script creating `plenum_app`
    (LOGIN, NOSUPERUSER, NOBYPASSRLS). API connects only as `plenum_app`.
  - Seed world: 8 territories, 17 users (12 reps/3 RMs/VP/admin), 48
    accounts, 60 sites, 107 contacts, 42 products, 232 installed units,
    17,353 orders / 25,497 order lines, all five §9 story beats. Determinism
    proven: two runs, identical count + sum(hashtext(id)).
  - Docs: README (P0 run + acceptance), root + docs copy of spec v01,
    repo CLAUDE.md, this log. `scripts/check.sh` gauntlet; `.sqlx/` committed.
- **Checks status (internal, all output pasted in the session report):**
  clippy -D warnings clean · 8/8 tests pass (money property test + 5 HTTP
  integration tests) · seed gate 17,353 > 15,000 · determinism checksum
  identical across runs · DB-level RLS matrix (rep 6 / RM 22 / VP 48 / no-GUC
  0 / random-uuid 0 / admin negative control) · HTTP matrix (rep SE-1-only,
  VP 8 codes, 401 no-cookie, identical 401s for bad creds, 422 limit=500) ·
  restart survival.
- **Phase gate: P0 ACCEPTED by D., 2026-07-17** ("I'm good with it then"),
  on the basis of the pasted evidence report plus a live browser
  demonstration (no-login 401 → rep sees SE-1 only → VP sees all 8
  territories, identity proven via /api/auth/me at each step). D. waived
  hands-on execution of the 7 checks — recorded as an evidence-based pass,
  not a hands-on pass. Merge not yet ordered; `p0-foundation` unmerged.
- **Out-of-scope observations:** logged in the session report only; nothing
  fixed beyond P0 scope.
- **Commit:** `64e4c13` on `p0-foundation` (this log line added in the
  immediate follow-up commit).
- **2026-07-17 closeout:** D. acceptance 7/7 PASS. Cowork audit PASS
  (evidence tier). master-plan.md added. Next: P1 unit from fresh Cowork
  session (skill plenum-01).
- **Bank-demo verification:** VERIFIED REAL: bank demo exists on this
  machine; b93e3d3 record stands. (Fresh disk check at closeout:
  stack-ledger-api.exe, Start-Bank-Demo.ps1, Check-Bank-Demo.ps1, and
  bank-demo-startup.log all present; this session also observed the
  process running from that exe before D.'s authorized stop.)
- **Owed carry-forward:** CC owes untrimmed clippy output in next unit's
  report.
- **Merge record:** `d4f512d` — p0-foundation merged to main (--no-ff),
  2026-07-17 22:47:10 -0400, on D.'s "merge". Repo remains local-only
  (no remote configured).
