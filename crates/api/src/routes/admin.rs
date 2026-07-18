//! POST /api/admin/refresh-rollups — the only P1 write-ish surface.
//!
//! The handler gates on session role = admin BEFORE touching the database:
//! 401 without a session (the SessionUser extractor), 403 for any other
//! authenticated role. The refresh itself runs through refresh_rollups(),
//! a SECURITY DEFINER function owned by plenum_admin — the API keeps its P0
//! doctrine (connects ONLY as plenum_app, no admin pool anywhere) and
//! plenum_app still has zero SELECT rights on the raw matviews; the row
//! counts come back through the definer boundary.

use axum::extract::State;
use axum::Json;
use domain::UserRole;
use serde::Serialize;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct RefreshedMatview {
    matview: String,
    row_count: i64,
}

#[derive(Serialize)]
pub struct RefreshBody {
    refreshed: Vec<RefreshedMatview>,
}

pub async fn refresh_rollups(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<RefreshBody>, ApiError> {
    if user.role != UserRole::Admin {
        return Err(ApiError::Forbidden);
    }

    let rows = sqlx::query!(
        r#"SELECT matview AS "matview!", row_count AS "row_count!"
           FROM refresh_rollups()"#
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(RefreshBody {
        refreshed: rows
            .into_iter()
            .map(|r| RefreshedMatview {
                matview: r.matview,
                row_count: r.row_count,
            })
            .collect(),
    }))
}
