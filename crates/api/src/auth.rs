//! Session identity. The extractor reads the logged-in user from the
//! tower-session; a missing session is a typed 401.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use domain::UserRole;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::error::ApiError;

pub const SESSION_USER_KEY: &str = "user";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionUser {
    pub id: Uuid,
    pub role: UserRole,
}

impl<S> FromRequestParts<S> for SessionUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|(_, msg)| {
                tracing::error!("session layer missing from request: {msg}");
                ApiError::Internal
            })?;
        session
            .get::<SessionUser>(SESSION_USER_KEY)
            .await?
            .ok_or_else(ApiError::no_session)
    }
}
