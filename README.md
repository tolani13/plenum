# PLENUM

CRM for the installed-base business — Camfil APC audition artifact.
Source of truth: [docs/plenum-crm-01.md](docs/plenum-crm-01.md) (spec v01).

**Phase state: P0 and P1 merged to main. P2 (Command + Leaderboards UI)
built on `p2-command-ui`, pending D.'s acceptance. P3+ not started.**
P2 adds the `web/` React app — login, app shell, the Territory Board
(Command), and the reps/items/customers Leaderboards with period/basis/kind
controls and CSV export — served by a Vite dev server that proxies to the
API. See **Run the UI (P2)** below. No backend change (P2 is UI only).
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

## Run the UI (P2)

The web app (`web/`) is a Vite dev server that proxies `/api` to the API, so
the browser talks to one origin and the session cookie just works. The API
stays on `127.0.0.1:5777`; the web page runs on **`127.0.0.1:5177`** (D.'s
call, 2026-07-19 — port 5173 was held by another program on this machine).

One-time setup (in `web/`):

```
cd "C:\AI_Projects\Camfil CRM\web"; npm install
cd "C:\AI_Projects\Camfil CRM\web"; npx playwright install chromium
```

Then three windows, one line each (PowerShell):

```
# W1 — database
cd "C:\AI_Projects\Camfil CRM"; docker compose up -d
# W2 — API (leave running)
cd "C:\AI_Projects\Camfil CRM"; cargo run --bin api
# W3 — web (leave running)
cd "C:\AI_Projects\Camfil CRM\web"; npm run dev
```

Browse to http://127.0.0.1:5177 in Chrome or Edge. Log in with any seeded
user (password `demo-plenum-2026`); the seed console prints the login table.

For the iPad checks, stop W3 and run `npm run dev:lan` (binds the web page to
all interfaces; the API stays loopback). Get the PC's address with:

```
(Get-NetIPAddress -AddressFamily IPv4 | Where-Object {$_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.254.*"} | Select-Object -First 1).IPAddress
```

Open `http://THAT-ADDRESS:5177` on the iPad.

## P2 acceptance checks (browser + PowerShell, paste-and-run)

Setup: W1/W2/W3 above running, then browse to http://127.0.0.1:5177.

```
□ 1. SCOPE FIRST. Log in as serena.estes@plenum.demo / demo-plenum-2026.
     → EXPECTED: Command loads; the Territory Board shows EXACTLY ONE tile,
       SE-1 Southeast 1; the user chip says rep · SE-1.
     FAIL LOOKS LIKE: eight tiles, or any tile that isn't SE-1 — scope
     breach; stop everything and report. (An error page = different failure.)

□ 2. NO GHOSTS BETWEEN LOGINS. Log out. Log in as
     valerie.price@plenum.demo (all 8 tiles appear). Log out. Log in as
     serena.estes@plenum.demo again and WATCH the first paint.
     → EXPECTED: SE-1's single tile only, from the first visible frame.
     FAIL LOOKS LIKE: the 8-tile board (or VP-sized numbers) flashing for
     even a moment before shrinking to SE-1 — cached cross-user data.

□ 3. GATE P2-1, amended by architect ruling 2026-07-19 (HANDOFF-LOG).
     As the VP on Command: note the big KPI number, any tile's dollar
     figure, and the coverage projected-$ sub-line. Click GROSS/NET once.
     → EXPECTED: in one motion, no reload, no white flash: the first KPI
       flips label (NET YTD ↔ GROSS YTD) and value; EVERY tile's dollar
       figure changes; coverage projected-$ changes. Rank badges recompute
       by chosen basis — and at 2026 the ORDER HOLDS, because this year's
       seeded book carries near-uniform margins across territories (honest
       reading, not a broken toggle; rank movement is check 3b).
     FAIL LOOKS LIKE: any dollar figure that does not change, a full
     refetch/blank flash, or the label not flipping.

□ 3b. Leaderboards → customers → period 2025 → basis GROSS. Note the top-10
     account names. Switch to NET.
     → EXPECTED: the order visibly changes AND Vantage Metalworks Coastal
       (gross top-10, #9) is GONE from the net top-10, with Blue Ridge
       Fabrication entering. P1-1's proven re-rank, now on screen.
     FAIL LOOKS LIKE: identical order both ways, or identical top-10
     membership.

□ 4. DRILL. As the VP, click the SE-1 tile.
     → EXPECTED: a drawer opens on the right: SE-1's revenue/leakage/
       attainment/order-count/active-accounts figures, its coverage row
       (units due, % covered, projected $), and an at-risk unit list that
       includes RIDGELINE GRAIN near the top. Esc closes it.
     FAIL LOOKS LIKE: empty drawer, a spinner that never resolves, or no
     Ridgeline Grain anywhere in SE-1's at-risk list.

□ 5. THE LEAKAGE-REP STORY (amended by architect ruling 2026-07-19).
     Leaderboards → reps → period CUMULATIVE → basis GROSS.
     → EXPECTED: the #1 rep (Wes Turner) also shows the WORST (highest)
       leakage % on the board — volume king, margin floor; the §13 demo
       line lives here. Sorting by the leakage % column puts that same rep
       on top. (The seed delivers the beat as "volume leader = worst
       margin," not a #1 flip — his gross lead survives his discounting;
       the flip form lives on customers, check 3b.)
     FAIL LOOKS LIKE: the #1 rep's leakage % mid-pack or blank, or the
     column unsortable.

□ 6. CONTROLS. Items tab → period 2025-Q3 → kind CONSUMABLE.
     → EXPECTED: rows are cartridge/filter products; an ATTACH % column
       shows values; switching kind to CAPITAL empties/dashes ATTACH % and
       changes the rows and totals. Every control change updates the table
       without a full page reload, and the URL updates with it.
     FAIL LOOKS LIKE: attach % on capital rows, controls that do nothing,
     or a 422/error toast from a control combination the UI itself offered.

□ 7. CSV. Customers tab → CUMULATIVE → NET → Export CSV. Open the file.
     → EXPECTED: plenum-customers-cumulative-net.csv downloads and opens in
       Excel; its data row count equals the table's visible row count; the
       first data row is the same account as the table's #1 row. Repeat
       once logged in as serena: the file's rows are SE-1 accounts only,
       matching her on-screen table exactly.
     FAIL LOOKS LIKE: garbled columns in Excel, row counts that don't
     match the screen, or the rep's file containing accounts her screen
     doesn't show (scope leak in export — stop and report).

□ 8. THE LEDGER ANCHOR. Still customers + CUMULATIVE + NET (as VP): read
     the footer total of the NET column.
     → EXPECTED: exactly $24,670,890.87 — the same cumulative net the API,
       the raw ledger, and P1's acceptance all agreed on.
     FAIL LOOKS LIKE: any other number — the UI is misadding or
     mis-rendering money; stop and report.

□ 9. GATE P2-2, automated half. One line (API from W2 still running):
     cd "C:\AI_Projects\Camfil CRM\web"; npm run tripwire
     → EXPECTED: a per-screen/per-width list ending
       TRIPWIRE 25/25 layout PASS · rep-scope PASS.
     FAIL LOOKS LIKE: any line naming a screen and width that FAILED
     (page wider than viewport), or the scope assertion failing.

□ 10. GATE P2-2, manual half (iPad portrait + landscape). In W3, stop the
     dev server (Ctrl+C) and run: cd "C:\AI_Projects\Camfil CRM\web"; npm run dev:lan
     Get the PC's address (one line):
     (Get-NetIPAddress -AddressFamily IPv4 | Where-Object {$_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.254.*"} | Select-Object -First 1).IPAddress
     On the iPad's Safari open http://THAT-ADDRESS:5177, log in as the VP,
     and try to swipe the page sideways on Command and on Leaderboards, in
     BOTH portrait and landscape.
     → EXPECTED: the page never moves horizontally; the Board sits 4-wide
       in landscape and collapses (4→2) in portrait; tables drop their
       lesser columns instead of spilling; everything stays readable.
     FAIL LOOKS LIKE: the whole page slides sideways, tiles/tables cut off
     at the right edge, or a horizontal scrollbar on the page itself.
```

