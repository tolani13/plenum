//! P4 signals — Tier-3 adversarial + integration matrix over the real router
//! against the live compose database. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Covers: the 401 sweep, RLS 404s on foreign signals, the assign scope gate,
//! reason/outcome requirements, terminal-state 422s, pagination law, rep/VP
//! scope on list + summary, generation idempotency (zero counts + zero audit
//! delta on a same-day rerun), no-resurrection of dismissed cards, the R1/R2
//! fixture proof (invented world inside a rolled-back transaction), and the
//! R13 telemetry endpoint contract.
//!
//! Concurrency notes (cargo test runs tests in parallel): scope assertions
//! are membership-based, never exact-count; the dismiss test targets only the
//! LOWEST-score discount_anomaly card; the telemetry happy path writes 85%
//! (ABOVE the 20% trigger — no signal can fire from it) and restores NULL.
//! The three tests that COMMIT signal mutations (write-backs, generation
//! idempotency, no-resurrection) serialize on DB_MUTATION_LOCK — the
//! idempotency test measures an audit-row delta, which must not see another
//! test's committed writes mid-window. Every consumed card is restored to
//! open at test end, so the suite is indefinitely re-runnable and never eats
//! the demo queue.

use api::routes;
use api::state::{AiConfig, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tower::util::ServiceExt;
use uuid::Uuid;

const SE1_REP: &str = "serena.estes@plenum.demo"; // SE-1
const W1_REP: &str = "wes.turner@plenum.demo"; // foreign to SE-1
const RM_SE: &str = "rachel.moore@plenum.demo"; // regional manager over SE-1
const VP: &str = "valerie.price@plenum.demo";
const ADMIN: &str = "priya.nair@plenum.demo";
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

fn test_ai_config() -> AiConfig {
    AiConfig {
        api_key: None,
        model: "claude-sonnet-5".to_string(),
        ask_flag: true,
        discount_flag: true,
    }
}

async fn test_app() -> Router {
    routes::app(
        AppState {
            pool: test_pool().await,
            ai: test_ai_config(),
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

fn items(page: &Value) -> &Vec<Value> {
    page["items"].as_array().expect("items array")
}

/// Serializes the signal-mutating tests (see the module note).
static DB_MUTATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Restore a signal the test consumed back to open (committed, admin GUC —
/// RLS applies to plenum_app, so the GUC must be pinned exactly like the API
/// path). Tests that action/dismiss seeded cards call this at the end so the
/// suite never starves its own material — and never eats the demo queue.
async fn restore_signal_to_open(pool: &PgPool, id: &str) {
    let admin_id: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'priya.nair@plenum.demo'")
            .fetch_one(pool)
            .await
            .expect("admin exists");
    let mut tx = pool.begin().await.expect("tx");
    sqlx::query(
        "SELECT set_config('app.user_id', $1, true), set_config('app.role', 'admin', true)",
    )
    .bind(admin_id.to_string())
    .execute(&mut *tx)
    .await
    .expect("GUC set");
    sqlx::query(
        "UPDATE signals SET status = 'open', assigned_to = NULL, assigned_at = NULL,
                actioned_at = NULL, outcome = NULL, dismissed_at = NULL,
                dismissed_reason = NULL
         WHERE id = $1::uuid",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .expect("restore");
    tx.commit().await.expect("commit restore");
}

// ── 401 on every new route, unauthenticated ─────────────────────────────────

#[tokio::test]
async fn unauthenticated_is_typed_401_on_every_new_route() {
    let app = test_app().await;
    let some = Uuid::new_v4();
    let cases: [(&str, String); 10] = [
        ("GET", "/api/signals".into()),
        ("GET", "/api/signals/summary".into()),
        ("GET", format!("/api/signals/assignees?account_id={some}")),
        ("POST", format!("/api/signals/{some}/assign")),
        ("POST", format!("/api/signals/{some}/action")),
        ("POST", format!("/api/signals/{some}/dismiss")),
        ("POST", "/api/admin/generate-signals".into()),
        ("GET", "/api/ai/status".into()),
        ("POST", "/api/ai/discount-recommendation".into()),
        ("POST", "/api/telemetry/filter-life".into()),
    ];
    for (method, uri) in cases {
        let (status, body) = send(&app, method, &uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(body["error"]["code"], "unauthorized", "{method} {uri}");
    }
    // /api/ai/ask 401s BEFORE its 503 gate would apply? No — the extractor
    // runs first regardless of flags; prove it explicitly.
    let (status, body) = send(
        &app,
        "POST",
        "/api/ai/ask",
        None,
        Some(json!({"question": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");
}

// ── scope: the rep sees SE-1 only; the VP a superset; foreign writes 404 ────

#[tokio::test]
async fn rep_scope_on_list_and_summary_and_foreign_404() {
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;

    // Every row a rep can list is SE-1 (membership, all four types).
    let (s, rep_page) = send(&app, "GET", "/api/signals?limit=200", Some(&rep), None).await;
    assert_eq!(s, StatusCode::OK);
    let rep_rows = items(&rep_page);
    assert!(!rep_rows.is_empty(), "SE-1 has active signals in the seed");
    for row in rep_rows {
        assert_eq!(row["territory_code"], "SE-1", "foreign card in a rep queue");
    }

    // The summary is internally consistent and SE-1-only for the rep.
    let (s, rep_sum) = send(&app, "GET", "/api/signals/summary", Some(&rep), None).await;
    assert_eq!(s, StatusCode::OK);
    let by = &rep_sum["by_type"];
    let type_sum = by["reorder_due"].as_i64().unwrap()
        + by["defection_risk"].as_i64().unwrap()
        + by["conquest"].as_i64().unwrap()
        + by["discount_anomaly"].as_i64().unwrap();
    assert_eq!(rep_sum["total"].as_i64().unwrap(), type_sum);
    let terr = rep_sum["territories"].as_array().unwrap();
    assert_eq!(terr.len(), 1, "rep summary lists exactly one territory");
    assert_eq!(terr[0]["territory_code"], "SE-1");
    let terr_sum: i64 = terr.iter().map(|t| t["open_count"].as_i64().unwrap()).sum();
    assert_eq!(rep_sum["total"].as_i64().unwrap(), terr_sum);

    // VP: strictly more scope — SE-1 plus others present.
    let (_, vp_sum) = send(&app, "GET", "/api/signals/summary", Some(&vp), None).await;
    let vp_terr = vp_sum["territories"].as_array().unwrap();
    assert!(vp_terr.len() > 1, "VP sees more than one territory");
    assert!(vp_terr.iter().any(|t| t["territory_code"] == "SE-1"));

    // A foreign signal id: 404 for the rep on every write.
    let (_, vp_page) = send(&app, "GET", "/api/signals?limit=200", Some(&vp), None).await;
    let foreign_id = items(&vp_page)
        .iter()
        .find(|r| r["territory_code"] != "SE-1")
        .expect("a foreign active signal exists")["id"]
        .as_str()
        .unwrap()
        .to_string();
    for verb in ["assign", "action", "dismiss"] {
        let body = match verb {
            "assign" => json!({ "assignee_id": Uuid::new_v4() }),
            "action" => json!({ "outcome": "x" }),
            _ => json!({ "reason": "x" }),
        };
        let (s, _) = send(
            &app,
            "POST",
            &format!("/api/signals/{foreign_id}/{verb}"),
            Some(&rep),
            Some(body),
        )
        .await;
        assert_eq!(s, StatusCode::NOT_FOUND, "foreign {verb} must 404");
    }

    // Assignees for a foreign account → 404 (no probing foreign teams).
    let foreign_account = items(&vp_page)
        .iter()
        .find(|r| r["territory_code"] != "SE-1")
        .unwrap()["account_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (s, _) = send(
        &app,
        "GET",
        &format!("/api/signals/assignees?account_id={foreign_account}"),
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Pagination law on the new list.
    let (s, _) = send(&app, "GET", "/api/signals?limit=201", Some(&rep), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "limit=201");
    // Garbage filters are typed 422s.
    let (s, _) = send(&app, "GET", "/api/signals?status=bogus", Some(&rep), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "status=bogus");
    let (s, _) = send(&app, "GET", "/api/signals?type=bogus", Some(&rep), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "type=bogus");
}

// ── the write-backs: assign scope gate, required strings, terminal states ───

#[tokio::test]
async fn write_backs_enforce_scope_reasons_and_terminal_states() {
    let _guard = DB_MUTATION_LOCK.lock().await;
    let app = test_app().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;

    // serena's own ids (SE-1 by the scope test above).
    let (_, page) = send(
        &app,
        "GET",
        "/api/signals?status=open&type=conquest&limit=200",
        Some(&rep),
        None,
    )
    .await;
    let rows = items(&page);
    assert!(rows.len() >= 2, "SE-1 has at least two open conquest cards");
    let sig_a = rows[0]["id"].as_str().unwrap().to_string();
    let sig_b = rows[1]["id"].as_str().unwrap().to_string();
    let serena_id = {
        let (_, me) = send(&app, "GET", "/api/auth/me", Some(&rep), None).await;
        me["id"].as_str().unwrap().to_string()
    };
    let wes_id = {
        let w = login(&app, W1_REP).await;
        let (_, me) = send(&app, "GET", "/api/auth/me", Some(&w), None).await;
        me["id"].as_str().unwrap().to_string()
    };
    let rm_id = {
        let m = login(&app, RM_SE).await;
        let (_, me) = send(&app, "GET", "/api/auth/me", Some(&m), None).await;
        me["id"].as_str().unwrap().to_string()
    };

    // Assign to an out-of-scope user (wes, W-1) → 422.
    let (s, body) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_a}/assign"),
        Some(&vp),
        Some(json!({ "assignee_id": wes_id })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "out-of-scope assignee");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("territory scope"));

    // Assign to serena (self-scope) → assigned, assigned_at set.
    let (s, assigned) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_a}/assign"),
        Some(&rep),
        Some(json!({ "assignee_id": serena_id })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(assigned["status"], "assigned");
    assert_eq!(assigned["assignee_name"], "Serena Estes");
    let first_assigned_at = assigned["assigned_at"].as_str().unwrap().to_string();

    // Re-assign (to the RM) is allowed; assigned_at records the FIRST time.
    let (s, reassigned) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_a}/assign"),
        Some(&rep),
        Some(json!({ "assignee_id": rm_id })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(reassigned["status"], "assigned");
    assert_eq!(
        reassigned["assigned_at"].as_str().unwrap(),
        first_assigned_at,
        "assigned_at is the first assignment"
    );

    // Action without an outcome → 422; with one → actioned (terminal).
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_a}/action"),
        Some(&rep),
        Some(json!({ "outcome": "  " })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "empty outcome");
    let (s, actioned) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_a}/action"),
        Some(&rep),
        Some(json!({ "outcome": "call_logged" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(actioned["status"], "actioned");
    assert_eq!(actioned["outcome"], "call_logged");

    // Terminal: every further transition is a 422.
    for (verb, body) in [
        ("assign", json!({ "assignee_id": serena_id })),
        ("action", json!({ "outcome": "again" })),
        ("dismiss", json!({ "reason": "again" })),
    ] {
        let (s, resp) = send(
            &app,
            "POST",
            &format!("/api/signals/{sig_a}/{verb}"),
            Some(&rep),
            Some(body),
        )
        .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{verb} on terminal");
        assert!(resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("terminal"));
    }

    // Dismiss requires a reason; with one, the card leaves the active set.
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_b}/dismiss"),
        Some(&rep),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::UNPROCESSABLE_ENTITY,
        "dismiss without reason"
    );
    let (s, dismissed) = send(
        &app,
        "POST",
        &format!("/api/signals/{sig_b}/dismiss"),
        Some(&rep),
        Some(json!({ "reason": "duplicate of another campaign" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(dismissed["status"], "dismissed");
    assert_eq!(
        dismissed["dismissed_reason"],
        "duplicate of another campaign"
    );

    let (_, active_page) = send(
        &app,
        "GET",
        "/api/signals?status=active&type=conquest&limit=200",
        Some(&rep),
        None,
    )
    .await;
    assert!(
        !items(&active_page).iter().any(|r| r["id"] == json!(sig_b)),
        "a dismissed card has left the active view"
    );
    let (_, dis_page) = send(
        &app,
        "GET",
        "/api/signals?status=dismissed&limit=200",
        Some(&rep),
        None,
    )
    .await;
    assert!(
        items(&dis_page).iter().any(|r| r["id"] == json!(sig_b)),
        "the dismissed card sits under the dismissed filter"
    );

    // Hand back what the test consumed — the demo queue keeps its cards and
    // the suite can run again without a reseed.
    let pool = test_pool().await;
    restore_signal_to_open(&pool, &sig_a).await;
    restore_signal_to_open(&pool, &sig_b).await;
}

// ── generation: admin gate, same-day idempotency, zero audit noise ──────────

#[tokio::test]
async fn generate_is_admin_gated_idempotent_and_audit_silent() {
    let _guard = DB_MUTATION_LOCK.lock().await;
    let app = test_app().await;
    let pool = test_pool().await;
    let rep = login(&app, SE1_REP).await;
    let vp = login(&app, VP).await;
    let admin = login(&app, ADMIN).await;

    // Role gate: rep and VP are refused — only admin runs the job.
    for cookie in [&rep, &vp] {
        let (s, _) = send(
            &app,
            "POST",
            "/api/admin/generate-signals",
            Some(cookie),
            None,
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }

    // First call may carry day-drift updates (clock-drifting scores). The
    // SECOND consecutive call must be all-zero and write ZERO audit rows.
    let (s, _) = send(
        &app,
        "POST",
        "/api/admin/generate-signals",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .expect("audit count");
    let (s, second) = send(
        &app,
        "POST",
        "/api/admin/generate-signals",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let audit_after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .expect("audit count");

    let generated = second["generated"].as_array().expect("generated array");
    assert_eq!(generated.len(), 4, "one row per signal type");
    for g in generated {
        assert_eq!(g["inserted"], 0, "second same-day run inserts nothing: {g}");
        assert_eq!(g["updated"], 0, "second same-day run updates nothing: {g}");
        assert_eq!(g["expired"], 0, "second same-day run expires nothing: {g}");
    }
    assert_eq!(
        audit_after - audit_before,
        0,
        "a no-change rerun writes zero audit rows"
    );
}

// ── no resurrection: a dismissed card survives a regeneration ───────────────

#[tokio::test]
async fn dismissed_signal_is_never_resurrected() {
    let _guard = DB_MUTATION_LOCK.lock().await;
    let app = test_app().await;
    let vp = login(&app, VP).await;
    let admin = login(&app, ADMIN).await;

    // The LOWEST-score open discount_anomaly (least demo-relevant; other
    // tests never touch anomaly cards).
    let (_, page) = send(
        &app,
        "GET",
        "/api/signals?status=open&type=discount_anomaly&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let victim = items(&page)
        .last()
        .expect("an open anomaly exists (score ASC tail)")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/signals/{victim}/dismiss"),
        Some(&vp),
        Some(json!({ "reason": "test: known one-off comp" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = send(
        &app,
        "POST",
        "/api/admin/generate-signals",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, after) = send(
        &app,
        "GET",
        "/api/signals?status=dismissed&type=discount_anomaly&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let row = items(&after)
        .iter()
        .find(|r| r["id"] == json!(victim))
        .expect("the dismissed card still exists, still dismissed");
    assert_eq!(row["status"], "dismissed");
    assert_eq!(row["dismissed_reason"], "test: known one-off comp");
    let (_, active) = send(
        &app,
        "GET",
        "/api/signals?status=active&type=discount_anomaly&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert!(
        !items(&active).iter().any(|r| r["id"] == json!(victim)),
        "regeneration did not resurrect the dismissed card"
    );

    // Hand the card back — the assertion is done and the world stays whole.
    let pool = test_pool().await;
    restore_signal_to_open(&pool, &victim).await;
}

// ── R1/R2 fixture proof: invented world, rolled back ────────────────────────
//
// Generators must fire on accounts/units that share NOTHING with the seed —
// no seed names, no seed constants — proving the signals EMERGE from table
// data. Everything happens inside one transaction on the app pool with the
// admin GUC pinned (the same identity the endpoint path uses), then rolls
// back: zero trace, zero audit.

#[tokio::test]
async fn fixture_worlds_produce_signals_from_pure_table_data() {
    let pool = test_pool().await;
    let admin_id: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'priya.nair@plenum.demo'")
            .fetch_one(&pool)
            .await
            .expect("admin exists");

    let mut tx = pool.begin().await.expect("tx");
    sqlx::query(
        "SELECT set_config('app.user_id', $1, true), set_config('app.role', 'admin', true)",
    )
    .bind(admin_id.to_string())
    .execute(&mut *tx)
    .await
    .expect("GUC set");

    // Territory + account + site with invented names.
    let terr: Uuid = sqlx::query_scalar(
        "INSERT INTO territories (code, name, region, quota_year_cents)
         VALUES ('ZZ-FIX', 'Fixture Territory', 'west', 0) RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("territory");
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts (name, industry, status, territory_id)
         VALUES ('Quartz Harbor Industrial', 'fixture-industry', 'customer', $1) RETURNING id",
    )
    .bind(terr)
    .fetch_one(&mut *tx)
    .await
    .expect("account");
    let site: Uuid = sqlx::query_scalar(
        "INSERT INTO sites (account_id, address, city, state)
         VALUES ($1, '1 Fixture Way', 'Nowhere', 'ZZ') RETURNING id",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await
    .expect("site");

    // Catalog: one of-ours capital family, one competitor family with TWO
    // fitting SKUs (best fit = the pricier), one cartridge for our units.
    let p_cart: Uuid = sqlx::query_scalar(
        "INSERT INTO products (sku, name, family, kind, list_price_cents, filter_fits)
         VALUES ('FIX-CART-STD', 'Fixture Cartridge', 'Fixture-Filters', 'consumable', 10000, '{FIXFAM}') RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("cartridge");
    sqlx::query(
        "INSERT INTO products (sku, name, family, kind, list_price_cents, filter_fits) VALUES
         ('FIX-CART-HE', 'Fixture HE Replacement', 'Fixture-Filters', 'consumable', 20000, '{FIXCOMP-DF}'),
         ('FIX-CART-LO', 'Fixture Std Replacement', 'Fixture-Filters', 'consumable', 15000, '{FIXCOMP-DF}')",
    )
    .execute(&mut *tx)
    .await
    .expect("replacement SKUs");
    let p_ours: Uuid = sqlx::query_scalar(
        "INSERT INTO products (sku, name, family, kind, list_price_cents)
         VALUES ('FIX-CAP-1', 'Fixture Collector', 'FIXFAM', 'capital', 0) RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("our capital");
    let p_comp: Uuid = sqlx::query_scalar(
        "INSERT INTO products (sku, name, family, kind, list_price_cents)
         VALUES ('FIX-CAP-COMP', 'Fixture Competitor Collector', 'FIXCOMP-DF', 'capital', 0) RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("competitor capital");

    // Four units: reorder-window, defection-silent, competitor-conquest
    // (NULL cadence → the R4 fallback), telemetry-low.
    let u_reorder: Uuid = sqlx::query_scalar(
        "INSERT INTO installed_units
           (site_id, product_id, serial, commissioned_on, source, cartridge_count,
            cartridge_product_id, expected_changeout_months, last_filter_order_on)
         VALUES ($1, $2, 'FIX-SN-REORDER', '2020-01-01', 'ours', 10, $3, 6, CURRENT_DATE - 170)
         RETURNING id",
    )
    .bind(site)
    .bind(p_ours)
    .bind(p_cart)
    .fetch_one(&mut *tx)
    .await
    .expect("reorder unit");
    let u_defect: Uuid = sqlx::query_scalar(
        "INSERT INTO installed_units
           (site_id, product_id, serial, commissioned_on, source, cartridge_count,
            cartridge_product_id, expected_changeout_months, last_filter_order_on)
         VALUES ($1, $2, 'FIX-SN-DEFECT', '2020-01-01', 'ours', 10, $3, 6, CURRENT_DATE - 400)
         RETURNING id",
    )
    .bind(site)
    .bind(p_ours)
    .bind(p_cart)
    .fetch_one(&mut *tx)
    .await
    .expect("defect unit");
    let u_conq: Uuid = sqlx::query_scalar(
        "INSERT INTO installed_units
           (site_id, product_id, serial, commissioned_on, source, cartridge_count,
            cartridge_product_id, expected_changeout_months, last_filter_order_on)
         VALUES ($1, $2, 'FIX-SN-CONQ', '2020-01-01', 'Fixture-Brand', 10, NULL, NULL, NULL)
         RETURNING id",
    )
    .bind(site)
    .bind(p_comp)
    .fetch_one(&mut *tx)
    .await
    .expect("conquest unit");
    let u_telem: Uuid = sqlx::query_scalar(
        "INSERT INTO installed_units
           (site_id, product_id, serial, commissioned_on, source, cartridge_count,
            cartridge_product_id, expected_changeout_months, last_filter_order_on, filter_life_pct)
         VALUES ($1, $2, 'FIX-SN-TELEM', '2020-01-01', 'ours', 10, $3, NULL, NULL, 8.00)
         RETURNING id",
    )
    .bind(site)
    .bind(p_ours)
    .bind(p_cart)
    .fetch_one(&mut *tx)
    .await
    .expect("telemetry unit");

    // Anomaly feed: three recent lines in the fixture family — two at 5%,
    // one at 60%. median 5, stddev_pop ≈ 25.93 → only the 60% line clears
    // median + 2σ (≈ 56.9).
    let mut anomaly_line: Option<Uuid> = None;
    for disc in [5.0_f64, 5.0, 60.0] {
        let order: Uuid = sqlx::query_scalar(
            "INSERT INTO orders (account_id, site_id, territory_id, rep_id, ordered_on)
             VALUES ($1, $2, $3, $4, CURRENT_DATE - 5) RETURNING id",
        )
        .bind(account)
        .bind(site)
        .bind(terr)
        .bind(admin_id)
        .fetch_one(&mut *tx)
        .await
        .expect("order");
        let line: Uuid = sqlx::query_scalar(
            "INSERT INTO order_lines (order_id, product_id, qty, list_unit_cents, net_unit_cents, discount_pct)
             SELECT $1, $2, 10, p.list_price_cents,
                    round(p.list_price_cents * (100 - $3::numeric) / 100)::bigint, $3::numeric
             FROM products p WHERE p.id = $2 RETURNING id",
        )
        .bind(order)
        .bind(p_cart)
        .bind(disc)
        .fetch_one(&mut *tx)
        .await
        .expect("order line");
        if disc == 60.0 {
            anomaly_line = Some(line);
        }
    }
    let anomaly_line = anomaly_line.unwrap();

    // Generate — as the app role under the admin GUC, exactly the endpoint's
    // identity — and read back what the fixtures earned.
    sqlx::query("SELECT * FROM generate_signals()")
        .fetch_all(&mut *tx)
        .await
        .expect("generate inside fixture tx");

    async fn fetch_signal(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        key: &str,
    ) -> Option<sqlx::postgres::PgRow> {
        sqlx::query(
            "SELECT type::text AS t, score::float8 AS score, reasons::text AS reasons
             FROM signals WHERE dedupe_key = $1",
        )
        .bind(key)
        .fetch_optional(&mut **tx)
        .await
        .expect("fetch signal")
    }

    // reorder_due — cadence window: due = last + round(6 × 30.44) = +183d →
    // 13 days out, inside the 30-day lookahead, under the 1.5× boundary.
    let due_key: String = sqlx::query_scalar(
        "SELECT 'reorder_due:' || $1::uuid || ':' || ((CURRENT_DATE - 170) + round(6 * 30.44)::int)",
    )
    .bind(u_reorder)
    .fetch_one(&mut *tx)
    .await
    .expect("due key");
    let row = fetch_signal(&mut tx, &due_key)
        .await
        .expect("reorder card emerged");
    let reasons: Value = serde_json::from_str(row.get::<String, _>("reasons").as_str()).unwrap();
    assert_eq!(reasons[0]["label"], "last order");
    assert_eq!(reasons[1]["detail"], "every 6 months");
    assert!(row.get::<f64, _>("score") > 0.0);

    // defection_risk — 400 days > 6 × 1.5 × 30.44 ≈ 274.
    let def_key: String = sqlx::query_scalar(
        "SELECT 'defection_risk:' || $1::uuid || ':' || ((CURRENT_DATE - 400) + round(6 * 30.44)::int)",
    )
    .bind(u_defect)
    .fetch_one(&mut *tx)
    .await
    .expect("def key");
    let row = fetch_signal(&mut tx, &def_key)
        .await
        .expect("defection card emerged");
    let reasons: Value = serde_json::from_str(row.get::<String, _>("reasons").as_str()).unwrap();
    assert!(reasons[1]["detail"]
        .as_str()
        .unwrap()
        .contains("400 days silent"));
    assert_eq!(reasons[2]["detail"], "expected every 6 months");

    // conquest — best fit is the PRICIER replacement; NULL cadence fired the
    // 12-month fallback; score = round(round(10 × 20000 × 12 / 12) / 100, 2).
    let row = fetch_signal(&mut tx, &format!("conquest:{u_conq}"))
        .await
        .expect("conquest card emerged");
    assert_eq!(row.get::<f64, _>("score"), 2000.0);
    let reasons: Value = serde_json::from_str(row.get::<String, _>("reasons").as_str()).unwrap();
    assert!(reasons[0]["detail"]
        .as_str()
        .unwrap()
        .starts_with("Fixture-Brand FIXCOMP-DF"));
    assert!(reasons[1]["detail"]
        .as_str()
        .unwrap()
        .contains("FIX-CART-HE fits (2 compatible SKUs)"));
    assert!(reasons[2]["detail"]
        .as_str()
        .unwrap()
        .contains("assumes 12-month change-out"));

    // telemetry — one live card per unit, keyed :telemetry, receipts say so.
    let row = fetch_signal(&mut tx, &format!("reorder_due:{u_telem}:telemetry"))
        .await
        .expect("telemetry card emerged");
    let reasons: Value = serde_json::from_str(row.get::<String, _>("reasons").as_str()).unwrap();
    assert_eq!(reasons[2]["detail"], "filter life 8% — telemetry");

    // discount_anomaly — exactly the 60% line, and only it; excess-leakage
    // score = 100000 × (60 − 5) / 100 / 100 = 550.00.
    let row = fetch_signal(&mut tx, &format!("discount_anomaly:{anomaly_line}"))
        .await
        .expect("anomaly card emerged");
    assert_eq!(row.get::<f64, _>("score"), 550.0);
    let fixture_anomalies: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM signals WHERE type = 'discount_anomaly' AND account_id = $1",
    )
    .bind(account)
    .fetch_one(&mut *tx)
    .await
    .expect("count");
    assert_eq!(fixture_anomalies, 1, "the 5% lines are not anomalies");

    // Idempotency INSIDE the fixture world: a second run changes nothing —
    // and (P5) expires nothing, because every emitted key is still emitted.
    let second: Vec<(String, i64, i64, i64)> =
        sqlx::query_as("SELECT signal_type, inserted, updated, expired FROM generate_signals()")
            .fetch_all(&mut *tx)
            .await
            .expect("second generate");
    for (t, ins, upd, expd) in &second {
        assert_eq!(
            (*ins, *upd, *expd),
            (0, 0, 0),
            "second run must be all-zero for {t}"
        );
    }

    // Roll it all back — the fixture world never existed.
    tx.rollback().await.expect("rollback");
}

// ── R13 telemetry endpoint contract ─────────────────────────────────────────

#[tokio::test]
async fn telemetry_endpoint_is_admin_gated_and_typed() {
    let app = test_app().await;
    let pool = test_pool().await;
    let rep = login(&app, SE1_REP).await;
    let admin = login(&app, ADMIN).await;

    // Non-admin → 403 (the integration-feed identity is admin).
    let (s, _) = send(
        &app,
        "POST",
        "/api/telemetry/filter-life",
        Some(&rep),
        Some(json!({ "serial": "SN-GS3-00001", "filter_life_pct": 50 })),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Unknown serial → 404.
    let (s, _) = send(
        &app,
        "POST",
        "/api/telemetry/filter-life",
        Some(&admin),
        Some(json!({ "serial": "SN-NO-SUCH-UNIT", "filter_life_pct": 50 })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Out of range and non-numeric → 422.
    let (s, _) = send(
        &app,
        "POST",
        "/api/telemetry/filter-life",
        Some(&admin),
        Some(json!({ "serial": "SN-GS3-00001", "filter_life_pct": 150 })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, _) = send(
        &app,
        "POST",
        "/api/telemetry/filter-life",
        Some(&admin),
        Some(json!({ "serial": "SN-GS3-00001", "filter_life_pct": "not-a-number" })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // Happy path — 85% is ABOVE the 20% trigger, so no signal can fire from
    // this write even if a generation interleaves; then restore NULL (the
    // seeded state) so the suite leaves no telemetry trace.
    let serial: String =
        sqlx::query_scalar("SELECT serial FROM installed_units ORDER BY serial LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("a unit exists");
    let (s, body) = send(
        &app,
        "POST",
        "/api/telemetry/filter-life",
        Some(&admin),
        Some(json!({ "serial": serial, "filter_life_pct": 85.0 })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["serial"], json!(serial));
    assert_eq!(body["filter_life_pct"], json!(85.0));
    assert!(body["unit_id"].as_str().is_some());

    sqlx::query("UPDATE installed_units SET filter_life_pct = NULL WHERE serial = $1")
        .bind(&serial)
        .execute(&pool)
        .await
        .expect("restore");
}

// ── audit immutability unchanged (0011 REVOKE still holds post-0012) ────────

#[tokio::test]
async fn audit_log_remains_app_immutable() {
    let pool = test_pool().await;
    assert!(
        sqlx::query("SELECT count(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .is_ok(),
        "plenum_app may still read audit_log"
    );
    assert!(
        sqlx::query("UPDATE audit_log SET action = 'tampered'")
            .execute(&pool)
            .await
            .is_err(),
        "plenum_app must NOT be able to UPDATE audit_log"
    );
    assert!(
        sqlx::query("DELETE FROM audit_log")
            .execute(&pool)
            .await
            .is_err(),
        "plenum_app must NOT be able to DELETE audit_log"
    );
}
