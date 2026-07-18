# PLENUM

CRM for the installed-base business — Camfil APC audition artifact.
Source of truth: [docs/plenum-crm-01.md](docs/plenum-crm-01.md) (spec v01).

**Phase state: P0 merged to main (D. acceptance 7/7 PASS). P1 (Metrics
core) built on `p1-metrics`, pending D.'s acceptance. P2+ not started.**
P0 = repo scaffold, Postgres schema + Row-Level Security + audit triggers,
deterministic seed engine, session auth, RLS session middleware, `GET
/api/accounts`. P1 = the derived analytics layer (`v_order_facts` +
`v_unit_facts`, four materialized rollups + scoped read views) and the seven
metric endpoint groups under `/api/metrics/*`, dual-basis (gross/net) in
every payload, plus `POST /api/admin/refresh-rollups` (admin-only). The
seed now refreshes the rollups after loading and prints one row-count line
per materialized view. No UI (that is P2).

---

## Prerequisites

- Docker Desktop (running)
- Rust ≥ 1.80 (`rust-toolchain.toml` pins 1.95.0)

## Ports on this machine

- **Database: host port 5434** → container 5432. This machine's native
  PostgreSQL services own 5432/5433, so compose maps 5434 (D.'s call,
  2026-07-17). Every `docker compose exec db psql …` command runs *inside*
  the container and is unaffected. Only the connection strings in `.env`
  carry 5434.
