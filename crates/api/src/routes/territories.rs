//! T1 — Territory Map Editing (planning view): the territory/geography write
//! surface, plus the editor's disclosed read.
//!
//! Every endpoint here is vp|admin (the generate-signals role-gate precedent
//! via common::require_role): 401 without a session, 403 for rep/manager.
//! Writes run inside rls_tx — not for row filtering (these config tables
//! carry no RLS) but so the app.user_id GUC is pinned and the 0006/0014
//! audit trigger records the ACTING user on every mutation; reassign + audit
//! are atomic in the one transaction. Defense stack for RLS-less config
//! tables, per the 0014 migration comment: role gate + app-immutable audit
//! trail + typed errors.
//!
//! PLANNING-VIEW LAW: nothing here touches order/account territory
//! attribution, RLS scope, or mv_* rollups — the map's geography config
//! moves; the book of business does not (docs/territory-realignment-prep.md
//! records the future commit-realignment unit).

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use domain::UserRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::routes::common::require_role;
use crate::state::AppState;

const EDIT_ROLES: &[UserRole] = &[UserRole::Vp, UserRole::Admin];

/// v1 lock (T1-D2/D10): the Canada blocks and their province rows are not
/// editable — a block and its provinces must move atomically, or province
/// editing arrives with province shapes (territory-realignment-prep.md §4).
const CANADA_CODES: &[&str] = &[
    "CA-E", "CA-W", "ON", "QC", "NB", "NS", "PE", "NL", "BC", "AB", "SK", "MB", "YT", "NT", "NU",
];

/// The planning palette (T1-D5): the ONLY color_token values the API accepts
/// — the same eight names tokens.css declares as --color-terr-plan-*. A free
/// hex picker was rejected (T1-D10): tokens.css stays the single palette
/// source, and this list is the server-side mirror of it.
const PLANNING_PALETTE: &[&str] = &[
    "terr-plan-1",
    "terr-plan-2",
    "terr-plan-3",
    "terr-plan-4",
    "terr-plan-5",
    "terr-plan-6",
    "terr-plan-7",
    "terr-plan-8",
];

/// The region enum's labels, for the typed 422 (bound as text + ::region
/// cast — the P3 account-status pattern).
const REGIONS: &[&str] = &[
    "northeast",
    "southeast",
    "midwest",
    "south_central",
    "mountain",
    "west",
    "canada_e",
    "canada_w",
];

#[derive(Serialize)]
pub struct TerritoryOut {
    id: Uuid,
    code: String,
    name: String,
    region: String,
    quota_year_cents: i64,
    color_token: Option<String>,
}

#[derive(Serialize)]
pub struct TerritoriesPage {
    items: Vec<TerritoryOut>,
    limit: i64,
    offset: i64,
    total: i64,
}

fn validate_color_token(token: &str) -> Result<(), ApiError> {
    if PLANNING_PALETTE.contains(&token) {
        Ok(())
    } else {
        Err(ApiError::Invalid(format!(
            "color_token must be one of the planning palette tokens ({})",
            PLANNING_PALETTE.join(", ")
        )))
    }
}

/// Short, uppercase-alnum-dash, matching the existing codes (NE-1, MT-1…):
/// 2–8 chars of A–Z / 0–9 / dash, no leading/trailing dash.
fn validate_code(code: &str) -> Result<(), ApiError> {
    let ok = (2..=8).contains(&code.len())
        && code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        && !code.starts_with('-')
        && !code.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(ApiError::Invalid(
            "code must be 2-8 uppercase letters, digits, or dashes (like GC-1)".into(),
        ))
    }
}

fn validate_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::Invalid("name is required".into()));
    }
    if name.len() > 60 {
        return Err(ApiError::Invalid(
            "name must be 60 characters or fewer".into(),
        ));
    }
    Ok(())
}

