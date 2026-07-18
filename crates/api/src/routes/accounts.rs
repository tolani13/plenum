//! GET /api/accounts — the one CRM endpoint P0 ships (P0-2 needs it).
//!
//! There is NO territory filtering in this handler, on purpose. The query is
//! a plain SELECT; the row count differing per caller is Postgres RLS doing
//! its job through the rls_tx transaction. That is the point of the phase.

use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use domain::AccountStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
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
