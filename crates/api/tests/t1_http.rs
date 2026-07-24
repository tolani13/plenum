//! T1 — Territory Map Editing (planning view): the Tier-3 adversarial +
//! integration matrix over the real router against the live compose
//! database. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Covers the T1-D8 minimums:
//!   · role gate — serena (rep) AND rachel (regional_manager) get 403 on
//!     EVERY T1 endpoint (writes and the disclosed read), 401 unauth;
//!   · the PUT validation matrix — unknown state 404, Canada block AND
//!     province 422 (v1 lock), unknown territory 422;
//!   · paint writes an audit row with the ACTING user as actor; an
//!     identical repaint writes zero audit noise (the no-clobber law);
//!   · PLANNING-VIEW LAW AS A TEST — the Territory Board feed
//!     (/api/metrics/territories) is byte-identical across a reassignment
//!     while /api/metrics/states regroups live (period=2023: a frozen year
//!     the concurrent crm_http bookings can never touch);
//!   · create/patch/delete — dup code / bad code / unknown region / bad
//!     color_token 422s; 201 with quota 0; delete refused with a reason
//!     NAMING the failing emptiness check; empty territory deletes clean.
//!
//! Concurrency: cargo runs test BINARIES in parallel, so an in-process
//! mutex cannot serialize this file against p5_http's states test (which
//! pins roster == 8 territories and all-canonical grouping). Every
//! geography-mutating test here — and p5's states reader — takes the SAME
//! Postgres advisory lock (transaction-scoped: a panic rolls back and
//! releases). The canonical map is restored before each lock is dropped.

use api::routes;
use api::state::{AiConfig, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};
use tower::util::ServiceExt;

const SE1_REP: &str = "serena.estes@plenum.demo";
const MANAGER: &str = "rachel.moore@plenum.demo";
const VP: &str = "valerie.price@plenum.demo";
const ADMIN: &str = "priya.nair@plenum.demo";
const PASSWORD: &str = "demo-plenum-2026";

/// The cross-binary geography lock key (shared with p5_http's states test).
const GEO_LOCK_KEY: i64 = 54_311_743;

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| api::DEFAULT_APP_URL.into());
    PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .expect("test database reachable — run: docker compose up -d && cargo run --bin seed")
}

fn test_ai_config() -> AiConfig {
    AiConfig {
        api_key: None,
        model: "claude-sonnet-5".to_string(),
        ask_flag: true,
        discount_flag: true,
    }
}

async fn test_app(pool: PgPool) -> Router {
    routes::app(
        AppState {
            pool,
            ai: test_ai_config(),
        },
        false,
    )
}

/// The cross-process geography serializer: transaction-scoped advisory lock
/// held for the caller's whole mutation window; drop (rollback) releases it
/// even on a panicked assertion.
async fn geo_lock(pool: &PgPool) -> Transaction<'_, Postgres> {
    let mut tx = pool.begin().await.expect("lock tx begins");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(GEO_LOCK_KEY)
        .execute(&mut *tx)
        .await
        .expect("advisory lock acquired");
    tx
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

fn items(page: &Value) -> &Vec<Value> {
    page["items"].as_array().expect("items array")
}

async fn me_id(app: &Router, cookie: &str) -> String {
    let (s, me) = send(app, "GET", "/api/auth/me", Some(cookie), None).await;
    assert_eq!(s, StatusCode::OK);
    me["id"].as_str().expect("me.id").to_string()
}

async fn audit_count(pool: &PgPool, entity: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE entity = $1")
        .bind(entity)
        .fetch_one(pool)
        .await
        .expect("audit readable")
}

// ── 1 · role gate: rep AND manager 403 everywhere, 401 unauth ───────────────

