//! POST /api/telemetry/filter-life — the R13 ingest stub, and the TEMPLATE
//! every future inbound feed (sensor push, ERP webhook) copies:
//!
//!   · identity: a real session with role=admin (the integration-feed
//!     identity — Artifact 1's simulator logs in as admin), 401/403 typed;
//!   · validation: 422 for a non-numeric or out-of-range value, with the
//!     bound stated in the message;
//!   · addressing: 404 for an unknown serial — the feed learns its target
//!     is wrong, never a silent no-op;
//!   · effect: one column on one row (installed_units.filter_life_pct), and
//!     the response echoes exactly what was written.
//!
//! installed_units carries no RLS (spec §4 lists exactly six RLS tables) and
//! no audit trigger — the admin role gate is the authorization boundary,
//! same as /api/admin/refresh-rollups. The R4 telemetry branch of
//! generate_signals() turns what this writes into reorder cards on the next
//! generation run.

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use domain::UserRole;
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::MidpointAwayFromZero;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct FilterLifeBody {
    serial: Option<String>,
    filter_life_pct: Option<f64>,
}

#[derive(Serialize)]
pub struct FilterLifeResult {
    unit_id: Uuid,
    serial: String,
    filter_life_pct: f64,
}

pub async fn filter_life(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<FilterLifeBody>, JsonRejection>,
) -> Result<Json<FilterLifeResult>, ApiError> {
    if user.role != UserRole::Admin {
        return Err(ApiError::Forbidden);
    }

    // A malformed body (non-numeric pct, wrong shape) is a typed 422 — the
    // JsonRejection carries serde's reason.
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;
    let serial = body
        .serial
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::Invalid("serial is required".into()))?;
    let pct = body
        .filter_life_pct
        .ok_or_else(|| ApiError::Invalid("filter_life_pct is required".into()))?;
    if !(0.0..=100.0).contains(&pct) {
        return Err(ApiError::Invalid(
            "filter_life_pct must be between 0 and 100".into(),
        ));
    }
    // Store at the column's precision (numeric(5,2)) — the exact value the
    // response echoes back.
    let stored = Decimal::from_f64_retain(pct)
        .ok_or_else(|| ApiError::Invalid("filter_life_pct must be a number".into()))?
        .round_dp_with_strategy(2, MidpointAwayFromZero);

    let row = sqlx::query!(
        r#"UPDATE installed_units SET filter_life_pct = $2
           WHERE serial = $1
           RETURNING id, serial, filter_life_pct::float8 AS "filter_life_pct!""#,
        serial,
        stored
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(FilterLifeResult {
        unit_id: row.id,
        serial: row.serial,
        filter_life_pct: row.filter_life_pct,
    }))
}
