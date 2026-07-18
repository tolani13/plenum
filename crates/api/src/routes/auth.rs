//! POST /api/auth/login · POST /api/auth/logout · GET /api/auth/me

use std::sync::OnceLock;

use argon2::password_hash::{PasswordHash, PasswordVerifier, SaltString};
use argon2::{Argon2, PasswordHasher};
use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use domain::UserRole;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::{SessionUser, SESSION_USER_KEY};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginBody {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct UserBody {
    id: Uuid,
    name: String,
    email: String,
    role: UserRole,
}

#[derive(Serialize)]
pub struct MeBody {
    id: Uuid,
    name: String,
    email: String,
    role: UserRole,
    territories: Vec<String>,
}

/// A syntactically valid argon2id hash used to equalize verification work
/// when the email is unknown — login timing does not reveal user existence.
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        let salt = SaltString::encode_b64(b"plenum-dummy-slt").expect("static salt encodes");
        Argon2::default()
            .hash_password(b"plenum-dummy-password", &salt)
            .expect("static hash succeeds")
            .to_string()
    })
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    body: Result<Json<LoginBody>, JsonRejection>,
) -> Result<Json<UserBody>, ApiError> {
    let Json(body) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;

    // users is not an RLS table; login runs on the plain app pool.
    let user = sqlx::query!(
        r#"SELECT id, email, name, role AS "role: UserRole", password_hash
           FROM users WHERE email = $1"#,
        body.email
    )
    .fetch_optional(&state.pool)
    .await?;

    // Same failure body for unknown email vs wrong password (no user
    // enumeration), and a dummy verification keeps the work comparable.
    let verified = match &user {
        Some(u) => {
            let parsed = PasswordHash::new(&u.password_hash).map_err(|e| {
                tracing::error!(error = %e, "stored password hash unparsable");
                ApiError::Internal
            })?;
            Argon2::default()
                .verify_password(body.password.as_bytes(), &parsed)
                .is_ok()
        }
        None => {
            let parsed = PasswordHash::new(dummy_hash()).expect("dummy hash parses");
            let _ = Argon2::default().verify_password(body.password.as_bytes(), &parsed);
            false
        }
    };
    let Some(user) = user.filter(|_| verified) else {
        return Err(ApiError::bad_credentials());
    };

    // Fresh session id on privilege change (anti-fixation), then persist.
    session.cycle_id().await?;
    session
        .insert(
            SESSION_USER_KEY,
            SessionUser {
                id: user.id,
                role: user.role,
            },
        )
        .await?;

    Ok(Json(UserBody {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
    }))
}

pub async fn logout(session: Session) -> Result<StatusCode, ApiError> {
    session.flush().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<MeBody>, ApiError> {
    let row = sqlx::query!(
        r#"SELECT id, email, name, role AS "role: UserRole" FROM users WHERE id = $1"#,
        user.id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(ApiError::no_session)?;

    // Scope codes via v_user_scope — the same view the RLS policies read.
    let territories: Vec<String> = sqlx::query_scalar!(
        r#"SELECT t.code AS "code!"
           FROM v_user_scope s
           JOIN territories t ON t.id = s.territory_id
           WHERE s.user_id = $1
           ORDER BY t.code"#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(MeBody {
        id: row.id,
        name: row.name,
        email: row.email,
        role: row.role,
        territories,
    }))
}
