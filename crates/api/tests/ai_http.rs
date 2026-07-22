//! P4 AI layer — Tier-3 matrix, hermetic by construction: every app instance
//! here pins api_key: None, so NO test can ever reach the vendor, whatever
//! .env holds. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Covers: the ask flag/key gate (503, typed), the discount recommender's
//! flag gate (503) and key-absent degradation (200, narrative null,
//! degraded true), rep-vs-VP comparables scope, request validation 422s, and
//! the generated-SQL EXECUTION path driven directly through run_ask_query —
//! rep scope under the read-only GUC transaction, the injected LIMIT 500
//! wrap, and the 5s statement-timeout dying as a typed 422.
//! (The validator's own adversarial matrix lives in src/ai/validate.rs.)

use api::ai::{run_ask_query, validate::validate_ask_sql};
use api::auth::SessionUser;
use api::error::ApiError;
use api::routes;
use api::state::{AiConfig, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use domain::UserRole;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::util::ServiceExt;
use uuid::Uuid;

const SE1_REP: &str = "serena.estes@plenum.demo";
const VP: &str = "valerie.price@plenum.demo";
const PASSWORD: &str = "demo-plenum-2026";

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .expect("test database reachable — run: docker compose up -d && cargo run --bin seed")
}

fn keyless_ai(ask_flag: bool, discount_flag: bool) -> AiConfig {
    AiConfig {
        api_key: None,
        model: "claude-sonnet-5".to_string(),
        ask_flag,
        discount_flag,
    }
}

async fn app_with(ai: AiConfig) -> Router {
    routes::app(
        AppState {
            pool: test_pool().await,
            ai,
        },
        false,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body collects")
        .to_bytes();
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).expect("body is JSON")
}

async fn login(app: &Router, email: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "email": email, "password": PASSWORD }).to_string(),
        ))
        .expect("request builds");
    let response = app.clone().oneshot(request).await.expect("router serves");
    assert_eq!(response.status(), StatusCode::OK, "login {email}");
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("login sets a session cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let request = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(request).await.expect("router serves");
    (response.status(), body_json(response).await)
}

async fn session_user(pool: &PgPool, email: &str, role: UserRole) -> SessionUser {
    let id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user exists");
    SessionUser { id, role }
}

// ── /api/ai/status + the ask gate ───────────────────────────────────────────

#[tokio::test]
async fn status_reflects_flags_and_ask_gates_typed_503() {
    // Flags on, no key: ask is effectively OFF (flag AND key), discount is on.
    let app = app_with(keyless_ai(true, true)).await;
    let rep = login(&app, SE1_REP).await;
    let (s, status) = send(&app, "GET", "/api/ai/status", Some(&rep), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(status["ask"], false, "no key → ask off");
    assert_eq!(status["discount"], true, "flag alone gates discount");

    let (s, body) = send(
        &app,
        "POST",
        "/api/ai/ask",
        Some(&rep),
        Some(json!({ "question": "top customers" })),
    )
    .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "ai_unavailable");

    // Ask flag explicitly off: same typed 503 (indistinguishable by design).
    let app_off = app_with(keyless_ai(false, false)).await;
    let rep_off = login(&app_off, SE1_REP).await;
    let (s, body) = send(
        &app_off,
        "POST",
        "/api/ai/ask",
        Some(&rep_off),
        Some(json!({ "question": "top customers" })),
    )
    .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "ai_unavailable");
    let (s, status) = send(&app_off, "GET", "/api/ai/status", Some(&rep_off), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(status["ask"], false);
    assert_eq!(status["discount"], false);
}

// ── the discount recommender: flag gate, degradation, scope, validation ─────

#[tokio::test]
async fn recommender_flag_off_is_typed_503() {
    let app = app_with(keyless_ai(true, false)).await;
    let rep = login(&app, SE1_REP).await;
    let (s, body) = send(
        &app,
        "POST",
        "/api/ai/discount-recommendation",
        Some(&rep),
        Some(json!({
            "product_id": Uuid::new_v4(), "account_id": Uuid::new_v4(),
            "qty": 1, "discount_pct": 10.0
        })),
    )
    .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "ai_unavailable");
}

