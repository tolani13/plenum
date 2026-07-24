# Territory realignment — forward prep (recorded 2026-07-23, T1)

T1 ships PLANNING-VIEW editing only: the map's geography config moves;
the book of business does not. This file records the agreed path for the
day an executive says "make it real," so none of T1's choices paint us
into a corner.

## 1 · Commit-realignment unit (future, Tier 3)

- Semantics: applying a map change moves the BOOK — accounts (and their
  sites/opportunities/quotes in flight) re-home to the new territory;
  ORDER HISTORY KEEPS its original territory_id (history is a ledger;
  rewriting it would corrupt every frozen anchor and audit trail).
  Reporting that wants "as-if" history reads geography, not orders.
- Mechanics: effective-dated assignment (territory_id + effective ranges
  or an assignment-events table), applied in one transaction per state
  move; RLS scope shifts the moment account rows re-home — which is
  exactly why this is vp/admin + probably a two-step propose/approve flow
  with the same audit discipline as quote approvals.
- Blast surface to test when built: RLS (rep gains/loses accounts),
  mv_* rollups, leaderboards, signals ownership, in-flight quotes and
  approvals, seed determinism.
- T1 already provides: the write-gated endpoints pattern, the audit
  plumbing on both territory tables, and a map UI that can render any
  assignment the config declares.

## 2 · Sub-state splits (TX / CA / FL and friends)

- Generalize territory_states to territory_regions: (region_type,
  region_code) → territory_code, where region_type ∈ state | county |
  zip3 | zip5. A state-level row is the degenerate case, so migration is
  additive: today's rows become region_type='state'.
- Attribution today joins sites.state; a split state joins the finest
  region_type present for that state (county/zip from the site address).
  Map rendering needs county/zip boundary geometry only for split states
  (lazy-loaded per state, same committed-asset discipline as the US SVG).
- Rule to preserve: exactly one owning territory per (finest) region — no
  overlaps; the PK carries this the same way state_code does today.

## 3 · Non-state geographies (other companies, re-skin path)

- Some businesses split by named accounts, verticals, or postal bands,
  not geography. The config-table pattern holds: an assignment table maps
  a DIMENSION (account list / industry / postal range) to a territory;
  the map view is then one optional projection among several. PLENUM's
  architecture keeps that swap contained to config + one screen.

## 4 · Canada (v2 of map editing)

- Blocks CA-E/CA-W are schematic shapes whose provinces ride along in
  territory_states. Editing rule when built: a block and its province
  rows move ATOMICALLY (one transaction), or province-level editing
  arrives with province shapes. T1 locks both to avoid a half-moved
  block diverging colors from dollars.
