# PLENUM — Handoff Log

One entry per build unit. Newest first.

---

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
