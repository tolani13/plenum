//! Integration test against the live compose database: login → accounts
//! scoping over the real router, in-process. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//! (Tier-3 verification for auth/tenancy — spec §4 non-negotiable.)

use api::routes;
use api::state::{AiConfig, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::util::ServiceExt;

const SE1_REP_EMAIL: &str = "serena.estes@plenum.demo";
const VP_EMAIL: &str = "valerie.price@plenum.demo";
const RM_CENTRAL_EMAIL: &str = "marcus.reed@plenum.demo";
const PASSWORD: &str = "demo-plenum-2026";

async fn test_app() -> Router {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(4)
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
    serde_json::from_slice(&bytes).expect("body is JSON")
}

async fn login(app: &Router, email: &str, password: &str) -> (StatusCode, Option<String>, Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "email": email, "password": password }).to_string(),
        ))
        .expect("request builds");
    let response = app.clone().oneshot(request).await.expect("router serves");
    let status = response.status();
    let cookie = response.headers().get(header::SET_COOKIE).map(|v| {
        v.to_str()
            .expect("cookie is ascii")
            .split(';')
            .next()
            .unwrap()
            .to_string()
    });
    (status, cookie, body_json(response).await)
}

async fn get_accounts(app: &Router, cookie: Option<&str>, query: &str) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!("/api/accounts{query}"));
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

fn territory_codes(page: &Value) -> Vec<String> {
    let mut codes: Vec<String> = page["items"]
        .as_array()
        .expect("items array")
        .iter()
        .map(|item| {
            item["territory_code"]
                .as_str()
                .expect("territory_code")
                .to_string()
        })
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

#[tokio::test]
async fn login_then_accounts_is_rls_scoped() {
    let app = test_app().await;

    // SE-1 rep: every account SE-1, nothing else, ever.
    let (status, cookie, _) = login(&app, SE1_REP_EMAIL, PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    let cookie = cookie.expect("login sets a session cookie");
    let (status, page) = get_accounts(&app, Some(&cookie), "?limit=200").await;
    assert_eq!(status, StatusCode::OK);
    let items = page["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "SE-1 rep sees their own accounts");
    assert_eq!(
        territory_codes(&page),
        vec!["SE-1"],
        "rep must see ONLY SE-1"
    );
    assert_eq!(page["total"].as_i64().expect("total"), items.len() as i64);

    // VP: all 8 territory codes present.
    let (status, vp_cookie, _) = login(&app, VP_EMAIL, PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    let vp_cookie = vp_cookie.expect("login sets a session cookie");
    let (status, page) = get_accounts(&app, Some(&vp_cookie), "?limit=200").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        territory_codes(&page),
        vec!["CE-1", "CW-1", "MT-1", "MW-1", "NE-1", "SC-1", "SE-1", "W-1"],
        "VP must see accounts from all 8 territories"
    );

    // Regional manager (Central): exactly the reports' territories.
    let (status, rm_cookie, _) = login(&app, RM_CENTRAL_EMAIL, PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    let rm_cookie = rm_cookie.expect("login sets a session cookie");
    let (status, page) = get_accounts(&app, Some(&rm_cookie), "?limit=200").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        territory_codes(&page),
        vec!["CE-1", "CW-1", "MT-1", "MW-1"],
        "RM scope is the union of their reports' assignments"
    );
}

#[tokio::test]
async fn no_session_is_typed_401() {
    let app = test_app().await;
    let (status, body) = get_accounts(&app, None, "").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn bad_credentials_are_401_without_user_enumeration() {
    let app = test_app().await;
    let (status_wrong_pw, cookie, body_wrong_pw) = login(&app, VP_EMAIL, "not-the-password").await;
    assert_eq!(status_wrong_pw, StatusCode::UNAUTHORIZED);
    assert!(
        cookie.is_none(),
        "failed login must not set a session cookie"
    );
    let (status_unknown, _, body_unknown) =
        login(&app, "nobody@plenum.demo", "not-the-password").await;
    assert_eq!(status_unknown, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body_wrong_pw, body_unknown,
        "unknown email and wrong password must be indistinguishable"
    );
}

#[tokio::test]
async fn pagination_limits_are_typed_422() {
    let app = test_app().await;
    let (_, cookie, _) = login(&app, VP_EMAIL, PASSWORD).await;
    let cookie = cookie.expect("login sets a session cookie");

    let (status, body) = get_accounts(&app, Some(&cookie), "?limit=500").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "invalid");

    let (status, _) = get_accounts(&app, Some(&cookie), "?limit=-1").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = get_accounts(&app, Some(&cookie), "?offset=-5").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = get_accounts(&app, Some(&cookie), "?limit=abc").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn me_reports_scope_codes() {
    let app = test_app().await;
    let (_, cookie, _) = login(&app, SE1_REP_EMAIL, PASSWORD).await;
    let cookie = cookie.expect("login sets a session cookie");

    let request = Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .expect("request builds");
    let response = app.clone().oneshot(request).await.expect("router serves");
    assert_eq!(response.status(), StatusCode::OK);
    let me = body_json(response).await;
    assert_eq!(me["email"], SE1_REP_EMAIL);
    assert_eq!(me["territories"], json!(["SE-1"]));
}
