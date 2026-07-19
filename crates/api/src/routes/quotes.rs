//! Quotes: the draft → approval state machine, the money law, and the audit
//! read. (Ruling R3, R4, R10, and the P0 money law.)
//!
//!   GET  /api/quotes?view=mine|approvals   — list (R8)
//!   POST /api/quotes                        — create draft (server derives prices)
//!   GET  /api/quotes/:id                     — detail (header + lines + verdict)
//!   POST /api/quotes/:id/submit              — R3 verdict (draft → approved|pending)
//!   POST /api/quotes/:id/approve             — R3 role gate (pending → approved)
//!   POST /api/quotes/:id/reject              — R3 role gate + reason (pending → rejected)
//!   GET  /api/quotes/:id/audit               — R4 (scoped through the quote)
//!
//! State machine (every other edge is 422):
//!   draft → pending_approval | approved   (submit)
//!   pending_approval → approved | rejected (role-gated)
//!   approved → accepted  (ONLY via the R1 booking in opportunities.rs)
//!   rejected / accepted  terminal
//!
//! Money law: the client sends product_id/qty/discount_pct only. The server
//! derives list_unit_cents from products.list_price_cents and computes
//! net_unit_cents IN SQL with the EXACT CHECK expression, so a client-supplied
//! price is ignored and rounding can never trip the constraint.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use domain::{ApprovalTier, DiscountPolicy, QuoteStatus, UserRole};
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::MidpointAwayFromZero;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::parse_page;
use crate::state::AppState;

// ── shared shapes ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct QuoteLineOut {
    id: Uuid,
    product_id: Uuid,
    product_sku: String,
    product_name: String,
    family: String,
    qty: i32,
    list_unit_cents: i64,
    net_unit_cents: i64,
    discount_pct: f64,
    line_gross_cents: i64,
    line_net_cents: i64,
}

#[derive(Serialize)]
pub struct QuoteDetail {
    id: Uuid,
    opportunity_id: Uuid,
    account_id: Uuid,
    account_name: String,
    territory_code: String,
    status: QuoteStatus,
    created_by: Uuid,
    created_by_name: String,
    approver_id: Option<Uuid>,
    approver_name: Option<String>,
    created_at: DateTime<Utc>,
    submitted_at: Option<DateTime<Utc>>,
    decided_at: Option<DateTime<Utc>>,
    decision_reason: Option<String>,
    discount_policy_result: Option<Value>,
    worst_discount_pct: Option<f64>,
    gross_cents: i64,
    net_cents: i64,
    leakage_cents: i64,
    lines: Vec<QuoteLineOut>,
}

