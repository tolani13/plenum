//! GET /api/metrics/states — the Territory Map's money feed (P5, R3;
//! disclosed addition on the P3 precedent).
//!
//! Per-state gross/net/leakage/order_count from v_order_facts joined through
//! sites (site state is the geographic key; PRE-2 proved 100% of every
//! territory's dollars land inside its territory_states set), plus the
//! territory roster the map's PANEL needs (TM = assigned reps via
//! territory_assignments, RM = their managers via the users chain) and the
//! state→territory coloring config from territory_states.
//!
//! Scope model, stated plainly (R3 "the map's shape is config, its money is
//! scoped"):
//!   · `items` (the money rows) read v_order_facts under rls_tx — a rep gets
//!     rows ONLY for states her own orders live in; foreign states simply
//!     have no row (the adversarial test pins this).
//!   · `territories` (the roster + state_codes) is CONFIG-level disclosure —
//!     territory names, geography, and staffing are org chart, not ledger
//!     (the P4 /api/signals/assignees precedent). It carries no money.
//! period/basis/kind follow the §5 metrics grammar; basis picks the ranking
//! only (the payload is dual-basis like every other metrics endpoint). All
//! periods aggregate v_order_facts live — states have no rollup axis.

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

#[derive(Deserialize)]
pub struct StatesParams {
    period: Option<String>,
    basis: Option<String>,
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct StateRow {
    state_code: String,
    territory_code: String,
    gross_cents: i64,
    net_cents: i64,
    leakage_cents: i64,
    leakage_pct: Option<f64>,
    order_count: i64,
}

/// Config-level roster row: who works the territory, and which states it
/// paints. rm_names is a list because a territory's reps can report to
/// different managers (CE-1: Celine Roy → Renee Vega, Dana Cross →
/// Marcus Reed).
#[derive(Serialize)]
pub struct TerritoryRoster {
    territory_id: Uuid,
    territory_code: String,
    territory_name: String,
    region: String,
    tm_names: Vec<String>,
    rm_names: Vec<String>,
    state_codes: Vec<String>,
}

#[derive(Serialize)]
pub struct StatesPage {
    items: Vec<StateRow>,
    limit: i64,
    offset: i64,
    total: i64,
    territories: Vec<TerritoryRoster>,
}

struct StateAgg {
    state_code: String,
    territory_code: String,
    gross_cents: i64,
    net_cents: i64,
    order_count: i64,
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn pct_of(numer: i64, denom: i64) -> Option<f64> {
    if denom == 0 {
        None
    } else {
        Some(round2(numer as f64 / denom as f64 * 100.0))
    }
}

fn live_bounds(period: Period) -> (Option<NaiveDate>, Option<NaiveDate>, bool) {
    match period.date_range() {
        Some((start, end)) => (Some(start), Some(end), false),
        None => (None, None, matches!(period, Period::Ttm)),
    }
}

pub async fn states(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<StatesParams>,
) -> Result<Json<StatesPage>, ApiError> {
    let period = params
        .period
        .ok_or_else(|| {
            ApiError::Invalid(
                "period is required (a year like 2025, a quarter like 2025-Q2, 'cumulative', or 'ttm')"
                    .into(),
            )
        })
        .and_then(|raw| Period::parse(&raw).map_err(ApiError::Invalid))?;
    let basis = params
        .basis
        .ok_or_else(|| ApiError::Invalid("basis is required ('gross' or 'net')".into()))
        .and_then(|raw| Basis::parse(&raw).map_err(ApiError::Invalid))?;
    let kind = match params.kind {
        None => KindFilter::All,
        Some(raw) => KindFilter::parse(&raw).map_err(ApiError::Invalid)?,
    };
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let (start, end, ttm) = live_bounds(period);

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Money per state: v_order_facts (RLS) → sites (geographic key) →
    // territory_states (config). The territory attribution is the ORDER's
    // territory — the same authority every other metric uses; PRE-2 proved
    // the state mapping and the order attribution agree on 100% of dollars.
    let rows = sqlx::query_as!(
        StateAgg,
        r#"SELECT s.state AS "state_code!",
                  t.code AS "territory_code!",
                  SUM(f.gross_cents)::bigint AS "gross_cents!",
                  SUM(f.net_cents)::bigint AS "net_cents!",
                  count(DISTINCT f.order_id) AS "order_count!"
           FROM v_order_facts f
           JOIN sites s ON s.id = f.site_id
           JOIN territories t ON t.id = f.territory_id
           WHERE ($1::date IS NULL OR f.ordered_on >= $1)
             AND ($2::date IS NULL OR f.ordered_on < $2)
             AND (NOT $3::boolean
                  OR f.ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR f.kind::text = $4)
           GROUP BY s.state, t.code
           ORDER BY CASE WHEN $5 = 'gross' THEN SUM(f.gross_cents)
                         ELSE SUM(f.net_cents) END DESC,
                    s.state
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
        r#"SELECT count(DISTINCT (s.state, f.territory_id)) AS "count!"
           FROM v_order_facts f
           JOIN sites s ON s.id = f.site_id
           WHERE ($1::date IS NULL OR f.ordered_on >= $1)
             AND ($2::date IS NULL OR f.ordered_on < $2)
             AND (NOT $3::boolean
                  OR f.ordered_on >= CURRENT_DATE - interval '12 months')
             AND ($4::text = 'all' OR f.kind::text = $4)"#,
        start,
        end,
        ttm,
        kind.as_str()
    )
    .fetch_one(&mut *tx)
    .await?;

    // The roster: every territory, its assigned reps (TM), their managers
    // (RM), and its mapped states. users/territories/territory_assignments/
    // territory_states carry no RLS — org chart and geography are config
    // (the P4 assignees precedent); money above is what scope guards.
    let roster = sqlx::query!(
        r#"SELECT t.id AS "territory_id!",
                  t.code AS "territory_code!",
                  t.name AS "territory_name!",
                  t.region::text AS "region!",
                  COALESCE((SELECT array_agg(DISTINCT u.name)
                            FROM territory_assignments ta
                            JOIN users u ON u.id = ta.user_id
                            WHERE ta.territory_id = t.id), '{}') AS "tm_names!",
                  COALESCE((SELECT array_agg(DISTINCT m.name)
                            FROM territory_assignments ta
                            JOIN users u ON u.id = ta.user_id
                            JOIN users m ON m.id = u.manager_id
                            WHERE ta.territory_id = t.id), '{}') AS "rm_names!",
                  COALESCE((SELECT array_agg(ts.state_code ORDER BY ts.state_code)
                            FROM territory_states ts
                            WHERE ts.territory_code = t.code), '{}') AS "state_codes!"
           FROM territories t
           ORDER BY t.code"#
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| StateRow {
            leakage_cents: r.gross_cents - r.net_cents,
            leakage_pct: pct_of(r.gross_cents - r.net_cents, r.gross_cents),
            state_code: r.state_code,
            territory_code: r.territory_code,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
            order_count: r.order_count,
        })
        .collect();

    let territories = roster
        .into_iter()
        .map(|r| TerritoryRoster {
            territory_id: r.territory_id,
            territory_code: r.territory_code,
            territory_name: r.territory_name,
            region: r.region,
            tm_names: r.tm_names,
            rm_names: r.rm_names,
            state_codes: r.state_codes,
        })
        .collect();

    Ok(Json(StatesPage {
        items,
        limit,
        offset,
        total,
        territories,
    }))
}
