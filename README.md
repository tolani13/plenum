# PLENUM

CRM for the installed-base business — Camfil APC audition artifact.
Source of truth: [docs/plenum-crm-01.md](docs/plenum-crm-01.md) (spec v01).

**Phase state: P0 (Foundation) built. P1+ not started.**
P0 = repo scaffold, Postgres schema + Row-Level Security + audit triggers,
deterministic seed engine, session auth, RLS session middleware, `GET
/api/accounts`. No UI.

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
- **API: 8080.**

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
Invoke-RestMethod -Method Post -Uri http://localhost:8080/api/auth/login -ContentType "application/json" -Body '{"email":"serena.estes@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable rep
(Invoke-RestMethod -Uri "http://localhost:8080/api/accounts?limit=200" -WebSession $rep).items.territory_code | Sort-Object -Unique

# 5 — VP side (expect all 8 codes: CE-1 CW-1 MT-1 MW-1 NE-1 SC-1 SE-1 W-1)
Invoke-RestMethod -Method Post -Uri http://localhost:8080/api/auth/login -ContentType "application/json" -Body '{"email":"valerie.price@plenum.demo","password":"demo-plenum-2026"}' -SessionVariable vp
(Invoke-RestMethod -Uri "http://localhost:8080/api/accounts?limit=200" -WebSession $vp).items.territory_code | Sort-Object -Unique

# 6 — no login, no data (expect HTTP/1.1 401 + JSON error, not a data list)
curl.exe -i http://localhost:8080/api/accounts

# 7 — survives restart (expect 17353 again, no re-seed)
docker compose restart
docker compose exec db psql -U plenum_admin -d plenum -c "SELECT count(*) FROM orders;"
```

If check 4 shows any code other than `SE-1`: **RLS breach — stop everything
and report it.**

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
