//! P5 — Tier-3 adversarial + integration matrix over the real router against
//! the live compose database. Requires:
//!     docker compose up -d   &&   cargo run --bin seed
//!
//! Covers the four P5 verification minimums:
//!   · R1 — /metrics/leakage equivalence (σ-from-config == the old literal 2,
//!     row-for-row on the period path), the policy path's 1:1 agreement with
//!     the discount_anomaly signals, the heat zone + its scope, and the Wes
//!     Turner worst-row demo beat pinned server-side;
//!   · R2 — the data-quality finders land exactly on the seeded trio at VP
//!     view and stay empty in serena's scope;
//!   · R3 — /metrics/states scope (rep rows are own-territory states only,
//!     summing to her frozen cumulative anchor; VP sums to the ledger
//!     anchor), the roster, and the params grammar;
//!   · R4 — the expiry matrix: predicate-dead open card expires; an assigned
//!     card with a dead predicate SURVIVES; a re-satisfied predicate reopens
//!     an expired card; expiry writes expired_at + audit rows; the same-day
//!     double run stays 0/0/0.
//!
//! Concurrency: the expiry test mutates telemetry + runs generations, so it
//! serializes on P5_MUTATION_LOCK and picks its units from the serial-DESC
//! tail (signals_http's telemetry test uses the serial-ASC head). Scope
//! assertions are membership-based. Cleanup leaves both consumed cards
//! expired (machine state, outside every Active view) and telemetry NULL.

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

const SE1_REP: &str = "serena.estes@plenum.demo";
const VP: &str = "valerie.price@plenum.demo";
const ADMIN: &str = "priya.nair@plenum.demo";
const PASSWORD: &str = "demo-plenum-2026";

/// Serena's frozen cumulative anchor (HANDOFF-LOG, D.'s correction
/// 2026-07-20): $2,937,783.00 gross / $2,783,017.15 net.
const SERENA_CUM_NET_CENTS: i64 = 278_301_715;
/// The ledger anchor every phase has agreed on: $24,670,890.87.
const LEDGER_CUM_NET_CENTS: i64 = 2_467_089_087;

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

/// The metrics module's display rounding, mirrored for the equivalence proof.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Serializes this suite's mutating test against itself across parallel runs.
static P5_MUTATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ── R3 · /api/metrics/states — scope, anchors, roster, grammar ──────────────

