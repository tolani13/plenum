//! Opportunities: list (filterable by stage), create, and the stage machine.
//!
//!   GET   /api/opportunities            — list, ?stage=, paginated (R8)
//!   POST  /api/opportunities            — create (stage=lead; territory derived)
//!   PATCH /api/opportunities/:id/stage   — the R1 stage machine, incl. Won-books
//!
//! Every read/write runs in rls_tx: an opportunity the caller cannot see is a
//! 404, a cross-scope create fails WITH CHECK (pre-checked to a clean typed
//! error). The stage machine: free movement among lead/qualified/quoted/
//! negotiation; → won is gated on an approved quote and BOOKS A REAL ORDER
//! (R1); → lost needs a reason; won/lost are terminal.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::NaiveDate;
use domain::{OppStage, UserRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::parse_page;
use crate::state::AppState;

use sqlx::{Postgres, Transaction};

/// The opportunity kinds the ontology names (spec §3). The column is free text;
/// the handler validates so garbage becomes a 422, not a mystery card.
const OPP_KINDS: [&str; 3] = ["capital", "retrofit", "filter-program"];

/// Valid stage tokens (for the ?stage= filter and the PATCH target).
fn parse_stage(raw: &str) -> Result<OppStage, ApiError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "lead" => Ok(OppStage::Lead),
        "qualified" => Ok(OppStage::Qualified),
        "quoted" => Ok(OppStage::Quoted),
        "negotiation" => Ok(OppStage::Negotiation),
        "won" => Ok(OppStage::Won),
        "lost" => Ok(OppStage::Lost),
        _ => Err(ApiError::Invalid(
            "stage must be one of lead, qualified, quoted, negotiation, won, lost".into(),
        )),
    }
}

fn stage_text(stage: OppStage) -> &'static str {
    match stage {
        OppStage::Lead => "lead",
        OppStage::Qualified => "qualified",
        OppStage::Quoted => "quoted",
        OppStage::Negotiation => "negotiation",
        OppStage::Won => "won",
        OppStage::Lost => "lost",
    }
}

#[derive(Serialize)]
pub struct OppRow {
    id: Uuid,
    account_id: Uuid,
    account_name: String,
    territory_code: String,
    owner_id: Uuid,
    owner_name: String,
    stage: OppStage,
    kind: String,
    amount_cents: i64,
    expected_close: Option<NaiveDate>,
    lost_reason: Option<String>,
    has_approved_quote: bool,
    /// Net total of the most-recent approved quote — what a drag-to-Won books.
    approved_quote_net_cents: Option<i64>,
}

#[derive(Serialize)]
pub struct OppsPage {
    items: Vec<OppRow>,
    limit: i64,
    offset: i64,
    total: i64,
}