/// Load the full quote (header + lines) under RLS. None → the quote is invisible
/// to the caller (404). Every line read joins quotes for the RLS parent scope.
async fn load_quote_detail(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<QuoteDetail>, ApiError> {
    let Some(h) = sqlx::query!(
        r#"SELECT q.id, q.opportunity_id, o.account_id, a.name AS account_name,
                  t.code AS "territory_code!", q.status AS "status: QuoteStatus",
                  q.created_by, cu.name AS created_by_name,
                  q.approver_id, au.name AS "approver_name?",
                  q.created_at, q.submitted_at, q.decided_at, q.decision_reason,
                  q.discount_policy_result,
                  agg.worst::float8 AS worst_discount_pct,
                  agg.gross AS "gross_cents!", agg.net AS "net_cents!"
           FROM quotes q
           JOIN opportunities o ON o.id = q.opportunity_id
           JOIN accounts a ON a.id = o.account_id
           JOIN territories t ON t.id = o.territory_id
           JOIN users cu ON cu.id = q.created_by
           LEFT JOIN users au ON au.id = q.approver_id
           LEFT JOIN LATERAL (
             SELECT max(ql.discount_pct) AS worst,
                    COALESCE(SUM(ql.list_unit_cents * ql.qty), 0)::bigint AS gross,
                    COALESCE(SUM(ql.net_unit_cents  * ql.qty), 0)::bigint AS net
             FROM quote_lines ql
             JOIN quotes qv ON qv.id = ql.quote_id
             WHERE ql.quote_id = q.id
           ) agg ON true
           WHERE q.id = $1"#,
        id
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };

    let lines = sqlx::query!(
        r#"SELECT ql.id, ql.product_id, p.sku AS product_sku, p.name AS product_name,
                  p.family, ql.qty, ql.list_unit_cents, ql.net_unit_cents,
                  ql.discount_pct::float8 AS "discount_pct!"
           FROM quote_lines ql
           JOIN quotes q ON q.id = ql.quote_id
           JOIN products p ON p.id = ql.product_id
           WHERE ql.quote_id = $1
           ORDER BY ql.id"#,
        id
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|l| QuoteLineOut {
        id: l.id,
        product_id: l.product_id,
        product_sku: l.product_sku,
        product_name: l.product_name,
        family: l.family,
        qty: l.qty,
        list_unit_cents: l.list_unit_cents,
        net_unit_cents: l.net_unit_cents,
        discount_pct: l.discount_pct,
        line_gross_cents: l.list_unit_cents * l.qty as i64,
        line_net_cents: l.net_unit_cents * l.qty as i64,
    })
    .collect();

    Ok(Some(QuoteDetail {
        id: h.id,
        opportunity_id: h.opportunity_id,
        account_id: h.account_id,
        account_name: h.account_name,
        territory_code: h.territory_code,
        status: h.status,
        created_by: h.created_by,
        created_by_name: h.created_by_name,
        approver_id: h.approver_id,
        approver_name: h.approver_name,
        created_at: h.created_at,
        submitted_at: h.submitted_at,
        decided_at: h.decided_at,
        decision_reason: h.decision_reason,
        discount_policy_result: h.discount_policy_result,
        worst_discount_pct: h.worst_discount_pct,
        gross_cents: h.gross_cents,
        net_cents: h.net_cents,
        leakage_cents: h.gross_cents - h.net_cents,
        lines,
    }))
}

async fn read_policy(tx: &mut Transaction<'_, Postgres>) -> Result<DiscountPolicy, ApiError> {
    let p = sqlx::query!(r#"SELECT self_max_pct, manager_max_pct FROM discount_policy LIMIT 1"#)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(ApiError::Internal)?;
    Ok(DiscountPolicy {
        self_max_pct: p.self_max_pct,
        manager_max_pct: p.manager_max_pct,
    })
}

/// The worst-line discount for a quote (max over lines), under RLS parent join.
async fn worst_discount(
    tx: &mut Transaction<'_, Postgres>,
    quote_id: Uuid,
) -> Result<Decimal, ApiError> {
    let worst = sqlx::query_scalar!(
        r#"SELECT max(ql.discount_pct)
           FROM quote_lines ql JOIN quotes q ON q.id = ql.quote_id
           WHERE ql.quote_id = $1"#,
        quote_id
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(worst.unwrap_or(Decimal::ZERO))
}

fn verdict_json(
    worst: Decimal,
    policy: &DiscountPolicy,
    tier: ApprovalTier,
    outcome_status: &str,
) -> Value {
    json!({
        "verdict": tier.verdict_code(),
        "worst_line_discount_pct": worst.to_string(),
        "self_max_pct": policy.self_max_pct.to_string(),
        "manager_max_pct": policy.manager_max_pct.to_string(),
        "outcome_status": outcome_status,
    })
}

// ── GET /api/quotes?view=mine|approvals ─────────────────────────────────────

#[derive(Serialize)]
pub struct QuoteListRow {
    id: Uuid,
    opportunity_id: Uuid,
    account_name: String,
    status: QuoteStatus,
    worst_discount_pct: Option<f64>,
    created_by_name: String,
    created_at: DateTime<Utc>,
    submitted_at: Option<DateTime<Utc>>,
    gross_cents: i64,
    net_cents: i64,
}

#[derive(Serialize)]
pub struct QuotesPage {
    items: Vec<QuoteListRow>,
    limit: i64,
    offset: i64,
    total: i64,
}

#[derive(Deserialize)]
pub struct QuoteListParams {
    view: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

pub async fn list_quotes(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<QuoteListParams>,
) -> Result<Json<QuotesPage>, ApiError> {
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let view = params.view.as_deref().unwrap_or("mine");
    if view != "mine" && view != "approvals" {
        return Err(ApiError::Invalid(
            "view must be 'mine' or 'approvals'".into(),
        ));
    }
    let role = user.role.as_str();

    let mut tx = rls_tx(&state.pool, &user).await?;

    // The approvals filter: pending quotes the caller's ROLE tier can decide —
    // vp/admin see every pending in scope; a regional_manager sees only
    // manager-tier (worst ≤ manager_max); a rep sees none (they cannot decide).
    let rows = sqlx::query!(
        r#"SELECT q.id, q.opportunity_id, a.name AS account_name,
                  q.status AS "status: QuoteStatus",
                  agg.worst::float8 AS worst_discount_pct,
                  cu.name AS created_by_name, q.created_at, q.submitted_at,
                  agg.gross AS "gross_cents!", agg.net AS "net_cents!"
           FROM quotes q
           JOIN opportunities o ON o.id = q.opportunity_id
           JOIN accounts a ON a.id = o.account_id
           JOIN users cu ON cu.id = q.created_by
           CROSS JOIN discount_policy dp
           LEFT JOIN LATERAL (
             SELECT max(ql.discount_pct) AS worst,
                    COALESCE(SUM(ql.list_unit_cents * ql.qty), 0)::bigint AS gross,
                    COALESCE(SUM(ql.net_unit_cents  * ql.qty), 0)::bigint AS net
             FROM quote_lines ql
             JOIN quotes qv ON qv.id = ql.quote_id
             WHERE ql.quote_id = q.id
           ) agg ON true
           WHERE CASE
             WHEN $1 = 'mine' THEN q.created_by = $2
             WHEN $1 = 'approvals' THEN q.status = 'pending_approval' AND CASE
               WHEN $3 IN ('vp','admin') THEN true
               WHEN $3 = 'regional_manager' THEN COALESCE(agg.worst, 0) <= dp.manager_max_pct
               ELSE false
             END
             ELSE false
           END
           ORDER BY q.created_at DESC, q.id
           LIMIT $4 OFFSET $5"#,
        view,
        user.id,
        role,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
           FROM quotes q
           CROSS JOIN discount_policy dp
           LEFT JOIN LATERAL (
             SELECT max(ql.discount_pct) AS worst
             FROM quote_lines ql JOIN quotes qv ON qv.id = ql.quote_id
             WHERE ql.quote_id = q.id
           ) agg ON true
           WHERE CASE
             WHEN $1 = 'mine' THEN q.created_by = $2
             WHEN $1 = 'approvals' THEN q.status = 'pending_approval' AND CASE
               WHEN $3 IN ('vp','admin') THEN true
               WHEN $3 = 'regional_manager' THEN COALESCE(agg.worst, 0) <= dp.manager_max_pct
               ELSE false
             END
             ELSE false
           END"#,
        view,
        user.id,
        role
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| QuoteListRow {
            id: r.id,
            opportunity_id: r.opportunity_id,
            account_name: r.account_name,
            status: r.status,
            worst_discount_pct: r.worst_discount_pct,
            created_by_name: r.created_by_name,
            created_at: r.created_at,
            submitted_at: r.submitted_at,
            gross_cents: r.gross_cents,
            net_cents: r.net_cents,
        })
        .collect();

    Ok(Json(QuotesPage {
        items,
        limit,
        offset,
        total,
    }))
}

// ── POST /api/quotes — create a draft ───────────────────────────────────────

#[derive(Deserialize)]
pub struct LineInput {
    product_id: Uuid,
    qty: i32,
    /// Percent, 0–100. Extra body fields (a forged list/net price) are IGNORED
    /// — serde drops unknown keys and the server derives every price itself.
    discount_pct: f64,
}

#[derive(Deserialize)]
pub struct CreateQuoteBody {
    opportunity_id: Uuid,
    lines: Vec<LineInput>,
}

pub async fn create_quote(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<CreateQuoteBody>, JsonRejection>,
) -> Result<(StatusCode, Json<QuoteDetail>), ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    if body.lines.is_empty() {
        return Err(ApiError::Invalid("a quote needs at least one line".into()));
    }

    // Validate + round each line's discount to 2dp BEFORE any SQL, so the net
    // computed in SQL uses the exact numeric that gets stored (no rounding
    // drift can trip the CHECK).
    let mut prepared: Vec<(Uuid, i32, Decimal)> = Vec::with_capacity(body.lines.len());
    for line in &body.lines {
        if line.qty <= 0 {
            return Err(ApiError::Invalid("qty must be > 0".into()));
        }
        let disc = Decimal::from_f64_retain(line.discount_pct)
            .ok_or_else(|| ApiError::Invalid("discount_pct must be a number".into()))?
            .round_dp_with_strategy(2, MidpointAwayFromZero);
        if disc < Decimal::ZERO || disc > Decimal::from(100) {
            return Err(ApiError::Invalid(
                "discount_pct must be between 0 and 100".into(),
            ));
        }
        prepared.push((line.product_id, line.qty, disc));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // The opp must be visible (RLS 404) and draftable (lead/qualified/quoted/
    // negotiation) — a won/lost opp cannot take a new quote (422).
    let opp = sqlx::query!(
        r#"SELECT stage::text AS "stage!" FROM opportunities WHERE id = $1"#,
        body.opportunity_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if !matches!(
        opp.stage.as_str(),
        "lead" | "qualified" | "quoted" | "negotiation"
    ) {
        return Err(ApiError::Invalid(
            "cannot draft a quote on a won or lost opportunity".into(),
        ));
    }

    let quote = sqlx::query!(
        r#"INSERT INTO quotes (opportunity_id, status, created_by)
           VALUES ($1, 'draft', $2) RETURNING id"#,
        body.opportunity_id,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Each line: server derives list_unit_cents from the catalog and computes
    // net_unit_cents with the EXACT CHECK expression. A product_id the caller
    // cannot resolve inserts nothing → 422 (never a silent skip).
    for (product_id, qty, disc) in &prepared {
        let inserted = sqlx::query_scalar!(
            r#"INSERT INTO quote_lines
                 (quote_id, product_id, qty, list_unit_cents, net_unit_cents, discount_pct)
               SELECT $1, p.id, $2, p.list_price_cents,
                      round(p.list_price_cents * (100 - $3::numeric) / 100)::bigint,
                      $3::numeric
               FROM products p WHERE p.id = $4
               RETURNING id"#,
            quote.id,
            qty,
            disc,
            product_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if inserted.is_none() {
            return Err(ApiError::Invalid("unknown product in a quote line".into()));
        }
    }

    // Drafting moves the opp to 'quoted' (spec stage meaning) — recorded via
    // the opp audit trigger. No-op if already quoted.
    if opp.stage != "quoted" {
        sqlx::query!(
            r#"UPDATE opportunities SET stage = 'quoted' WHERE id = $1"#,
            body.opportunity_id
        )
        .execute(&mut *tx)
        .await?;
    }

    let detail = load_quote_detail(&mut tx, quote.id)
        .await?
        .ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

// ── GET /api/quotes/:id ─────────────────────────────────────────────────────

pub async fn get_quote(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<QuoteDetail>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;
    let detail = load_quote_detail(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    Ok(Json(detail))
}

// ── POST /api/quotes/:id/submit — the R3 verdict ────────────────────────────

pub async fn submit_quote(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<QuoteDetail>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;

    let quote = sqlx::query!(
        r#"SELECT status AS "status: QuoteStatus", created_by FROM quotes WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    if quote.status != QuoteStatus::Draft {
        return Err(ApiError::Invalid("only a draft can be submitted".into()));
    }
    // Creator or the manager chain (a non-rep in scope) may submit.
    if quote.created_by != user.id && user.role == UserRole::Rep {
        return Err(ApiError::Forbidden);
    }

    let policy = read_policy(&mut tx).await?;
    let worst = worst_discount(&mut tx, id).await?;
    let tier = ApprovalTier::from_worst(worst, &policy);

    match tier {
        ApprovalTier::SelfApprove => {
            let result = verdict_json(worst, &policy, tier, "approved");
            sqlx::query!(
                r#"UPDATE quotes
                   SET status = 'approved', approver_id = created_by,
                       submitted_at = now(), decided_at = now(),
                       discount_policy_result = $2
                   WHERE id = $1"#,
                id,
                result
            )
            .execute(&mut *tx)
            .await?;
        }
        ApprovalTier::Manager | ApprovalTier::Vp => {
            let result = verdict_json(worst, &policy, tier, "pending_approval");
            sqlx::query!(
                r#"UPDATE quotes
                   SET status = 'pending_approval', submitted_at = now(),
                       approver_id = NULL, discount_policy_result = $2
                   WHERE id = $1"#,
                id,
                result
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    let detail = load_quote_detail(&mut tx, id)
        .await?
        .ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(detail))
}

// ── POST /api/quotes/:id/approve · /reject — the R3 role gate ────────────────

async fn decide(
    tx: &mut Transaction<'_, Postgres>,
    user: &SessionUser,
    id: Uuid,
) -> Result<ApprovalTier, ApiError> {
    let quote = sqlx::query!(
        r#"SELECT status AS "status: QuoteStatus" FROM quotes WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    if quote.status != QuoteStatus::PendingApproval {
        return Err(ApiError::Invalid(
            "only a pending quote can be decided".into(),
        ));
    }

    let policy = read_policy(tx).await?;
    let worst = worst_discount(tx, id).await?;
    let tier = ApprovalTier::from_worst(worst, &policy);

    // The role gate — RLS alone would let an in-territory rep approve their own
    // quote, so the handler asks whether the caller's ROLE may decide this tier.
    if !domain::role_can_decide(user.role, tier) {
        return Err(ApiError::Forbidden);
    }
    Ok(tier)
}

pub async fn approve_quote(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<QuoteDetail>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;
    decide(&mut tx, &user, id).await?;

    sqlx::query!(
        r#"UPDATE quotes
           SET status = 'approved', approver_id = $2, decided_at = now(),
               decision_reason = NULL
           WHERE id = $1"#,
        id,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    let detail = load_quote_detail(&mut tx, id)
        .await?
        .ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(detail))
}

#[derive(Deserialize)]
pub struct RejectBody {
    reason: Option<String>,
}

pub async fn reject_quote(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
    body: Result<Json<RejectBody>, JsonRejection>,
) -> Result<Json<QuoteDetail>, ApiError> {
    let reason = body
        .ok()
        .and_then(|Json(b)| b.reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::Invalid("reject requires a reason".into()))?;

    let mut tx = rls_tx(&state.pool, &user).await?;
    decide(&mut tx, &user, id).await?;

    sqlx::query!(
        r#"UPDATE quotes
           SET status = 'rejected', approver_id = $2, decided_at = now(),
               decision_reason = $3
           WHERE id = $1"#,
        id,
        user.id,
        reason
    )
    .execute(&mut *tx)
    .await?;

    let detail = load_quote_detail(&mut tx, id)
        .await?
        .ok_or(ApiError::Internal)?;
    tx.commit().await?;
    Ok(Json(detail))
}

// ── GET /api/quotes/:id/audit — the R4 scoped audit read ────────────────────
//
// Scoped THROUGH the quote: an invisible quote is a 404 (the visibility check
// under RLS), and only that quote's audit rows are returned — no generic
// /api/audit endpoint exists (an unscoped audit feed would leak cross-territory
// entity state through before/after jsonb).

#[derive(Serialize)]
pub struct AuditRow {
    id: Uuid,
    action: String,
    actor_id: Option<Uuid>,
    actor_name: Option<String>,
    at: DateTime<Utc>,
    before_status: Option<String>,
    after_status: Option<String>,
}

#[derive(Serialize)]
pub struct AuditPage {
    items: Vec<AuditRow>,
    total: i64,
}

pub async fn quote_audit(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AuditPage>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;

    // Visibility gate under RLS — invisible quote → 404.
    let visible: Option<Uuid> = sqlx::query_scalar!(r#"SELECT id FROM quotes WHERE id = $1"#, id)
        .fetch_optional(&mut *tx)
        .await?;
    if visible.is_none() {
        return Err(ApiError::NotFound);
    }

    let items = sqlx::query!(
        r#"SELECT al.id, al.action, al.actor, u.name AS "actor_name?", al.at,
                  al.before->>'status' AS before_status,
                  al.after->>'status'  AS after_status
           FROM audit_log al
           LEFT JOIN users u ON u.id = al.actor
           WHERE al.entity = 'quotes' AND al.entity_id = $1
           ORDER BY al.at, al.id"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|r| AuditRow {
        id: r.id,
        action: r.action,
        actor_id: r.actor,
        actor_name: r.actor_name,
        at: r.at,
        before_status: r.before_status,
        after_status: r.after_status,
    })
    .collect::<Vec<_>>();

    tx.commit().await?;
    let total = items.len() as i64;
    Ok(Json(AuditPage { items, total }))
}
