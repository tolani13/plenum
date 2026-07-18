//! P1 integration tests against the live compose database, over the real
//! router in-process (the rls_http.rs pattern). Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Tier-3 verification: the adversarial scope matrix is part of the build.
//! Every metric endpoint is checked from the rep side (SE-1 only, ever),
//! the VP side (all eight territories), and the hostile side (no session,
//! garbage params, wrong role on the admin refresh).

use api::routes;
use api::state::AppState;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::util::ServiceExt;

const SE1_REP_EMAIL: &str = "serena.estes@plenum.demo";
const VP_EMAIL: &str = "valerie.price@plenum.demo";
const ADMIN_EMAIL: &str = "priya.nair@plenum.demo";
const PASSWORD: &str = "demo-plenum-2026";

const ALL_CODES: [&str; 8] = [
    "CE-1", "CW-1", "MT-1", "MW-1", "NE-1", "SC-1", "SE-1", "W-1",
];

async fn test_app() -> Router {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("test database reachable — run: docker compose up -d && cargo run --bin seed");
    routes::app(AppState { pool }, false)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body collects")
        .to_bytes();
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
        .expect("cookie is ascii")
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn get(app: &Router, cookie: Option<&str>, uri: &str) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("request builds"))
        .await
        .expect("router serves");
    (response.status(), body_json(response).await)
}

async fn post(app: &Router, cookie: Option<&str>, uri: &str) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("POST").uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("request builds"))
        .await
        .expect("router serves");
    (response.status(), body_json(response).await)
}

fn items(page: &Value) -> &Vec<Value> {
    page["items"].as_array().expect("items array")
}

fn strings(page: &Value, field: &str) -> Vec<String> {
    items(page)
        .iter()
        .map(|i| i[field].as_str().expect(field).to_string())
        .collect()
}

fn sorted_unique(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

fn cents(row: &Value, field: &str) -> i64 {
    row[field].as_i64().unwrap_or_else(|| panic!("{field} i64"))
}

fn assert_gross_covers_net(page: &Value, context: &str) {
    for row in items(page) {
        assert!(
            cents(row, "gross_cents") >= cents(row, "net_cents"),
            "{context}: gross < net in {row}"
        );
    }
}

// ── the rep side: SE-1 and nothing else, on every endpoint ─────────────────

#[tokio::test]
async fn rep_scope_on_every_metric_endpoint() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP_EMAIL).await;

    // 1 · territories — exactly one tile.
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(strings(&page, "territory_code"), vec!["SE-1"]);
    assert_eq!(page["total"], json!(1));
    assert_gross_covers_net(&page, "rep territories");

    // 2 · leaderboard — SE-1 has exactly one rep; nobody else's name leaks.
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/leaderboard?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(strings(&page, "rep_name"), vec!["Serena Estes"]);
    assert_gross_covers_net(&page, "rep leaderboard");

    // 3 · items — rows exist and the dual ledger holds; consumable rows
    // carry an attach rate.
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/items?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!items(&page).is_empty());
    assert_gross_covers_net(&page, "rep items");
    assert!(
        items(&page)
            .iter()
            .any(|r| r["kind"] == "consumable" && r["attach_rate_pct"].is_number()),
        "some consumable item carries an attach rate"
    );

    // 4 · customers — every account the metric names is an account the P0
    // accounts endpoint (RLS-proven in rls_http.rs) shows this rep.
    let (_, accounts_page) = get(&app, Some(&rep), "/api/accounts?limit=200").await;
    let visible = sorted_unique(strings(&accounts_page, "name"));
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/customers?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!items(&page).is_empty());
    for name in strings(&page, "account_name") {
        assert!(
            visible.binary_search(&name).is_ok(),
            "customer {name} is outside the rep's RLS scope"
        );
    }
    assert_gross_covers_net(&page, "rep customers");

    // 5 · leakage by territory — the only group the board may show is SE-1.
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/leakage?period=cumulative&by=territory&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(strings(&page, "name"), vec!["SE-1"]);
    for row in page["outliers"].as_array().expect("outliers") {
        assert!(row["family"].as_str().is_some());
    }

    // 6 · coverage — SE-1 has due units this quarter; only SE-1 appears.
    let (status, page) = get(
        &app,
        Some(&rep),
        "/api/metrics/coverage?basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(strings(&page, "territory_code"), vec!["SE-1"]);

    // 7 · defection — the Ridgeline beat lives in SE-1; nothing foreign.
    let (status, page) = get(&app, Some(&rep), "/api/metrics/defection?limit=200").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!items(&page).is_empty());
    assert_eq!(
        sorted_unique(strings(&page, "territory_code")),
        vec!["SE-1"]
    );
    assert!(
        strings(&page, "account_name")
            .iter()
            .any(|n| n == "Ridgeline Grain Cooperative"),
        "the Ridgeline silence beat surfaces for its own rep"
    );
}

