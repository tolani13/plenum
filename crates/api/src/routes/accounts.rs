//! GET /api/accounts — the one CRM endpoint P0 ships (P0-2 needs it).
//!
//! There is NO territory filtering in this handler, on purpose. The query is
//! a plain SELECT; the row count differing per caller is Postgres RLS doing
//! its job through the rls_tx transaction. That is the point of the phase.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use domain::{AccountStatus, ActivityKind, OppStage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::signals::{account_signals, SignalRow};
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

/// Raw string params so a non-numeric value becomes a typed 422, not a
/// framework 400.
#[derive(Deserialize)]
pub struct PageParams {
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
pub struct AccountItem {
    id: Uuid,
    name: String,
    industry: String,
    status: AccountStatus,
    territory_code: String,
    parent_account_id: Option<Uuid>,
    created: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AccountsPage {
    items: Vec<AccountItem>,
    limit: i64,
    offset: i64,
    total: i64,
}

fn parse_param(value: Option<String>, default: i64, name: &str) -> Result<i64, ApiError> {
    match value {
        None => Ok(default),
        Some(s) => s
            .parse::<i64>()
            .map_err(|_| ApiError::Invalid(format!("{name} must be an integer"))),
    }
}

pub async fn list_accounts(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<PageParams>,
) -> Result<Json<AccountsPage>, ApiError> {
    let limit = parse_param(params.limit, DEFAULT_LIMIT, "limit")?;
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ApiError::Invalid(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    let offset = parse_param(params.offset, 0, "offset")?;
    if offset < 0 {
        return Err(ApiError::Invalid("offset must be >= 0".into()));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    let items = sqlx::query_as!(
        AccountItem,
        r#"SELECT a.id, a.name, a.industry, a.status AS "status: AccountStatus",
                  t.code AS "territory_code!", a.parent_account_id, a.created
           FROM accounts a
           JOIN territories t ON t.id = a.territory_id
           ORDER BY a.name, a.id
           LIMIT $1 OFFSET $2"#,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    // total under the SAME RLS transaction: it counts what this caller may
    // see, not what exists.
    let total: i64 = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM accounts"#)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(AccountsPage {
        items,
        limit,
        offset,
        total,
    }))
}

// ── GET /api/accounts/:id — the Account 360 payload (R5) ────────────────────
//
// Everything is read inside ONE rls_tx: an account the caller cannot see is a
// 404 (the header select returns no row), and every sub-query is scoped by the
// same transaction. Money (cumulative gross/net/leakage, order totals) comes
// from v_order_facts under RLS; the installed-base timeline comes from
// v_unit_facts under RLS (NULL expected_changeout_months passes through as a
// designed "cadence unknown" state — the seeded data-quality beat, rendered,
// never "fixed"). signals is always [] until P4.

const ORDERS_CAP: i64 = 20;
const ACTIVITIES_CAP: i64 = 50;
/// R14: the 360 shows the account's signals — active first, score DESC — with
/// the recent_orders cap pattern.
const SIGNALS_CAP: i64 = 20;

#[derive(Serialize)]
struct MoneyTriple {
    gross_cents: i64,
    net_cents: i64,
    leakage_cents: i64,
    leakage_pct: Option<f64>,
}

#[derive(Serialize)]
struct SiteBrief {
    id: Uuid,
    address: String,
    city: String,
    state: String,
}

#[derive(Serialize)]
struct ContactBrief {
    id: Uuid,
    site_id: Uuid,
    name: String,
    title: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    role: Option<String>,
}

/// One installed unit on the life-axis. Nullable cadence/last-order/cartridge
/// are designed states, not errors.
#[derive(Serialize)]
struct UnitTimeline {
    id: Uuid,
    serial: String,
    family: String,
    source: String,
    site_label: String,
    commissioned_on: NaiveDate,
    expected_changeout_months: Option<i32>,
    last_filter_order_on: Option<NaiveDate>,
    cartridge_sku: Option<String>,
    cartridge_name: Option<String>,
    cartridge_count: i32,
}

#[derive(Serialize)]
struct OrderBrief {
    id: Uuid,
    ordered_on: NaiveDate,
    gross_cents: i64,
    net_cents: i64,
}

#[derive(Serialize)]
struct OppBrief {
    id: Uuid,
    stage: OppStage,
    kind: String,
    amount_cents: i64,
    expected_close: Option<NaiveDate>,
    owner_name: String,
    has_approved_quote: bool,
}

#[derive(Serialize)]
struct ActivityItem {
    id: Uuid,
    kind: ActivityKind,
    body: String,
    occurred_at: DateTime<Utc>,
    rep_name: String,
}

#[derive(Serialize)]
struct ActivitiesPage {
    items: Vec<ActivityItem>,
    limit: i64,
    offset: i64,
    total: i64,
}

#[derive(Serialize)]
pub struct AccountDetail {
    id: Uuid,
    name: String,
    industry: String,
    status: AccountStatus,
    territory_code: String,
    territory_name: String,
    parent_account_id: Option<Uuid>,
    created: DateTime<Utc>,
    cumulative: MoneyTriple,
    sites: Vec<SiteBrief>,
    contacts: Vec<ContactBrief>,
    units: Vec<UnitTimeline>,
    recent_orders: Vec<OrderBrief>,
    opportunities: Vec<OppBrief>,
    activities: ActivitiesPage,
    /// P4 (R14): the account's signals — active first, then score DESC,
    /// capped at SIGNALS_CAP — in the SAME enriched shape as GET /api/signals.
    /// An account with none gets an empty array (the designed empty state).
    signals: Vec<SignalRow>,
}

fn leakage_pct(gross: i64, net: i64) -> Option<f64> {
    if gross == 0 {
        None
    } else {
        Some(((gross - net) as f64 / gross as f64 * 1000.0).round() / 10.0)
    }
}

pub async fn get_account(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AccountDetail>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;

    // Header — the RLS gate. Invisible account -> no row -> 404.
    let header = sqlx::query!(
        r#"SELECT a.id, a.name, a.industry, a.status AS "status: AccountStatus",
                  t.code AS "territory_code!", t.name AS "territory_name!",
                  a.parent_account_id, a.created
           FROM accounts a
           JOIN territories t ON t.id = a.territory_id
           WHERE a.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    // Cumulative money from v_order_facts (security_invoker — RLS applies).
    let money = sqlx::query!(
        r#"SELECT COALESCE(SUM(gross_cents), 0)::bigint AS "gross!",
                  COALESCE(SUM(net_cents), 0)::bigint   AS "net!",
                  COALESCE(SUM(discount_cents), 0)::bigint AS "leakage!"
           FROM v_order_facts WHERE account_id = $1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    let sites = sqlx::query_as!(
        SiteBrief,
        r#"SELECT s.id, s.address, s.city, s.state
           FROM sites s JOIN accounts a ON a.id = s.account_id
           WHERE a.id = $1 ORDER BY s.city, s.id"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    let contacts = sqlx::query_as!(
        ContactBrief,
        r#"SELECT c.id, c.site_id, c.name, c.title, c.email, c.phone, c.role
           FROM contacts c
           JOIN sites s ON s.id = c.site_id
           JOIN accounts a ON a.id = s.account_id
           WHERE a.id = $1 ORDER BY c.name, c.id"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    let units = sqlx::query_as!(
        UnitTimeline,
        r#"SELECT unit_id AS "id!", serial AS "serial!",
                  unit_family AS "family!", source AS "source!",
                  site_label AS "site_label!",
                  commissioned_on AS "commissioned_on!",
                  expected_changeout_months, last_filter_order_on,
                  cartridge_sku, cartridge_name,
                  cartridge_count AS "cartridge_count!"
           FROM v_unit_facts WHERE account_id = $1
           ORDER BY commissioned_on, serial"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    let recent_orders = sqlx::query_as!(
        OrderBrief,
        r#"SELECT order_id AS "id!", ordered_on AS "ordered_on!",
                  SUM(gross_cents)::bigint AS "gross_cents!",
                  SUM(net_cents)::bigint   AS "net_cents!"
           FROM v_order_facts WHERE account_id = $1
           GROUP BY order_id, ordered_on
           ORDER BY ordered_on DESC, order_id DESC
           LIMIT $2"#,
        id,
        ORDERS_CAP
    )
    .fetch_all(&mut *tx)
    .await?;

    let opportunities = sqlx::query_as!(
        OppBrief,
        r#"SELECT o.id, o.stage AS "stage: OppStage", o.kind,
                  o.amount_cents, o.expected_close, u.name AS owner_name,
                  EXISTS(SELECT 1 FROM quotes q
                         WHERE q.opportunity_id = o.id AND q.status = 'approved')
                      AS "has_approved_quote!"
           FROM opportunities o
           JOIN users u ON u.id = o.owner_id
           WHERE o.account_id = $1
           ORDER BY o.amount_cents DESC, o.id"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    let activity_items = sqlx::query_as!(
        ActivityItem,
        r#"SELECT a.id, a.kind AS "kind: ActivityKind", a.body,
                  a.occurred_at, u.name AS rep_name
           FROM activities a JOIN users u ON u.id = a.rep_id
           WHERE a.account_id = $1
           ORDER BY a.occurred_at DESC, a.id DESC
           LIMIT $2"#,
        id,
        ACTIVITIES_CAP
    )
    .fetch_all(&mut *tx)
    .await?;

    let activity_total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM activities WHERE account_id = $1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    let signals = account_signals(&mut tx, id, SIGNALS_CAP).await?;

    tx.commit().await?;

    Ok(Json(AccountDetail {
        id: header.id,
        name: header.name,
        industry: header.industry,
        status: header.status,
        territory_code: header.territory_code,
        territory_name: header.territory_name,
        parent_account_id: header.parent_account_id,
        created: header.created,
        cumulative: MoneyTriple {
            gross_cents: money.gross,
            net_cents: money.net,
            leakage_cents: money.leakage,
            leakage_pct: leakage_pct(money.gross, money.net),
        },
        sites,
        contacts,
        units,
        recent_orders,
        opportunities,
        activities: ActivitiesPage {
            items: activity_items,
            limit: ACTIVITIES_CAP,
            offset: 0,
            total: activity_total,
        },
        signals,
    }))
}

// ── POST /api/accounts — route-only create (R6) ─────────────────────────────
//
// name/industry/territory_id/status/parent. WITH CHECK enforces territory
// scope, but a raw WITH CHECK violation would surface as a 500 — so the
// handler pre-checks the territory is in the caller's v_user_scope (403 if
// not, 422 if the territory does not exist at all) and validates the body
// (422 on garbage). No §8 screen needs this in P3; acceptance is one curl.

#[derive(Deserialize)]
pub struct CreateAccountBody {
    name: String,
    industry: String,
    territory_id: Uuid,
    status: AccountStatus,
    parent_account_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct CreatedAccount {
    id: Uuid,
    name: String,
    industry: String,
    status: AccountStatus,
    territory_id: Uuid,
    parent_account_id: Option<Uuid>,
    created: DateTime<Utc>,
}

pub async fn create_account(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<CreateAccountBody>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<CreatedAccount>), ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    if body.name.trim().is_empty() {
        return Err(ApiError::Invalid("name is required".into()));
    }
    if body.industry.trim().is_empty() {
        return Err(ApiError::Invalid("industry is required".into()));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // The territory must exist (422) and be in the caller's scope (403) — this
    // turns the WITH CHECK guard into a clean typed error instead of a 500.
    let exists: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM territories WHERE id = $1) AS "e!""#,
        body.territory_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !exists {
        return Err(ApiError::Invalid("unknown territory".into()));
    }
    let in_scope: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(
             SELECT 1 FROM v_user_scope
             WHERE user_id = $1 AND territory_id = $2
           ) AS "e!""#,
        user.id,
        body.territory_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !in_scope {
        return Err(ApiError::Forbidden);
    }

    // Optional parent must itself be visible (RLS) — else 422.
    if let Some(parent) = body.parent_account_id {
        let parent_ok: bool = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1) AS "e!""#,
            parent
        )
        .fetch_one(&mut *tx)
        .await?;
        if !parent_ok {
            return Err(ApiError::Invalid("unknown parent account".into()));
        }
    }

    // Bind status as text and cast (a plain $3::account_status has no built-in
    // Rust mapping in the sqlx macro; $3::text pins the param to text).
    let status_text = match body.status {
        AccountStatus::Customer => "customer",
        AccountStatus::Prospect => "prospect",
        AccountStatus::AtRisk => "at_risk",
        AccountStatus::Dormant => "dormant",
    };
    let row = sqlx::query!(
        r#"INSERT INTO accounts (name, industry, status, territory_id, parent_account_id)
           VALUES ($1, $2, $3::text::account_status, $4, $5)
           RETURNING id, name, industry, status AS "status: AccountStatus",
                     territory_id, parent_account_id, created"#,
        body.name.trim(),
        body.industry.trim(),
        status_text,
        body.territory_id,
        body.parent_account_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreatedAccount {
            id: row.id,
            name: row.name,
            industry: row.industry,
            status: row.status,
            territory_id: row.territory_id,
            parent_account_id: row.parent_account_id,
            created: row.created,
        }),
    ))
}