#[tokio::test]
async fn t1_role_gate_rep_and_manager_forbidden_everywhere() {
    let pool = test_pool().await;
    let app = test_app(pool.clone()).await;

    let endpoints: Vec<(&str, &str, Option<Value>)> = vec![
        ("GET", "/api/territories", None),
        (
            "PUT",
            "/api/territory-states/GA",
            Some(json!({ "territory_code": "MT-1" })),
        ),
        (
            "POST",
            "/api/territories",
            Some(json!({ "code": "T8-1", "name": "Nope", "region": "mountain" })),
        ),
        (
            "PATCH",
            "/api/territories/SE-1",
            Some(json!({ "name": "Nope" })),
        ),
        ("DELETE", "/api/territories/SE-1", None),
    ];

    // 401 without a session, on every endpoint.
    for (method, uri, body) in &endpoints {
        let (s, resp) = send(&app, method, uri, None, body.clone()).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED, "{method} {uri} unauth");
        assert_eq!(resp["error"]["code"], "unauthorized", "{method} {uri}");
    }

    // 403 for the rep AND for the manager — managers were explicitly
    // rejected from this surface (T1-D10).
    for email in [SE1_REP, MANAGER] {
        let cookie = login(&app, email).await;
        for (method, uri, body) in &endpoints {
            let (s, resp) = send(&app, method, uri, Some(&cookie), body.clone()).await;
            assert_eq!(s, StatusCode::FORBIDDEN, "{email}: {method} {uri}");
            assert_eq!(
                resp["error"]["code"], "forbidden",
                "{email}: {method} {uri}"
            );
        }
    }

    // Nothing moved: GA still canonical SE-1.
    let ga: String =
        sqlx::query_scalar("SELECT territory_code FROM territory_states WHERE state_code = 'GA'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ga, "SE-1", "role-gated attempts must not move geography");
}

// ── 2 · PUT validation + audit actor + the planning-view law ────────────────