// ── the VP side: all eight, and the same cents the rep sees ────────────────

#[tokio::test]
async fn vp_sees_all_eight_and_rep_numbers_reconcile() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP_EMAIL).await;
    let vp = login(&app, VP_EMAIL).await;

    let (_, rep_page) = get(
        &app,
        Some(&rep),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;
    let (status, vp_page) = get(
        &app,
        Some(&vp),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_unique(strings(&vp_page, "territory_code")),
        ALL_CODES
    );

    // The VP's SE-1 row and the rep's SE-1 row are the same cents — scope
    // changes WHICH rows exist, never what a row is worth.
    let rep_se1 = &items(&rep_page)[0];
    let vp_se1 = items(&vp_page)
        .iter()
        .find(|r| r["territory_code"] == "SE-1")
        .expect("VP sees SE-1");
    for field in ["gross_cents", "net_cents", "leakage_cents", "order_count"] {
        assert_eq!(rep_se1[field], vp_se1[field], "SE-1 {field} rep vs VP");
    }

    let rep_net: i64 = items(&rep_page).iter().map(|r| cents(r, "net_cents")).sum();
    let vp_net: i64 = items(&vp_page).iter().map(|r| cents(r, "net_cents")).sum();
    assert!(
        rep_net < vp_net,
        "one territory must be worth less than all eight"
    );
}

// ── Gate P1-1 (spec §11): the basis toggle re-ranks the top customers ──────