#[derive(Deserialize)]
pub struct OppListParams {
    stage: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

/// Load one opportunity row (with the approved-quote affordance) under RLS.
async fn load_opp(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<OppRow>, ApiError> {
    let row = sqlx::query!(
        r#"SELECT o.id, o.account_id, a.name AS account_name, t.code AS "territory_code!",
                  o.owner_id, u.name AS owner_name,
                  o.stage AS "stage: OppStage", o.kind, o.amount_cents,
                  o.expected_close, o.lost_reason,
                  aq.net_cents AS approved_quote_net_cents
           FROM opportunities o
           JOIN accounts a ON a.id = o.account_id
           JOIN territories t ON t.id = o.territory_id
           JOIN users u ON u.id = o.owner_id
           LEFT JOIN LATERAL (
             SELECT (SELECT COALESCE(SUM(ql.net_unit_cents * ql.qty), 0)::bigint
                     FROM quote_lines ql WHERE ql.quote_id = q.id) AS net_cents
             FROM quotes q
             WHERE q.opportunity_id = o.id AND q.status = 'approved'
             ORDER BY q.decided_at DESC NULLS LAST, q.created_at DESC, q.id DESC
             LIMIT 1
           ) aq ON true
           WHERE o.id = $1"#,
        id
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|r| OppRow {
        id: r.id,
        account_id: r.account_id,
        account_name: r.account_name,
        territory_code: r.territory_code,
        owner_id: r.owner_id,
        owner_name: r.owner_name,
        stage: r.stage,
        kind: r.kind,
        amount_cents: r.amount_cents,
        expected_close: r.expected_close,
        lost_reason: r.lost_reason,
        has_approved_quote: r.approved_quote_net_cents.is_some(),
        approved_quote_net_cents: r.approved_quote_net_cents,
    }))
}

pub async fn list_opportunities(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<OppListParams>,
) -> Result<Json<OppsPage>, ApiError> {
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let stage_filter = match params.stage {
        None => None,
        Some(s) => Some(stage_text(parse_stage(&s)?).to_string()),
    };

    let mut tx = rls_tx(&state.pool, &user).await?;

    let rows = sqlx::query!(
        r#"SELECT o.id, o.account_id, a.name AS account_name, t.code AS "territory_code!",
                  o.owner_id, u.name AS owner_name,
                  o.stage AS "stage: OppStage", o.kind, o.amount_cents,
                  o.expected_close, o.lost_reason,
                  aq.net_cents AS approved_quote_net_cents
           FROM opportunities o
           JOIN accounts a ON a.id = o.account_id
           JOIN territories t ON t.id = o.territory_id
           JOIN users u ON u.id = o.owner_id
           LEFT JOIN LATERAL (
             SELECT (SELECT COALESCE(SUM(ql.net_unit_cents * ql.qty), 0)::bigint
                     FROM quote_lines ql WHERE ql.quote_id = q.id) AS net_cents
             FROM quotes q
             WHERE q.opportunity_id = o.id AND q.status = 'approved'
             ORDER BY q.decided_at DESC NULLS LAST, q.created_at DESC, q.id DESC
             LIMIT 1
           ) aq ON true
           WHERE ($1::text IS NULL OR o.stage::text = $1)
           ORDER BY o.amount_cents DESC, o.id
           LIMIT $2 OFFSET $3"#,
        stage_filter,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM opportunities o
           WHERE ($1::text IS NULL OR o.stage::text = $1)"#,
        stage_filter
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let items = rows
        .into_iter()
        .map(|r| OppRow {
            id: r.id,
            account_id: r.account_id,
            account_name: r.account_name,
            territory_code: r.territory_code,
            owner_id: r.owner_id,
            owner_name: r.owner_name,
            stage: r.stage,
            kind: r.kind,
            amount_cents: r.amount_cents,
            expected_close: r.expected_close,
            lost_reason: r.lost_reason,
            has_approved_quote: r.approved_quote_net_cents.is_some(),
            approved_quote_net_cents: r.approved_quote_net_cents,
        })
        .collect();

    Ok(Json(OppsPage {
        items,
        limit,
        offset,
        total,
    }))
}

// ── POST /api/opportunities ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateOppBody {
    account_id: Uuid,
    kind: String,
    amount_cents: i64,
    expected_close: Option<NaiveDate>,
    /// vp/admin may assign an owner; anyone else owns what they create.
    owner_id: Option<Uuid>,
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<CreateOppBody>, JsonRejection>,
) -> Result<(StatusCode, Json<OppRow>), ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    let kind = body.kind.trim().to_ascii_lowercase();
    if !OPP_KINDS.contains(&kind.as_str()) {
        return Err(ApiError::Invalid(
            "kind must be one of capital, retrofit, filter-program".into(),
        ));
    }
    if body.amount_cents < 0 {
        return Err(ApiError::Invalid("amount_cents must be >= 0".into()));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Account must be visible (RLS) — else 404. Its territory becomes the opp's.
    let account = sqlx::query!(
        r#"SELECT territory_id FROM accounts WHERE id = $1"#,
        body.account_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    // Owner assignment: default to the caller; only vp/admin may name another.
    let owner_id = match body.owner_id {
        None => user.id,
        Some(oid) => {
            if !matches!(user.role, UserRole::Vp | UserRole::Admin) {
                return Err(ApiError::Forbidden);
            }
            let exists: bool =
                sqlx::query_scalar!(r#"SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) AS "e!""#, oid)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                return Err(ApiError::Invalid("unknown owner_id".into()));
            }
            oid
        }
    };

    let created = sqlx::query!(
        r#"INSERT INTO opportunities
             (account_id, territory_id, owner_id, stage, kind, amount_cents, expected_close)
           VALUES ($1, $2, $3, 'lead', $4, $5, $6)
           RETURNING id"#,
        body.account_id,
        account.territory_id,
        owner_id,
        kind,
        body.amount_cents,
        body.expected_close
    )
    .fetch_one(&mut *tx)
    .await?;

    let row = load_opp(&mut tx, created.id)
        .await?
        .ok_or(ApiError::Internal)?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(row)))
}

