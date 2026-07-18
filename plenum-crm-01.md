# PLENUM — CRM Build Spec / Master Context

**Version:** 01 — concept painted end-to-end. **NOTHING LOCKED.** Devil's-case pass not yet run; it gates the CC handoff.
**Date:** 2026-07-17
**Owner:** D. · **Builder:** Claude Code · **Architect:** Claude (Cowork)
**Working name:** PLENUM — the clean-air chamber where everything a collector processes arrives, unified, before going back to work. Same job, for revenue data. Name not locked.
**Purpose:** Audition artifact for the AI Sales & Solutions Architect role at Camfil APC. This document is the single source of truth. CC does not ask D. to re-explain anything in this file.

---

## 0 · The audition frame (why this exists)

- **Outcome metric:** an offer. Everything in this spec is judged by one question — does it make the interview panel say "nobody else showed us this"?
- **Operator:** the interview panel (sales VP types + technical evaluators), not a live sales org. This inverts one FDA rule — real data day one is impossible — so the spec compensates two ways: (a) a seed engine that deliberately simulates *messy, story-bearing* data, and (b) an explicit "day-one plan" slide in the demo script showing exactly how seed swaps for their ERP extract. Knowing that demos on fake data are usually a trap, and designing around it out loud, **is itself the audition.**
- **The two-artifact platform story.** Artifact 1 (already built): the 3D unit + telemetry demo — a collector predicting its own filter service date. Artifact 2 (this): the CRM where that prediction becomes a reorder opportunity in a rep's queue. Together: *"from the collector's differential pressure sensor to the sales ledger."* No other candidate walks in with a sensor-to-invoice story. The two apps share one design language (control-room palette, nameplate typography) so they read as one platform.

## 1 · The Five Theses (what "not traditional, not plain" means, precisely)

