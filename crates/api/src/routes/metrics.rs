//! GET /api/metrics/* — the seven metric endpoint groups (spec §5, §10).
//!
//! Conventions (mirroring P0's accounts route):
//!   · every handler runs its reads inside rls_tx — scope comes from
//!     Postgres (RLS on the facts views, the v_user_scope predicate inside
//!     the scoped rollup views), NEVER from handler logic;
//!   · envelope { items, limit, offset, total }, MAX_LIMIT 200;
//!   · missing/garbage params -> typed 422; empty result -> 200, never an
//!     error;
//!   · every query is static and sqlx-compile-checked — basis/kind/by
//!     variation rides on bind-parameter CASE expressions and null-folded
//!     filters, no SQL is ever built from strings.
//!
//! Period resolution (architect-locked): whole calendar quarters and years
//! read the scoped rollup views (v_*_period = matview history + live current
//! quarter); cumulative and ttm aggregate v_order_facts directly (they need
//! day precision). kind=capital|consumable also reads v_order_facts live:
//! the rollups are keyed (entity, territory_id, quarter_start) with no kind
//! axis, and distinct order counts do not survive a per-kind split — the
//! facts view is the ground truth either way (the rollup-vs-live equivalence
//! check in the P1 report proves the two paths agree).
//!
//! Money is BIGINT cents end to end; ratios/percentages are display numbers
//! (f64, 2 decimals), computed from the integer sums.

use axum::extract::{Query, State};
use axum::Json;
use chrono::NaiveDate;
use domain::{Basis, KindFilter, Period};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::parse_page;
use crate::state::AppState;

// ── shared param plumbing ───────────────────────────────────────────────────
// P5 (R9b): the P0-era local copies of parse_int/parse_page moved to
// routes/common.rs when P3 landed; this module now uses the shared helpers —
// identical grammar (limit default 50, max 200; offset >= 0), identical 422
// messages, proven by the untouched P1 test matrix.

fn parse_period(value: Option<String>) -> Result<Period, ApiError> {
    let raw = value.ok_or_else(|| {
        ApiError::Invalid(
            "period is required (a year like 2025, a quarter like 2025-Q2, 'cumulative', or 'ttm')"
                .into(),
        )
    })?;
    Period::parse(&raw).map_err(ApiError::Invalid)
}

fn parse_basis(value: Option<String>) -> Result<Basis, ApiError> {
    let raw =
        value.ok_or_else(|| ApiError::Invalid("basis is required ('gross' or 'net')".into()))?;
    Basis::parse(&raw).map_err(ApiError::Invalid)
}

fn parse_kind(value: Option<String>) -> Result<KindFilter, ApiError> {
    match value {
        None => Ok(KindFilter::All),
        Some(raw) => KindFilter::parse(&raw).map_err(ApiError::Invalid),
    }
}

/// Metrics 6–7 are defined without a period/basis axis; a caller passing one
/// gets told exactly that instead of a silently ignored parameter.
fn reject_param(value: &Option<String>, message: &str) -> Result<(), ApiError> {
    if value.is_some() {
        return Err(ApiError::Invalid(message.into()));
    }
    Ok(())
}

/// Bind values for the live v_order_facts filter:
///   ($1::date IS NULL OR ordered_on >= $1)
///   AND ($2::date IS NULL OR ordered_on < $2)
///   AND (NOT $3::boolean OR ordered_on >= CURRENT_DATE - interval '12 months')
/// ttm stays in SQL because "today" must be the database's today.
fn live_bounds(period: Period) -> (Option<NaiveDate>, Option<NaiveDate>, bool) {
    match period.date_range() {
        Some((start, end)) => (Some(start), Some(end), false),
        None => (None, None, matches!(period, Period::Ttm)),
    }
}

/// True when this request reads the scoped rollup views; false -> live
/// v_order_facts aggregation (cumulative/ttm, or any kind-filtered slice).
fn rollup_path(period: Period, kind: KindFilter) -> bool {
    period.uses_rollups() && kind == KindFilter::All
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// percentage numer/denom, 2 decimals; None when the denominator is zero
/// (spec: leakage_pct is null when gross = 0 — never a division error).
fn pct_of(numer: i64, denom: i64) -> Option<f64> {
    if denom == 0 {
        None
    } else {
        Some(round2(numer as f64 / denom as f64 * 100.0))
    }
}

/// net / prorated quota (quarter = /4; year & ttm = full annual quota;
/// cumulative = None — undefined across multi-year history).
fn quota_attainment(net_cents: i64, quota_year_cents: i64, period: Period) -> Option<f64> {
    let divisor = period.quota_divisor()?;
    if quota_year_cents == 0 {
        return None;
    }
    Some(round2(
        net_cents as f64 / (quota_year_cents as f64 / divisor as f64) * 100.0,
    ))
}

#[derive(Serialize)]
pub struct Page<T> {
    items: Vec<T>,
    limit: i64,
    offset: i64,
    total: i64,
}

// ═══ 1 · GET /api/metrics/territories ══════════════════════════════════════

#[derive(Deserialize)]
pub struct TerritoriesParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct TerritoryRow {
    territory_code: String,
    territory_name: String,
    gross_cents: i64,
    net_cents: i64,
    leakage_cents: i64,
    leakage_pct: Option<f64>,
    order_count: i64,
    active_accounts: i64,
    quota_attainment_pct: Option<f64>,
}

struct TerritoryAgg {
    territory_code: String,
    territory_name: String,
    quota_year_cents: i64,
    gross_cents: i64,
    net_cents: i64,
    order_count: i64,
    active_accounts: i64,
}

pub async fn territories(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<TerritoriesParams>,
) -> Result<Json<Page<TerritoryRow>>, ApiError> {
    let period = parse_period(params.period)?;
    let basis = parse_basis(params.basis)?;
    let kind = parse_kind(params.kind)?;
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    let (rows, total) = if rollup_path(period, kind) {
        let (start, end) = period.date_range().expect("rollup periods have a range");
        // Money and order counts come from the scoped rollup view;
        // active_accounts is a DISTINCT over accounts, which does not
        // survive pre-aggregation across quarters, so it joins in live from
        // v_order_facts (RLS-scoped) over the same date range.
        let rows = sqlx::query_as!(
            TerritoryAgg,
            r#"SELECT r.territory_code AS "territory_code!",
                      r.territory_name AS "territory_name!",
                      r.quota_year_cents AS "quota_year_cents!",
                      r.gross_cents AS "gross_cents!",
                      r.net_cents AS "net_cents!",
                      r.order_count AS "order_count!",
                      act.active_accounts AS "active_accounts!"
               FROM (
                   SELECT territory_id, territory_code, territory_name,
                          quota_year_cents,
                          SUM(gross_cents)::bigint AS gross_cents,
                          SUM(net_cents)::bigint AS net_cents,
                          SUM(order_count)::bigint AS order_count
                   FROM v_territory_period
                   WHERE quarter_start >= $1 AND quarter_start < $2
                   GROUP BY territory_id, territory_code, territory_name,
                            quota_year_cents
               ) r
               JOIN (
                   SELECT territory_id,
                          count(DISTINCT account_id) AS active_accounts
                   FROM v_order_facts
                   WHERE ordered_on >= $1 AND ordered_on < $2
                   GROUP BY territory_id
               ) act ON act.territory_id = r.territory_id
               ORDER BY CASE WHEN $3 = 'gross' THEN r.gross_cents
                             ELSE r.net_cents END DESC,
                        r.territory_code
               LIMIT $4 OFFSET $5"#,
            start,
            end,
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT territory_id) AS "count!"
               FROM v_territory_period
               WHERE quarter_start >= $1 AND quarter_start < $2"#,
            start,
            end
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    } else {
        let (start, end, ttm) = live_bounds(period);
        let rows = sqlx::query_as!(
            TerritoryAgg,
            r#"SELECT territory_code AS "territory_code!",
                      territory_name AS "territory_name!",
                      max(quota_year_cents) AS "quota_year_cents!",
                      SUM(gross_cents)::bigint AS "gross_cents!",
                      SUM(net_cents)::bigint AS "net_cents!",
                      count(DISTINCT order_id) AS "order_count!",
                      count(DISTINCT account_id) AS "active_accounts!"
               FROM v_order_facts
               WHERE ($1::date IS NULL OR ordered_on >= $1)
                 AND ($2::date IS NULL OR ordered_on < $2)
                 AND (NOT $3::boolean
                      OR ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR kind::text = $4)
               GROUP BY territory_id, territory_code, territory_name
               ORDER BY CASE WHEN $5 = 'gross' THEN SUM(gross_cents)
                             ELSE SUM(net_cents) END DESC,
                        territory_code
               LIMIT $6 OFFSET $7"#,
            start,
            end,
            ttm,
            kind.as_str(),
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT territory_id) AS "count!"
               FROM v_order_facts
               WHERE ($1::date IS NULL OR ordered_on >= $1)
                 AND ($2::date IS NULL OR ordered_on < $2)
                 AND (NOT $3::boolean
                      OR ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR kind::text = $4)"#,
            start,
            end,
            ttm,
            kind.as_str()
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    };

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| TerritoryRow {
            leakage_cents: r.gross_cents - r.net_cents,
            leakage_pct: pct_of(r.gross_cents - r.net_cents, r.gross_cents),
            quota_attainment_pct: quota_attainment(r.net_cents, r.quota_year_cents, period),
            territory_code: r.territory_code,
            territory_name: r.territory_name,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
            order_count: r.order_count,
            active_accounts: r.active_accounts,
        })
        .collect();

    Ok(Json(Page {
        items,
        limit,
        offset,
        total,
    }))
}

