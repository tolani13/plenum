//! Activities: the write-back log (spec §3 "log activity").
//!
//!   GET  /api/activities?account_id=  — list (RLS-scoped), paginated (R8)
//!   POST /api/activities              — log a call/visit/email/note
//!
//! Both run under RLS: an activity for an account the caller cannot see never
//! appears (GET) and cannot be written (POST — pre-checked to a clean 404
//! instead of a WITH CHECK 500). occurred_at defaults to now; body is required.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use domain::ActivityKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::parse_page;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ActivityRow {
    id: Uuid,
    account_id: Uuid,
    kind: ActivityKind,
    body: String,
    occurred_at: DateTime<Utc>,
    rep_id: Uuid,
    rep_name: String,
}

#[derive(Serialize)]
pub struct ActivitiesPage {
    items: Vec<ActivityRow>,
    limit: i64,
    offset: i64,
    total: i64,
}

#[derive(Deserialize)]
pub struct ActivityListParams {
    account_id: Option<Uuid>,
    limit: Option<String>,
    offset: Option<String>,
}

pub async fn list_activities(
    State(state): State<AppState>,
    user: SessionUser,
    Query(params): Query<ActivityListParams>,
) -> Result<Json<ActivitiesPage>, ApiError> {
    let (limit, offset) = parse_page(params.limit, params.offset)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    let items = sqlx::query_as!(
        ActivityRow,
        r#"SELECT a.id, a.account_id, a.kind AS "kind: ActivityKind", a.body,
                  a.occurred_at, a.rep_id, u.name AS rep_name
           FROM activities a JOIN users u ON u.id = a.rep_id
           WHERE ($1::uuid IS NULL OR a.account_id = $1)
           ORDER BY a.occurred_at DESC, a.id DESC
           LIMIT $2 OFFSET $3"#,
        params.account_id,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM activities a
           WHERE ($1::uuid IS NULL OR a.account_id = $1)"#,
        params.account_id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ActivitiesPage {
        items,
        limit,
        offset,
        total,
    }))
}

#[derive(Deserialize)]
pub struct CreateActivityBody {
    account_id: Uuid,
    kind: ActivityKind,
    body: String,
    occurred_at: Option<DateTime<Utc>>,
}

pub async fn create_activity(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<CreateActivityBody>, JsonRejection>,
) -> Result<(StatusCode, Json<ActivityRow>), ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    if body.body.trim().is_empty() {
        return Err(ApiError::Invalid("body is required".into()));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // The account must be visible (RLS) — a clean 404 instead of a WITH CHECK
    // 500 when a rep aims at a foreign account.
    let visible: Option<Uuid> =
        sqlx::query_scalar!(r#"SELECT id FROM accounts WHERE id = $1"#, body.account_id)
            .fetch_optional(&mut *tx)
            .await?;
    if visible.is_none() {
        return Err(ApiError::NotFound);
    }

    let occurred_at = body.occurred_at.unwrap_or_else(Utc::now);
    // Bind kind as text and cast (a plain enum param has no built-in Rust
    // mapping in the sqlx macro; $3::text pins the param to text).
    let kind_text = match body.kind {
        ActivityKind::Call => "call",
        ActivityKind::Visit => "visit",
        ActivityKind::Email => "email",
        ActivityKind::Note => "note",
    };

    let row = sqlx::query_as!(
        ActivityRow,
        r#"WITH ins AS (
             INSERT INTO activities (account_id, rep_id, kind, occurred_at, body)
             VALUES ($1, $2, $3::text::activity_kind, $4, $5)
             RETURNING id, account_id, rep_id, kind, occurred_at, body
           )
           SELECT ins.id, ins.account_id, ins.kind AS "kind: ActivityKind", ins.body,
                  ins.occurred_at, ins.rep_id, u.name AS rep_name
           FROM ins JOIN users u ON u.id = ins.rep_id"#,
        body.account_id,
        user.id,
        kind_text,
        occurred_at,
        body.body.trim()
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(row)))
}