## P4 — Signals + AI

**The four signal generators (all deterministic, all derived).** After every
seed — and on demand — `generate_signals()` derives the queue from table
data alone: **reorder_due** (units inside `reorder_lookahead_days` of their
cadence due date, plus any unit whose telemetry `filter_life_pct` is at or
under the trigger), **defection_risk** (metric 7's view verbatim: silence
past `expected_changeout_months × 1.5`, scored by cycles-missed × annual
value), **conquest** (competitor units with no order history, cross-referenced
through `filter_fits` to our best-fitting replacement SKU), and
**discount_anomaly** (order lines in the trailing window whose discount sits
more than 2σ above their family's median). Every card carries its reasons on
its face; thresholds live in the `signal_policy` config row (survives
reseeds); reruns upsert by a deterministic `dedupe_key` — no duplicates, no
touching assigned/actioned/dismissed cards, zero audit noise when nothing
changed.

Regenerate on demand (admin session): `POST /api/admin/generate-signals` —
one PowerShell line, after logging in as priya.nair@plenum.demo:

```
cd "C:\AI_Projects\Camfil CRM"; $s = New-Object Microsoft.PowerShell.Commands.WebRequestSession; Invoke-RestMethod -Uri "http://127.0.0.1:5777/api/auth/login" -Method Post -ContentType "application/json" -Body '{"email":"priya.nair@plenum.demo","password":"demo-plenum-2026"}' -WebSession $s | Out-Null; Invoke-RestMethod -Uri "http://127.0.0.1:5777/api/admin/generate-signals" -Method Post -WebSession $s | ConvertTo-Json -Depth 4
```

**AI env keys (.env only — never commit a key).** Copy `.env.example`; the
four P4 keys are `ANTHROPIC_API_KEY` (leave empty to run with AI off —
every screen still works), `ANTHROPIC_MODEL` (default `claude-sonnet-5`),
`AI_ASK_ENABLED`, `AI_DISCOUNT_ENABLED` (default true). Put your key in
`.env`, never in any committed file — `.env` is gitignored and the key never
reaches the client bundle or a log line.

**Ask PLENUM** (`/ask`, or Ctrl-K anywhere): your question becomes ONE
PostgreSQL SELECT over a whitelisted semantic layer — `v_order_facts`, the
four `v_*_period` rollup views, and `v_defection_risk` — validated against
the real SQL AST (single statement, SELECT-only, whitelisted relations
only), executed inside YOUR read-only RLS session with a 5s timeout and an
injected LIMIT 500, and shown with the SQL itself as receipts. A rep cannot
ask their way into another territory. With no key the page serves the
saved-question library instead, and nothing errors.

Acceptance walk: the P4 unit's 12 checks (Signals queue + Ridgeline card,
draft-from-signal, scope, write-backs, idempotent regeneration, Command
rewire, flag-off/flag-on Ask, recommender degradation, telemetry, tripwire
55+3, reseed) live in the session report and HANDOFF-LOG — run them from
docs/HANDOFF-LOG.md's newest entry.

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