// ── GET /api/territories — the editor's disclosed read ─────────────────────
// Full list INCLUDING empty territories (a just-created one has no states,
// no money, no roster presence worth relying on) + color_token. vp|admin —
// this is the edit surface's own read; everyone else keeps the config-level
// roster inside /api/metrics/states. Plain full list, well under the
// pagination law's 200 (8 canonical + runtime creations).
pub async fn list_territories(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<TerritoriesPage>, ApiError> {
    require_role(&user, EDIT_ROLES)?;

    let rows = sqlx::query!(
        r#"SELECT id, code, name, region::text AS "region!",
                  quota_year_cents, color_token
           FROM territories
           ORDER BY code"#
    )
    .fetch_all(&state.pool)
    .await?;

    let total = rows.len() as i64;
    Ok(Json(TerritoriesPage {
        items: rows
            .into_iter()
            .map(|r| TerritoryOut {
                id: r.id,
                code: r.code,
                name: r.name,
                region: r.region,
                quota_year_cents: r.quota_year_cents,
                color_token: r.color_token,
            })
            .collect(),
        limit: 200,
        offset: 0,
        total,
    }))
}

// ── PUT /api/territory-states/:state_code — click-to-paint / drag ──────────

#[derive(Deserialize)]
pub struct AssignBody {
    territory_code: String,
}

#[derive(Serialize)]
pub struct AssignedState {
    state_code: String,
    territory_code: String,
}

pub async fn put_territory_state(
    State(state): State<AppState>,
    user: SessionUser,
    Path(state_code): Path<String>,
    body: Result<Json<AssignBody>, JsonRejection>,
) -> Result<Json<AssignedState>, ApiError> {
    require_role(&user, EDIT_ROLES)?;
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;
    let territory_code = body.territory_code.trim();

    let mut tx = rls_tx(&state.pool, &user).await?;

    let known: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM territory_states WHERE state_code = $1) AS "e!""#,
        state_code
    )
    .fetch_one(&mut *tx)
    .await?;
    if !known {
        return Err(ApiError::NotFound);
    }
    if CANADA_CODES.contains(&state_code.as_str()) {
        return Err(ApiError::Invalid(
            "Canada blocks and provinces are locked in v1 — Canada editing lands in v2 \
             (a block and its provinces move atomically; see docs/territory-realignment-prep.md)"
                .into(),
        ));
    }
    let target_exists: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM territories WHERE code = $1) AS "e!""#,
        territory_code
    )
    .fetch_one(&mut *tx)
    .await?;
    if !target_exists {
        return Err(ApiError::Invalid("unknown territory".into()));
    }

    // Only-when-changed (the P4 no-clobber discipline): a repeat paint of the
    // same territory updates zero rows and writes zero audit noise.
    sqlx::query!(
        r#"UPDATE territory_states SET territory_code = $2
           WHERE state_code = $1 AND territory_code IS DISTINCT FROM $2"#,
        state_code,
        territory_code
    )
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query!(
        r#"SELECT state_code, territory_code FROM territory_states WHERE state_code = $1"#,
        state_code
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(AssignedState {
        state_code: row.state_code,
        territory_code: row.territory_code,
    }))
}

// ── POST /api/territories — create (quota stays 0: creation never sets it) ──

#[derive(Deserialize)]
pub struct CreateTerritoryBody {
    code: String,
    name: String,
    region: String,
    color_token: Option<String>,
}

pub async fn create_territory(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<CreateTerritoryBody>, JsonRejection>,
) -> Result<(StatusCode, Json<TerritoryOut>), ApiError> {
    require_role(&user, EDIT_ROLES)?;
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    let code = body.code.trim().to_string();
    let name = body.name.trim().to_string();
    let region = body.region.trim().to_string();
    validate_code(&code)?;
    validate_name(&name)?;
    if !REGIONS.contains(&region.as_str()) {
        return Err(ApiError::Invalid(format!(
            "unknown region (one of: {})",
            REGIONS.join(", ")
        )));
    }
    if let Some(token) = body.color_token.as_deref() {
        validate_color_token(token)?;
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    let dup: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM territories WHERE code = $1) AS "e!""#,
        code
    )
    .fetch_one(&mut *tx)
    .await?;
    if dup {
        return Err(ApiError::Invalid("territory code already exists".into()));
    }

    let row = sqlx::query!(
        r#"INSERT INTO territories (code, name, region, quota_year_cents, color_token)
           VALUES ($1, $2, $3::text::region, 0, $4)
           RETURNING id, code, name, region::text AS "region!",
                     quota_year_cents, color_token"#,
        code,
        name,
        region,
        body.color_token
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(TerritoryOut {
            id: row.id,
            code: row.code,
            name: row.name,
            region: row.region,
            quota_year_cents: row.quota_year_cents,
            color_token: row.color_token,
        }),
    ))
}