- **API: 127.0.0.1:5777** (D.'s call, 2026-07-18). PLENUM owns 5777; the
  Local-Secure-Ops bank demo (`stack-ledger-api.exe`, another agent's active
  project) keeps 8080 — no contention, the two run side by side. The
  never-touch rule for other agents' processes and folders still stands:
  PLENUM sessions never stop or modify the bank demo, ever. Loopback bind
  by default — the demo API is not exposed off-machine.

## Run it (three commands)

```
docker compose up -d
cargo run --bin seed
cargo run --bin api
```

- First `docker compose up` initializes the database and creates the
  `plenum_app` role (the only role the API ever uses — RLS applies to it).
- The seed applies migrations (it is the only thing that does), truncates,
  and regenerates the identical world every run (PRNG seed 20260717). It
  prints per-entity counts, `ORDERS TOTAL: 17353 (gate: >15000)`, and the
  login table. Every demo user's password: `demo-plenum-2026`.
- The API connects only as `plenum_app` and fails fast with a plain-language
  message if the database is down or unseeded.

Fresh clone note: `.env` is gitignored. Copy `.env.example` → `.env` (the
dev values are in `docker-compose.yml` / `docker/initdb/01-app-role.sql`).
Without a `.env`, the binaries fall back to the same dev defaults, except
the session cookie's `Secure` flag defaults **true** — set
`COOKIE_SECURE=false` for plain-HTTP localhost or curl will not send the
cookie back.

## P0 acceptance checks (PowerShell, paste-and-run)

Run in the repo folder. Full walkthrough with expected output lives in the
P0 handoff report; condensed here.

```powershell
# 1 — seed: expect "ORDERS TOTAL: 17353 (gate: >15000)" + a 17-login table
docker compose up -d
cargo run --bin seed

# 2 — ask the DB directly (expect the same 17353)
docker compose exec db psql -U plenum_admin -d plenum -c "SELECT count(*) FROM orders;"

# 3 — start the API (leave running; open a second terminal for 4–6)
cargo run --bin api

# 4 — RLS breach check FIRST, rep side (expect: SE-1 only, items 6, total 6)
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/auth/login -ContentType "application/json" -Body '{"email":"serena.estes@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable rep
(Invoke-RestMethod -Uri "http://localhost:5777/api/accounts?limit=200" -WebSession $rep).items.territory_code | Sort-Object -Unique

# 5 — VP side (expect all 8 codes: CE-1 CW-1 MT-1 MW-1 NE-1 SC-1 SE-1 W-1)
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/auth/login -ContentType "application/json" -Body '{"email":"valerie.price@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable vp
(Invoke-RestMethod -Uri "http://localhost:5777/api/accounts?limit=200" -WebSession $vp).items.territory_code | Sort-Object -Unique

# 6 — no login, no data (expect HTTP/1.1 401 + JSON error, not a data list)
curl.exe -i http://localhost:5777/api/accounts

# 7 — survives restart (expect 17353 again, no re-seed)
docker compose restart
docker compose exec db psql -U plenum_admin -d plenum -c "SELECT count(*) FROM orders;"
```

If check 4 shows any code other than `SE-1`: **RLS breach — stop everything
and report it.**

## P1 acceptance checks (PowerShell, paste-and-run)

Prereqs: DB up, seeded, API running (the three commands above), run in the
repo folder. Checks 1–2 need a fresh PowerShell window if `$rep`/`$vp`
don't exist yet.

PLENUM listens on `127.0.0.1:5777`; the bank demo keeps 8080. No port
contention — the bank demo can stay up, untouched, while these checks run.

```powershell
# 1 — SCOPE BREACH CHECK FIRST (rep must see exactly one territory)
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/auth/login -ContentType "application/json" -Body '{"email":"serena.estes@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable rep
(Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/territories?period=cumulative&basis=net&limit=200" -WebSession $rep).items.territory_code
# EXPECTED: exactly one line: SE-1
# FAIL LOOKS LIKE: any other code, or more than one line -> scope breach —
# stop everything and report. (An error message instead = feature broken, different failure.)

# 2 — VP sees all eight
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/auth/login -ContentType "application/json" -Body '{"email":"valerie.price@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable vp
(Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/territories?period=cumulative&basis=net&limit=200" -WebSession $vp).items.territory_code | Sort-Object
# EXPECTED: 8 lines: CE-1 CW-1 MT-1 MW-1 NE-1 SC-1 SE-1 W-1
# FAIL LOOKS LIKE: fewer than 8, or an error.

# 3 — GATE P1-1: the basis toggle re-ranks the top customers (spec §11 verbatim)
$g = (Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/customers?period=2025&basis=gross&limit=10" -WebSession $vp).items
$n = (Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/customers?period=2025&basis=net&limit=10" -WebSession $vp).items
"ORDER DIFFERS: " + (([string]::Join('|',$g.account_name)) -ne ([string]::Join('|',$n.account_name)))
"ALL GROSS >= NET: " + (@($g + $n | Where-Object { $_.gross_cents -lt $_.net_cents }).Count -eq 0)
"SAME TOP-10 SET: " + (-not (Compare-Object ($g.account_name | Sort-Object) ($n.account_name | Sort-Object)))
# EXPECTED: ORDER DIFFERS: True · ALL GROSS >= NET: True · SAME TOP-10 SET: True
# (Spec expects the same accounts reordered. If SAME TOP-10 SET prints False
# while ORDER DIFFERS is True, report it — that is the same fact in stronger
# form, and the auditor rules on it; the hard failures are the other two.)
# FAIL LOOKS LIKE: ORDER DIFFERS: False (toggle does nothing) or
# ALL GROSS >= NET: False (a net number exceeds its gross — money math wrong).

# 4 — GATE P1-2: the API's cumulative net equals the raw ledger (spec §11 verbatim)
$t = (Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/territories?period=cumulative&basis=net&limit=200" -WebSession $vp).items
"API TOTAL:   " + [int64](($t | Measure-Object -Property net_cents -Sum).Sum)
docker compose exec db psql -U plenum_admin -d plenum -t -c "SELECT 'LEDGER TOTAL: ' || SUM(net_unit_cents * qty)::bigint FROM order_lines;"
# EXPECTED: the two numbers are IDENTICAL, digit for digit.
# FAIL LOOKS LIKE: any difference — the rollup layer is lying about money;
# that is a stop-and-report, not a rounding footnote.

# 5 — No login, no numbers
curl.exe -i "http://localhost:5777/api/metrics/leaderboard?period=2025&basis=net"
# EXPECTED: HTTP/1.1 401 + the same JSON error envelope as P0's check 6 — not data.
# FAIL LOOKS LIKE: 200 with items, or a crash/stack trace.

# 6 — Garbage in, typed error out  (try/catch form — works on Windows
#     PowerShell 5.1 AND PowerShell 7)
try { Invoke-RestMethod -Uri "http://localhost:5777/api/metrics/customers?period=2025&basis=vibes" -WebSession $vp } catch { "STATUS: " + $_.Exception.Response.StatusCode.value__; "BODY: " + $_.ErrorDetails.Message }
# EXPECTED: STATUS: 422 and a BODY saying basis must be gross|net.
# FAIL LOOKS LIKE: data comes back (no error at all), or STATUS 500.

# 7 — Refresh is admin-only, and refreshing changes nothing it shouldn't
try { Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/admin/refresh-rollups -WebSession $rep } catch { "STATUS: " + $_.Exception.Response.StatusCode.value__ }
# EXPECTED: STATUS: 403 (a rep may not refresh)
# then log in as the ADMIN from the seed's login table (priya.nair@plenum.demo)
# and repeat with that session:
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/auth/login -ContentType "application/json" -Body '{"email":"priya.nair@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable adm
Invoke-RestMethod -Method Post -Uri http://localhost:5777/api/admin/refresh-rollups -WebSession $adm | ConvertTo-Json -Depth 4
# EXPECTED: 200 + per-matview row counts; re-run check 4 -> numbers still IDENTICAL.
# FAIL LOOKS LIKE: 200 as rep (privilege hole), or check 4 diverging after
# refresh (rollups drifting from the ledger).
```

## Development

```
bash scripts/check.sh   # fmt + clippy -D warnings + sqlx prepare --check + tests
```

Requires the dev DB up + seeded (integration tests and the sqlx check talk
to it). `.sqlx/` is committed so offline builds work.

## Dev credentials warning

Every password in this repo (`docker-compose.yml`,
`docker/initdb/01-app-role.sql`, the seeded `demo-plenum-2026`) is a
**dev/demo-only** value for a localhost demo database of synthetic data.
None of them may ever be reused for anything real.

## Known demo-phase limits

- Sessions live in API process memory (tower-sessions MemoryStore): restart
  the API and everyone logs in again. Fine for the demo; a store swap is a
  later-phase concern.
- The seed's story beats (Ridgeline Grain silence, the 28% pending quote,
  the Alpenglow conquest prospect, the leakage rep, duplicate-ish names, two
  NULL change-out units, one 100%-discount line) are seeded ON PURPOSE —
  demo script material, not bugs.