#[tokio::test]
async fn recommender_degrades_without_key_and_scopes_comparables() {
    let app = app_with(keyless_ai(true, true)).await;
    let pool = test_pool().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;

    // A consumable the SE-1 world actually buys, and one of serena's accounts.
    let product_id: Uuid =
        sqlx::query_scalar("SELECT id FROM products WHERE sku = 'FLT-STATSAFE-GS3'")
            .fetch_one(&pool)
            .await
            .expect("catalog SKU exists");
    let (_, accounts) = send(&app, "GET", "/api/accounts?limit=1", Some(&rep), None).await;
    let account_id = accounts["items"][0]["id"].as_str().unwrap().to_string();

    let req = json!({
        "product_id": product_id, "account_id": account_id,
        "qty": 32, "discount_pct": 28.0
    });

    // Key absent → 200, comparables only, narrative null, degraded true.
    let (s, rep_body) = send(
        &app,
        "POST",
        "/api/ai/discount-recommendation",
        Some(&rep),
        Some(req.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(rep_body["degraded"], true, "no key → degraded");
    assert!(rep_body["narrative"].is_null(), "no key → no narrative");
    let comps = &rep_body["comparables"];
    assert!(comps["band_label"].as_str().unwrap().contains("line gross"));
    assert!(comps["count"].as_i64().unwrap() >= 0);
    assert!(comps["sample"].as_array().unwrap().len() <= 10);

    // Scope: the VP's cohort (all territories) is at least the rep's (SE-1).
    let (_, vp_body) = send(
        &app,
        "POST",
        "/api/ai/discount-recommendation",
        Some(&vp),
        Some(req),
    )
    .await;
    assert!(
        vp_body["comparables"]["count"].as_i64().unwrap() >= comps["count"].as_i64().unwrap(),
        "VP comparables ⊇ rep comparables"
    );

    // Foreign account → 404 under RLS.
    let (_, vp_accounts) = send(&app, "GET", "/api/accounts?limit=200", Some(&vp), None).await;
    let foreign = vp_accounts["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["territory_code"] != "SE-1")
        .expect("a foreign account exists")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (s, _) = send(
        &app,
        "POST",
        "/api/ai/discount-recommendation",
        Some(&rep),
        Some(json!({
            "product_id": product_id, "account_id": foreign,
            "qty": 1, "discount_pct": 10.0
        })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "foreign account cohort is a 404");

    // Validation 422s: qty and discount bounds, unknown product.
    for bad in [
        json!({ "product_id": product_id, "account_id": account_id, "qty": 0, "discount_pct": 10.0 }),
        json!({ "product_id": product_id, "account_id": account_id, "qty": 1, "discount_pct": 150.0 }),
        json!({ "product_id": Uuid::new_v4(), "account_id": account_id, "qty": 1, "discount_pct": 10.0 }),
    ] {
        let (s, _) = send(
            &app,
            "POST",
            "/api/ai/discount-recommendation",
            Some(&rep),
            Some(bad),
        )
        .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

// ── the execution path: scope, LIMIT wrap, timeout (no key needed) ──────────

#[tokio::test]
async fn run_ask_query_is_rls_scoped_for_a_rep() {
    let pool = test_pool().await;
    let rep = session_user(&pool, SE1_REP, UserRole::Rep).await;

    let sql = validate_ask_sql(
        "SELECT DISTINCT territory_code FROM v_order_facts ORDER BY territory_code",
    )
    .expect("whitelisted");
    let run = run_ask_query(&pool, &rep, &sql).await.expect("runs");
    assert_eq!(run.columns, vec!["territory_code"]);
    assert_eq!(
        run.rows,
        vec![vec![json!("SE-1")]],
        "a rep's generated-SQL result contains ONLY their territory"
    );
    assert!(!run.truncated);

    // The VP sees the full book through the identical SQL — scope is the
    // session, never the query.
    let vp = session_user(&pool, VP, UserRole::Vp).await;
    let vp_run = run_ask_query(&pool, &vp, &sql).await.expect("runs");
    assert!(
        vp_run.rows.len() > 1,
        "the VP's identical question spans territories"
    );
}

#[tokio::test]
async fn run_ask_query_wraps_with_limit_500() {
    let pool = test_pool().await;
    let vp = session_user(&pool, VP, UserRole::Vp).await;

    // 25k+ line facts — the injected wrap caps the answer at 500.
    let sql = validate_ask_sql("SELECT order_line_id FROM v_order_facts").expect("whitelisted");
    let run = run_ask_query(&pool, &vp, &sql).await.expect("runs");
    assert_eq!(
        run.rows.len(),
        500,
        "the injected LIMIT 500 capped the rows"
    );
    assert!(run.truncated, "truncated: true when 500 came back");
}

#[tokio::test]
async fn run_ask_query_statement_timeout_is_a_typed_422() {
    let pool = test_pool().await;
    let vp = session_user(&pool, VP, UserRole::Vp).await;

    // pg_sleep passes the validator ON PURPOSE (it is not a relation and not
    // denylisted) — the 5s statement timeout is the guardrail that kills it.
    let sql = validate_ask_sql("SELECT pg_sleep(10)").expect("validator lets the timeout catch it");
    let started = std::time::Instant::now();
    let err = run_ask_query(&pool, &vp, &sql)
        .await
        .expect_err("must die at the statement timeout");
    let elapsed = started.elapsed();
    match &err {
        ApiError::Invalid(msg) => {
            assert!(
                msg.contains("timed out"),
                "typed 422 names the timeout: {msg}"
            );
        }
        other => panic!("expected ApiError::Invalid, got {other:?}"),
    }
    assert!(
        elapsed.as_secs() < 10,
        "died at the 5s limit, not the sleep's 10s ({elapsed:?})"
    );
}

// ── the read-only transaction refuses writes that could slip through ────────

#[tokio::test]
async fn readonly_helper_transaction_refuses_writes() {
    let pool = test_pool().await;
    let vp = session_user(&pool, VP, UserRole::Vp).await;

    // Drive the helper DIRECTLY with a raw write (deliberately bypassing the
    // validator AND the wrap): defense in depth means even then, Postgres
    // itself refuses — 25006 read_only_sql_transaction. (Sent through
    // run_ask_query, a bare UPDATE dies even earlier: the SELECT wrap makes
    // it unparseable — proven implicitly by the validator tests.)
    let mut tx = api::rls::rls_readonly_tx(&pool, &vp)
        .await
        .expect("read-only tx opens");
    let err = sqlx::query("UPDATE accounts SET name = name")
        .execute(&mut *tx)
        .await
        .expect_err("a write must die in the read-only transaction");
    let msg = err.to_string();
    assert!(
        msg.contains("read-only"),
        "the database named the refusal: {msg}"
    );
    tx.rollback().await.ok();
}
