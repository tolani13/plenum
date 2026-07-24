//! Router assembly: the P0 surface, P1's metrics + admin refresh, P3's CRM
//! operational core (accounts 360, opportunities, quotes, policy, products,
//! activities), P4's signals/AI/telemetry, and P5's map states + data-quality
//! reads.

pub mod accounts;
pub mod activities;
pub mod admin;
pub mod auth;
pub mod common;
pub mod data_quality;
pub mod metrics;
pub mod opportunities;
pub mod policy;
pub mod products;
pub mod quotes;
pub mod signals;
pub mod states;
pub mod telemetry;
pub mod territories;

use std::path::PathBuf;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{any, get, patch, post, put};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::cookie::time::Duration;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use crate::error::ApiError;
use crate::state::AppState;

/// Deploy unit: GET /api/health — unauthenticated, trivial, no database
/// touch. Render's health checker (and any uptime probe) polls it; a 200
/// means the process is up and serving. Everything real stays behind auth.
async fn health() -> &'static str {
    "ok"
}

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

    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/api/accounts",
            get(accounts::list_accounts).post(accounts::create_account),
        )
        .route("/api/accounts/{id}", get(accounts::get_account))
        .route(
            "/api/opportunities",
            get(opportunities::list_opportunities).post(opportunities::create_opportunity),
        )
        .route(
            "/api/opportunities/{id}/stage",
            patch(opportunities::patch_stage),
        )
        .route(
            "/api/quotes",
            get(quotes::list_quotes).post(quotes::create_quote),
        )
        .route("/api/quotes/{id}", get(quotes::get_quote))
        .route("/api/quotes/{id}/submit", post(quotes::submit_quote))
        .route("/api/quotes/{id}/approve", post(quotes::approve_quote))
        .route("/api/quotes/{id}/reject", post(quotes::reject_quote))
        .route("/api/quotes/{id}/audit", get(quotes::quote_audit))
        .route("/api/policy/discount", get(policy::discount_policy))
        .route("/api/products", get(products::list_products))
        .route(
            "/api/activities",
            get(activities::list_activities).post(activities::create_activity),
        )
        .route("/api/signals", get(signals::list_signals))
        .route("/api/signals/summary", get(signals::signals_summary))
        .route("/api/signals/assignees", get(signals::signal_assignees))
        .route("/api/signals/{id}/assign", post(signals::assign_signal))
        .route("/api/signals/{id}/action", post(signals::action_signal))
        .route("/api/signals/{id}/dismiss", post(signals::dismiss_signal))
        .route("/api/ai/status", get(crate::ai::ai_status))
        .route("/api/ai/ask", post(crate::ai::ask))
        .route(
            "/api/ai/discount-recommendation",
            post(crate::ai::discount_recommendation),
        )
        .route("/api/telemetry/filter-life", post(telemetry::filter_life))
        .route("/api/metrics/territories", get(metrics::territories))
        .route("/api/metrics/leaderboard", get(metrics::leaderboard))
        .route("/api/metrics/items", get(metrics::items))
        .route("/api/metrics/customers", get(metrics::customers))
        .route("/api/metrics/leakage", get(metrics::leakage))
        .route("/api/metrics/coverage", get(metrics::coverage))
        .route("/api/metrics/defection", get(metrics::defection))
        .route("/api/metrics/states", get(states::states))
        // T1 — territory editing (planning view): vp|admin-gated in the
        // handlers (the generate-signals precedent), audited via 0014.
        .route(
            "/api/territories",
            get(territories::list_territories).post(territories::create_territory),
        )
        .route(
            "/api/territories/{code}",
            patch(territories::patch_territory).delete(territories::delete_territory),
        )
        .route(
            "/api/territory-states/{state_code}",
            put(territories::put_territory_state),
        )
        .route("/api/data-quality", get(data_quality::data_quality))
        .route("/api/admin/refresh-rollups", post(admin::refresh_rollups))
        .route(
            "/api/admin/generate-signals",
            post(signals::generate_signals),
        )
        // Deploy unit: unknown /api/* paths keep the P0 typed-JSON 404
        // contract even when the SPA static tier below is mounted — the
        // wildcard loses to every registered /api route (static segments
        // outrank catch-alls in the matcher) and catches only the misses.
        .route("/api", any(|| async { ApiError::NotFound }))
        .route("/api/{*rest}", any(|| async { ApiError::NotFound }));

    // Deploy unit (env-gated static tier): in the release container the API
    // is the ONLY server, so non-/api paths serve the built SPA from disk —
    // same origin, which is exactly why the SameSite=Lax + Secure session
    // cookie needs no CORS story. WEB_DIST defaults to web/dist: present in
    // the container (copied by the Dockerfile), absent from a dev/test
    // working directory (Vite owns dev on 5177; integration tests run from
    // crates/api) — so when index.html is missing the P0 JSON-404 fallback
    // stands byte-for-byte and dev behavior is unchanged.
    let web_dist = std::env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".to_string());
    let index_html = PathBuf::from(&web_dist).join("index.html");
    let router = if index_html.is_file() {
        tracing::info!(dir = %web_dist, "static tier mounted — serving the SPA for non-/api paths");
        router.fallback_service(
            // Unknown non-API path -> index.html WITH ITS 200 (ServeDir's
            // `fallback` passes the inner response through untouched; its
            // `not_found_service` would stamp 404 on deep links like
            // /command — the SPA router owns those client-side routes).
            ServeDir::new(&web_dist).fallback(ServeFile::new(index_html)),
        )
    } else {
        router.fallback(|| async { ApiError::NotFound })
    };

    router
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
