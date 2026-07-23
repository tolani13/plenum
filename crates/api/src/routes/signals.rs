//! Signals: the queue reads, the write-back actions, the Command summary
//! feed, and the admin generation trigger (rulings R2, R5, R7).
//!
//!   GET  /api/signals?status=&type=&limit=&offset=  — enriched list (R5)
//!   GET  /api/signals/summary                        — Command's KPI + tile feed
//!   GET  /api/signals/assignees?account_id=          — scope-valid picker feed
//!   POST /api/signals/:id/assign                     — open|assigned → assigned
//!   POST /api/signals/:id/action                     — open|assigned → actioned
//!   POST /api/signals/:id/dismiss                    — open|assigned → dismissed
//!   POST /api/admin/generate-signals                 — role=admin, runs the job
//!
//! State machine: actioned and dismissed are TERMINAL (422 out). Every read
//! and write runs inside rls_tx — an out-of-scope signal is a 404, and the
//! 0006 audit trigger records every UPDATE with the actor from the GUC (no
//! audit code here). Signals are only ever CREATED by generate_signals();
//! this surface changes their lifecycle, never their derivation.
//!
//! The three query_as! blocks below (list / by-account / by-id) share one
//! SELECT list — keep them in sync when a column changes. Enrichment beyond
//! the signals row itself (account name, territory code, site label, serial,
//! cartridge, best-fit, annual value) is computed via RLS-scoped joins:
//! v_unit_facts for unit-carrying rows, and — for conquest rows — the SAME
//! deterministic best-fit lateral generate_signals() uses (highest list
//! price, tie-break sku ASC), so the card and the generator can never
//! disagree about the weapon.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use domain::{SignalStatus, SignalType, UserRole};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::parse_page;
use crate::state::AppState;

// ── the enriched signal row (shared by list, 360 fill, and reloads) ─────────