// ── PATCH /api/territories/:code — rename / recolor ONLY ───────────────────

#[derive(Deserialize)]
pub struct PatchTerritoryBody {
    name: Option<String>,
    color_token: Option<String>,
}

pub async fn patch_territory(
    State(state): State<AppState>,
    user: SessionUser,
    Path(code): Path<String>,
    body: Result<Json<PatchTerritoryBody>, JsonRejection>,
) -> Result<Json<TerritoryOut>, ApiError> {
    require_role(&user, EDIT_ROLES)?;
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    let name = body.name.as_deref().map(str::trim).map(str::to_string);
    if let Some(n) = name.as_deref() {
        validate_name(n)?;
    }
    if let Some(token) = body.color_token.as_deref() {
        validate_color_token(token)?;
    }
    if name.is_none() && body.color_token.is_none() {
        return Err(ApiError::Invalid(
            "nothing to update — provide name and/or color_token".into(),
        ));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // Only-when-changed, same discipline as the paint path: an identical
    // rename/recolor updates zero rows and writes zero audit noise.
    sqlx::query!(
        r#"UPDATE territories
           SET name = COALESCE($2, name),
               color_token = COALESCE($3, color_token)
           WHERE code = $1
             AND (name IS DISTINCT FROM COALESCE($2, name)
                  OR color_token IS DISTINCT FROM COALESCE($3, color_token))"#,
        code,
        name,
        body.color_token
    )
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query!(
        r#"SELECT id, code, name, region::text AS "region!",
                  quota_year_cents, color_token
           FROM territories WHERE code = $1"#,
        code
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    tx.commit().await?;

    Ok(Json(TerritoryOut {
        id: row.id,
        code: row.code,
        name: row.name,
        region: row.region,
        quota_year_cents: row.quota_year_cents,
        color_token: row.color_token,
    }))
}

// ── DELETE /api/territories/:code — refused unless completely empty ────────

#[derive(Serialize)]
pub struct DeletedTerritory {
    id: Uuid,
    code: String,
    deleted: bool,
}

pub async fn delete_territory(
    State(state): State<AppState>,
    user: SessionUser,
    Path(code): Path<String>,
) -> Result<Json<DeletedTerritory>, ApiError> {
    require_role(&user, EDIT_ROLES)?;

    let mut tx = rls_tx(&state.pool, &user).await?;

    let territory = sqlx::query!(r#"SELECT id, code FROM territories WHERE code = $1"#, code)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;

    // The five emptiness checks (T1-D2). accounts/orders/opportunities are
    // RLS tables, but the caller is vp|admin — scope is every territory, so
    // the counts are complete by construction.
    let checks = sqlx::query!(
        r#"SELECT
             (SELECT count(*) FROM territory_states WHERE territory_code = $1) AS "states!",
             (SELECT count(*) FROM territory_assignments WHERE territory_id = $2) AS "assignments!",
             (SELECT count(*) FROM accounts WHERE territory_id = $2) AS "accounts!",
             (SELECT count(*) FROM orders WHERE territory_id = $2) AS "orders!",
             (SELECT count(*) FROM opportunities WHERE territory_id = $2) AS "opportunities!""#,
        territory.code,
        territory.id
    )
    .fetch_one(&mut *tx)
    .await?;

    let mut blockers: Vec<String> = Vec::new();
    for (count, noun) in [
        (checks.states, "mapped state"),
        (checks.assignments, "rep assignment"),
        (checks.accounts, "account"),
        (checks.orders, "order"),
        (checks.opportunities, "opportunity"),
    ] {
        if count > 0 {
            let plural = if noun == "opportunity" {
                "opportunities".to_string()
            } else {
                format!("{noun}s")
            };
            blockers.push(format!(
                "{count} {}",
                if count == 1 { noun.to_string() } else { plural }
            ));
        }
    }
    if !blockers.is_empty() {
        return Err(ApiError::Invalid(format!(
            "cannot delete {} — it still has {}; only a completely empty territory \
             (no mapped states, rep assignments, accounts, orders, or opportunities) \
             can be deleted",
            territory.code,
            blockers.join(", ")
        )));
    }

    sqlx::query!(r#"DELETE FROM territories WHERE id = $1"#, territory.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(DeletedTerritory {
        id: territory.id,
        code: territory.code,
        deleted: true,
    }))
}