// ═══ 2 · GET /api/metrics/leaderboard ══════════════════════════════════════

#[derive(Deserialize)]
pub struct LeaderboardParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct LeaderboardRow {
    rep_name: String,
    gross_cents: i64,
    net_cents: i64,
    leakage_pct: Option<f64>,
    capital_gross_cents: i64,
    capital_net_cents: i64,
    consumable_gross_cents: i64,
    consumable_net_cents: i64,
    top_account_name: Option<String>,
}

struct RepAgg {
    rep_name: String,
    gross_cents: i64,
    net_cents: i64,
    capital_gross_cents: i64,
    capital_net_cents: i64,
    consumable_gross_cents: i64,
    consumable_net_cents: i64,
    top_account_name: Option<String>,
}

pub async fn leaderboard(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<LeaderboardParams>,
) -> Result<Json<Page<LeaderboardRow>>, ApiError> {
    let period = parse_period(params.period)?;
    let basis = parse_basis(params.basis)?;
    let kind = parse_kind(params.kind)?;
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Ranking: chosen basis DESC; ties broken by leakage_pct ASC — margin
    // discipline wins the tie (spec §5.2). top_account_name is the rep's
    // best account by the SAME basis over the SAME period, resolved live
    // from v_order_facts (RLS-scoped) in a per-rep lateral.
    let (rows, total) = if rollup_path(period, kind) {
        let (start, end) = period.date_range().expect("rollup periods have a range");
        let rows = sqlx::query_as!(
            RepAgg,
            r#"SELECT r.rep_name AS "rep_name!",
                      r.gross_cents AS "gross_cents!",
                      r.net_cents AS "net_cents!",
                      r.capital_gross_cents AS "capital_gross_cents!",
                      r.capital_net_cents AS "capital_net_cents!",
                      r.consumable_gross_cents AS "consumable_gross_cents!",
                      r.consumable_net_cents AS "consumable_net_cents!",
                      top.account_name AS "top_account_name?"
               FROM (
                   SELECT rep_id, rep_name,
                          SUM(gross_cents)::bigint AS gross_cents,
                          SUM(net_cents)::bigint AS net_cents,
                          SUM(capital_gross_cents)::bigint AS capital_gross_cents,
                          SUM(capital_net_cents)::bigint AS capital_net_cents,
                          SUM(consumable_gross_cents)::bigint AS consumable_gross_cents,
                          SUM(consumable_net_cents)::bigint AS consumable_net_cents
                   FROM v_rep_period
                   WHERE quarter_start >= $1 AND quarter_start < $2
                   GROUP BY rep_id, rep_name
               ) r
               LEFT JOIN LATERAL (
                   SELECT f.account_name
                   FROM v_order_facts f
                   WHERE f.rep_id = r.rep_id
                     AND f.ordered_on >= $1 AND f.ordered_on < $2
                   GROUP BY f.account_id, f.account_name
                   ORDER BY CASE WHEN $3 = 'gross' THEN SUM(f.gross_cents)
                                 ELSE SUM(f.net_cents) END DESC,
                            f.account_name
                   LIMIT 1
               ) top ON true
               ORDER BY CASE WHEN $3 = 'gross' THEN r.gross_cents
                             ELSE r.net_cents END DESC,
                        (r.gross_cents - r.net_cents)::numeric
                            / NULLIF(r.gross_cents, 0) ASC NULLS LAST,
                        r.rep_name
               LIMIT $4 OFFSET $5"#,
            start,
            end,
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT rep_id) AS "count!"
               FROM v_rep_period
               WHERE quarter_start >= $1 AND quarter_start < $2"#,
            start,
            end
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    } else {
        let (start, end, ttm) = live_bounds(period);
        let rows = sqlx::query_as!(
            RepAgg,
            r#"SELECT r.rep_name AS "rep_name!",
                      r.gross_cents AS "gross_cents!",
                      r.net_cents AS "net_cents!",
                      r.capital_gross_cents AS "capital_gross_cents!",
                      r.capital_net_cents AS "capital_net_cents!",
                      r.consumable_gross_cents AS "consumable_gross_cents!",
                      r.consumable_net_cents AS "consumable_net_cents!",
                      top.account_name AS "top_account_name?"
               FROM (
                   SELECT rep_id, rep_name,
                          SUM(gross_cents)::bigint AS gross_cents,
                          SUM(net_cents)::bigint AS net_cents,
                          COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint AS capital_gross_cents,
                          COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint AS capital_net_cents,
                          COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_gross_cents,
                          COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint AS consumable_net_cents
                   FROM v_order_facts
                   WHERE ($1::date IS NULL OR ordered_on >= $1)
                     AND ($2::date IS NULL OR ordered_on < $2)
                     AND (NOT $3::boolean
                          OR ordered_on >= CURRENT_DATE - interval '12 months')
                     AND ($4::text = 'all' OR kind::text = $4)
                   GROUP BY rep_id, rep_name
               ) r
               LEFT JOIN LATERAL (
                   SELECT f.account_name
                   FROM v_order_facts f
                   WHERE f.rep_id = r.rep_id
                     AND ($1::date IS NULL OR f.ordered_on >= $1)
                     AND ($2::date IS NULL OR f.ordered_on < $2)
                     AND (NOT $3::boolean
                          OR f.ordered_on >= CURRENT_DATE - interval '12 months')
                     AND ($4::text = 'all' OR f.kind::text = $4)
                   GROUP BY f.account_id, f.account_name
                   ORDER BY CASE WHEN $5 = 'gross' THEN SUM(f.gross_cents)
                                 ELSE SUM(f.net_cents) END DESC,
                            f.account_name
                   LIMIT 1
               ) top ON true
               ORDER BY CASE WHEN $5 = 'gross' THEN r.gross_cents
                             ELSE r.net_cents END DESC,
                        (r.gross_cents - r.net_cents)::numeric
                            / NULLIF(r.gross_cents, 0) ASC NULLS LAST,
                        r.rep_name
               LIMIT $6 OFFSET $7"#,
            start,
            end,
            ttm,
            kind.as_str(),
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT rep_id) AS "count!"
               FROM v_order_facts
               WHERE ($1::date IS NULL OR ordered_on >= $1)
                 AND ($2::date IS NULL OR ordered_on < $2)
                 AND (NOT $3::boolean
                      OR ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR kind::text = $4)"#,
            start,
            end,
            ttm,
            kind.as_str()
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    };

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| LeaderboardRow {
            leakage_pct: pct_of(r.gross_cents - r.net_cents, r.gross_cents),
            rep_name: r.rep_name,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
            capital_gross_cents: r.capital_gross_cents,
            capital_net_cents: r.capital_net_cents,
            consumable_gross_cents: r.consumable_gross_cents,
            consumable_net_cents: r.consumable_net_cents,
            top_account_name: r.top_account_name,
        })
        .collect();

    Ok(Json(Page {
        items,
        limit,
        offset,
        total,
    }))
}