#[derive(Serialize)]
pub struct SignalRow {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub signal_type: SignalType,
    pub status: SignalStatus,
    pub score: f64,
    pub reasons: Value,
    pub account_id: Uuid,
    pub account_name: String,
    pub territory_code: String,
    pub site_id: Option<Uuid>,
    pub site_label: Option<String>,
    pub installed_unit_id: Option<Uuid>,
    pub serial: Option<String>,
    pub cartridge_product_id: Option<Uuid>,
    pub cartridge_sku: Option<String>,
    pub cartridge_count: Option<i32>,
    /// The unit's annual consumable value (conquest: at the best-fit price;
    /// fallback cadence where ecm is NULL) — what draft-quote-from-signal
    /// uses as the new opportunity's amount (R6). NULL on anomaly rows.
    pub annual_value_cents: Option<i64>,
    pub order_line_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub assignee_name: Option<String>,
    pub outcome: Option<String>,
    pub dismissed_reason: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub actioned_at: Option<DateTime<Utc>>,
    pub dismissed_at: Option<DateTime<Utc>>,
    /// P5 (R4): when the generator auto-expired the card (predicate stopped
    /// holding). NULL on every other status; cleared again on reopen.
    pub expired_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct SignalsPage {
    items: Vec<SignalRow>,
    limit: i64,
    offset: i64,
    total: i64,
}

// ── GET /api/signals ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SignalListParams {
    status: Option<String>,
    #[serde(rename = "type")]
    signal_type: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

/// status grammar: the five lifecycle states, plus `active` (= open ∪
/// assigned) as the queue's default view. `expired` (P5, R4) is the
/// machine-retired shelf — never part of `active`.
fn parse_status_filter(raw: Option<String>) -> Result<String, ApiError> {
    let s = raw.unwrap_or_else(|| "active".to_string());
    let s = s.trim().to_ascii_lowercase();
    match s.as_str() {
        "active" | "open" | "assigned" | "actioned" | "dismissed" | "expired" => Ok(s),
        _ => Err(ApiError::Invalid(
            "status must be one of active, open, assigned, actioned, dismissed, expired".into(),
        )),
    }
}

fn parse_type_filter(raw: Option<String>) -> Result<Option<String>, ApiError> {
    match raw {
        None => Ok(None),
        Some(t) => {
            let t = t.trim().to_ascii_lowercase();
            match t.as_str() {
                "reorder_due" | "defection_risk" | "conquest" | "discount_anomaly" => Ok(Some(t)),
                _ => Err(ApiError::Invalid(
                    "type must be one of reorder_due, defection_risk, conquest, discount_anomaly"
                        .into(),
                )),
            }
        }
    }
}

pub async fn list_signals(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<SignalListParams>,
) -> Result<Json<SignalsPage>, ApiError> {
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let status = parse_status_filter(params.status)?;
    let type_filter = parse_type_filter(params.signal_type)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    let items = sqlx::query_as!(
        SignalRow,
        r#"SELECT s.id,
                  s.type AS "signal_type: SignalType",
                  s.status AS "status: SignalStatus",
                  s.score::float8 AS "score!",
                  s.reasons,
                  s.account_id,
                  a.name AS account_name,
                  t.code AS "territory_code!",
                  s.site_id,
                  COALESCE(uf.site_label, os.city || ', ' || os.state) AS site_label,
                  s.installed_unit_id,
                  uf.serial AS "serial?",
                  CASE WHEN s.type = 'conquest' THEN bf.product_id
                       ELSE uf.cartridge_product_id END AS "cartridge_product_id?",
                  CASE WHEN s.type = 'conquest' THEN bf.sku
                       ELSE uf.cartridge_sku END AS "cartridge_sku?",
                  uf.cartridge_count AS "cartridge_count?",
                  CASE WHEN s.type = 'conquest'
                       THEN round(uf.cartridge_count::numeric * bf.list_price_cents * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       WHEN s.installed_unit_id IS NOT NULL
                       THEN round(uf.cartridge_count::numeric
                                  * COALESCE(uf.cartridge_list_unit_cents, 0) * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       ELSE NULL END AS "annual_value_cents?",
                  s.order_line_id,
                  s.assigned_to,
                  au.name AS "assignee_name?",
                  s.outcome,
                  s.dismissed_reason,
                  s.opened_at,
                  s.assigned_at,
                  s.actioned_at,
                  s.dismissed_at,
                  s.expired_at
           FROM signals s
           JOIN accounts a ON a.id = s.account_id
           JOIN territories t ON t.id = a.territory_id
           CROSS JOIN signal_policy sp
           LEFT JOIN v_unit_facts uf ON uf.unit_id = s.installed_unit_id
           LEFT JOIN sites os ON os.id = s.site_id
           LEFT JOIN users au ON au.id = s.assigned_to
           LEFT JOIN LATERAL (
               SELECT p.id AS product_id, p.sku, p.list_price_cents
               FROM products p
               WHERE s.type = 'conquest'
                 AND p.kind = 'consumable'
                 AND p.filter_fits @> ARRAY[uf.unit_family]
               ORDER BY p.list_price_cents DESC, p.sku ASC
               LIMIT 1
           ) bf ON true
           WHERE CASE WHEN $1::text = 'active' THEN s.status IN ('open', 'assigned')
                      ELSE s.status::text = $1 END
             AND ($2::text IS NULL OR s.type::text = $2)
           ORDER BY s.score DESC, s.id ASC
           LIMIT $3 OFFSET $4"#,
        status,
        type_filter,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM signals s
           WHERE CASE WHEN $1::text = 'active' THEN s.status IN ('open', 'assigned')
                      ELSE s.status::text = $1 END
             AND ($2::text IS NULL OR s.type::text = $2)"#,
        status,
        type_filter
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(SignalsPage {
        items,
        limit,
        offset,
        total,
    }))
}

/// The account-360 fill (R14): the account's signals, active first, then
/// score DESC, capped — same enriched shape as the list. Called from
/// accounts.rs inside ITS rls_tx (the account itself is the 404 gate there).
pub async fn account_signals(
    tx: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
    cap: i64,
) -> Result<Vec<SignalRow>, ApiError> {
    let rows = sqlx::query_as!(
        SignalRow,
        r#"SELECT s.id,
                  s.type AS "signal_type: SignalType",
                  s.status AS "status: SignalStatus",
                  s.score::float8 AS "score!",
                  s.reasons,
                  s.account_id,
                  a.name AS account_name,
                  t.code AS "territory_code!",
                  s.site_id,
                  COALESCE(uf.site_label, os.city || ', ' || os.state) AS site_label,
                  s.installed_unit_id,
                  uf.serial AS "serial?",
                  CASE WHEN s.type = 'conquest' THEN bf.product_id
                       ELSE uf.cartridge_product_id END AS "cartridge_product_id?",
                  CASE WHEN s.type = 'conquest' THEN bf.sku
                       ELSE uf.cartridge_sku END AS "cartridge_sku?",
                  uf.cartridge_count AS "cartridge_count?",
                  CASE WHEN s.type = 'conquest'
                       THEN round(uf.cartridge_count::numeric * bf.list_price_cents * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       WHEN s.installed_unit_id IS NOT NULL
                       THEN round(uf.cartridge_count::numeric
                                  * COALESCE(uf.cartridge_list_unit_cents, 0) * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       ELSE NULL END AS "annual_value_cents?",
                  s.order_line_id,
                  s.assigned_to,
                  au.name AS "assignee_name?",
                  s.outcome,
                  s.dismissed_reason,
                  s.opened_at,
                  s.assigned_at,
                  s.actioned_at,
                  s.dismissed_at,
                  s.expired_at
           FROM signals s
           JOIN accounts a ON a.id = s.account_id
           JOIN territories t ON t.id = a.territory_id
           CROSS JOIN signal_policy sp
           LEFT JOIN v_unit_facts uf ON uf.unit_id = s.installed_unit_id
           LEFT JOIN sites os ON os.id = s.site_id
           LEFT JOIN users au ON au.id = s.assigned_to
           LEFT JOIN LATERAL (
               SELECT p.id AS product_id, p.sku, p.list_price_cents
               FROM products p
               WHERE s.type = 'conquest'
                 AND p.kind = 'consumable'
                 AND p.filter_fits @> ARRAY[uf.unit_family]
               ORDER BY p.list_price_cents DESC, p.sku ASC
               LIMIT 1
           ) bf ON true
           WHERE s.account_id = $1
           ORDER BY (s.status IN ('open', 'assigned')) DESC, s.score DESC, s.id ASC
           LIMIT $2"#,
        account_id,
        cap
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

/// One signal by id under the caller's RLS — the post-write reload. None →
/// invisible → 404 at the caller.
async fn load_signal(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<SignalRow>, ApiError> {
    let row = sqlx::query_as!(
        SignalRow,
        r#"SELECT s.id,
                  s.type AS "signal_type: SignalType",
                  s.status AS "status: SignalStatus",
                  s.score::float8 AS "score!",
                  s.reasons,
                  s.account_id,
                  a.name AS account_name,
                  t.code AS "territory_code!",
                  s.site_id,
                  COALESCE(uf.site_label, os.city || ', ' || os.state) AS site_label,
                  s.installed_unit_id,
                  uf.serial AS "serial?",
                  CASE WHEN s.type = 'conquest' THEN bf.product_id
                       ELSE uf.cartridge_product_id END AS "cartridge_product_id?",
                  CASE WHEN s.type = 'conquest' THEN bf.sku
                       ELSE uf.cartridge_sku END AS "cartridge_sku?",
                  uf.cartridge_count AS "cartridge_count?",
                  CASE WHEN s.type = 'conquest'
                       THEN round(uf.cartridge_count::numeric * bf.list_price_cents * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       WHEN s.installed_unit_id IS NOT NULL
                       THEN round(uf.cartridge_count::numeric
                                  * COALESCE(uf.cartridge_list_unit_cents, 0) * 12
                                  / COALESCE(uf.expected_changeout_months,
                                             sp.conquest_default_changeout_months))::bigint
                       ELSE NULL END AS "annual_value_cents?",
                  s.order_line_id,
                  s.assigned_to,
                  au.name AS "assignee_name?",
                  s.outcome,
                  s.dismissed_reason,
                  s.opened_at,
                  s.assigned_at,
                  s.actioned_at,
                  s.dismissed_at,
                  s.expired_at
           FROM signals s
           JOIN accounts a ON a.id = s.account_id
           JOIN territories t ON t.id = a.territory_id
           CROSS JOIN signal_policy sp
           LEFT JOIN v_unit_facts uf ON uf.unit_id = s.installed_unit_id
           LEFT JOIN sites os ON os.id = s.site_id
           LEFT JOIN users au ON au.id = s.assigned_to
           LEFT JOIN LATERAL (
               SELECT p.id AS product_id, p.sku, p.list_price_cents
               FROM products p
               WHERE s.type = 'conquest'
                 AND p.kind = 'consumable'
                 AND p.filter_fits @> ARRAY[uf.unit_family]
               ORDER BY p.list_price_cents DESC, p.sku ASC
               LIMIT 1
           ) bf ON true
           WHERE s.id = $1"#,
        id
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

// ── GET /api/signals/summary — Command's feed (disclosed addition) ──────────

#[derive(Serialize)]
pub struct SummaryByType {
    reorder_due: i64,
    defection_risk: i64,
    conquest: i64,
    discount_anomaly: i64,
}

#[derive(Serialize)]
pub struct TerritorySignalCount {
    territory_id: Uuid,
    territory_code: String,
    open_count: i64,
}

#[derive(Serialize)]
pub struct SignalsSummary {
    total: i64,
    by_type: SummaryByType,
    territories: Vec<TerritorySignalCount>,
}

pub async fn signals_summary(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<SignalsSummary>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;

    // "Open" for the KPI = the active set (open ∪ assigned): a card someone
    // is working is still on the radar; only actioned/dismissed leave it.
    let counts = sqlx::query!(
        r#"SELECT count(*) AS "total!",
                  count(*) FILTER (WHERE type = 'reorder_due')      AS "reorder_due!",
                  count(*) FILTER (WHERE type = 'defection_risk')   AS "defection_risk!",
                  count(*) FILTER (WHERE type = 'conquest')         AS "conquest!",
                  count(*) FILTER (WHERE type = 'discount_anomaly') AS "discount_anomaly!"
           FROM signals WHERE status IN ('open', 'assigned')"#
    )
    .fetch_one(&mut *tx)
    .await?;

    let territories = sqlx::query_as!(
        TerritorySignalCount,
        r#"SELECT t.id AS territory_id, t.code AS "territory_code!",
                  count(*) AS "open_count!"
           FROM signals s
           JOIN accounts a ON a.id = s.account_id
           JOIN territories t ON t.id = a.territory_id
           WHERE s.status IN ('open', 'assigned')
           GROUP BY t.id, t.code
           ORDER BY t.code"#
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(SignalsSummary {
        total: counts.total,
        by_type: SummaryByType {
            reorder_due: counts.reorder_due,
            defection_risk: counts.defection_risk,
            conquest: counts.conquest,
            discount_anomaly: counts.discount_anomaly,
        },
        territories,
    }))
}

// ── GET /api/signals/assignees?account_id= — the R6 picker feed ─────────────
// Disclosed addition (the P3 "beyond the R-route-list" pattern): R6 mandates
// a scope-valid user picker for RM/VP/admin, and no user directory exists.
// Scoped tightly: the account must be VISIBLE to the caller (RLS 404 — no
// probing foreign teams), and the rows are exactly the users whose
// v_user_scope contains that account's territory — the same predicate the
// assign endpoint enforces, so the picker can never offer an invalid choice.

#[derive(Deserialize)]
pub struct AssigneesParams {
    account_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct AssigneeRow {
    id: Uuid,
    name: String,
    role: UserRole,
}

#[derive(Serialize)]
pub struct AssigneesBody {
    items: Vec<AssigneeRow>,
}

pub async fn signal_assignees(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<AssigneesParams>,
) -> Result<Json<AssigneesBody>, ApiError> {
    let account_id = params
        .account_id
        .ok_or_else(|| ApiError::Invalid("account_id is required".into()))?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // RLS gate: an invisible account is a 404, exactly like the 360.
    let visible: Option<Uuid> =
        sqlx::query_scalar!(r#"SELECT id FROM accounts WHERE id = $1"#, account_id)
            .fetch_optional(&mut *tx)
            .await?;
    if visible.is_none() {
        return Err(ApiError::NotFound);
    }

    let items = sqlx::query_as!(
        AssigneeRow,
        r#"SELECT u.id, u.name, u.role AS "role: UserRole"
           FROM users u
           WHERE EXISTS (
               SELECT 1 FROM v_user_scope vs
               JOIN accounts a ON a.territory_id = vs.territory_id
               WHERE vs.user_id = u.id AND a.id = $1
           )
           ORDER BY u.name, u.id"#,
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(AssigneesBody { items }))
}

// ── the write-backs (R5) ────────────────────────────────────────────────────

/// Load the signal's lifecycle fields under RLS (404 if invisible) and refuse
/// terminal states (422) — shared by all three writes.
async fn load_active_signal(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(SignalStatus, Uuid), ApiError> {
    let row = sqlx::query!(
        r#"SELECT status AS "status: SignalStatus", account_id FROM signals WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    match row.status {
        SignalStatus::Open | SignalStatus::Assigned => Ok((row.status, row.account_id)),
        SignalStatus::Actioned | SignalStatus::Dismissed => Err(ApiError::Invalid(
            "an actioned or dismissed signal is terminal".into(),
        )),
        // R4: expired cards belong to the machine — no human write-backs.
        // The generator reopens the card itself if its predicate returns.
        SignalStatus::Expired => Err(ApiError::Invalid(
            "an expired signal is closed — the generator reopens it if its predicate returns"
                .into(),
        )),
    }
}

#[derive(Deserialize)]
pub struct AssignBody {
    assignee_id: Uuid,
}

pub async fn assign_signal(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
    body: Result<Json<AssignBody>, JsonRejection>,
) -> Result<Json<SignalRow>, ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    let mut tx = rls_tx(&state.pool, &user).await?;
    let (_, account_id) = load_active_signal(&mut tx, id).await?;

    // The assignee must carry the signal's territory in v_user_scope — the
    // same scope predicate RLS enforces on reads (422 otherwise, R5).
    let in_scope: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM v_user_scope vs
               JOIN accounts a ON a.territory_id = vs.territory_id
               WHERE vs.user_id = $1 AND a.id = $2
           ) AS "e!""#,
        body.assignee_id,
        account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !in_scope {
        return Err(ApiError::Invalid(
            "assignee is not in this signal's territory scope".into(),
        ));
    }

    // Re-assign allowed; assigned_at records the FIRST assignment (R5).
    sqlx::query!(
        r#"UPDATE signals
           SET status = 'assigned', assigned_to = $2,
               assigned_at = COALESCE(assigned_at, now())
           WHERE id = $1"#,
        id,
        body.assignee_id
    )
    .execute(&mut *tx)
    .await?;

    let row = load_signal(&mut tx, id).await?.ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct ActionBody {
    outcome: Option<String>,
}

pub async fn action_signal(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
    body: Result<Json<ActionBody>, JsonRejection>,
) -> Result<Json<SignalRow>, ApiError> {
    let outcome = body
        .ok()
        .and_then(|Json(b)| b.outcome)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::Invalid("action requires an outcome".into()))?;

    let mut tx = rls_tx(&state.pool, &user).await?;
    load_active_signal(&mut tx, id).await?;

    sqlx::query!(
        r#"UPDATE signals
           SET status = 'actioned', outcome = $2, actioned_at = now()
           WHERE id = $1"#,
        id,
        outcome
    )
    .execute(&mut *tx)
    .await?;

    let row = load_signal(&mut tx, id).await?.ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct DismissBody {
    reason: Option<String>,
}

pub async fn dismiss_signal(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
    body: Result<Json<DismissBody>, JsonRejection>,
) -> Result<Json<SignalRow>, ApiError> {
    let reason = body
        .ok()
        .and_then(|Json(b)| b.reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::Invalid("dismiss requires a reason".into()))?;

    let mut tx = rls_tx(&state.pool, &user).await?;
    load_active_signal(&mut tx, id).await?;

    sqlx::query!(
        r#"UPDATE signals
           SET status = 'dismissed', dismissed_reason = $2, dismissed_at = now()
           WHERE id = $1"#,
        id,
        reason
    )
    .execute(&mut *tx)
    .await?;

    let row = load_signal(&mut tx, id).await?.ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(row))
}

// ── POST /api/admin/generate-signals — the R2 trigger surface ───────────────
// The refresh-rollups handler pattern exactly: 401 from the extractor, 403
// for any non-admin role, then the invoker-rights SQL function inside the
// ADMIN'S rls_tx (their v_user_scope is every territory, so the generators
// see — and may write — the whole world while remaining under RLS).

#[derive(Serialize)]
pub struct GeneratedType {
    signal_type: String,
    inserted: i64,
    updated: i64,
    /// P5 (R4): open cards of this type whose predicate stopped holding on
    /// this run — auto-expired, never touching human-owned statuses.
    expired: i64,
}

#[derive(Serialize)]
pub struct GenerateBody {
    generated: Vec<GeneratedType>,
}

pub async fn generate_signals(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<GenerateBody>, ApiError> {
    if user.role != UserRole::Admin {
        return Err(ApiError::Forbidden);
    }

    let mut tx = rls_tx(&state.pool, &user).await?;
    let rows = sqlx::query!(
        r#"SELECT signal_type AS "signal_type!", inserted AS "inserted!",
                  updated AS "updated!", expired AS "expired!"
           FROM generate_signals()"#
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(GenerateBody {
        generated: rows
            .into_iter()
            .map(|r| GeneratedType {
                signal_type: r.signal_type,
                inserted: r.inserted,
                updated: r.updated,
                expired: r.expired,
            })
            .collect(),
    }))
}