1. **Installed-base-first.** A traditional CRM's center of gravity is the Opportunity. PLENUM's is the **Installed Unit**. Camfil's world: a six-figure collector sale creates a 10–15 year annuity of cartridge replacements. The installed base of collectors in the field — theirs *and competitors'* — is the map of all future revenue. Every screen ultimately answers "what does the installed base owe us next?"
2. **Dual-ledger revenue.** Every money number in the system exists twice — **gross (list)** and **net (after discount)** — computed from line-level facts, never stored as a single blended figure. Discount is not a footnote; **discount leakage** (gross − net, by rep, territory, customer, product line) is a first-class KPI with its own screen. This is the feature sales VPs feel in their chest.
3. **Both directions of the filter war.** Camfil sells replacement filters that fit other brands' collectors (conquest), and will-fit competitors attack Camfil's own installed base (defection). PLENUM models both: conquest targets (competitor units on file → filter cross-reference → campaign) and **defection alarms** (a customer whose cartridge reorder cadence goes silent is buying will-fit — the AI's highest-value signal, given retention economics).
4. **AI with receipts.** Every AI output — score, prediction, recommendation — carries its reasons on its face (top contributing factors, the comparable deals, the cadence math). No black-box numbers anywhere. Consistent with Artifact 1's C-06 story: AI that shows its work is AI a 40-year sales veteran will trust.
5. **Read → decide → act → write back.** No dashboard graveyards. Every surfaced signal lands in a queue with actions (assign, quote, log outcome, dismiss-with-reason), and every action writes back as data. A CRM that only displays gets abandoned by week four.

## 2 · Domain grounding (from camfilapc.com, 2026-07)

- **Capital equipment:** Gold Series III (new flagship), Gold Series X-Flo, GSX P, Gold Series Camtain (pharma containment), Gold Series High Vacuum, Quad Pulse Package, wet scrubbers, machine mist collectors (EM-O class), explosion protection systems, accessories.
- **Consumables:** OmniPleat® filter technology (current flagship pleating), OptiCone cartridges (GS III / Camtain / High Vacuum), X-Flo cartridges, Tenkay, Quad Pulse cartridges, Stat-Safe™ static-dissipating cartridges (new), plus **replacement filters for other brands**.
- **Industries served:** automotive, pharmaceutical, metals (welding, laser/plasma cutting, thermal spray, machining mist, abrasive blasting), food & beverage, chemical processing, mining, grain/feed/seed, woodworking, packaging.
- **Go-to-market:** territory-based sales force + independent rep network ("Find a sales rep," "Join Our Sales Force"), with an aftermarket portal. Compliance drivers everywhere: NFPA, OSHA, ATEX, EPA.
- Marketing claims a very large installed base and high customer retention — meaning single-digit retention improvements are worth real money, which is the defection-alarm business case.

Seed data mirrors this world with fictional customers. Real Camfil product families appear by name (nominative use in an interview artifact); all customers, reps, and numbers are synthetic.

## 3 · Ontology (operator language — objects, properties, links, actions)

**Objects** (the nouns a Camfil regional manager already says):

| Object | Key properties (decision-relevant only) |
|---|---|
| **Account** | name, industry, parent_account (corporate → plant sites), territory, status (customer / prospect / at-risk / dormant), created |
| **Site** | account, address, city/state, primary_contact |
| **Contact** | site, name, title, email/phone, role (buyer / EHS / plant engineer / maintenance) |
| **Territory** | code (e.g. SE-1), name, region (Northeast / Southeast / Midwest / South-Central / Mountain / West / Canada-E / Canada-W), quota_year_cents |
| **Rep** (user) | name, role (rep / regional_manager / vp / admin), manager, territories[] |
| **Product** | sku, name, family (GS III / X-Flo / Camtain / High-Vac / QPP / Mist / Filters-OptiCone / Filters-XFlo / Filters-Tenkay / Filters-StatSafe / Filters-Replacement-Brand / Accessories / Service), kind (capital / consumable / part / service), list_price_cents, filter_fits[] (collector families a cartridge serves — incl. competitor families) |
| **InstalledUnit** | site, product (capital), serial, commissioned_on, source (ours / competitor-brand name), cartridge_count, cartridge_product, expected_changeout_months, last_filter_order_on, filter_life_pct (nullable — telemetry stretch) |
| **Opportunity** | account, territory, owner (rep), stage (lead / qualified / quoted / negotiation / won / lost), kind (capital / retrofit / filter-program), amount_cents (gross), expected_close, lost_reason |
| **Quote** | opportunity, lines[], status (draft / pending_approval / approved / sent / accepted / rejected), approver, discount_policy_result |
| **Order** | account, site, territory, rep, ordered_on, lines[] |
| **OrderLine** | product, qty, **list_unit_cents, net_unit_cents, discount_pct** (all three stored; CHECK-constrained consistent) |
| **Signal** | type (reorder_due / defection_risk / conquest / discount_anomaly), account/site/unit, score, **reasons[] (structured)**, status (open / assigned / actioned / dismissed), assigned_to, outcome, dismissed_reason |
| **Activity** | account, rep, kind (call / visit / email / note), occurred_at, body |

**Links that matter:** Account has Sites; Site has InstalledUnits; InstalledUnit consumes a cartridge Product; Product filter_fits collector families (the cross-reference that powers conquest); Territory has Reps via assignment; Orders roll up Site → Account → Territory → Region. Most questions in this business are link-walks — model these and the analytics become window functions instead of heroics.

**Actions (the verbs = the write-back API):** assign signal · draft quote from signal · submit quote for approval · approve/reject quote (with reason) · mark won/lost (with reason) · log activity · dismiss signal (reason required) · reassign account.

## 4 · Data model (Postgres)

Money is **BIGINT cents**, always. Timestamps `timestamptz`. IDs `uuid` default `gen_random_uuid()`. Migrations via `sqlx migrate`.

Tables (columns beyond the ontology's, where structural):

- `users` (id, email, password_hash argon2id, name, role enum, manager_id nullable FK users)
- `territories` (id, code unique, name, region, quota_year_cents)
- `territory_assignments` (user_id, territory_id, PK both) — a rep can carry >1 territory
- `accounts` (…, territory_id FK, parent_account_id nullable self-FK)
- `sites`, `contacts` per ontology
- `products` (…, `filter_fits text[]` of family codes)
- `installed_units` per ontology; index on (site_id), (cartridge_product_id, last_filter_order_on)
- `opportunities` (…, stage enum, amount_cents)
- `quotes` (id, opportunity_id, status enum, approver_id nullable, created_by, created_at)
- `quote_lines` (quote_id, product_id, qty, list_unit_cents, net_unit_cents, discount_pct numeric(5,2), CHECK (net_unit_cents = round(list_unit_cents * (1 - discount_pct/100))))
- `orders` (id, account_id, site_id, territory_id, rep_id, ordered_on date)
- `order_lines` (same price triplet + CHECK as quote_lines)
- `signals` (…, `reasons jsonb` — array of {label, weight, detail}, status enum, timestamps for each transition)
- `activities`
- `audit_log` (actor, action, entity, entity_id, before/after jsonb, at) — written by triggers on quotes, signals, opportunities

**Derived analytics layer (SQL views, not app code):**
- `v_order_facts` — one row per line joined to account/territory/rep/product/family/kind, with gross_cents, net_cents, discount_cents, calendar quarter/year columns. Every rollup in the product reads from this one view; the metric definitions below are its contract.
- `mv_territory_period`, `mv_rep_period`, `mv_product_period`, `mv_customer_period` — materialized rollups by (entity, quarter) with gross/net/discount; refreshed after seed and on demand (`POST /api/admin/refresh-rollups`). Live quarter is computed from `v_order_facts` directly and unioned in, so "today's order moves today's number" — pre-aggregation never goes stale on the demo path.

**RLS (this is the enterprise tell — do it properly):**
- Every request runs in a transaction that first executes `SET LOCAL app.user_id = $1; SET LOCAL app.role = $2`.
- Policies: **rep** sees accounts/orders/opps/signals in their assigned territories; **regional_manager** sees their reports' territories (resolve subtree via a `v_user_scope(user_id) → territory_id` view over the manager chain); **vp/admin** see all. Enforced in Postgres RLS on `accounts`, `orders`, `opportunities`, `quotes`, `signals`, `activities` — not in app code. App code treats an empty result as normal, never as an error to "fix" by widening a query.
- 🚩 Non-negotiable: no query path bypasses RLS except the seed/refresh admin path, which requires role=admin.

## 5 · Metrics dictionary (exact definitions — CC implements these verbatim)

Every metric returns **both** `gross` (Σ list_unit_cents·qty) and `net` (Σ net_unit_cents·qty); `discount_leakage = gross − net`; `leakage_pct = leakage / gross`. Periods: calendar quarters (Q1=Jan–Mar), calendar years, `cumulative` (all history), `ttm` (trailing 12 months). Every ranking endpoint takes `period`, `basis` (gross|net), and optional `kind` (capital|consumable|all) — because capital lumps and consumable annuities are different businesses and mixing them hides both.

1. **Territory totals:** per territory per period — gross, net, leakage, order count, active accounts, quota attainment (net / prorated quota).
2. **Sales leaderboard:** reps ranked by chosen basis per period; ties broken by leakage_pct ascending (rewarding margin discipline); each row shows gross, net, leakage%, capital/consumable split, top account.
3. **Item leaders:** products and product families ranked per period by basis; units and revenue; consumable view includes attach rate (share of installed units of the served family that ordered this period).
4. **Top customers:** accounts ranked per quarter / per year / cumulatively, by gross and by net (toggle) — with each account's leakage% and capital:consumable mix. (This is the requirement list verbatim: per quarter, per year, cumulative, with and without discounts.)
5. **Discount leakage board:** leakage by rep, territory, family, and account; distribution of discount_pct; outliers = line items > 2σ above family median discount → feed `discount_anomaly` signals.
6. **Aftermarket coverage:** per territory — installed units due for change-out this quarter, % with an order or open quote, projected consumable revenue (units due × cartridge_count × cartridge list/net).
7. **Defection risk:** units where `today − last_filter_order_on > expected_changeout_months × 1.5` → signal, scored by gap size × annual consumable value.

## 6 · The AI layer (the role is *AI* Sales & Solutions Architect — this is the audition's core)

All AI features obey the **receipts contract**: response = value + structured `reasons[]`, rendered in UI. All are feature-flagged; the app is fully demoable with flags off.

1. **Reorder radar (deterministic, always on).** Cadence math per installed unit → `reorder_due` signals with reasons (last order date, expected cadence, cartridge value). One tap: *draft quote* — pre-filled with the unit's cartridge SKU, qty = cartridge_count, list price. This is the sensor-to-ledger bridge; a stretch endpoint `POST /api/telemetry/filter-life` lets Artifact 1's simulator push filter_life_pct and accelerate the signal.
2. **Defection alarms (deterministic, always on).** Metric 7 → `defection_risk` signals → triage queue with assign / call-logged / dismiss(reason). Dismissal reasons are captured data — next quarter's model food.
3. **Conquest finder (deterministic, always on).** Competitor-brand InstalledUnits ⨯ `filter_fits` cross-reference → ranked conquest list per territory (annual filter value estimate).
4. **Discount recommender (Claude API, flagged).** On a quote line: median/IQR of discount_pct for won deals of same family + account industry + order-size band (SQL), summarized by Claude into a one-line recommendation with the comparables listed. Degrades to showing the raw comparables table without narrative.
5. **Ask PLENUM (Claude API, flagged).** Natural-language analytics over a *whitelisted semantic layer* (`v_order_facts` + the mv_ rollups only). Backend-proxied; the model gets the view schemas + metric dictionary and returns SQL; server validates: SELECT-only, whitelisted relations, injected LIMIT 500, statement_timeout 5s, executed **under the caller's RLS session** so a rep cannot ask their way into another territory. UI renders table + auto-chart + the SQL itself (receipts). No key present → the bar serves the curated question library (the 7 metric screens as saved questions).
   - 🚩 Never call Anthropic from the browser; key lives server-side in env. Absent key must not error any screen.

## 7 · Backend (Rust)

- **Stack:** axum 0.8 · tokio · sqlx 0.8 (postgres, runtime-tokio, tls-rustls) · tower-sessions (cookie sessions) · argon2 · serde · thiserror · tracing + tracing-subscriber · reqwest (Claude proxy). Postgres 16.
- **Workspace:** `crates/domain` (types, metric definitions as typed query builders), `crates/api` (axum bin: routers, extractors, RLS session guard middleware), `crates/seed` (bin: deterministic generator), `migrations/`.
- **API surface (JSON; `/api` prefix):**
  - Auth: `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`
  - Analytics: `GET /metrics/territories`, `/metrics/leaderboard`, `/metrics/items`, `/metrics/customers`, `/metrics/leakage`, `/metrics/coverage` — all take `?period=&basis=&kind=`
  - CRM: `GET/POST /accounts`, `GET /accounts/:id` (360 payload: sites, units, orders, opps, signals, activities), `GET/POST /opportunities`, `PATCH /opportunities/:id/stage`, `GET/POST /quotes`, `POST /quotes/:id/submit`, `POST /quotes/:id/approve|reject`, `GET/POST /activities`
  - Signals: `GET /signals?status=&type=`, `POST /signals/:id/assign|action|dismiss`
  - AI: `POST /ai/ask`, `POST /ai/discount-recommendation` (503 + typed body when flag off)
  - Admin: `POST /admin/refresh-rollups`
- **Discount governance:** quote submit computes worst line discount → policy: rep may self-approve ≤10%, manager approval 10–25%, VP >25%. Approval writes `audit_log`. (The thresholds are seed-config, not hardcoded.)
- **Non-negotiables (senior-dev scan):** no secrets in client; argon2id + secure/httponly/samesite cookies; every handler returns typed errors (401/403/404/422 distinct — 403 vs empty-200 must match RLS semantics); pagination (`limit/offset`, max 200) on every list; destructive/dismissive actions require reason strings; sqlx compile-checked queries (`sqlx prepare` in CI script); `cargo clippy -- -D warnings` clean.

## 8 · Frontend (React/TS)

- **Stack:** React 19 · TypeScript · Vite 7 · Tailwind v4 · TanStack Query 5 + TanStack Table 8 · Recharts 3 · lucide-react · Fontsource (Barlow Condensed 500/600 + Inter). No component framework — the design language is ours.
- **Design language = Artifact 1's, exactly:** graphite `#0b0f14`, panel `#121a23`, seam `#23303d`, air `#e8edf2`, mist `#8ca0b3`, brand green `#19b36b` (state semantics only), amber `#f5b81c`, alarm `#e5484d`, flow blue `#3e9bff` (data). Nameplate labels (Barlow Condensed, uppercase, 0.14em tracking). Tabular numerals everywhere money appears. Dark, instrumented, SCADA-honest — a sales control room, not a pastel SaaS.
- **Signature element (one, per design doctrine): the Territory Board.** Not a geographic choropleth (fragile geodata, unreadable on tablets) — an abstract cartogram: eight territory tiles laid out roughly geographically, each tile a live instrument — net revenue, quota-attainment bar, leakage LED, open-signal count. Click a tile → territory drill. It's the first thing on screen and the thing the panel remembers. Everything else stays quiet and disciplined.
- **Screens:**
  1. **Command** — Territory Board + top-line KPIs (net YTD, leakage%, coverage%, open signals) + gross/net toggle that flips *every number on screen at once* (Thesis 2 made visceral).
  2. **Leaderboards** — reps / items / customers tabs; period scrubber (Q toggle rail + year + cumulative + TTM); basis toggle; capital/consumable filter. TanStack tables, CSV export.
  3. **Leakage** — discount distribution, outlier feed, rep×family heat table.
  4. **Signals queue** — the daily-driver: reorder / defection / conquest / discount-anomaly lanes, each card shows the receipts, actions inline (assign, draft quote, log call, dismiss+reason).
  5. **Account 360** — header KPIs (cumulative gross/net, leakage%), **installed-base timeline** (units on a horizontal life-axis with change-out due markers — the aftermarket annuity made visible), orders, opps, activity log, contacts.
  6. **Pipeline** — opportunity kanban by stage with stage-change write-back; quote panel with approval state machine.
  7. **Quotes** — builder (lines, discount entry, live policy verdict), approval inbox for managers.
  8. **Ask PLENUM** — cmd-K palette + full page: NL question → table + chart + SQL receipts; saved-question library when flag off.
  9. **Login.**
- **Responsive doctrine (embedded, non-negotiable):** supported width range 375px → 3440px. No fixed pixel widths on containers; grids collapse 4→2→1 by available width; tables get priority-hide columns and contained horizontal scroll (element-level, never page-level); page-level horizontal scrollbar = build failure; both tablet orientations defined. **Tripwire:** a Playwright script renders every screen at 375 / 768×1024 / 1024×768 / 1440 / 2560 and fails CI if `document.documentElement.scrollWidth > innerWidth`. Ships in P2, first UI phase — not retrofitted.

## 9 · Seed engine (the demo's screenwriter)

Deterministic (fixed PRNG seed) so every run tells the identical story. `crates/seed` generates:

- 8 territories across the regions; 12 reps, 3 regional managers, 1 VP, 1 admin (login table printed to console on seed; password `demo-plenum-2026` hashed).
- ~48 accounts (some with 2–4 sites, some corporate parents), industry-distributed like the real book (metals-heavy, pharma high-value, grain seasonal).
- Product catalog: ~10 capital SKUs across the real families, ~20 cartridge SKUs (incl. Stat-Safe premium and replacement-brand SKUs with competitor `filter_fits`), parts, service.
- ~220 installed units, 2010–2026 commissioning; ~15% competitor-brand (conquest fuel).
- 3.5 years of orders: capital lumps (long gaps, big tickets, higher discounts), consumable cadences per unit (qty ≈ cartridge_count, every `expected_changeout_months` ± jitter), grain/feed seasonality (Q3 bump), pharma premium pricing, realistic discount distributions by family.
- **Story beats seeded on purpose** (the demo script depends on these):
  1. A named account (call it *Ridgeline Grain*, SE-1) whose cartridge cadence went silent 11 months ago → the defection alarm the demo opens on.
  2. One rep with league-leading gross and league-worst leakage% → the leaderboard's teachable moment (basis toggle flips their rank).
  3. A Mountain-territory pharma prospect with three competitor units → the conquest card.
  4. A pending quote at 28% discount sitting in the VP approval inbox → the governance walkthrough.
  5. Deliberate mess: a handful of duplicate-ish account names, two units missing `expected_changeout_months`, one order with a 100% discount (data-quality panel finds them — mess is information, and showing we expect it is FDA literacy on display).

## 10 · Build order for CC (FDA layers → phases; each gated by acceptance)

Standing instruction to CC: *work the phases in order; do not start a phase until the prior phase's checks are stated as passing; report completion by walking through the checks and stating what D. will observe — never on internal tests alone.*

- **P0 — Foundation.** Repo scaffold, Docker compose (postgres), migrations (full schema + RLS policies + triggers), seed engine, auth, RLS session middleware. *No UI.*
- **P1 — Metrics core.** `v_order_facts`, materialized rollups + refresh, all 7 metric endpoint groups, dual-basis everywhere, pagination.
- **P2 — Command + Leaderboards UI.** Login, app shell, Territory Board, Command screen, Leaderboards with period/basis/kind controls, CSV export. Responsive tripwire lands here.
- **P3 — CRM operational core.** Account 360 (with installed-base timeline), Pipeline kanban, Quote builder + approval state machine + audit log, Activities.
- **P4 — Signals + AI.** Deterministic signal generators (reorder/defection/conquest/discount-anomaly) + queue UI with write-back actions; Ask PLENUM + discount recommender behind flags; telemetry ingest stub (stretch).
- **P5 — Polish + demo hardening.** Leakage screen, data-quality panel, empty/error/loading states pass, seed console login table, README with run instructions + demo script.

## 11 · Acceptance (no-proof-no-run; the phase gates, condensed)

Build is NOT done until every check passes under D.'s own hands.

```
□ P0-1  docker compose up + cargo run --bin seed → console prints login
        table; psql count of orders > 15,000.
        FAIL: errors, or empty tables.
□ P0-2  Login as rep SE-1 via curl/UI → GET /api/accounts returns only
        SE-1 accounts; as VP → all territories present.
        FAIL: rep sees foreign accounts (RLS breach — stop everything).
□ P1-1  GET /metrics/customers?period=2025&basis=gross vs basis=net →
        same accounts, different order somewhere in top 10; every row
        shows gross ≥ net.
□ P1-2  /metrics/territories cumulative: Σ territory net == Σ order_lines
        net (one psql cross-check command provided in README).
□ P2-1  Command screen: gross/net toggle visibly re-ranks the Territory
        Board and changes every KPI at once.
□ P2-2  Playwright responsive tripwire passes at all five widths; manual:
        iPad portrait + landscape, no page-level horizontal movement.
□ P3-1  Draft a quote at 28% discount as a rep → submit → status
        pending_approval; approve as VP → audit_log row visible in UI.
□ P3-2  Stage-drag an opportunity to Won → account 360 and rep
        leaderboard reflect it after rollup refresh.
□ P4-1  Signals queue shows Ridgeline Grain defection card with reasons
        (last order date, cadence, value); Draft Quote fills the correct
        cartridge SKU and qty.
□ P4-2  With ANTHROPIC_API_KEY unset: Ask PLENUM serves saved questions,
        no screen errors. With key: "top 10 customers by net revenue in
        2025" returns table + chart + SQL; as a rep, asking for another
        territory returns only own-scope rows.
□ P5-1  Fresh clone → README steps → running app in ≤ 3 commands.
```

## 12 · Anti-goals (CC: do not wander here)

Email/calendar sync · marketing automation · payments · mobile app · SSO/OAuth (session auth only) · multi-currency · configurable fiscal calendars (calendar quarters, noted as configurable-later) · real Camfil data or logos (name product families nominatively; no logo assets) · geographic map data.

## 13 · Demo script (7 minutes, for the interview)

1. **Open on the Signals queue** — Ridgeline Grain defection card. "This account went quiet 11 months ago; at their cadence that's ~$34k/yr of cartridges now going to a will-fit competitor. PLENUM caught it, showed its math, and drafted the win-back quote." *(Theses 1, 3, 4, 5 in 60 seconds.)*
2. Command screen — Territory Board; flip **gross → net**: watch rankings move. "Your best rep on volume is your worst on margin. Traditional CRMs can't show you this because they store one revenue number."
3. Leaderboards — customers per quarter / year / cumulative, both bases (the requirement list, live).
4. Account 360 — installed-base timeline: "every unit is an annuity with a due date."
5. Quote at 28% → approval flow → audit trail. Governance, not vibes.
6. Ask PLENUM — one live NL question, show the SQL receipts.
7. Close on the platform story: Artifact 1's collector pushing filter-life into this queue. "Sensor to ledger. That's the solutions-architect job, and this is what it looks like."

## 14 · Open items / not locked

- Name (PLENUM working). · Discount thresholds (10/25 defaults). · Telemetry ingest = stretch, cut first if P4 runs long. · Whether Ask PLENUM demos live or flagged-off in the room (network risk — decide at rehearsal; saved-question fallback is the safety).
- **Gate:** devil's-case pass on this spec, then D. says **lock/go**, then CC gets P0.