#[tokio::test]
async fn gate_p1_1_customers_basis_toggle() {
    let app = test_app().await;
    let vp = login(&app, VP_EMAIL).await;

    let (status, by_gross) = get(
        &app,
        Some(&vp),
        "/api/metrics/customers?period=2025&basis=gross&limit=10",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, by_net) = get(
        &app,
        Some(&vp),
        "/api/metrics/customers?period=2025&basis=net&limit=10",
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let gross_names = strings(&by_gross, "account_name");
    let net_names = strings(&by_net, "account_name");
    assert_eq!(gross_names.len(), 10);
    assert_ne!(
        gross_names, net_names,
        "the gross/net toggle must re-rank the top 10"
    );
    assert_gross_covers_net(&by_gross, "customers by gross");
    assert_gross_covers_net(&by_net, "customers by net");
}

// ── rollup path vs live path agree (per territory, both bases) ─────────────

#[tokio::test]
async fn rollup_years_sum_to_live_cumulative() {
    let app = test_app().await;
    let vp = login(&app, VP_EMAIL).await;

    // cumulative rides the live v_order_facts path; the years ride the
    // scoped rollup views (plus the live current quarter inside the view).
    // Order history spans 2023..2026, so the four years must sum exactly to
    // cumulative — cents, not approximately.
    let (_, cumulative) = get(
        &app,
        Some(&vp),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;
    let mut year_sums: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();
    for year in ["2023", "2024", "2025", "2026"] {
        let (status, page) = get(
            &app,
            Some(&vp),
            &format!("/api/metrics/territories?period={year}&basis=net&limit=200"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "year {year}");
        for row in items(&page) {
            let e = year_sums
                .entry(row["territory_code"].as_str().unwrap().to_string())
                .or_insert((0, 0));
            e.0 += cents(row, "gross_cents");
            e.1 += cents(row, "net_cents");
        }
    }
    assert_eq!(items(&cumulative).len(), 8);
    for row in items(&cumulative) {
        let code = row["territory_code"].as_str().unwrap();
        let (gross, net) = year_sums[code];
        assert_eq!(cents(row, "gross_cents"), gross, "{code} gross");
        assert_eq!(cents(row, "net_cents"), net, "{code} net");
    }
}

// ── kind slices: the split columns obey the filter ─────────────────────────

#[tokio::test]
async fn kind_filter_zeroes_the_other_ledger() {
    let app = test_app().await;
    let vp = login(&app, VP_EMAIL).await;

    let (status, page) = get(
        &app,
        Some(&vp),
        "/api/metrics/leaderboard?period=2025&basis=net&kind=capital&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!items(&page).is_empty());
    for row in items(&page) {
        assert_eq!(cents(row, "consumable_gross_cents"), 0, "capital slice");
        assert_eq!(
            cents(row, "gross_cents"),
            cents(row, "capital_gross_cents"),
            "under kind=capital the whole ledger IS the capital ledger"
        );
    }
    assert_gross_covers_net(&page, "leaderboard kind=capital");

    // ttm exercises the live path with the database's own clock.
    let (status, page) = get(
        &app,
        Some(&vp),
        "/api/metrics/leaderboard?period=ttm&basis=gross&limit=200",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!items(&page).is_empty());
    assert_gross_covers_net(&page, "leaderboard ttm");
}

// ── no session, no numbers — all seven plus the refresh ────────────────────

#[tokio::test]
async fn unauthenticated_is_typed_401_everywhere() {
    let app = test_app().await;
    for uri in [
        "/api/metrics/territories?period=2025&basis=net",
        "/api/metrics/leaderboard?period=2025&basis=net",
        "/api/metrics/items?period=2025&basis=net",
        "/api/metrics/customers?period=2025&basis=net",
        "/api/metrics/leakage?period=2025",
        "/api/metrics/coverage?basis=net",
        "/api/metrics/defection",
    ] {
        let (status, body) = get(&app, None, uri).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(body["error"]["code"], "unauthorized", "{uri}");
    }
    let (status, body) = post(&app, None, "/api/admin/refresh-rollups").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");
}

// ── garbage in, typed 422 out ──────────────────────────────────────────────

#[tokio::test]
async fn garbage_params_are_typed_422() {
    let app = test_app().await;
    let vp = login(&app, VP_EMAIL).await;

    let cases: [(&str, &str); 14] = [
        // (uri, expected fragment of the plain-language message)
        (
            "/api/metrics/customers?period=2025&basis=vibes",
            "basis must be",
        ),
        ("/api/metrics/customers?basis=net", "period is required"),
        (
            "/api/metrics/customers?period=20255&basis=net",
            "period must be",
        ),
        (
            "/api/metrics/customers?period=2025-Q5&basis=net",
            "period must be",
        ),
        (
            "/api/metrics/customers?period=2025&basis=net&kind=parts",
            "kind must be",
        ),
        (
            "/api/metrics/territories?period=2025&basis=net&limit=500",
            "limit must be",
        ),
        (
            "/api/metrics/territories?period=2025&basis=net&offset=-5",
            "offset must be",
        ),
        (
            "/api/metrics/territories?period=2025&basis=net&limit=abc",
            "limit must be",
        ),
        (
            "/api/metrics/items?period=2025&basis=net&group=vibes",
            "group must be",
        ),
        ("/api/metrics/leakage?period=2025&by=vibes", "by must be"),
        (
            "/api/metrics/leakage?period=2025&basis=net",
            "not a parameter",
        ),
        (
            "/api/metrics/coverage?basis=net&period=2025",
            "current quarter",
        ),
        ("/api/metrics/coverage", "basis is required"),
        ("/api/metrics/defection?period=2025", "no period parameter"),
    ];
    for (uri, fragment) in cases {
        let (status, body) = get(&app, Some(&vp), uri).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body}");
        assert_eq!(body["error"]["code"], "invalid", "{uri}");
        let message = body["error"]["message"].as_str().expect("message");
        assert!(
            message.contains(fragment),
            "{uri}: message {message:?} lacks {fragment:?}"
        );
    }

    // A valid but empty period is a 200 with empty items — empty ≠ error.
    let (status, page) = get(
        &app,
        Some(&vp),
        "/api/metrics/customers?period=2074&basis=net",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(items(&page).is_empty());
    assert_eq!(page["total"], json!(0));
}

// ── the refresh is admin-only and does not move settled numbers ────────────

#[tokio::test]
async fn refresh_rollups_is_admin_gated_and_stable() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP_EMAIL).await;
    let vp = login(&app, VP_EMAIL).await;
    let admin = login(&app, ADMIN_EMAIL).await;

    let (status, body) = post(&app, Some(&rep), "/api/admin/refresh-rollups").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "rep must not refresh");
    assert_eq!(body["error"]["code"], "forbidden");

    // The VP outranks a rep everywhere EXCEPT here: the gate is role=admin,
    // not seniority.
    let (status, _) = post(&app, Some(&vp), "/api/admin/refresh-rollups").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "vp must not refresh either");

    let (_, before) = get(
        &app,
        Some(&vp),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;

    let (status, body) = post(&app, Some(&admin), "/api/admin/refresh-rollups").await;
    assert_eq!(status, StatusCode::OK, "admin refresh: {body}");
    let refreshed = body["refreshed"].as_array().expect("refreshed array");
    let names: Vec<&str> = refreshed
        .iter()
        .map(|r| r["matview"].as_str().expect("matview"))
        .collect();
    assert_eq!(
        names,
        vec![
            "mv_territory_period",
            "mv_rep_period",
            "mv_product_period",
            "mv_customer_period"
        ]
    );
    for r in refreshed {
        assert!(r["row_count"].as_i64().expect("row_count") > 0);
    }

    // Refreshing rollups over unchanged facts must not move a single cent.
    let (_, after) = get(
        &app,
        Some(&vp),
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
    )
    .await;
    assert_eq!(before["items"], after["items"]);
}
