//! P3 CRM operational core — Tier-3 adversarial + integration matrix, over the
//! real router in-process against the live compose database. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Every test creates its OWN opportunities/quotes on an in-scope account, so
//! tests are isolated and re-runnable without a reseed; the assertions are
//! about scope, the role gate, the state machine, and booking integrity — never
//! about absolute seed totals (so the P1 metrics anchors are untouched by a
//! run that leaves a few extra rows behind, which a reseed clears anyway).

use api::routes;
use api::state::{AiConfig, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::util::ServiceExt;
use uuid::Uuid;

const SE1_REP: &str = "serena.estes@plenum.demo"; // SE-1
const VP: &str = "valerie.price@plenum.demo";
const RM_SE: &str = "rachel.moore@plenum.demo"; // regional manager over SE-1
const W1_REP: &str = "wes.turner@plenum.demo"; // foreign to SE-1
const PASSWORD: &str = "demo-plenum-2026";

async fn test_app() -> Router {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .expect("test database reachable — run: docker compose up -d && cargo run --bin seed");
    // P4: AppState grew an AiConfig. Tests pin a hermetic no-key config so
    // the suite NEVER makes a vendor call regardless of what .env holds.
    routes::app(
        AppState {
            pool,
            ai: test_ai_config(),
        },
        false,
    )
}

fn test_ai_config() -> AiConfig {
    AiConfig {
        api_key: None,
        model: "claude-sonnet-5".to_string(),
        ask_flag: true,
        discount_flag: true,
    }
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

/// Send a request; None cookie = anonymous, None body = no body.
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

fn items(page: &Value) -> &Vec<Value> {
    page["items"].as_array().expect("items array")
}

/// First account id in the caller's scope whose territory matches `code`.
async fn account_in(app: &Router, cookie: &str, code: &str) -> Uuid {
    let (_, page) = send(app, "GET", "/api/accounts?limit=200", Some(cookie), None).await;
    let id = items(&page)
        .iter()
        .find(|a| a["territory_code"] == code)
        .unwrap_or_else(|| panic!("no account in {code} for this scope"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    Uuid::parse_str(&id).unwrap()
}

/// An id in another caller's scope but NOT in `code` — a foreign target.
async fn foreign_account(app: &Router, cookie: &str, not_code: &str) -> Uuid {
    let (_, page) = send(app, "GET", "/api/accounts?limit=200", Some(cookie), None).await;
    let id = items(&page)
        .iter()
        .find(|a| a["territory_code"] != not_code)
        .expect("some foreign account")["id"]
        .as_str()
        .unwrap()
        .to_string();
    Uuid::parse_str(&id).unwrap()
}

async fn a_consumable_product(app: &Router, cookie: &str) -> Uuid {
    let (_, page) = send(
        app,
        "GET",
        "/api/products?kind=consumable&limit=1",
        Some(cookie),
        None,
    )
    .await;
    Uuid::parse_str(items(&page)[0]["id"].as_str().unwrap()).unwrap()
}

async fn me_id(app: &Router, cookie: &str) -> Uuid {
    let (_, me) = send(app, "GET", "/api/auth/me", Some(cookie), None).await;
    Uuid::parse_str(me["id"].as_str().unwrap()).unwrap()
}

/// Create a fresh opportunity (stage=lead) on an in-scope account.
async fn create_opp(app: &Router, cookie: &str, account_id: Uuid) -> Uuid {
    let (status, opp) = send(
        app,
        "POST",
        "/api/opportunities",
        Some(cookie),
        Some(json!({
            "account_id": account_id,
            "kind": "filter-program",
            "amount_cents": 1_000_000
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create opp: {opp}");
    Uuid::parse_str(opp["id"].as_str().unwrap()).unwrap()
}

/// Draft a quote (one line) at `discount` percent; returns the quote id.
async fn draft_quote(
    app: &Router,
    cookie: &str,
    opp_id: Uuid,
    product_id: Uuid,
    qty: i64,
    discount: f64,
) -> (Uuid, Value) {
    let (status, q) = send(
        app,
        "POST",
        "/api/quotes",
        Some(cookie),
        Some(json!({
            "opportunity_id": opp_id,
            "lines": [{ "product_id": product_id, "qty": qty, "discount_pct": discount }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "draft quote: {q}");
    let id = Uuid::parse_str(q["id"].as_str().unwrap()).unwrap();
    (id, q)
}

// ── 401 on every new route, unauthenticated ─────────────────────────────────

#[tokio::test]
async fn unauthenticated_is_typed_401_on_every_new_route() {
    let app = test_app().await;
    let some = Uuid::new_v4();
    let cases: [(&str, String); 11] = [
        (
            "GET",
            "/api/accounts/00000000-0000-0000-0000-000000000000".into(),
        ),
        ("POST", "/api/accounts".into()),
        ("GET", "/api/opportunities".into()),
        ("POST", "/api/opportunities".into()),
        ("PATCH", format!("/api/opportunities/{some}/stage")),
        ("GET", "/api/quotes".into()),
        ("POST", "/api/quotes".into()),
        ("POST", format!("/api/quotes/{some}/submit")),
        ("GET", format!("/api/quotes/{some}/audit")),
        ("GET", "/api/policy/discount".into()),
        ("GET", "/api/activities".into()),
    ];
    for (method, uri) in cases {
        let (status, body) = send(&app, method, &uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(body["error"]["code"], "unauthorized", "{method} {uri}");
    }
}

// ── serena cannot reach across the RLS boundary — 404, never 500 ────────────

#[tokio::test]
async fn rep_foreign_access_is_404() {
    let app = test_app().await;
    let vp = login(&app, VP).await;
    let rep = login(&app, SE1_REP).await;
    let product = a_consumable_product(&app, &rep).await;

    // A foreign account + a foreign opportunity (resolved from the VP's view).
    let foreign_acct = foreign_account(&app, &vp, "SE-1").await;
    let (_, opps) = send(&app, "GET", "/api/opportunities?limit=200", Some(&vp), None).await;
    let foreign_opp = items(&opps)
        .iter()
        .find(|o| o["territory_code"] != "SE-1")
        .expect("a foreign opp exists")["id"]
        .as_str()
        .unwrap()
        .to_string();

    // GET foreign account 360 → 404.
    let (s, _) = send(
        &app,
        "GET",
        &format!("/api/accounts/{foreign_acct}"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "foreign account 360");

    // PATCH foreign opp stage → 404.
    let (s, _) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{foreign_opp}/stage"),
        Some(&rep),
        Some(json!({ "stage": "negotiation" })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "foreign opp stage");

    // POST quote on foreign opp → 404.
    let (s, _) = send(
        &app,
        "POST",
        "/api/quotes",
        Some(&rep),
        Some(json!({ "opportunity_id": foreign_opp,
                     "lines": [{ "product_id": product, "qty": 1, "discount_pct": 5.0 }] })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "quote on foreign opp");

    // POST activity on foreign account → 404.
    let (s, _) = send(
        &app,
        "POST",
        "/api/activities",
        Some(&rep),
        Some(json!({ "account_id": foreign_acct, "kind": "call", "body": "hello" })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "activity on foreign account");
}

// ── the R3 role gate — the heart of the phase ────────────────────────────────

#[tokio::test]
async fn rep_cannot_approve_own_pending_quote() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    let opp = create_opp(&app, &rep, acct).await;
    let (quote, _) = draft_quote(&app, &rep, opp, product, 32, 28.0).await;
    let (s, body) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["status"], "pending_approval");

    // serena is the creator and a rep — the role gate refuses her own approval.
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/approve"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "a rep cannot approve their own quote"
    );
    // ...and reject is likewise gated.
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/reject"),
        Some(&rep),
        Some(json!({ "reason": "no" })),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::FORBIDDEN,
        "a rep cannot reject their own quote"
    );
}

#[tokio::test]
async fn regional_manager_tier_gate() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let rm = login(&app, RM_SE).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    // >25% quote → VP tier: the regional manager is refused (403).
    let opp_hi = create_opp(&app, &rep, acct).await;
    let (q_hi, _) = draft_quote(&app, &rep, opp_hi, product, 10, 28.0).await;
    send(
        &app,
        "POST",
        &format!("/api/quotes/{q_hi}/submit"),
        Some(&rep),
        None,
    )
    .await;
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q_hi}/approve"),
        Some(&rm),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "RM cannot approve a >25% quote");

    // 10–25% quote → manager tier: the regional manager CAN approve (200).
    let opp_mid = create_opp(&app, &rep, acct).await;
    let (q_mid, _) = draft_quote(&app, &rep, opp_mid, product, 10, 18.0).await;
    let (s, body) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q_mid}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        body["status"], "pending_approval",
        "18% needs manager approval"
    );
    let (s, approved) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q_mid}/approve"),
        Some(&rm),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "RM approves an in-scope 10–25% quote");
    assert_eq!(approved["status"], "approved");
}

#[tokio::test]
async fn vp_approves_28_and_audit_actor_is_vp() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;
    let vp_id = me_id(&app, &vp).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    let opp = create_opp(&app, &rep, acct).await;
    let (quote, _) = draft_quote(&app, &rep, opp, product, 32, 28.0).await;
    let (_, submitted) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(submitted["status"], "pending_approval");
    assert_eq!(
        submitted["discount_policy_result"]["verdict"],
        "vp_approval"
    );

    let (s, approved) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/approve"),
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "VP approves the 28%");
    assert_eq!(approved["status"], "approved");

    // The audit drawer: the last row is the VP's approval, actor = VP's id.
    let (s, audit) = send(
        &app,
        "GET",
        &format!("/api/quotes/{quote}/audit"),
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rows = items(&audit);
    let last = rows.last().expect("at least one audit row");
    assert_eq!(last["after_status"], "approved");
    assert_eq!(last["actor_id"], json!(vp_id.to_string()));
    assert_eq!(last["actor_name"], "Valerie Price");
    // and the submit row belongs to serena.
    assert!(
        rows.iter()
            .any(|r| r["after_status"] == "pending_approval" && r["actor_name"] == "Serena Estes"),
        "the submit row is attributed to serena"
    );
}

// ── the state machine says no in all the right places (422) ─────────────────

#[tokio::test]
async fn state_machine_rejects_illegal_transitions() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    // submit on non-draft → 422 (submit twice).
    let opp = create_opp(&app, &rep, acct).await;
    let (q, _) = draft_quote(&app, &rep, opp, product, 32, 28.0).await;
    send(
        &app,
        "POST",
        &format!("/api/quotes/{q}/submit"),
        Some(&rep),
        None,
    )
    .await;
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "submit on non-draft");

    // approve on draft → 422.
    let opp2 = create_opp(&app, &rep, acct).await;
    let (q2, _) = draft_quote(&app, &rep, opp2, product, 1, 5.0).await;
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q2}/approve"),
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "approve on draft");

    // won without an approved quote → 422.
    let opp3 = create_opp(&app, &rep, acct).await;
    let (s, body) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{opp3}/stage"),
        Some(&rep),
        Some(json!({ "stage": "won" })),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::UNPROCESSABLE_ENTITY,
        "won without approved quote"
    );
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("approved quote"));

    // lost without a reason → 422.
    let opp4 = create_opp(&app, &rep, acct).await;
    let (s, _) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{opp4}/stage"),
        Some(&rep),
        Some(json!({ "stage": "lost" })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "lost without reason");

    // stage change OUT of won → 422 (won is terminal). Book opp5 to won first
    // via a self-approved quote, then try to move it.
    let opp5 = create_opp(&app, &rep, acct).await;
    let (q5, _) = draft_quote(&app, &rep, opp5, product, 1, 5.0).await; // ≤10% self-approves
    let (_, sub5) = send(
        &app,
        "POST",
        &format!("/api/quotes/{q5}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(sub5["status"], "approved", "5% self-approves at submit");
    let (s, _) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{opp5}/stage"),
        Some(&rep),
        Some(json!({ "stage": "won" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "won books");
    let (s, body) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{opp5}/stage"),
        Some(&rep),
        Some(json!({ "stage": "negotiation" })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "out of won");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("terminal"));

    // limit=201 → 422 (R8, on a new list endpoint).
    let (s, _) = send(
        &app,
        "GET",
        "/api/opportunities?limit=201",
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "limit=201");
}

// ── the money law: forged client prices are ignored ─────────────────────────

#[tokio::test]
async fn client_supplied_prices_are_ignored() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    // The catalog's real list price for the product.
    let (_, prods) = send(
        &app,
        "GET",
        "/api/products?kind=consumable&limit=1",
        Some(&rep),
        None,
    )
    .await;
    let catalog_list = items(&prods)[0]["list_price_cents"].as_i64().unwrap();

    let opp = create_opp(&app, &rep, acct).await;
    // Body carries FORGED list/net fields — the server must ignore them.
    let (status, q) = send(
        &app,
        "POST",
        "/api/quotes",
        Some(&rep),
        Some(json!({
            "opportunity_id": opp,
            "lines": [{ "product_id": product, "qty": 3, "discount_pct": 20.0,
                        "list_unit_cents": 1, "net_unit_cents": 1 }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let line = &q["lines"][0];
    assert_eq!(
        line["list_unit_cents"].as_i64().unwrap(),
        catalog_list,
        "server used the catalog list price, not the forged 1"
    );
    // net = round(list * (100 - 20)/100), computed in SQL — never the forged 1.
    let expected_net = ((catalog_list * 80) as f64 / 100.0).round() as i64;
    assert_eq!(line["net_unit_cents"].as_i64().unwrap(), expected_net);
    assert_ne!(line["net_unit_cents"].as_i64().unwrap(), 1);
}

// ── booking integrity: order lines equal the source quote lines, cent-exact ─

#[tokio::test]
async fn booking_copies_quote_lines_cent_exact_and_scopes_the_order() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;
    let foreign = login(&app, W1_REP).await;
    let acct = account_in(&app, &rep, "SE-1").await;
    let product = a_consumable_product(&app, &rep).await;

    let opp = create_opp(&app, &rep, acct).await;
    // A self-approving quote (≤10%) so we own an approved quote to book.
    let (quote, qdetail) = draft_quote(&app, &rep, opp, product, 7, 8.0).await;
    let (_, submitted) = send(
        &app,
        "POST",
        &format!("/api/quotes/{quote}/submit"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(submitted["status"], "approved");
    let q_gross = submitted["gross_cents"].as_i64().unwrap();
    let q_net = submitted["net_cents"].as_i64().unwrap();
    assert_eq!(q_gross, qdetail["gross_cents"].as_i64().unwrap());

    let (s, result) = send(
        &app,
        "PATCH",
        &format!("/api/opportunities/{opp}/stage"),
        Some(&rep),
        Some(json!({ "stage": "won" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "won books");
    let order = &result["booked_order"];
    // cent-exact equality, gross AND net.
    assert_eq!(
        order["gross_cents"].as_i64().unwrap(),
        q_gross,
        "order gross == quote gross"
    );
    assert_eq!(
        order["net_cents"].as_i64().unwrap(),
        q_net,
        "order net == quote net"
    );
    // the consumed quote is now accepted.
    assert_eq!(result["accepted_quote_id"], json!(quote.to_string()));

    let order_id = order["id"].as_str().unwrap();

    // Visibility: serena and the VP both see the booked order in the 360 (it is
    // present and recent; several tests may book on this account the same day,
    // so we assert membership, not the exact top slot — a fresh-seed acceptance
    // run sees exactly one today-order and it IS at the top). A foreign rep sees
    // the account as 404.
    let has_order = |v: &Value| {
        v["recent_orders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["id"].as_str().unwrap() == order_id)
    };
    let (_, s360) = send(
        &app,
        "GET",
        &format!("/api/accounts/{acct}"),
        Some(&rep),
        None,
    )
    .await;
    assert!(has_order(&s360), "serena sees the booked order");
    let (_, v360) = send(
        &app,
        "GET",
        &format!("/api/accounts/{acct}"),
        Some(&vp),
        None,
    )
    .await;
    assert!(has_order(&v360), "the VP sees the booked order");
    let (fs, _) = send(
        &app,
        "GET",
        &format!("/api/accounts/{acct}"),
        Some(&foreign),
        None,
    )
    .await;
    assert_eq!(
        fs,
        StatusCode::NOT_FOUND,
        "a foreign rep cannot see the SE-1 account"
    );
}

// ── audit_log is immutable to the app role (0011 REVOKE) ────────────────────

#[tokio::test]
async fn audit_log_is_app_immutable() {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("app pool");

    // plenum_app retains SELECT + INSERT (the triggers write; the UI reads)…
    assert!(
        sqlx::query("SELECT count(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .is_ok(),
        "plenum_app may still read audit_log"
    );
    // …but UPDATE and DELETE are revoked — the trail is tamper-proof.
    let upd = sqlx::query("UPDATE audit_log SET action = 'tampered'")
        .execute(&pool)
        .await;
    assert!(
        upd.is_err(),
        "plenum_app must NOT be able to UPDATE audit_log"
    );
    let del = sqlx::query("DELETE FROM audit_log").execute(&pool).await;
    assert!(
        del.is_err(),
        "plenum_app must NOT be able to DELETE audit_log"
    );
}