#[tokio::test]
async fn t1_paint_validation_audit_actor_and_planning_view_law() {
    let pool = test_pool().await;
    let _geo = geo_lock(&pool).await;
    let app = test_app(pool.clone()).await;
    let vp = login(&app, VP).await;
    let vp_id = me_id(&app, &vp).await;

    // Validation matrix.
    let (s, _) = send(
        &app,
        "PUT",
        "/api/territory-states/ZZ",
        Some(&vp),
        Some(json!({ "territory_code": "MT-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "unknown state is a 404");

    for canada in ["CA-E", "ON"] {
        let (s, resp) = send(
            &app,
            "PUT",
            &format!("/api/territory-states/{canada}"),
            Some(&vp),
            Some(json!({ "territory_code": "NE-1" })),
        )
        .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{canada} is v1-locked");
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(msg.contains("v2"), "the lock message says so: {msg}");
    }

    let (s, resp) = send(
        &app,
        "PUT",
        "/api/territory-states/MT",
        Some(&vp),
        Some(json!({ "territory_code": "NOPE" })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(resp["error"]["message"], "unknown territory");

    // Pick a non-SE-1 state with 2023 money (a frozen year: the concurrent
    // crm_http bookings land on CURRENT_DATE and can never touch it).
    let (s, states_before) = send(
        &app,
        "GET",
        "/api/metrics/states?period=2023&basis=net&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let victim = items(&states_before)
        .iter()
        .find(|r| r["territory_code"] == "MT-1")
        .expect("an MT-1 state with 2023 money")
        .clone();
    let victim_code = victim["state_code"].as_str().unwrap().to_string();

    // The Board feed, before (the official figure the law protects).
    let board_uri = "/api/metrics/territories?period=2023&basis=net&limit=200";
    let (s, board_before) = send(&app, "GET", board_uri, Some(&vp), None).await;
    assert_eq!(s, StatusCode::OK);

    let audit_before = audit_count(&pool, "territory_states").await;

    // Paint it into W-1.
    let (s, resp) = send(
        &app,
        "PUT",
        &format!("/api/territory-states/{victim_code}"),
        Some(&vp),
        Some(json!({ "territory_code": "W-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(resp["state_code"], json!(victim_code));
    assert_eq!(resp["territory_code"], "W-1");

    // metrics/states regroups LIVE (T1-D7): the row now carries W-1, the
    // roster moves the code, and the two territories' sums mirror the move.
    let (s, states_after) = send(
        &app,
        "GET",
        "/api/metrics/states?period=2023&basis=net&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let moved = items(&states_after)
        .iter()
        .find(|r| r["state_code"] == json!(victim_code))
        .expect("moved state still has its row");
    assert_eq!(moved["territory_code"], "W-1", "regrouped under the target");
    assert_eq!(
        moved["net_cents"], victim["net_cents"],
        "the state's dollars did not change — only their grouping"
    );
    let sum_of = |page: &Value, terr: &str| -> i64 {
        items(page)
            .iter()
            .filter(|r| r["territory_code"] == json!(terr))
            .map(|r| r["net_cents"].as_i64().unwrap())
            .sum()
    };
    let delta = victim["net_cents"].as_i64().unwrap();
    assert_eq!(
        sum_of(&states_after, "MT-1"),
        sum_of(&states_before, "MT-1") - delta,
        "MT-1's planning sum drops by exactly the state's dollars"
    );
    assert_eq!(
        sum_of(&states_after, "W-1"),
        sum_of(&states_before, "W-1") + delta,
        "W-1's planning sum gains exactly the state's dollars"
    );
    let roster_after = states_after["territories"].as_array().unwrap();
    let w1_states: Vec<&str> = roster_after
        .iter()
        .find(|t| t["territory_code"] == "W-1")
        .unwrap()["state_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(w1_states.contains(&victim_code.as_str()), "roster follows");

    // THE PLANNING-VIEW LAW: the Board feed is byte-identical.
    let (s, board_after) = send(&app, "GET", board_uri, Some(&vp), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        board_before, board_after,
        "map edits must move NOTHING official (Territory Board feed)"
    );

    // Audit: exactly one territory_states row, actor = the acting VP.
    assert_eq!(
        audit_count(&pool, "territory_states").await,
        audit_before + 1
    );
    let (actor, action, before_code, after_code): (String, String, String, String) =
        sqlx::query_as(
            "SELECT actor::text, action, before->>'territory_code', after->>'territory_code'
             FROM audit_log WHERE entity = 'territory_states'
             ORDER BY at DESC, id LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(actor, vp_id, "audit actor is the acting user");
    assert_eq!(action, "UPDATE");
    assert_eq!(before_code, "MT-1");
    assert_eq!(after_code, "W-1");

    // Idempotent repaint: same target, zero audit noise.
    let (s, _) = send(
        &app,
        "PUT",
        &format!("/api/territory-states/{victim_code}"),
        Some(&vp),
        Some(json!({ "territory_code": "W-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        audit_count(&pool, "territory_states").await,
        audit_before + 1,
        "a repaint of the same territory writes no audit row"
    );

    // Restore canon before releasing the geography lock.
    let (s, _) = send(
        &app,
        "PUT",
        &format!("/api/territory-states/{victim_code}"),
        Some(&vp),
        Some(json!({ "territory_code": "MT-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        audit_count(&pool, "territory_states").await,
        audit_before + 2
    );
}

// ── 3 · create / patch / delete — validations, guard reasons, audit ─────────

#[tokio::test]
async fn t1_create_patch_delete_matrix() {
    let pool = test_pool().await;
    let _geo = geo_lock(&pool).await;
    let app = test_app(pool.clone()).await;
    let admin = login(&app, ADMIN).await;
    let admin_id = me_id(&app, &admin).await;
    let vp = login(&app, VP).await;

    // Creation validations, each its own typed 422.
    let cases: Vec<(Value, &str)> = vec![
        (
            json!({ "code": "SE-1", "name": "Dup", "region": "southeast" }),
            "already exists",
        ),
        (
            json!({ "code": "gc1!", "name": "Bad code", "region": "southeast" }),
            "uppercase",
        ),
        (
            json!({ "code": "T9-1", "name": "Bad region", "region": "atlantis" }),
            "unknown region",
        ),
        (
            json!({ "code": "T9-1", "name": "Bad color", "region": "mountain",
                    "color_token": "hotpink" }),
            "planning palette",
        ),
    ];
    for (body, needle) in cases {
        let (s, resp) = send(&app, "POST", "/api/territories", Some(&admin), Some(body)).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(msg.contains(needle), "expected '{needle}' in: {msg}");
    }

    // Create — 201, quota pinned to 0, color_token echoed.
    let (s, created) = send(
        &app,
        "POST",
        "/api/territories",
        Some(&admin),
        Some(json!({ "code": "T9-1", "name": "T1 Test Territory",
                     "region": "mountain", "color_token": "terr-plan-3" })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(created["code"], "T9-1");
    assert_eq!(created["quota_year_cents"], 0, "creation never sets quota");
    assert_eq!(created["color_token"], "terr-plan-3");

    // The disclosed read shows it (vp works too — both roles allowed).
    let (s, list) = send(&app, "GET", "/api/territories", Some(&vp), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        items(&list).iter().any(|t| t["code"] == "T9-1"),
        "a just-created territory appears before it has any states"
    );

    // Rename/recolor.
    let (s, patched) = send(
        &app,
        "PATCH",
        "/api/territories/T9-1",
        Some(&admin),
        Some(json!({ "name": "Renamed Territory", "color_token": "terr-plan-5" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(patched["name"], "Renamed Territory");
    assert_eq!(patched["color_token"], "terr-plan-5");

    let (s, _) = send(
        &app,
        "PATCH",
        "/api/territories/ZZ-9",
        Some(&admin),
        Some(json!({ "name": "Ghost" })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, resp) = send(
        &app,
        "PATCH",
        "/api/territories/T9-1",
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(resp["error"]["message"]
        .as_str()
        .unwrap()
        .contains("nothing to update"));

    // Delete a REAL territory: refused, and the reason NAMES what blocks it.
    let (s, resp) = send(&app, "DELETE", "/api/territories/SE-1", Some(&admin), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "SE-1 must survive");
    let msg = resp["error"]["message"].as_str().unwrap().to_string();
    for needle in ["mapped state", "account", "order"] {
        assert!(msg.contains(needle), "expected '{needle}' in: {msg}");
    }

    // A territory with ONLY a mapped state is still not empty.
    let (s, _) = send(
        &app,
        "PUT",
        "/api/territory-states/MT",
        Some(&admin),
        Some(json!({ "territory_code": "T9-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, resp) = send(&app, "DELETE", "/api/territories/T9-1", Some(&admin), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(resp["error"]["message"]
        .as_str()
        .unwrap()
        .contains("1 mapped state"));

    // Restore the state, then the now-empty territory deletes clean.
    let (s, _) = send(
        &app,
        "PUT",
        "/api/territory-states/MT",
        Some(&admin),
        Some(json!({ "territory_code": "MT-1" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, deleted) = send(&app, "DELETE", "/api/territories/T9-1", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(deleted["deleted"], true);
    let (s, list) = send(&app, "GET", "/api/territories", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        !items(&list).iter().any(|t| t["code"] == "T9-1"),
        "deleted territory is gone from the list"
    );

    // Audit trail: this run's lifecycle is the LAST three T9-1 rows (earlier
    // gauntlet runs may have left their own trio — the suite must be
    // re-runnable without a reseed), actor = the acting admin on each.
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT action, actor::text FROM audit_log
         WHERE entity = 'territories'
           AND (after->>'code' = 'T9-1' OR before->>'code' = 'T9-1')
         ORDER BY at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(rows.len() >= 3, "create/rename/delete all audited");
    let last3: Vec<&str> = rows[rows.len() - 3..]
        .iter()
        .map(|(a, _)| a.as_str())
        .collect();
    assert_eq!(last3, vec!["INSERT", "UPDATE", "DELETE"]);
    for (_, actor) in &rows[rows.len() - 3..] {
        assert_eq!(actor, &admin_id, "every mutation audited to the actor");
    }
}
