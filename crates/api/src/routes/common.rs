//! Shared plumbing for every route module: pagination parsing and the role
//! gate. P5 (R9b) settled the parked cleanup — accounts.rs and metrics.rs
//! dropped their P0/P1-era local copies and read these helpers now; the
//! grammar and 422 messages are the ones the P0/P1 test matrices pinned.

use domain::UserRole;

use crate::auth::SessionUser;
use crate::error::ApiError;

pub const DEFAULT_LIMIT: i64 = 50;
pub const MAX_LIMIT: i64 = 200;

/// Raw string params so a non-numeric value becomes a typed 422, not a
/// framework 400 (the P0/P1 convention).
pub fn parse_int(value: Option<String>, default: i64, name: &str) -> Result<i64, ApiError> {
    match value {
        None => Ok(default),
        Some(s) => s
            .parse::<i64>()
            .map_err(|_| ApiError::Invalid(format!("{name} must be an integer"))),
    }
}

/// limit (default 50, max 200) / offset >= 0 — the R8 discipline, on every list.
pub fn parse_page(limit: Option<String>, offset: Option<String>) -> Result<(i64, i64), ApiError> {
    let limit = parse_int(limit, DEFAULT_LIMIT, "limit")?;
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ApiError::Invalid(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    let offset = parse_int(offset, 0, "offset")?;
    if offset < 0 {
        return Err(ApiError::Invalid("offset must be >= 0".into()));
    }
    Ok((limit, offset))
}

/// A privileged-action role gate → typed 403 (never an empty 200; 403 is for
/// actions a logged-in caller lacks the role for, per error.rs).
pub fn require_role(user: &SessionUser, allowed: &[UserRole]) -> Result<(), ApiError> {
    if allowed.contains(&user.role) {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
