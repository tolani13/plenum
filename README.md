# PLENUM

PLENUM is a CRM built for the installed-base business — every screen answers
"what does the installed base owe us next?", every money figure exists as
gross AND net, and every AI output carries its receipts. It was built as the
audition artifact for the AI Sales & Solutions Architect role at Camfil APC:
synthetic data on purpose, production-shaped architecture on purpose (see
[PRODUCTION.md](PRODUCTION.md) for the honest map from demo to deployment).
Source of truth: [docs/plenum-crm-01.md](docs/plenum-crm-01.md) (spec v01);
build history: [docs/HANDOFF-LOG.md](docs/HANDOFF-LOG.md).

**Phase state: P0–P4 merged; P5 (polish + Territory Map + Leakage +
Data Quality + signal auto-expiry) built on `p5-polish-map`.**

## Prerequisites

- Docker Desktop (running)
- Rust ≥ 1.80 (`rust-toolchain.toml` pins 1.95.0)
- Node 20+ / npm (for the web app)

## Quickstart — fresh clone to running app in three commands

```powershell
cd "C:\AI_Projects\Camfil CRM"; docker compose up -d
```

```powershell
cd "C:\AI_Projects\Camfil CRM"; cargo run --bin seed
```

```powershell
cd "C:\AI_Projects\Camfil CRM"; .\scripts\run-all.ps1
```

Then open **http://127.0.0.1:5177** and log in from the table below.

What the three commands do:

1. `docker compose up -d` — starts Postgres 16 (host port **5434**; this
   machine's native PostgreSQL owns 5432/5433) and, on first run, creates the
   non-privileged `plenum_app` role the API is confined to (RLS applies to
   every query it makes).
2. `cargo run --bin seed` — applies migrations (the only thing that does),
   then truncates and regenerates the identical synthetic world every run
   (PRNG seed 20260717): prints per-entity counts, the
   `ORDERS TOTAL: 17353 (gate: >15000)` line, refreshes rollups, analyzes,
   derives the signal queue, and prints the login table.
3. `.\scripts\run-all.ps1` — starts the API (127.0.0.1:5777) and the Vite
   web server (127.0.0.1:5177) in their own windows. On a fresh clone it
   first materializes a dev `.env` (dev-only credentials that already live in
   `docker-compose.yml`; `COOKIE_SECURE=false` so plain-HTTP localhost
   sessions work; AI key left empty — every screen still works with AI off).

## Logins

Password for **every** demo user: `demo-plenum-2026` (dev-only by design —
it is printed by the seed and lives nowhere real).

| email                      | role             | territories |
| -------------------------- | ---------------- | ----------- |
| valerie.price@plenum.demo  | vp               | ALL (8)     |
| priya.nair@plenum.demo     | admin            | ALL (8)     |
| rachel.moore@plenum.demo   | regional_manager | NE-1+SC-1+SE-1 |
| marcus.reed@plenum.demo    | regional_manager | CE-1+CW-1+MT-1+MW-1 |
| renee.vega@plenum.demo     | regional_manager | CE-1+CW-1+W-1 |
| nora.ellery@plenum.demo    | rep              | NE-1        |
| nathan.eastman@plenum.demo | rep              | NE-1        |
| serena.estes@plenum.demo   | rep              | SE-1 — the scope-isolation rep |
| sam.cole@plenum.demo       | rep              | SC-1        |
| miles.webb@plenum.demo     | rep              | MW-1        |
| mia.winters@plenum.demo    | rep              | MW-1        |
| mona.tate@plenum.demo      | rep              | MT-1        |
| dana.cross@plenum.demo     | rep              | CE-1+CW-1 (dual-territory) |
| wes.turner@plenum.demo     | rep              | W-1 — the leakage rep |
| willa.reyes@plenum.demo    | rep              | W-1         |
| celine.roy@plenum.demo     | rep              | CE-1        |
| cole.brandt@plenum.demo    | rep              | CW-1        |

## Demo reset — one line

```powershell
cd "C:\AI_Projects\Camfil CRM"; .\scripts\demo-reset.ps1
```

Reseeds the identical world (same anchors, same story beats), refreshes the
rollups, re-analyzes, and regenerates the signal queue. Config tables
(`discount_policy`, `signal_policy`, `territory_states`) survive — they are
seeded in migrations, not in the wipe. **Open browser sessions must log in
again after a reset** (API sessions are in-memory by design).

## The 7-minute demo script (spec §13)

1. **Open on the Signals queue** — Ridgeline Grain defection card. "This
   account went quiet 11 months ago; at their cadence that's ~$34k/yr of
   cartridges now going to a will-fit competitor. PLENUM caught it, showed
   its math, and drafted the win-back quote." *(Theses 1, 3, 4, 5 in 60
   seconds.)*