// ═══ 3 · GET /api/metrics/items ════════════════════════════════════════════

#[derive(Deserialize)]
pub struct ItemsParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    group: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct ItemRow {
    product_sku: Option<String>,
    product_name: String,
    family: String,
    kind: String,
    units: i64,
    gross_cents: i64,
    net_cents: i64,
    /// Consumable rows only: share of caller-visible installed units of the
    /// families this product serves (filter_fits) that ordered it in the
    /// period. Null for capital/part/service rows.
    attach_rate_pct: Option<f64>,
}

struct ItemAgg {
    product_sku: Option<String>,
    product_name: String,
    family: String,
    kind: String,
    units: i64,
    gross_cents: i64,
    net_cents: i64,
    attach_numer: i64,
    attach_denom: i64,
}

fn item_row(r: ItemAgg) -> ItemRow {
    let attach_rate_pct = if r.kind == "consumable" {
        pct_of(r.attach_numer, r.attach_denom)
    } else {
        None
    };
    ItemRow {
        product_sku: r.product_sku,
        product_name: r.product_name,
        family: r.family,
        kind: r.kind,
        units: r.units,
        gross_cents: r.gross_cents,
        net_cents: r.net_cents,
        attach_rate_pct,
    }
}

pub async fn items(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<ItemsParams>,
) -> Result<Json<Page<ItemRow>>, ApiError> {
    let period = parse_period(params.period)?;
    let basis = parse_basis(params.basis)?;
    let kind = parse_kind(params.kind)?;
    let group_by_family = match params.group.as_deref().map(str::trim) {
        None => false,
        Some(g) if g.eq_ignore_ascii_case("product") => false,
        Some(g) if g.eq_ignore_ascii_case("family") => true,
        Some(_) => {
            return Err(ApiError::Invalid(
                "group must be 'product' or 'family'".into(),
            ))
        }
    };
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Attach rate (consumable rows): numerator = distinct caller-visible
    // installed units of the served families with >= 1 order line of this
    // product (this family, in the family grouping) in the period, linked
    // through the unit's SITE (the tightest join the schema offers — orders
    // carry site_id, units live at sites); denominator = all such units.
    // Both sides read v_unit_facts / v_order_facts, so RLS scoping is
    // inherited, and the empty-scope case yields 0/0 -> null, not an error.
    //
    // ── D-1 (2026-07-25): the attach numbers are computed SET-WISE ──────────
    // These four queries used to hang the attach math off a
    // `LEFT JOIN LATERAL (... WHERE EXISTS (SELECT 1 FROM v_order_facts ...))`
    // evaluated once per result row. Two costs, both measured:
    //   · the real per-row work — one v_unit_facts scan per consumable row
    //     (v_unit_facts is itself a lateral over order_lines, so this
    //     re-derived the unit's last-paid price 16× over) plus a correlated
    //     EXISTS per unit: 213k–687k buffer touches per request;
    //   · the one that actually dominated the clock — the LATERAL drove the
    //     planner's cost estimate to 0.69M–12.05M, far past
    //     jit_optimize_above_cost / jit_inline_above_cost (500 000), so every
    //     single request LLVM-compiled ~460–560 functions with full inlining
    //     and optimization BEFORE executing. That compilation was 83–97 % of
    //     the wall clock (e.g. 1519 ms of 1794 ms locally; 34–39 s on the
    //     free-tier instance), and three concurrent requests exhausted the
    //     free Postgres and got their backends killed — the typed 500 behind
    //     D-2.
    // The rewrite reads v_unit_facts ONCE and v_order_facts ONCE and joins
    // them set-wise. Scoping is unchanged and inherited exactly as before:
    // both relations are still the same security_invoker facts views read
    // inside the same rls_tx, so the 0005 policies apply to every row; the
    // empty-scope case still produces no attach row at all -> COALESCE 0 ->
    // 0/0 -> null (item_row), never an error and never everything.
    // The attach CTEs are restricted to the requested PAGE, which is legal
    // because the ORDER BY never reads an attach column.
    let (rows, total) = if rollup_path(period, kind) {
        let (start, end) = period.date_range().expect("rollup periods have a range");
        if group_by_family {
            let rows = sqlx::query_as!(
                ItemAgg,
                r#"WITH r AS (
                       SELECT family,
                              CASE WHEN count(DISTINCT kind) = 1
                                   THEN min(kind::text) ELSE 'mixed' END AS kind,
                              SUM(units)::bigint AS units,
                              SUM(gross_cents)::bigint AS gross_cents,
                              SUM(net_cents)::bigint AS net_cents
                       FROM v_product_period
                       WHERE quarter_start >= $1 AND quarter_start < $2
                       GROUP BY family
                   ),
                   page AS (
                       SELECT r.* FROM r
                       ORDER BY CASE WHEN $3 = 'gross' THEN r.gross_cents
                                     ELSE r.net_cents END DESC,
                                r.family
                       LIMIT $4 OFFSET $5
                   ),
                   fits AS (
                       SELECT DISTINCT pg.family, ff.unit_family
                       FROM page pg
                       JOIN products p2 ON p2.family = pg.family
                       CROSS JOIN LATERAL unnest(p2.filter_fits) AS ff(unit_family)
                       WHERE pg.kind = 'consumable'
                   ),
                   served AS (
                       SELECT DISTINCT f.site_id, f.family
                       FROM v_order_facts f
                       WHERE f.ordered_on >= $1
                         AND f.ordered_on < $2
                         AND f.family IN (SELECT family FROM page
                                          WHERE kind = 'consumable')
                   ),
                   att AS (
                       SELECT fits.family,
                              count(DISTINCT u.unit_id) AS denom,
                              count(DISTINCT u.unit_id)
                                  FILTER (WHERE s.site_id IS NOT NULL) AS numer
                       FROM fits
                       JOIN v_unit_facts u ON u.unit_family = fits.unit_family
                       LEFT JOIN served s ON s.site_id = u.site_id
                                         AND s.family = fits.family
                       GROUP BY fits.family
                   )
                   SELECT NULL::text AS "product_sku?",
                          pg.family AS "product_name!",
                          pg.family AS "family!",
                          pg.kind AS "kind!",
                          pg.units AS "units!",
                          pg.gross_cents AS "gross_cents!",
                          pg.net_cents AS "net_cents!",
                          COALESCE(att.numer, 0) AS "attach_numer!",
                          COALESCE(att.denom, 0) AS "attach_denom!"
                   FROM page pg
                   LEFT JOIN att ON att.family = pg.family
                   ORDER BY CASE WHEN $3 = 'gross' THEN pg.gross_cents
                                 ELSE pg.net_cents END DESC,
                            pg.family"#,
                start,
                end,
                basis.as_str(),
                limit,
                offset
            )
            .fetch_all(&mut *tx)
            .await?;
            let total: i64 = sqlx::query_scalar!(
                r#"SELECT count(DISTINCT family) AS "count!"
                   FROM v_product_period
                   WHERE quarter_start >= $1 AND quarter_start < $2"#,
                start,
                end
            )
            .fetch_one(&mut *tx)
            .await?;
            (rows, total)
        } else {
            let rows = sqlx::query_as!(
                ItemAgg,
                r#"WITH r AS (
                       SELECT product_id, product_sku, product_name, family,
                              kind,
                              SUM(units)::bigint AS units,
                              SUM(gross_cents)::bigint AS gross_cents,
                              SUM(net_cents)::bigint AS net_cents
                       FROM v_product_period
                       WHERE quarter_start >= $1 AND quarter_start < $2
                       GROUP BY product_id, product_sku, product_name, family,
                                kind
                   ),
                   page AS (
                       SELECT r.* FROM r
                       JOIN products p ON p.id = r.product_id
                       ORDER BY CASE WHEN $3 = 'gross' THEN r.gross_cents
                                     ELSE r.net_cents END DESC,
                                r.product_sku
                       LIMIT $4 OFFSET $5
                   ),
                   fits AS (
                       SELECT DISTINCT pg.product_id, ff.unit_family
                       FROM page pg
                       JOIN products p ON p.id = pg.product_id
                       CROSS JOIN LATERAL unnest(p.filter_fits) AS ff(unit_family)
                       WHERE pg.kind = 'consumable'
                   ),
                   served AS (
                       SELECT DISTINCT f.site_id, f.product_id
                       FROM v_order_facts f
                       WHERE f.ordered_on >= $1
                         AND f.ordered_on < $2
                         AND f.product_id IN (SELECT product_id FROM page
                                              WHERE kind = 'consumable')
                   ),
                   att AS (
                       SELECT fits.product_id,
                              count(DISTINCT u.unit_id) AS denom,
                              count(DISTINCT u.unit_id)
                                  FILTER (WHERE s.site_id IS NOT NULL) AS numer
                       FROM fits
                       JOIN v_unit_facts u ON u.unit_family = fits.unit_family
                       LEFT JOIN served s ON s.site_id = u.site_id
                                         AND s.product_id = fits.product_id
                       GROUP BY fits.product_id
                   )
                   SELECT pg.product_sku AS "product_sku?",
                          pg.product_name AS "product_name!",
                          pg.family AS "family!",
                          pg.kind::text AS "kind!",
                          pg.units AS "units!",
                          pg.gross_cents AS "gross_cents!",
                          pg.net_cents AS "net_cents!",
                          COALESCE(att.numer, 0) AS "attach_numer!",
                          COALESCE(att.denom, 0) AS "attach_denom!"
                   FROM page pg
                   LEFT JOIN att ON att.product_id = pg.product_id
                   ORDER BY CASE WHEN $3 = 'gross' THEN pg.gross_cents
                                 ELSE pg.net_cents END DESC,
                            pg.product_sku"#,
                start,
                end,
                basis.as_str(),
                limit,
                offset
            )
            .fetch_all(&mut *tx)
            .await?;
            let total: i64 = sqlx::query_scalar!(
                r#"SELECT count(DISTINCT product_id) AS "count!"
                   FROM v_product_period
                   WHERE quarter_start >= $1 AND quarter_start < $2"#,
                start,
                end
            )
            .fetch_one(&mut *tx)
            .await?;
            (rows, total)
        }
    } else {
        let (start, end, ttm) = live_bounds(period);
        if group_by_family {
            let rows = sqlx::query_as!(
                ItemAgg,
                r#"WITH r AS (
                       SELECT family,
                              CASE WHEN count(DISTINCT kind) = 1
                                   THEN min(kind::text) ELSE 'mixed' END AS kind,
                              SUM(qty)::bigint AS units,
                              SUM(gross_cents)::bigint AS gross_cents,
                              SUM(net_cents)::bigint AS net_cents
                       FROM v_order_facts
                       WHERE ($1::date IS NULL OR ordered_on >= $1)
                         AND ($2::date IS NULL OR ordered_on < $2)
                         AND (NOT $3::boolean
                              OR ordered_on >= CURRENT_DATE - interval '12 months')
                         AND ($4::text = 'all' OR kind::text = $4)
                       GROUP BY family
                   ),
                   page AS (
                       SELECT r.* FROM r
                       ORDER BY CASE WHEN $5 = 'gross' THEN r.gross_cents
                                     ELSE r.net_cents END DESC,
                                r.family
                       LIMIT $6 OFFSET $7
                   ),
                   fits AS (
                       SELECT DISTINCT pg.family, ff.unit_family
                       FROM page pg
                       JOIN products p2 ON p2.family = pg.family
                       CROSS JOIN LATERAL unnest(p2.filter_fits) AS ff(unit_family)
                       WHERE pg.kind = 'consumable'
                   ),
                   served AS (
                       SELECT DISTINCT f.site_id, f.family
                       FROM v_order_facts f
                       WHERE ($1::date IS NULL OR f.ordered_on >= $1)
                         AND ($2::date IS NULL OR f.ordered_on < $2)
                         AND (NOT $3::boolean
                              OR f.ordered_on >= CURRENT_DATE - interval '12 months')
                         AND f.family IN (SELECT family FROM page
                                          WHERE kind = 'consumable')
                   ),
                   att AS (
                       SELECT fits.family,
                              count(DISTINCT u.unit_id) AS denom,
                              count(DISTINCT u.unit_id)
                                  FILTER (WHERE s.site_id IS NOT NULL) AS numer
                       FROM fits
                       JOIN v_unit_facts u ON u.unit_family = fits.unit_family
                       LEFT JOIN served s ON s.site_id = u.site_id
                                         AND s.family = fits.family
                       GROUP BY fits.family
                   )
                   SELECT NULL::text AS "product_sku?",
                          pg.family AS "product_name!",
                          pg.family AS "family!",
                          pg.kind AS "kind!",
                          pg.units AS "units!",
                          pg.gross_cents AS "gross_cents!",
                          pg.net_cents AS "net_cents!",
                          COALESCE(att.numer, 0) AS "attach_numer!",
                          COALESCE(att.denom, 0) AS "attach_denom!"
                   FROM page pg
                   LEFT JOIN att ON att.family = pg.family
                   ORDER BY CASE WHEN $5 = 'gross' THEN pg.gross_cents
                                 ELSE pg.net_cents END DESC,
                            pg.family"#,
                start,
                end,
                ttm,
                kind.as_str(),
                basis.as_str(),
                limit,
                offset
            )
            .fetch_all(&mut *tx)
            .await?;
            let total: i64 = sqlx::query_scalar!(
                r#"SELECT count(DISTINCT family) AS "count!"
                   FROM v_order_facts
                   WHERE ($1::date IS NULL OR ordered_on >= $1)
                     AND ($2::date IS NULL OR ordered_on < $2)
                     AND (NOT $3::boolean
                          OR ordered_on >= CURRENT_DATE - interval '12 months')
                     AND ($4::text = 'all' OR kind::text = $4)"#,
                start,
                end,
                ttm,
                kind.as_str()
            )
            .fetch_one(&mut *tx)
            .await?;
            (rows, total)
        } else {
            let rows = sqlx::query_as!(
                ItemAgg,
                r#"WITH r AS (
                       SELECT product_id, product_sku, product_name, family,
                              kind,
                              SUM(qty)::bigint AS units,
                              SUM(gross_cents)::bigint AS gross_cents,
                              SUM(net_cents)::bigint AS net_cents
                       FROM v_order_facts
                       WHERE ($1::date IS NULL OR ordered_on >= $1)
                         AND ($2::date IS NULL OR ordered_on < $2)
                         AND (NOT $3::boolean
                              OR ordered_on >= CURRENT_DATE - interval '12 months')
                         AND ($4::text = 'all' OR kind::text = $4)
                       GROUP BY product_id, product_sku, product_name, family,
                                kind
                   ),
                   page AS (
                       SELECT r.* FROM r
                       JOIN products p ON p.id = r.product_id
                       ORDER BY CASE WHEN $5 = 'gross' THEN r.gross_cents
                                     ELSE r.net_cents END DESC,
                                r.product_sku
                       LIMIT $6 OFFSET $7
                   ),
                   fits AS (
                       SELECT DISTINCT pg.product_id, ff.unit_family
                       FROM page pg
                       JOIN products p ON p.id = pg.product_id
                       CROSS JOIN LATERAL unnest(p.filter_fits) AS ff(unit_family)
                       WHERE pg.kind = 'consumable'
                   ),
                   served AS (
                       SELECT DISTINCT f.site_id, f.product_id
                       FROM v_order_facts f
                       WHERE ($1::date IS NULL OR f.ordered_on >= $1)
                         AND ($2::date IS NULL OR f.ordered_on < $2)
                         AND (NOT $3::boolean
                              OR f.ordered_on >= CURRENT_DATE - interval '12 months')
                         AND f.product_id IN (SELECT product_id FROM page
                                              WHERE kind = 'consumable')
                   ),
                   att AS (
                       SELECT fits.product_id,
                              count(DISTINCT u.unit_id) AS denom,
                              count(DISTINCT u.unit_id)
                                  FILTER (WHERE s.site_id IS NOT NULL) AS numer
                       FROM fits
                       JOIN v_unit_facts u ON u.unit_family = fits.unit_family
                       LEFT JOIN served s ON s.site_id = u.site_id
                                         AND s.product_id = fits.product_id
                       GROUP BY fits.product_id
                   )
                   SELECT pg.product_sku AS "product_sku?",
                          pg.product_name AS "product_name!",
                          pg.family AS "family!",
                          pg.kind::text AS "kind!",
                          pg.units AS "units!",
                          pg.gross_cents AS "gross_cents!",
                          pg.net_cents AS "net_cents!",
                          COALESCE(att.numer, 0) AS "attach_numer!",
                          COALESCE(att.denom, 0) AS "attach_denom!"
                   FROM page pg
                   LEFT JOIN att ON att.product_id = pg.product_id
                   ORDER BY CASE WHEN $5 = 'gross' THEN pg.gross_cents
                                 ELSE pg.net_cents END DESC,
                            pg.product_sku"#,
                start,
                end,
                ttm,
                kind.as_str(),
                basis.as_str(),
                limit,
                offset
            )
            .fetch_all(&mut *tx)
            .await?;
            let total: i64 = sqlx::query_scalar!(
                r#"SELECT count(DISTINCT product_id) AS "count!"
                   FROM v_order_facts
                   WHERE ($1::date IS NULL OR ordered_on >= $1)
                     AND ($2::date IS NULL OR ordered_on < $2)
                     AND (NOT $3::boolean
                          OR ordered_on >= CURRENT_DATE - interval '12 months')
                     AND ($4::text = 'all' OR kind::text = $4)"#,
                start,
                end,
                ttm,
                kind.as_str()
            )
            .fetch_one(&mut *tx)
            .await?;
            (rows, total)
        }
    };

    tx.commit().await?;

    Ok(Json(Page {
        items: rows.into_iter().map(item_row).collect(),
        limit,
        offset,
        total,
    }))
}