// ── PATCH /api/opportunities/:id/stage — the R1 stage machine ───────────────

#[derive(Deserialize)]
pub struct StageBody {
    stage: String,
    lost_reason: Option<String>,
}

#[derive(Serialize)]
pub struct BookedOrder {
    id: Uuid,
    ordered_on: NaiveDate,
    gross_cents: i64,
    net_cents: i64,
}

#[derive(Serialize)]
pub struct StageResult {
    opportunity: OppRow,
    /// Present only when the transition was → won (R1 booking).
    booked_order: Option<BookedOrder>,
    accepted_quote_id: Option<Uuid>,
}

pub async fn patch_stage(
    State(state): State<AppState>,
    user: SessionUser,
    Path(id): Path<Uuid>,
    body: Result<Json<StageBody>, JsonRejection>,
) -> Result<Json<StageResult>, ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;
    let target = parse_stage(&body.stage)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Load the opp under RLS (404 if invisible) — capture current stage/account.
    let opp = sqlx::query!(
        r#"SELECT stage AS "stage: OppStage", account_id, owner_id, territory_id
           FROM opportunities WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    // won/lost are terminal — any transition out is a 422.
    if matches!(opp.stage, OppStage::Won | OppStage::Lost) {
        return Err(ApiError::Invalid(format!(
            "{} is terminal",
            stage_text(opp.stage)
        )));
    }

    let ctx = OppContext {
        account_id: opp.account_id,
        owner_id: opp.owner_id,
        territory_id: opp.territory_id,
    };

    let mut booked_order = None;
    let mut accepted_quote_id = None;

    match target {
        OppStage::Lead | OppStage::Qualified | OppStage::Quoted | OppStage::Negotiation => {
            // Bind the stage as text and cast to the enum ($2::text forces the
            // param OID to text so the sqlx macro can map it — a plain
            // $2::opp_stage has no built-in Rust mapping).
            sqlx::query!(
                r#"UPDATE opportunities SET stage = $2::text::opp_stage WHERE id = $1"#,
                id,
                stage_text(target)
            )
            .execute(&mut *tx)
            .await?;
        }
        OppStage::Lost => {
            let reason = body
                .lost_reason
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ApiError::Invalid("lost requires a lost_reason".into()))?;
            sqlx::query!(
                r#"UPDATE opportunities
                   SET stage = 'lost', lost_reason = $2 WHERE id = $1"#,
                id,
                reason
            )
            .execute(&mut *tx)
            .await?;
        }
        OppStage::Won => {
            let (order, quote_id) = book_won(&mut tx, id, &ctx).await?;
            booked_order = Some(order);
            accepted_quote_id = Some(quote_id);
            sqlx::query!(r#"UPDATE opportunities SET stage = 'won' WHERE id = $1"#, id)
                .execute(&mut *tx)
                .await?;
        }
    }

    let opportunity = load_opp(&mut tx, id).await?.ok_or(ApiError::Internal)?;
    tx.commit().await?;

    Ok(Json(StageResult {
        opportunity,
        booked_order,
        accepted_quote_id,
    }))
}