2. Command screen — Territory Board; flip **gross → net**: every number on
   screen moves at once. Then deliver the "watch rankings move" line on
   **Leaderboards → customers → 2025**: flip GROSS → NET and watch Vantage
   Metalworks Coastal drop out of the top-10 while Blue Ridge Fabrication
   enters (the P2 gate amendment: the frozen seed holds territory order at
   2026, so the re-rank observable lives on the customers tab). "Your best
   rep on volume is your worst on margin. Traditional CRMs can't show you
   this because they store one revenue number."
3. Leaderboards — customers per quarter / year / cumulative, both bases (the
   requirement list, live).
4. Account 360 — installed-base timeline: "every unit is an annuity with a
   due date."
5. Quote at 28% → approval flow → audit trail. Governance, not vibes.
6. Ask PLENUM — **both forms, the live-or-not call is D.'s at rehearsal:**
   - **Live** (key present): ask "top 10 customers by net revenue in 2025" —
     table + chart + the SQL receipts; as a rep the same question returns
     own-scope rows only (RLS, not prompt engineering).
   - **Flags-off fallback** (no key / no network): the page serves the
     saved-question library — seven standing questions, each landing on a
     live screen; nothing errors.
7. Close on the platform story: Artifact 1's collector pushing filter-life
   into this queue (`POST /api/telemetry/filter-life` → regenerate → the
   card appears; push it back up → the card expires). "Sensor to ledger.
   That's the solutions-architect job, and this is what it looks like."

Bonus beats now on screen: **Territory Map** (the board projected on the
continent — rep view shows foreign territories as dimmed silhouettes with no
dollars), **Leakage** (distribution, the anomaly feed that matches the
signal chips 1:1, and the rep × family heat table where Wes Turner reads
worst), **Data Quality** (the seeded mess, found — mess is information).

## Territory Map — the committed map asset

The US map is a static, public-domain, committed-to-repo SVG (spec §12 as
amended 2026-07-22 — no tile services, no geocoding, no geo libraries, no
runtime fetches):

- Asset: [web/src/map/blank-us-map-states-only.svg](web/src/map/blank-us-map-states-only.svg)
  — "Blank US Map (states only).svg" by **Heitordp**, Wikimedia Commons,
  license **CC0 1.0 Universal Public Domain Dedication**.
- The app renders [web/src/map/usStates.ts](web/src/map/usStates.ts), a
  typed module derived from that file (per-state path + USPS code + name).
- State→territory geography is config: the `territory_states` table
  (migration 0013), seeded along US Census division lines; Canada renders as
  two schematic blocks (CA-E / CA-W — province detail out of scope).

## Deploy (Render)

The live demo runs the PRODUCTION.md conversion on Render: **one public web
service** (a Docker image serving the Rust API under `/api/*` and the built
SPA for everything else — one origin, so the `SameSite=Lax` + `Secure`
session cookie needs no CORS) and **one managed Postgres 16**. Both on free
plans. [render.yaml](render.yaml) is the blueprint; the same topology can be
recreated by re-applying it (Render dashboard → New → Blueprint) or by the
Render CLI/API following it.

**Provisioned resources:** database `plenum-db` (free, oregon, PG16) ·
web service `plenum` (free, oregon, Docker, health check `/api/health`,
`autoDeploy` off — deploys are explicit).

**Env vars on the service** (no secrets in the repo or image — everything
arrives here): `APP_DATABASE_URL` + `DATABASE_URL` (the managed database's
internal connection string), `COOKIE_SECURE=true`, `MIGRATE_ON_BOOT=true`
(the API applies embedded migrations on boot — idempotent — and serves an
empty-but-migrated world instead of exiting), `AI_ASK_ENABLED=false`,
`AI_DISCOUNT_ENABLED=false`, `RUST_LOG=info`. **No `ANTHROPIC_API_KEY`
exists in prod** — AI is off, provably: Ask serves the saved-question
library, the COMPS button hides, zero vendor spend is possible.

**Seed / production reset** (explicitly, never on deploy — a redeploy never
wipes demo state). Render one-off jobs need a **paid** instance plan
("free tier plans are not supported for jobs"), so on the free plan the
reset runs the seed binary locally against the database's EXTERNAL
connection string over TLS (TRUNCATE + regenerate: every frozen anchor
restored, signals regenerated). Copy the external string from the Render
dashboard (plenum-db → Connect → External Database URL) and run:

```powershell
cd "C:\AI_Projects\Camfil CRM"; $env:DATABASE_URL = "<EXTERNAL_DATABASE_URL>?sslmode=require"; cargo run --bin seed; $env:DATABASE_URL = $null
```