// ═══ 4 · GET /api/metrics/customers ════════════════════════════════════════

#[derive(Deserialize)]
pub struct CustomersParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct CustomerRow {
    account_name: String,
    gross_cents: i64,
    net_cents: i64,
    leakage_pct: Option<f64>,
    capital_gross_cents: i64,
    capital_net_cents: i64,
    consumable_gross_cents: i64,
    consumable_net_cents: i64,
}

struct CustomerAgg {
    account_name: String,
    gross_cents: i64,
    net_cents: i64,
    capital_gross_cents: i64,
    capital_net_cents: i64,
    consumable_gross_cents: i64,
    consumable_net_cents: i64,
}

pub async fn customers(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<CustomersParams>,
) -> Result<Json<Page<CustomerRow>>, ApiError> {
    let period = parse_period(params.period)?;
    let basis = parse_basis(params.basis)?;
    let kind = parse_kind(params.kind)?;
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    let (rows, total) = if rollup_path(period, kind) {
        let (start, end) = period.date_range().expect("rollup periods have a range");
        let rows = sqlx::query_as!(
            CustomerAgg,
            r#"SELECT account_name AS "account_name!",
                      SUM(gross_cents)::bigint AS "gross_cents!",
                      SUM(net_cents)::bigint AS "net_cents!",
                      SUM(capital_gross_cents)::bigint AS "capital_gross_cents!",
                      SUM(capital_net_cents)::bigint AS "capital_net_cents!",
                      SUM(consumable_gross_cents)::bigint AS "consumable_gross_cents!",
                      SUM(consumable_net_cents)::bigint AS "consumable_net_cents!"
               FROM v_customer_period
               WHERE quarter_start >= $1 AND quarter_start < $2
               GROUP BY account_id, account_name
               ORDER BY CASE WHEN $3 = 'gross' THEN SUM(gross_cents)
                             ELSE SUM(net_cents) END DESC,
                        account_name
               LIMIT $4 OFFSET $5"#,
            start,
            end,
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT account_id) AS "count!"
               FROM v_customer_period
               WHERE quarter_start >= $1 AND quarter_start < $2"#,
            start,
            end
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    } else {
        let (start, end, ttm) = live_bounds(period);
        let rows = sqlx::query_as!(
            CustomerAgg,
            r#"SELECT account_name AS "account_name!",
                      SUM(gross_cents)::bigint AS "gross_cents!",
                      SUM(net_cents)::bigint AS "net_cents!",
                      COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'capital'), 0)::bigint AS "capital_gross_cents!",
                      COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'capital'), 0)::bigint AS "capital_net_cents!",
                      COALESCE(SUM(gross_cents) FILTER (WHERE kind = 'consumable'), 0)::bigint AS "consumable_gross_cents!",
                      COALESCE(SUM(net_cents)   FILTER (WHERE kind = 'consumable'), 0)::bigint AS "consumable_net_cents!"
               FROM v_order_facts
               WHERE ($1::date IS NULL OR ordered_on >= $1)
                 AND ($2::date IS NULL OR ordered_on < $2)
                 AND (NOT $3::boolean
                      OR ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR kind::text = $4)
               GROUP BY account_id, account_name
               ORDER BY CASE WHEN $5 = 'gross' THEN SUM(gross_cents)
                             ELSE SUM(net_cents) END DESC,
                        account_name
               LIMIT $6 OFFSET $7"#,
            start,
            end,
            ttm,
            kind.as_str(),
            basis.as_str(),
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"SELECT count(DISTINCT account_id) AS "count!"
               FROM v_order_facts
               WHERE ($1::date IS NULL OR ordered_on >= $1)
                 AND ($2::date IS NULL OR ordered_on < $2)
                 AND (NOT $3::boolean
                      OR ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR kind::text = $4)"#,
            start,
            end,
            ttm,
            kind.as_str()
        )
        .fetch_one(&mut *tx)
        .await?;
        (rows, total)
    };

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| CustomerRow {
            leakage_pct: pct_of(r.gross_cents - r.net_cents, r.gross_cents),
            account_name: r.account_name,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
            capital_gross_cents: r.capital_gross_cents,
            capital_net_cents: r.capital_net_cents,
            consumable_gross_cents: r.consumable_gross_cents,
            consumable_net_cents: r.consumable_net_cents,
        })
        .collect();

    Ok(Json(Page {
        items,
        limit,
        offset,
        total,
    }))
}