/// R1 booking. Copies the most-recent approved quote's lines VERBATIM (the
/// price triplet passes the order_lines CHECK by construction) into a new order
/// dated today; rep_id follows opportunity ownership; site is the account's
/// MIN(id) site; the consumed quote flips approved → accepted. All inside the
/// caller's rls_tx — atomic, and every write is territory-scoped by RLS.
async fn book_won(
    tx: &mut Transaction<'_, Postgres>,
    opp_id: Uuid,
    opp: &OppContext,
) -> Result<(BookedOrder, Uuid), ApiError> {
    // Most-recent approved quote on this opp (RLS-scoped). None → 422.
    let quote = sqlx::query!(
        r#"SELECT q.id FROM quotes q
           WHERE q.opportunity_id = $1 AND q.status = 'approved'
           ORDER BY q.decided_at DESC NULLS LAST, q.created_at DESC, q.id DESC
           LIMIT 1"#,
        opp_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| ApiError::Invalid("won requires an approved quote".into()))?;

    // The account's MIN(id) site (stable; single-site accounts unambiguous).
    let site_id: Uuid = sqlx::query_scalar!(
        r#"SELECT s.id FROM sites s
           JOIN accounts a ON a.id = s.account_id
           WHERE a.id = $1 ORDER BY s.id LIMIT 1"#,
        opp.account_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| ApiError::Invalid("account has no site to book against".into()))?;

    // New order — rep_id = opportunity owner (attribution follows ownership).
    let order = sqlx::query!(
        r#"INSERT INTO orders (account_id, site_id, territory_id, rep_id, ordered_on)
           VALUES ($1, $2, $3, $4, CURRENT_DATE)
           RETURNING id, ordered_on"#,
        opp.account_id,
        site_id,
        opp.territory_id,
        opp.owner_id
    )
    .fetch_one(&mut **tx)
    .await?;

    // Copy the approved quote's lines verbatim (join through quotes for RLS).
    sqlx::query!(
        r#"INSERT INTO order_lines
             (order_id, product_id, qty, list_unit_cents, net_unit_cents, discount_pct)
           SELECT $1, ql.product_id, ql.qty, ql.list_unit_cents, ql.net_unit_cents, ql.discount_pct
           FROM quote_lines ql
           JOIN quotes q ON q.id = ql.quote_id
           WHERE ql.quote_id = $2"#,
        order.id,
        quote.id
    )
    .execute(&mut **tx)
    .await?;

    // The consumed quote flips approved → accepted (audit trigger records it).
    sqlx::query!(
        r#"UPDATE quotes SET status = 'accepted', decided_at = now()
           WHERE id = $1 AND status = 'approved'"#,
        quote.id
    )
    .execute(&mut **tx)
    .await?;

    // Order totals for the toast (Σ of the just-written lines).
    let totals = sqlx::query!(
        r#"SELECT COALESCE(SUM(list_unit_cents * qty), 0)::bigint AS "gross!",
                  COALESCE(SUM(net_unit_cents * qty), 0)::bigint  AS "net!"
           FROM order_lines WHERE order_id = $1"#,
        order.id
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok((
        BookedOrder {
            id: order.id,
            ordered_on: order.ordered_on,
            gross_cents: totals.gross,
            net_cents: totals.net,
        },
        quote.id,
    ))
}

/// The stage-machine's view of the opp row it loaded (avoids re-querying inside
/// book_won).
pub struct OppContext {
    pub account_id: Uuid,
    pub owner_id: Uuid,
    pub territory_id: Uuid,
}