Requirements for that command: your public IP must be on the database's
**Access Control allowlist** (dashboard → plenum-db → Access Control; the
deploy added `74.124.184.78/32`, this machine), and `?sslmode=require`
stays on the URL. On a paid instance type the job form works instead:
`render jobs create srv-d9goii4vikkc739qverg --start-command "/app/seed"`.
Note: the seed detects a non-superuser (managed) connection and pins the
seeded admin's identity for its session — managed owners are subject to
the FORCEd RLS by design; local superuser runs are unchanged.

**Who can see this (privacy posture, stated honestly):** there is no email
gate. The link's privacy is the unguessable `onrender.com` URL plus the demo
login; every row of data is synthetic. A hard gate (edge auth / custom
domain behind an access proxy) is a later add if wanted.

**Known behaviors on the free plan:**

- The service **spins down when idle**; the first hit afterward cold-starts
  (tens of seconds). By design of the free plan.
- Sessions are in-memory (accepted demo posture): a **redeploy or
  spin-down signs everyone out** — log back in.
- After a redeploy, a browser holding the old page may fetch a stale hashed
  chunk and get the SPA shell instead; a reload fixes it.
- **Render's free Postgres expires after 30 days** unless upgraded — the
  database (and the demo data) is deleted then. Re-applying the blueprint +
  one seed job restores everything, deterministically.

## Development

```
bash scripts/check.sh   # fmt + clippy -D warnings + sqlx prepare --check + tests
```

Requires the dev DB up + seeded (integration tests and the sqlx check talk
to it). `.sqlx/` is committed so offline builds work. The responsive
tripwire (70 layout checks across 14 screens × 5 widths + 5 scope
assertions) runs with the API up:

```powershell
cd "C:\AI_Projects\Camfil CRM\web"; npm run tripwire
```

## Known behaviors (recorded, by design)

- **Quota attainment is measured against the full-year quota** (quarters are
  prorated /4; years and TTM use the whole annual figure) — mid-year bars
  sit low on purpose; they are honest.
- **`mv_product_period` transiently reports 1700** rows after an in-quarter
  booking + refresh (the booked current-quarter order enters the matview but
  stays read-filtered by the live-quarter boundary). Benign; reseed restores
  1699.
- **Signal counts drift with the clock** by design (due windows, silence
  boundaries, and the anomaly recency window all read CURRENT_DATE). Two
  same-day reseeds print identical counts; different days differ. The frozen
  anchors (row counts, checksums, money totals) never move.
- **Signal auto-expiry (P5):** an OPEN card whose predicate stops holding is
  expired by the next generation run (visible under the queue's Expired
  filter); assigned/actioned/dismissed cards are never auto-touched, and an
  expired card whose predicate returns is reopened by the generator.
- **Manager-tier self-approval nicety:** a regional manager who authors a
  10–25% quote may also approve it (the role tier, not authorship, gates the
  decision) — reviewed and recorded as accepted for the demo phase.
- Sessions live in API process memory: restart the API and everyone logs in
  again.
- The seed's story beats (Ridgeline Grain silence, the 28% pending quote,
  the Alpenglow conquest prospect, the leakage rep, duplicate-ish names, two
  NULL change-out units, one 100%-discount line) are seeded ON PURPOSE —
  demo script material, not bugs. The Data Quality screen finds the mess;
  that is the feature.

## Troubleshooting

- **Ports on this machine:** database host port **5434** (container-internal
  5432 — every `docker compose exec db psql …` is unaffected) · API
  **127.0.0.1:5777** · web **127.0.0.1:5177** (strictPort — if it is taken,
  Vite fails loudly rather than drifting). **8080 belongs to another tenant
  of this machine** (the Local-Secure-Ops bank demo — another agent's active
  project; never stop or modify it).
- `cannot bind 127.0.0.1:5777` → a previous PLENUM API window is still open;
  close it.
- API window says `database is empty — run: cargo run --bin seed` → exactly
  that.
- Login fails from the browser on a machine without a `.env` → run
  `.\scripts\run-all.ps1` once (it writes the dev `.env` with
  `COOKIE_SECURE=false`); without it the API defaults to the production
  cookie posture and plain-HTTP localhost drops the session cookie.
- **Fresh-clone test on a machine already running PLENUM:** stop the
  original stack first (`docker compose down` in the original folder) — the
  database container name and host port are pinned, so two stacks cannot run
  side by side. The original volume (and its data) survives `down` and comes
  back with the next `up`.

## Dev credentials warning

Every password in this repo (`docker-compose.yml`,
`docker/initdb/01-app-role.sql`, the seeded `demo-plenum-2026`, and the dev
`.env` that `run-all.ps1` writes) is a **dev/demo-only** value for a
localhost demo database of synthetic data. None of them may ever be reused
for anything real. The Anthropic API key is env-only: put it in `.env`
(gitignored) to light up Ask PLENUM and the COMPS narrative; leave it empty
and every screen still works.