// ═══ 5 · GET /api/metrics/leakage ══════════════════════════════════════════

#[derive(Deserialize)]
pub struct LeakageParams {
    period: Option<String>,
    by: Option<String>,
    kind: Option<String>,
    basis: Option<String>,
    /// P5 (R1): which math feeds the OUTLIER zone.
    ///   `period` (default) — the P1 behavior verbatim: stats and candidates
    ///   from the requested period/kind slice (σ = stddev_samp), threshold =
    ///   median + discount_sigma × σ. At the seeded default (2.00) this is
    ///   byte-identical to the old hardcoded 2σ — the equivalence proof.
    ///   `policy` — THE SAME math and config the discount_anomaly generator
    ///   uses: family stats over ALL history (stddev_pop), candidates in the
    ///   trailing signal_policy.discount_window_days, threshold = median +
    ///   discount_sigma × σ. The Leakage screen's outlier feed asks for this,
    ///   so its rows and the signal chips can never disagree. Ignores
    ///   period/kind by construction (the generator has neither axis).
    outliers: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct LeakageRow {
    name: String,
    gross_cents: i64,
    net_cents: i64,
    leakage_cents: i64,
    leakage_pct: Option<f64>,
}

#[derive(Serialize)]
pub struct DistributionBucket {
    bucket: String,
    line_count: i64,
}

#[derive(Serialize)]
pub struct OutlierRow {
    order_id: Uuid,
    /// P5 (R1) disclosed additions: the line identity + account link the
    /// Leakage screen's outlier feed needs (row → account 360), and the
    /// matching discount_anomaly signal when one exists (matched by
    /// order_line under the caller's RLS — the chip).
    order_line_id: Uuid,
    account_id: Uuid,
    territory_code: String,
    ordered_on: NaiveDate,
    account_name: String,
    rep_name: String,
    product_sku: String,
    product_name: String,
    family: String,
    qty: i32,
    list_unit_cents: i64,
    net_unit_cents: i64,
    discount_pct: f64,
    family_median_pct: f64,
    threshold_pct: f64,
    signal_id: Option<Uuid>,
    signal_status: Option<String>,
}

/// P5 (R1) disclosed addition: one rep × family money cell for the heat
/// table (the second signature element). Client derives leakage/percent and
/// the row/column totals; the payload stays cents-only facts.
#[derive(Serialize)]
pub struct HeatCell {
    rep_name: String,
    family: String,
    gross_cents: i64,
    net_cents: i64,
}

#[derive(Serialize)]
pub struct LeakagePage {
    items: Vec<LeakageRow>,
    limit: i64,
    offset: i64,
    total: i64,
    discount_distribution: Vec<DistributionBucket>,
    outliers: Vec<OutlierRow>,
    heat: Vec<HeatCell>,
}

/// Histogram edges, lower-bound inclusive: [0,5) [5,10) [10,15) [15,20)
/// [20,25) and >25 = [25, 100].
const BUCKETS: [&str; 6] = ["0-5", "5-10", "10-15", "15-20", "20-25", ">25"];

pub async fn leakage(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<LeakageParams>,
) -> Result<Json<LeakagePage>, ApiError> {
    reject_param(
        &params.basis,
        "basis is not a parameter of /metrics/leakage — leakage is its own ranking basis",
    )?;
    let period = parse_period(params.period)?;
    let kind = parse_kind(params.kind)?;
    let by = match params
        .by
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        None => "rep".to_string(),
        Some(b @ ("rep" | "territory" | "family" | "account")) => b.to_string(),
        Some(_) => {
            return Err(ApiError::Invalid(
                "by must be 'rep', 'territory', 'family', or 'account'".into(),
            ))
        }
    };
    let outlier_policy = match params
        .outliers
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        None | Some("period") => false,
        Some("policy") => true,
        Some(_) => {
            return Err(ApiError::Invalid(
                "outliers must be 'period' or 'policy'".into(),
            ))
        }
    };
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let (start, end, ttm) = live_bounds(period);

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Leakage is line-level analytics by definition (medians, deviations,
    // histograms do not survive pre-aggregation), so all three queries read
    // v_order_facts live regardless of period.
    let board = sqlx::query!(
        r#"SELECT CASE WHEN $5 = 'rep' THEN rep_name
                       WHEN $5 = 'territory' THEN territory_code
                       WHEN $5 = 'family' THEN family
                       ELSE account_name END AS "name!",
                  SUM(gross_cents)::bigint AS "gross_cents!",
                  SUM(net_cents)::bigint AS "net_cents!"
           FROM v_order_facts
           WHERE ($1::date IS NULL OR ordered_on >= $1)
             AND ($2::date IS NULL OR ordered_on < $2)
             AND (NOT $3::boolean
                  OR ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR kind::text = $4)
           GROUP BY 1
           ORDER BY (SUM(gross_cents) - SUM(net_cents)) DESC, 1
           LIMIT $6 OFFSET $7"#,
        start,
        end,
        ttm,
        kind.as_str(),
        by,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT CASE WHEN $5 = 'rep' THEN rep_name
                                      WHEN $5 = 'territory' THEN territory_code
                                      WHEN $5 = 'family' THEN family
                                      ELSE account_name END) AS "count!"
           FROM v_order_facts
           WHERE ($1::date IS NULL OR ordered_on >= $1)
             AND ($2::date IS NULL OR ordered_on < $2)
             AND (NOT $3::boolean
                  OR ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR kind::text = $4)"#,
        start,
        end,
        ttm,
        kind.as_str(),
        by
    )
    .fetch_one(&mut *tx)
    .await?;

    let dist = sqlx::query!(
        r#"SELECT CASE WHEN discount_pct < 5 THEN '0-5'
                       WHEN discount_pct < 10 THEN '5-10'
                       WHEN discount_pct < 15 THEN '10-15'
                       WHEN discount_pct < 20 THEN '15-20'
                       WHEN discount_pct < 25 THEN '20-25'
                       ELSE '>25' END AS "bucket!",
                  count(*) AS "line_count!"
           FROM v_order_facts
           WHERE ($1::date IS NULL OR ordered_on >= $1)
             AND ($2::date IS NULL OR ordered_on < $2)
             AND (NOT $3::boolean
                  OR ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR kind::text = $4)
           GROUP BY 1"#,
        start,
        end,
        ttm,
        kind.as_str()
    )
    .fetch_all(&mut *tx)
    .await?;

    // Outliers, spec §5.5. Two modes (R1):
    //
    // period (default) — the P1 behavior verbatim except the σ multiplier now
    // reads signal_policy.discount_sigma instead of a hardcoded 2. numeric
    // 2.00 casts to float8 2.0 exactly, so at the seeded default every
    // computed threshold — and therefore every returned row — is
    // byte-identical to the P1 output (the PRE-3/report equivalence proof).
    // Stats stay period/kind-sliced with stddev_samp, exactly as before.
    //
    // policy — the discount_anomaly generator's math verbatim (0013):
    // family stats over ALL history with stddev_pop, candidates in the
    // trailing discount_window_days, threshold = median + discount_sigma × σ.
    // No period/kind axis, because the generator has none — this is the
    // Leakage screen's outlier zone, and its rows match the signal chips 1:1.
    //
    // Both modes LEFT JOIN the caller-visible discount_anomaly signal by
    // order_line (the chip), and carry the line/account identity the screen
    // links through — disclosed payload additions, R1.
    let outliers = if outlier_policy {
        sqlx::query_as!(
            OutlierRow,
            r#"WITH sp AS (SELECT * FROM signal_policy),
               fam_stats AS (
                   SELECT family,
                          percentile_cont(0.5) WITHIN GROUP
                              (ORDER BY discount_pct::float8) AS median_pct,
                          stddev_pop(discount_pct::float8) AS sd
                   FROM v_order_facts
                   GROUP BY family
               )
               SELECT f.order_id AS "order_id!",
                      f.order_line_id AS "order_line_id!",
                      f.account_id AS "account_id!",
                      f.territory_code AS "territory_code!",
                      f.ordered_on AS "ordered_on!",
                      f.account_name AS "account_name!",
                      f.rep_name AS "rep_name!",
                      f.product_sku AS "product_sku!",
                      f.product_name AS "product_name!",
                      f.family AS "family!",
                      f.qty AS "qty!",
                      f.list_unit_cents AS "list_unit_cents!",
                      f.net_unit_cents AS "net_unit_cents!",
                      f.discount_pct::float8 AS "discount_pct!",
                      s.median_pct AS "family_median_pct!",
                      (s.median_pct + sp.discount_sigma * s.sd) AS "threshold_pct!",
                      sig.id AS "signal_id?",
                      sig.status::text AS "signal_status?"
               FROM v_order_facts f
               JOIN fam_stats s ON s.family = f.family
               CROSS JOIN sp
               LEFT JOIN signals sig
                      ON sig.order_line_id = f.order_line_id
                     AND sig.type = 'discount_anomaly'
               WHERE f.ordered_on >= CURRENT_DATE - sp.discount_window_days
                 AND s.sd > 0
                 AND f.discount_pct::float8 > s.median_pct + sp.discount_sigma * s.sd
               ORDER BY f.discount_pct DESC, f.order_line_id
               LIMIT $1"#,
            limit
        )
        .fetch_all(&mut *tx)
        .await?
    } else {
        sqlx::query_as!(
            OutlierRow,
            r#"WITH sp AS (SELECT discount_sigma::float8 AS sigma FROM signal_policy),
               fam_stats AS (
                   SELECT family,
                          percentile_cont(0.5) WITHIN GROUP
                              (ORDER BY discount_pct::float8) AS median_pct,
                          stddev_samp(discount_pct::float8) AS sd
                   FROM v_order_facts
                   WHERE ($1::date IS NULL OR ordered_on >= $1)
                     AND ($2::date IS NULL OR ordered_on < $2)
                     AND (NOT $3::boolean
                          OR ordered_on >= CURRENT_DATE - interval '12 months')
                     AND ($4::text = 'all' OR kind::text = $4)
                   GROUP BY family
               )
               SELECT f.order_id AS "order_id!",
                      f.order_line_id AS "order_line_id!",
                      f.account_id AS "account_id!",
                      f.territory_code AS "territory_code!",
                      f.ordered_on AS "ordered_on!",
                      f.account_name AS "account_name!",
                      f.rep_name AS "rep_name!",
                      f.product_sku AS "product_sku!",
                      f.product_name AS "product_name!",
                      f.family AS "family!",
                      f.qty AS "qty!",
                      f.list_unit_cents AS "list_unit_cents!",
                      f.net_unit_cents AS "net_unit_cents!",
                      f.discount_pct::float8 AS "discount_pct!",
                      s.median_pct AS "family_median_pct!",
                      (s.median_pct + sp.sigma * s.sd) AS "threshold_pct!",
                      sig.id AS "signal_id?",
                      sig.status::text AS "signal_status?"
               FROM v_order_facts f
               JOIN fam_stats s ON s.family = f.family
               CROSS JOIN sp
               LEFT JOIN signals sig
                      ON sig.order_line_id = f.order_line_id
                     AND sig.type = 'discount_anomaly'
               WHERE ($1::date IS NULL OR f.ordered_on >= $1)
                 AND ($2::date IS NULL OR f.ordered_on < $2)
                 AND (NOT $3::boolean
                      OR f.ordered_on >= CURRENT_DATE - interval '12 months')
                 AND ($4::text = 'all' OR f.kind::text = $4)
                 AND s.sd IS NOT NULL AND s.sd > 0
                 AND f.discount_pct::float8 > s.median_pct + sp.sigma * s.sd
               ORDER BY f.discount_pct DESC, f.order_line_id
               LIMIT $5"#,
            start,
            end,
            ttm,
            kind.as_str(),
            limit
        )
        .fetch_all(&mut *tx)
        .await?
    };

    // The rep × family heat cells (R1) — the whole visible slice, no
    // pagination (12 reps × ~13 families at VP view). RLS-scoped like every
    // other read here; the client derives leakage %, bands, and totals.
    let heat = sqlx::query_as!(
        HeatCell,
        r#"SELECT rep_name AS "rep_name!", family AS "family!",
                  SUM(gross_cents)::bigint AS "gross_cents!",
                  SUM(net_cents)::bigint AS "net_cents!"
           FROM v_order_facts
           WHERE ($1::date IS NULL OR ordered_on >= $1)
             AND ($2::date IS NULL OR ordered_on < $2)
             AND (NOT $3::boolean
                  OR ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR kind::text = $4)
           GROUP BY rep_name, family
           ORDER BY rep_name, family"#,
        start,
        end,
        ttm,
        kind.as_str()
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let items = board
        .into_iter()
        .map(|r| LeakageRow {
            leakage_cents: r.gross_cents - r.net_cents,
            leakage_pct: pct_of(r.gross_cents - r.net_cents, r.gross_cents),
            name: r.name,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
        })
        .collect();

    // Always all six buckets, in order, zero-filled — a histogram with holes
    // reads as missing data.
    let discount_distribution = BUCKETS
        .iter()
        .map(|b| DistributionBucket {
            bucket: (*b).to_string(),
            line_count: dist
                .iter()
                .find(|d| d.bucket == *b)
                .map_or(0, |d| d.line_count),
        })
        .collect();

    // The P1 display rounding, unchanged (2 decimals on the three percent
    // fields) — part of the byte-identity contract for the period mode.
    let outliers = {
        let mut rows = outliers;
        for r in &mut rows {
            r.discount_pct = round2(r.discount_pct);
            r.family_median_pct = round2(r.family_median_pct);
            r.threshold_pct = round2(r.threshold_pct);
        }
        rows
    };

    Ok(Json(LeakagePage {
        items,
        limit,
        offset,
        total,
        discount_distribution,
        outliers,
        heat,
    }))
}