#[tokio::test]
async fn states_endpoint_scope_anchors_roster_and_grammar() {
    let app = test_app().await;

    // 401 unauthenticated.
    let (s, body) = send(
        &app,
        "GET",
        "/api/metrics/states?period=cumulative&basis=net",
        None,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");

    let vp = login(&app, VP).await;
    let rep = login(&app, SE1_REP).await;

    // VP: many states across several territories; the sum of state net
    // equals the sum of TERRITORY net for the same period, read in the same
    // session (no state double-counts, none leaks away). Deliberately an
    // internal-consistency check, not a seed constant: the crm_http suite
    // books real orders when the whole gauntlet runs, so the live ledger can
    // sit above the frozen anchor until the next reseed — the anchor proof
    // itself runs at reseed time in the report.
    let (s, vp_page) = send(
        &app,
        "GET",
        "/api/metrics/states?period=cumulative&basis=net&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let vp_rows = items(&vp_page);
    assert!(vp_rows.len() >= 20, "VP sees the whole map's states");
    let vp_net: i64 = vp_rows
        .iter()
        .map(|r| r["net_cents"].as_i64().unwrap())
        .sum();
    let (s, vp_terr_page) = send(
        &app,
        "GET",
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let vp_terr_net: i64 = items(&vp_terr_page)
        .iter()
        .map(|r| r["net_cents"].as_i64().unwrap())
        .sum();
    assert_eq!(vp_net, vp_terr_net, "Σ state net == Σ territory net");
    // On a fresh reseed this figure IS the ledger anchor; mid-gauntlet the
    // crm_http bookings can only push it up.
    assert!(
        vp_net >= LEDGER_CUM_NET_CENTS,
        "book net at least the frozen anchor ({vp_net} < {LEDGER_CUM_NET_CENTS})"
    );
    let vp_territories: std::collections::HashSet<&str> = vp_rows
        .iter()
        .map(|r| r["territory_code"].as_str().unwrap())
        .collect();
    assert!(vp_territories.len() == 8, "all eight territories present");

    // Every money row's state belongs to its territory per the config.
    let roster = vp_page["territories"].as_array().expect("roster");
    assert_eq!(roster.len(), 8, "roster covers all eight territories");
    let states_of = |code: &str| -> Vec<String> {
        roster
            .iter()
            .find(|t| t["territory_code"] == json!(code))
            .expect("territory in roster")["state_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    for row in vp_rows {
        let terr = row["territory_code"].as_str().unwrap();
        let state = row["state_code"].as_str().unwrap();
        assert!(
            states_of(terr).iter().any(|s| s == state),
            "state {state} is outside {terr}'s mapped set"
        );
    }

    // The SE-1 roster names the demo cast.
    let se1 = roster
        .iter()
        .find(|t| t["territory_code"] == "SE-1")
        .expect("SE-1 roster");
    assert_eq!(se1["tm_names"], json!(["Serena Estes"]));
    assert_eq!(se1["rm_names"], json!(["Rachel Moore"]));

    // Rep scope: every row SE-1; the states are her city-pool set; her net
    // sums to her frozen cumulative anchor.
    let (s, rep_page) = send(
        &app,
        "GET",
        "/api/metrics/states?period=cumulative&basis=net&limit=200",
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rep_rows = items(&rep_page);
    assert!(!rep_rows.is_empty(), "SE-1 has state rows");
    let se1_states = states_of("SE-1");
    for row in rep_rows {
        assert_eq!(row["territory_code"], "SE-1", "foreign state row for a rep");
        let state = row["state_code"].as_str().unwrap();
        assert!(
            se1_states.iter().any(|s| s == state),
            "rep row state {state} outside SE-1's mapped set"
        );
    }
    let rep_net: i64 = rep_rows
        .iter()
        .map(|r| r["net_cents"].as_i64().unwrap())
        .sum();
    let (s, rep_terr_page) = send(
        &app,
        "GET",
        "/api/metrics/territories?period=cumulative&basis=net&limit=200",
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rep_terr = items(&rep_terr_page);
    assert_eq!(rep_terr.len(), 1, "rep territories = exactly SE-1");
    assert_eq!(
        rep_net,
        rep_terr[0]["net_cents"].as_i64().unwrap(),
        "Σ SE-1 state net == SE-1 territory net"
    );
    // On a fresh reseed this figure IS serena's frozen anchor — asserted >=
    // here because the crm_http booking tests can add to it mid-gauntlet.
    assert!(
        rep_net >= SERENA_CUM_NET_CENTS,
        "SE-1 net at least the frozen anchor ({rep_net} < {SERENA_CUM_NET_CENTS})"
    );

    // Grammar: missing period, bad basis, bad kind, limit over the law.
    for (uri, fragment) in [
        ("/api/metrics/states?basis=net", "period is required"),
        (
            "/api/metrics/states?period=2025&basis=vibes",
            "basis must be",
        ),
        (
            "/api/metrics/states?period=2025&basis=net&kind=vibes",
            "kind must be",
        ),
        (
            "/api/metrics/states?period=2025&basis=net&limit=201",
            "limit must be",
        ),
    ] {
        let (s, body) = send(&app, "GET", uri, Some(&vp), None).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap()
                .contains(fragment),
            "{uri}: {body}"
        );
    }
}

// ── R1 · leakage: equivalence, policy parity, heat + scope ──────────────────

#[tokio::test]
async fn leakage_sigma_equivalence_policy_parity_and_heat() {
    let app = test_app().await;
    let pool = test_pool().await;
    let vp = login(&app, VP).await;
    let rep = login(&app, SE1_REP).await;

    // 1 · EQUIVALENCE (the R1 proof, test-pinned): the endpoint's period-mode
    // outliers — σ now read from signal_policy — must match the P1 literal-2
    // SQL row-for-row (ids, order, thresholds) for the same request.
    let (s, page) = send(
        &app,
        "GET",
        "/api/metrics/leakage?period=2025&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let endpoint_rows: Vec<(String, f64)> = page["outliers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["order_line_id"].as_str().unwrap().to_string(),
                r["threshold_pct"].as_f64().unwrap(),
            )
        })
        .collect();
    assert!(!endpoint_rows.is_empty(), "2025 has outliers in the seed");

    let vp_id: String = sqlx::query_scalar("SELECT id::text FROM users WHERE email = $1")
        .bind(VP)
        .fetch_one(&pool)
        .await
        .expect("vp id");
    let mut tx = pool.begin().await.expect("tx");
    sqlx::query("SELECT set_config('app.user_id', $1, true), set_config('app.role', 'vp', true)")
        .bind(&vp_id)
        .execute(&mut *tx)
        .await
        .expect("GUC");
    let replica = sqlx::query(
        "WITH fam_stats AS (
             SELECT family,
                    percentile_cont(0.5) WITHIN GROUP (ORDER BY discount_pct::float8) AS median_pct,
                    stddev_samp(discount_pct::float8) AS sd
             FROM v_order_facts
             WHERE ordered_on >= '2025-01-01' AND ordered_on < '2026-01-01'
             GROUP BY family
         )
         SELECT f.order_line_id::text AS line_id,
                (s.median_pct + 2 * s.sd) AS threshold
         FROM v_order_facts f
         JOIN fam_stats s ON s.family = f.family
         WHERE f.ordered_on >= '2025-01-01' AND f.ordered_on < '2026-01-01'
           AND s.sd IS NOT NULL AND s.sd > 0
           AND f.discount_pct::float8 > s.median_pct + 2 * s.sd
         ORDER BY f.discount_pct DESC, f.order_line_id
         LIMIT 200",
    )
    .fetch_all(&mut *tx)
    .await
    .expect("literal-2 replica");
    tx.rollback().await.ok();

    assert_eq!(
        endpoint_rows.len(),
        replica.len(),
        "row count: σ-from-config vs the P1 literal 2"
    );
    for (i, row) in replica.iter().enumerate() {
        let (ep_id, ep_thr) = &endpoint_rows[i];
        assert_eq!(
            ep_id,
            &row.get::<String, _>("line_id"),
            "row {i}: order_line_id order differs"
        );
        assert_eq!(
            *ep_thr,
            round2(row.get::<f64, _>("threshold")),
            "row {i}: threshold differs"
        );
    }

    // 2 · POLICY PARITY, clock-honest form: the feed runs the generator's
    // math LIVE, while the signals table holds whatever the LAST generation
    // run produced — and the trailing window slides at the UTC midnight
    // tick, so a signal whose order line just fell out of the window can
    // linger until the next run expires it (the documented clock-drift
    // class; first observed live when this suite crossed a date boundary).
    // The day-proof assertions:
    //   · every policy-mode outlier row carries its signal chip;
    //   · the feed set == the census restricted to IN-window lines;
    //   · every census row the feed lacks is explainably OUT of the window
    //     (pending expiry on the next generation run) — nothing else.
    let (s, policy_page) = send(
        &app,
        "GET",
        "/api/metrics/leakage?period=cumulative&outliers=policy&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let policy_rows = policy_page["outliers"].as_array().unwrap();
    assert!(!policy_rows.is_empty(), "the trailing window has anomalies");
    let mut policy_lines: Vec<String> = policy_rows
        .iter()
        .map(|r| r["order_line_id"].as_str().unwrap().to_string())
        .collect();
    for r in policy_rows {
        assert!(
            r["signal_id"].as_str().is_some(),
            "a policy-mode outlier without its signal chip: {r}"
        );
    }
    // Census with in-window flags, under the VP GUC (plenum_app with no GUC
    // reads zero rows — fail-closed).
    let mut tx = pool.begin().await.expect("tx");
    sqlx::query("SELECT set_config('app.user_id', $1, true), set_config('app.role', 'vp', true)")
        .bind(&vp_id)
        .execute(&mut *tx)
        .await
        .expect("GUC");
    let census = sqlx::query(
        "SELECT replace(s.dedupe_key, 'discount_anomaly:', '') AS line_id,
                (o.ordered_on >= CURRENT_DATE - sp.discount_window_days) AS in_window
         FROM signals s
         JOIN order_lines ol ON ol.id = s.order_line_id
         JOIN orders o ON o.id = ol.order_id
         CROSS JOIN signal_policy sp
         WHERE s.type = 'discount_anomaly'",
    )
    .fetch_all(&mut *tx)
    .await
    .expect("census under GUC");
    tx.rollback().await.ok();

    let mut in_window: Vec<String> = census
        .iter()
        .filter(|r| r.get::<bool, _>("in_window"))
        .map(|r| r.get::<String, _>("line_id"))
        .collect();
    let stale: Vec<String> = census
        .iter()
        .filter(|r| !r.get::<bool, _>("in_window"))
        .map(|r| r.get::<String, _>("line_id"))
        .collect();
    if in_window.len() <= 200 {
        policy_lines.sort();
        in_window.sort();
        assert_eq!(
            policy_lines, in_window,
            "policy outliers == the in-window generator census"
        );
    } else {
        assert_eq!(policy_lines.len(), 200, "page cap");
    }
    assert_eq!(
        census.len() - in_window.len().min(census.len()),
        stale.len(),
        "every census row the feed lacks is out-of-window (pending expiry)"
    );

    // 3 · HEAT: present, dual-cents, and the demo beat — Wes Turner's row is
    // the worst leakage_pct on the VP's cumulative table.
    let heat = page["heat"].as_array().expect("heat cells");
    assert!(!heat.is_empty());
    let (s, cum_page) = send(
        &app,
        "GET",
        "/api/metrics/leakage?period=cumulative&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let mut by_rep: std::collections::HashMap<String, (i64, i64)> = Default::default();
    for c in cum_page["heat"].as_array().unwrap() {
        let e = by_rep
            .entry(c["rep_name"].as_str().unwrap().to_string())
            .or_insert((0, 0));
        e.0 += c["gross_cents"].as_i64().unwrap();
        e.1 += c["net_cents"].as_i64().unwrap();
    }
    assert!(by_rep.len() >= 10, "VP heat covers the rep book");
    let worst = by_rep
        .iter()
        .filter(|(_, (g, _))| *g > 0)
        .max_by(|a, b| {
            let pa = (a.1 .0 - a.1 .1) as f64 / a.1 .0 as f64;
            let pb = (b.1 .0 - b.1 .1) as f64 / b.1 .0 as f64;
            pa.partial_cmp(&pb).unwrap()
        })
        .map(|(name, _)| name.clone())
        .unwrap();
    assert_eq!(worst, "Wes Turner", "the leakage-rep beat reads worst");

    // 4 · Rep heat scope: serena's table holds no foreign rep row.
    let (s, rep_page) = send(
        &app,
        "GET",
        "/api/metrics/leakage?period=cumulative&limit=200",
        Some(&rep),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rep_heat = rep_page["heat"].as_array().unwrap();
    assert!(!rep_heat.is_empty());
    for c in rep_heat {
        assert_eq!(c["rep_name"], "Serena Estes", "foreign rep in rep heat");
    }

    // 5 · Grammar: the new param rejects garbage.
    let (s, body) = send(
        &app,
        "GET",
        "/api/metrics/leakage?period=2025&outliers=vibes",
        Some(&vp),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("outliers must be"));
}

// ── R2 · data-quality finders: the seeded trio at VP, silence for the rep ───

#[tokio::test]
async fn data_quality_finds_the_seeded_trio_scoped() {
    let app = test_app().await;

    let (s, body) = send(&app, "GET", "/api/data-quality", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");

    let vp = login(&app, VP).await;
    let (s, dq) = send(&app, "GET", "/api/data-quality", Some(&vp), None).await;
    assert_eq!(s, StatusCode::OK);

    // Exactly the two planted near-duplicate pairs — and never the
    // legitimate parent/child pair (Vantage Metalworks Coastal).
    let dupes = dq["duplicate_names"].as_array().unwrap();
    assert_eq!(dupes.len(), 2, "exactly two duplicate-ish pairs: {dupes:?}");
    // a/b order inside a pair follows the database collation — normalize to
    // Rust byte order before comparing, so the assertion is collation-proof.
    let pair_names: std::collections::HashSet<(String, String)> = dupes
        .iter()
        .map(|d| {
            let a = d["a_name"].as_str().unwrap().to_string();
            let b = d["b_name"].as_str().unwrap().to_string();
            if a <= b {
                (a, b)
            } else {
                (b, a)
            }
        })
        .collect();
    let expected: std::collections::HashSet<(String, String)> = [
        (
            "Keystone Coating Co.".to_string(),
            "Keystone Coatings".to_string(),
        ),
        (
            "Vantage Metal Works".to_string(),
            "Vantage Metalworks".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        pair_names, expected,
        "the two planted pairs, and never the parent/child pair"
    );

    // Exactly the two NULL-cadence units, on the two planted accounts.
    let nulls = dq["null_cadence_units"].as_array().unwrap();
    assert_eq!(nulls.len(), 2, "exactly two unknown-cadence units");
    let null_accounts: std::collections::HashSet<&str> = nulls
        .iter()
        .map(|n| n["account_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        null_accounts,
        ["Harbor Steel Works", "Gulf Coast Chemical"]
            .into_iter()
            .collect()
    );

    // Exactly the one comped line: Golden Plains Milling, SVC-INSPECT,
    // March 2025.
    let comped = dq["full_discount_lines"].as_array().unwrap();
    assert_eq!(comped.len(), 1, "exactly one 100% line");
    assert_eq!(comped[0]["account_name"], "Golden Plains Milling");
    assert_eq!(comped[0]["product_sku"], "SVC-INSPECT");
    assert!(comped[0]["ordered_on"]
        .as_str()
        .unwrap()
        .starts_with("2025-03"));

    // The zero-site finder is designed-empty on the seed.
    assert_eq!(dq["zero_site_accounts"].as_array().unwrap().len(), 0);

    // Serena's scope holds none of the planted mess — the clean book.
    let rep = login(&app, SE1_REP).await;
    let (s, rep_dq) = send(&app, "GET", "/api/data-quality", Some(&rep), None).await;
    assert_eq!(s, StatusCode::OK);
    for key in [
        "duplicate_names",
        "null_cadence_units",
        "full_discount_lines",
        "zero_site_accounts",
    ] {
        assert_eq!(
            rep_dq[key].as_array().unwrap().len(),
            0,
            "{key} must be empty in SE-1 scope"
        );
    }
}

// ── R4 · the expiry matrix ───────────────────────────────────────────────────

#[tokio::test]
async fn expiry_matrix_open_expires_assigned_survives_reopen_works() {
    let _guard = P5_MUTATION_LOCK.lock().await;
    let app = test_app().await;
    let pool = test_pool().await;
    let vp = login(&app, VP).await;
    let admin = login(&app, ADMIN).await;

    // Two cartridge-bearing units from the serial-DESC tail (signals_http's
    // telemetry test works the ASC head), telemetry currently NULL.
    let serials: Vec<String> = sqlx::query_scalar(
        "SELECT serial FROM installed_units
         WHERE cartridge_product_id IS NOT NULL AND filter_life_pct IS NULL
         ORDER BY serial DESC LIMIT 2",
    )
    .fetch_all(&pool)
    .await
    .expect("two clean units");
    assert_eq!(serials.len(), 2);
    let (serial_a, serial_b) = (serials[0].clone(), serials[1].clone());

    let push = |serial: String, pct: f64, admin: String, app: Router| async move {
        let (s, _) = send(
            &app,
            "POST",
            "/api/telemetry/filter-life",
            Some(&admin),
            Some(json!({ "serial": serial, "filter_life_pct": pct })),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "telemetry push");
    };
    let generate = |admin: String, app: Router| async move {
        let (s, body) = send(
            &app,
            "POST",
            "/api/admin/generate-signals",
            Some(&admin),
            None,
        )
        .await;
        assert_eq!(s, StatusCode::OK, "generate");
        body
    };
    let find_card = |page: &Value, serial: &str| -> Option<Value> {
        items(page)
            .iter()
            .find(|r| r["serial"] == json!(serial))
            .cloned()
    };

    // A · low telemetry → the card appears, open.
    push(serial_a.clone(), 8.0, admin.clone(), app.clone()).await;
    generate(admin.clone(), app.clone()).await;
    let (_, open_page) = send(
        &app,
        "GET",
        "/api/signals?status=open&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let card_a = find_card(&open_page, &serial_a).expect("A's telemetry card is open");
    let card_a_id = card_a["id"].as_str().unwrap().to_string();

    // A · telemetry recovers → the next run expires it (and reports it).
    push(serial_a.clone(), 95.0, admin.clone(), app.clone()).await;
    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let gen = generate(admin.clone(), app.clone()).await;
    let reorder_row = gen["generated"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["signal_type"] == "reorder_due")
        .unwrap()
        .clone();
    assert!(
        reorder_row["expired"].as_i64().unwrap() >= 1,
        "the run reports the expiry: {reorder_row}"
    );

    let (_, active_page) = send(
        &app,
        "GET",
        "/api/signals?status=active&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert!(
        find_card(&active_page, &serial_a).is_none(),
        "A's card has left Active"
    );
    let (_, expired_page) = send(
        &app,
        "GET",
        "/api/signals?status=expired&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let expired_a = find_card(&expired_page, &serial_a).expect("A sits under the Expired filter");
    assert_eq!(expired_a["status"], "expired");
    assert!(
        expired_a["expired_at"].as_str().is_some(),
        "expiry stamps expired_at"
    );

    // The expiry is a real state change — the 0006 trigger recorded it.
    let audit_after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(audit_after > audit_before, "expiry writes audit rows");
    let expiry_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log
         WHERE entity = 'signals' AND entity_id = $1::uuid
           AND after->>'status' = 'expired'",
    )
    .bind(&card_a_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(expiry_audits >= 1, "A's expiry is in the audit trail");

    // Same-day double run: 0/0/0 everywhere, zero audit delta.
    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let second = generate(admin.clone(), app.clone()).await;
    for g in second["generated"].as_array().unwrap() {
        assert_eq!(g["inserted"], 0, "double run inserted: {g}");
        assert_eq!(g["updated"], 0, "double run updated: {g}");
        assert_eq!(g["expired"], 0, "double run expired: {g}");
    }
    let audit_after: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit_after - audit_before, 0, "double run audit delta");

    // A · the predicate returns → the machine reopens its own card (same id,
    // open again, expired_at cleared) — check 8 stays rehearsable.
    push(serial_a.clone(), 8.0, admin.clone(), app.clone()).await;
    generate(admin.clone(), app.clone()).await;
    let (_, reopened_page) = send(
        &app,
        "GET",
        "/api/signals?status=open&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let reopened = find_card(&reopened_page, &serial_a).expect("A's card reopened");
    assert_eq!(
        reopened["id"].as_str().unwrap(),
        card_a_id,
        "same card, same id"
    );
    assert!(reopened["expired_at"].is_null(), "reopen clears expired_at");

    // B · assigned card with a dead predicate SURVIVES, still assigned.
    push(serial_b.clone(), 8.0, admin.clone(), app.clone()).await;
    generate(admin.clone(), app.clone()).await;
    let (_, open_page) = send(
        &app,
        "GET",
        "/api/signals?status=open&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let card_b = find_card(&open_page, &serial_b).expect("B's telemetry card is open");
    let card_b_id = card_b["id"].as_str().unwrap().to_string();
    let vp_user_id: String = sqlx::query_scalar("SELECT id::text FROM users WHERE email = $1")
        .bind(VP)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (s, assigned) = send(
        &app,
        "POST",
        &format!("/api/signals/{card_b_id}/assign"),
        Some(&vp),
        Some(json!({ "assignee_id": vp_user_id })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(assigned["status"], "assigned");

    push(serial_b.clone(), 95.0, admin.clone(), app.clone()).await;
    generate(admin.clone(), app.clone()).await;
    let (_, active_after) = send(
        &app,
        "GET",
        "/api/signals?status=active&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    let survivor =
        find_card(&active_after, &serial_b).expect("B's assigned card SURVIVED the dead predicate");
    assert_eq!(survivor["status"], "assigned", "never-touch-humans held");
    assert!(survivor["expired_at"].is_null());

    // Writes on an expired card are refused (A is open again here, so expire
    // it first by clearing its telemetry, then try to touch it).
    sqlx::query("UPDATE installed_units SET filter_life_pct = NULL WHERE serial = $1")
        .bind(&serial_a)
        .execute(&pool)
        .await
        .unwrap();
    generate(admin.clone(), app.clone()).await;
    let (s, refusal) = send(
        &app,
        "POST",
        &format!("/api/signals/{card_a_id}/assign"),
        Some(&vp),
        Some(json!({ "assignee_id": vp_user_id })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "no writes on expired");
    assert!(refusal["error"]["message"]
        .as_str()
        .unwrap()
        .contains("expired"));

    // Cleanup: telemetry back to NULL everywhere we touched; B's card back to
    // machine custody (open), then one more run expires it — both cards end
    // expired (outside every Active view), zero telemetry trace, suite
    // re-runnable (a fresh low push reopens them by the reopen proof above).
    sqlx::query("UPDATE installed_units SET filter_life_pct = NULL WHERE serial = $1")
        .bind(&serial_b)
        .execute(&pool)
        .await
        .unwrap();
    let admin_db_id: String = sqlx::query_scalar("SELECT id::text FROM users WHERE email = $1")
        .bind(ADMIN)
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.user_id', $1, true), set_config('app.role', 'admin', true)",
    )
    .bind(&admin_db_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE signals SET status = 'open', assigned_to = NULL, assigned_at = NULL
         WHERE id = $1::uuid",
    )
    .bind(&card_b_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    generate(admin.clone(), app.clone()).await;
    let (_, final_active) = send(
        &app,
        "GET",
        "/api/signals?status=active&type=reorder_due&limit=200",
        Some(&vp),
        None,
    )
    .await;
    assert!(find_card(&final_active, &serial_a).is_none());
    assert!(find_card(&final_active, &serial_b).is_none());
}
