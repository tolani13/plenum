//! Typed errors, one envelope: { "error": { "code": ..., "message": ... } }
//! with matching HTTP status. 401/403/404/422 are distinct; an empty result
//! set is a normal 200, never an error.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Unauthorized(&'static str),
    /// RLS answers out-of-scope READS with empty result sets, not errors;
    /// 403 is for privileged ACTIONS a logged-in caller lacks the role for
    /// (P1: POST /api/admin/refresh-rollups as a non-admin).
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Invalid(String),
    /// P4: the AI layer is off (flag or missing key) or the vendor call
    /// failed. 503 with code `ai_unavailable` — the UI renders its designed
    /// "AI is off" state, never an error screen (R8).
    #[error("{0}")]
    AiUnavailable(&'static str),
    #[error("internal error")]
    Internal,
}

impl ApiError {
    /// Login failure — identical body for unknown email and wrong password,
    /// so responses cannot be used to enumerate users.
    pub fn bad_credentials() -> Self {
        ApiError::Unauthorized("invalid email or password")
    }

    /// Missing or expired session.
    pub fn no_session() -> Self {
        ApiError::Unauthorized("authentication required")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(error = %e, "database error");
        ApiError::Internal
    }
}

impl From<tower_sessions::session::Error> for ApiError {
    fn from(e: tower_sessions::session::Error) -> Self {
        tracing::error!(error = %e, "session store error");
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
            ApiError::AiUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "ai_unavailable"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        let body = json!({ "error": { "code": code, "message": self.to_string() } });
        (status, Json(body)).into_response()
    }
}