// ═══ 6 · GET /api/metrics/coverage ═════════════════════════════════════════

#[derive(Deserialize)]
pub struct CoverageParams {
    basis: Option<String>,
    period: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct CoverageRow {
    territory_code: String,
    territory_name: String,
    units_due: i64,
    pct_covered: Option<f64>,
    projected_consumable_gross_cents: i64,
    projected_consumable_net_cents: i64,
}

pub async fn coverage(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<CoverageParams>,
) -> Result<Json<Page<CoverageRow>>, ApiError> {
    // Spec §5.6 defines aftermarket coverage on the CURRENT quarter — the
    // metric has no period axis, so a period parameter is a caller error,
    // not something to silently ignore.
    reject_param(
        &params.period,
        "coverage is defined on the current quarter and takes no period parameter",
    )?;
    reject_param(
        &params.kind,
        "kind is not a parameter of /metrics/coverage — the metric is consumable by definition",
    )?;
    let basis = parse_basis(params.basis)?;
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // due   = units whose next change-out (last filter order + expected
    //         cadence, calendar months) lands in the current quarter; units
    //         missing cadence/cartridge/history data cannot be "due" (the
    //         two seeded NULL-cadence units are P5 data-quality material).
    // covered = the unit's site ordered its cartridge this quarter, OR an
    //         open quote (draft/pending/approved/sent, created this quarter)
    //         on the unit's account carries the cartridge. quotes/
    //         opportunities are RLS tables — scope inherited.
    let rows = sqlx::query!(
        r#"WITH due AS (
               SELECT u.unit_id, u.site_id, u.account_id, u.territory_id,
                      u.territory_code, u.territory_name,
                      u.cartridge_product_id, u.cartridge_count,
                      u.cartridge_list_unit_cents, u.cartridge_net_unit_cents
               FROM v_unit_facts u
               WHERE u.cartridge_product_id IS NOT NULL
                 AND u.expected_changeout_months IS NOT NULL
                 AND u.last_filter_order_on IS NOT NULL
                 AND date_trunc('quarter',
                         u.last_filter_order_on
                         + u.expected_changeout_months * interval '1 month')
                     = date_trunc('quarter', CURRENT_DATE::timestamp)
           ),
           flags AS (
               SELECT d.*,
                      (EXISTS (
                           SELECT 1 FROM v_order_facts f
                           WHERE f.site_id = d.site_id
                             AND f.product_id = d.cartridge_product_id
                             AND f.ordered_on
                                 >= date_trunc('quarter', CURRENT_DATE)::date
                       ) OR EXISTS (
                           SELECT 1
                           FROM quotes q
                           JOIN opportunities o ON o.id = q.opportunity_id
                           JOIN quote_lines ql ON ql.quote_id = q.id
                           WHERE o.account_id = d.account_id
                             AND ql.product_id = d.cartridge_product_id
                             AND q.status IN ('draft', 'pending_approval',
                                              'approved', 'sent')
                             AND q.created_at >= date_trunc('quarter', now())
                       )) AS covered
               FROM due d
           )
           SELECT territory_code AS "territory_code!",
                  territory_name AS "territory_name!",
                  count(*) AS "units_due!",
                  count(*) FILTER (WHERE covered) AS "units_covered!",
                  SUM(cartridge_count::bigint * cartridge_list_unit_cents)::bigint
                      AS "projected_gross_cents!",
                  SUM(cartridge_count::bigint * cartridge_net_unit_cents)::bigint
                      AS "projected_net_cents!"
           FROM flags
           GROUP BY territory_id, territory_code, territory_name
           ORDER BY CASE WHEN $1 = 'gross'
                         THEN SUM(cartridge_count::bigint * cartridge_list_unit_cents)
                         ELSE SUM(cartridge_count::bigint * cartridge_net_unit_cents)
                    END DESC,
                    territory_code
           LIMIT $2 OFFSET $3"#,
        basis.as_str(),
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT u.territory_id) AS "count!"
           FROM v_unit_facts u
           WHERE u.cartridge_product_id IS NOT NULL
             AND u.expected_changeout_months IS NOT NULL
             AND u.last_filter_order_on IS NOT NULL
             AND date_trunc('quarter',
                     u.last_filter_order_on
                     + u.expected_changeout_months * interval '1 month')
                 = date_trunc('quarter', CURRENT_DATE::timestamp)"#
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| CoverageRow {
            territory_code: r.territory_code,
            territory_name: r.territory_name,
            units_due: r.units_due,
            pct_covered: pct_of(r.units_covered, r.units_due),
            projected_consumable_gross_cents: r.projected_gross_cents,
            projected_consumable_net_cents: r.projected_net_cents,
        })
        .collect();

    Ok(Json(Page {
        items,
        limit,
        offset,
        total,
    }))
}

