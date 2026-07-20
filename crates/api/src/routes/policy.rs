//! GET /api/policy/discount — the two discount thresholds (R10).
//!
//! The client computes the builder's LIVE verdict from this server truth; the
//! submit handler recomputes server-side regardless (client verdict is advisory
//! UI, the server verdict is law). Auth-guarded; the policy is global config,
//! not tenant data, so it is not RLS-scoped.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DiscountPolicyBody {
    self_max_pct: f64,
    manager_max_pct: f64,
}

pub async fn discount_policy(
    State(state): State<AppState>,
    _user: SessionUser,
) -> Result<Json<DiscountPolicyBody>, ApiError> {
    let row = sqlx::query!(
        r#"SELECT self_max_pct::float8 AS "self_max!",
                  manager_max_pct::float8 AS "manager_max!"
           FROM discount_policy LIMIT 1"#
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::Internal)?;

    Ok(Json(DiscountPolicyBody {
        self_max_pct: row.self_max,
        manager_max_pct: row.manager_max,
    }))
}
