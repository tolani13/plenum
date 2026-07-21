//! THE RLS guard. Every handler that touches an RLS table gets its database
//! access through this helper and only this helper — the handler receives a
//! transaction with the caller's identity pinned, never the raw pool.
//!
//! set_config(..., true) is transaction-local — exactly the spec's SET LOCAL,
//! but bindable (no string interpolation anywhere near session identity).
//! When the transaction ends, the GUCs vanish; a connection returned to the
//! pool carries nothing. Any query path that skips this helper runs with no
//! app.user_id set, and the policies' NULLIF(...) predicate then matches
//! nothing: fail closed, zero rows.

use sqlx::{PgPool, Postgres, Transaction};

use crate::auth::SessionUser;
use crate::error::ApiError;

pub async fn rls_tx(
    pool: &PgPool,
    user: &SessionUser,
) -> Result<Transaction<'static, Postgres>, ApiError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.user_id', $1, true), set_config('app.role', $2, true)")
        .bind(user.id.to_string())
        .bind(user.role.as_str())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

/// The READ-ONLY variant, for Ask PLENUM's model-generated SQL (R9/R11 — the
/// identity seam stays HERE, not in ai/). Same GUC pinning as rls_tx, with
/// two hardenings ordered before it:
///   · SET TRANSACTION READ ONLY — Postgres requires this before the
///     transaction's first query, so it precedes the set_config call; any
///     write the validator somehow missed dies in the database;
///   · SET LOCAL statement_timeout = '5s' — a runaway generated query is
///     cancelled by Postgres (surfaced as a typed 422 by the ask handler).
pub async fn rls_readonly_tx(
    pool: &PgPool,
    user: &SessionUser,
) -> Result<Transaction<'static, Postgres>, ApiError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT set_config('app.user_id', $1, true), set_config('app.role', $2, true)")
        .bind(user.id.to_string())
        .bind(user.role.as_str())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