// ═══ 7 · GET /api/metrics/defection ════════════════════════════════════════

#[derive(Deserialize)]
pub struct DefectionParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct DefectionRow {
    serial: String,
    site: String,
    account_name: String,
    territory_code: String,
    days_silent: i32,
    expected_changeout_months: i32,
    annual_consumable_value_cents: i64,
    score: f64,
}

pub async fn defection(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<DefectionParams>,
) -> Result<Json<Page<DefectionRow>>, ApiError> {
    // Metric 7 is a ranking over the CURRENT silence gap — no period, no
    // basis (annual value is gross by definition), no kind.
    reject_param(
        &params.period,
        "defection risk is computed from today's silence gap and takes no period parameter",
    )?;
    reject_param(
        &params.basis,
        "basis is not a parameter of /metrics/defection — annual value is gross by definition",
    )?;
    reject_param(
        &params.kind,
        "kind is not a parameter of /metrics/defection — the metric is consumable by definition",
    )?;
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // v_defection_risk is security_invoker over v_unit_facts: the read-only
    // ranking P4's signal generator will reuse. RLS scope inherited.
    let rows = sqlx::query!(
        r#"SELECT serial AS "serial!",
                  site_label AS "site!",
                  account_name AS "account_name!",
                  territory_code AS "territory_code!",
                  days_silent AS "days_silent!",
                  expected_changeout_months AS "expected_changeout_months!",
                  annual_consumable_value_cents AS "annual_consumable_value_cents!",
                  score::float8 AS "score!"
           FROM v_defection_risk
           ORDER BY score DESC, serial
           LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM v_defection_risk"#)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| DefectionRow {
            serial: r.serial,
            site: r.site,
            account_name: r.account_name,
            territory_code: r.territory_code,
            days_silent: r.days_silent,
            expected_changeout_months: r.expected_changeout_months,
            annual_consumable_value_cents: r.annual_consumable_value_cents,
            score: r.score,
        })
        .collect();

    Ok(Json(Page {
        items,
        limit,
        offset,
        total,
    }))
}
