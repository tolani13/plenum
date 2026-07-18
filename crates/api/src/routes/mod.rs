//! Router assembly: the full P0 API surface and nothing more.

pub mod accounts;
pub mod auth;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use crate::error::ApiError;
use crate::state::AppState;

pub fn app(state: AppState, cookie_secure: bool) -> Router {
    // MemoryStore: sessions live in process memory — restart the API and
    // everyone re-logs-in. Acceptable for this demo phase; noted in README.
    let store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(store)
        .with_name("plenum_session")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(cookie_secure)
        .with_expiry(Expiry::OnInactivity(Duration::hours(8)));

    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/accounts", get(accounts::list_accounts))
        .fallback(|| async { ApiError::NotFound })
        .layer(axum::middleware::from_fn(trace_requests))
        .layer(session_layer)
        .with_state(state)
}

/// Request span logging without pulling tower-http into the dependency tree.
async fn trace_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let started = std::time::Instant::now();
    let response = next.run(req).await;
    tracing::info!(
        method = %method,
        uri = %uri,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "request"
    );
    response
}
