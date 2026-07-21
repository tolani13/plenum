//! The P4 AI layer (spec §6, rulings R8–R10) — behind ONE seam.
//!
//!   GET  /api/ai/status                   — { ask, discount } for UI gating
//!   POST /api/ai/ask                      — NL question → SQL → RLS-scoped rows
//!   POST /api/ai/discount-recommendation  — comparables + optional narrative
//!
//! client.rs owns the only vendor call; validate.rs is the pure AST validator
//! with its own adversarial tests. Everything model-generated is (1) parsed
//! and whitelisted, (2) executed ONLY inside the caller's READ-ONLY rls
//! transaction with a 5s statement timeout, (3) capped by an injected
//! LIMIT — and the validated SQL itself is always returned to the caller
//! (the receipts contract). A rep cannot ask their way into another
//! territory: scope is Postgres RLS on the same GUC path every other read
//! uses, not anything the model or this module decides.

pub mod client;
pub mod validate;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Column, Executor, PgPool, Row};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::{rls_readonly_tx, rls_tx};
use crate::state::AppState;
use validate::validate_ask_sql;

// ── GET /api/ai/status ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AiStatusBody {
    ask: bool,
    discount: bool,
}

/// Authed, so the UI can gate affordances without probing errors (R8).
pub async fn ai_status(
    State(state): State<AppState>,
    _user: SessionUser,
) -> Result<Json<AiStatusBody>, ApiError> {
    Ok(Json(AiStatusBody {
        ask: state.ai.ask_enabled(),
        discount: state.ai.discount_enabled(),
    }))
}

// ── POST /api/ai/ask (R9) ───────────────────────────────────────────────────

/// The injected row cap — run as SELECT * FROM (…) plenum_ask LIMIT 500.
const ASK_ROW_CAP: usize = 500;
const ASK_QUESTION_MAX_CHARS: usize = 2000;

/// The system prompt: the whitelisted view schemas (0008/0010 column lists),
/// the §5 metric dictionary digest, and the hard rules. The model sees the
/// SEMANTIC LAYER only — no base tables, no row-level anything (scope is
/// enforced by RLS at execution, not by prompt).
const ASK_SYSTEM_PROMPT: &str = "\
You translate a sales-analytics question into ONE PostgreSQL SELECT statement \
over a fixed semantic layer. Reply with ONLY the SQL — no prose, no markdown \
fences, no explanation.

RELATIONS (the only ones you may reference):
- v_order_facts(order_line_id uuid, order_id uuid, account_id uuid, \
account_name text, site_id uuid, territory_id uuid, territory_code text, \
territory_name text, quota_year_cents bigint, rep_id uuid, rep_name text, \
product_id uuid, product_sku text, product_name text, family text, kind text, \
qty int, list_unit_cents bigint, net_unit_cents bigint, discount_pct numeric, \
gross_cents bigint, net_cents bigint, discount_cents bigint, ordered_on date, \
quarter_start date, year int) — one row per order line, all history.
- v_territory_period(territory_id uuid, territory_code text, territory_name \
text, quota_year_cents bigint, quarter_start date, gross_cents bigint, \
net_cents bigint, discount_cents bigint, order_count bigint) — quarterly \
rollup per territory, including the live current quarter.
- v_rep_period(rep_id uuid, rep_name text, territory_id uuid, quarter_start \
date, gross_cents bigint, net_cents bigint, discount_cents bigint, \
order_count bigint, capital_gross_cents bigint, capital_net_cents bigint, \
consumable_gross_cents bigint, consumable_net_cents bigint) — quarterly \
rollup per rep per territory.
- v_product_period(product_id uuid, product_sku text, product_name text, \
family text, kind text, territory_id uuid, quarter_start date, units bigint, \
gross_cents bigint, net_cents bigint, discount_cents bigint, order_count \
bigint) — quarterly rollup per product per territory.
- v_customer_period(account_id uuid, account_name text, territory_id uuid, \
quarter_start date, gross_cents bigint, net_cents bigint, discount_cents \
bigint, order_count bigint, capital_gross_cents bigint, capital_net_cents \
bigint, consumable_gross_cents bigint, consumable_net_cents bigint) — \
quarterly rollup per account per territory.
- v_defection_risk(unit_id uuid, serial text, site_id uuid, site_label text, \
account_id uuid, account_name text, territory_id uuid, territory_code text, \
days_silent int, expected_changeout_months int, \
annual_consumable_value_cents bigint, score numeric) — installed units past \
1.5x their reorder cadence.

METRIC DICTIONARY:
- Money columns are integer CENTS. Dollars = cents / 100.0. gross = list-price \
revenue; net = revenue after discount; discount leakage = gross - net; \
leakage pct = leakage / gross.
- Periods are calendar. Quarters key on quarter_start (Q1 = Jan 1). Years: \
the year column on v_order_facts, or EXTRACT(year FROM quarter_start) on the \
rollups. cumulative = all history. TTM = trailing 12 months by ordered_on.
- kind: capital | consumable | part | service. Capital lumps and consumable \
annuities are different businesses; keep them separate when asked.
- A ranking = aggregate, ORDER BY the chosen basis DESC, LIMIT.

RULES:
- Exactly ONE statement, SELECT only. Only the relations above; CTEs you \
define yourself are fine.
- Prefer the *_period rollups for whole-quarter/year questions; use \
v_order_facts for cumulative, TTM, or line-level questions.
- Alias aggregates with readable snake_case names. Include a sensible \
ORDER BY, and a LIMIT when ranking.
- PostgreSQL dialect.";

#[derive(Deserialize)]
pub struct AskRequest {
    question: Option<String>,
}

#[derive(Serialize)]
pub struct AskBody {
    /// The VALIDATED sql that actually ran — receipts, always present.
    sql: String,
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
    row_count: i64,
    truncated: bool,
}

/// Strip a markdown fence if the model added one despite the instructions.
fn extract_sql(raw: &str) -> String {
    let t = raw.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let rest = rest
            .trim_start_matches("sql")
            .trim_start_matches("SQL")
            .trim_start();
        return rest.split("```").next().unwrap_or(rest).trim().to_string();
    }
    t.to_string()
}

/// A database failure on the generated-SQL path is the CALLER's 422 (their
/// question produced it), with the statement-timeout case named plainly.
/// Nothing here is a 500.
fn ask_db_error(e: sqlx::Error) -> ApiError {
    match &e {
        sqlx::Error::Database(db) => {
            if db.code().as_deref() == Some("57014") {
                ApiError::Invalid("the query timed out (5s limit)".into())
            } else {
                ApiError::Invalid(format!("the query failed: {}", db.message()))
            }
        }
        _ => ApiError::from(e),
    }
}

#[derive(Debug)]
pub struct AskRun {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub truncated: bool,
}

/// Execute ALREADY-VALIDATED sql under the caller's READ-ONLY rls
/// transaction (5s statement timeout, GUC identity — rls.rs), wrapped in the
/// injected row cap. Public so the Tier-3 suite can drive the execution path
/// without a vendor key.
///
/// This is the ONE place the plain runtime `sqlx::query` API is used instead
/// of the compile-checked macros: the SQL is model-generated and only known
/// at runtime, so there is nothing for the macro to check at build time —
/// the AST validator + READ ONLY + RLS + timeout are its guardrails instead.
pub async fn run_ask_query(
    pool: &PgPool,
    user: &SessionUser,
    validated_sql: &str,
) -> Result<AskRun, ApiError> {
    let mut tx = rls_readonly_tx(pool, user).await?;

    // Ordered column names come from a server-side prepare (describe) of the
    // validated statement — no execution, works even for zero-row results.
    let described = (&mut *tx)
        .describe(validated_sql)
        .await
        .map_err(ask_db_error)?;
    let columns: Vec<String> = described
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    // row_to_json lets Postgres serialize every column type (enums, dates,
    // numerics) — the JSON objects are then re-ordered into the described
    // column order for the table payload.
    let wrapped = format!(
        "SELECT row_to_json(plenum_ask) AS r FROM ( {validated_sql} ) plenum_ask LIMIT {ASK_ROW_CAP}"
    );
    let raw_rows = sqlx::query(&wrapped)
        .fetch_all(&mut *tx)
        .await
        .map_err(ask_db_error)?;
    tx.commit().await?;

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(raw_rows.len());
    for row in &raw_rows {
        let obj: Value = row.try_get("r")?;
        let map = obj.as_object().cloned().unwrap_or_default();
        rows.push(
            columns
                .iter()
                .map(|c| map.get(c).cloned().unwrap_or(Value::Null))
                .collect(),
        );
    }

    let truncated = rows.len() == ASK_ROW_CAP;
    Ok(AskRun {
        columns,
        rows,
        truncated,
    })
}

pub async fn ask(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<AskRequest>, JsonRejection>,
) -> Result<Json<AskBody>, ApiError> {
    if !state.ai.ask_enabled() {
        return Err(ApiError::AiUnavailable(
            "Ask PLENUM is off (no API key or the flag is disabled)",
        ));
    }

    let question = body
        .ok()
        .and_then(|Json(b)| b.question)
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .ok_or_else(|| ApiError::Invalid("a question is required".into()))?;
    if question.chars().count() > ASK_QUESTION_MAX_CHARS {
        return Err(ApiError::Invalid(format!(
            "the question is too long (max {ASK_QUESTION_MAX_CHARS} characters)"
        )));
    }

    let raw = client::complete(&state.ai, ASK_SYSTEM_PROMPT, &question).await?;
    let candidate = extract_sql(&raw);
    let validated = validate_ask_sql(&candidate)
        .map_err(|m| ApiError::Invalid(format!("the generated SQL failed validation: {m}")))?;

    let run = run_ask_query(&state.pool, &user, &validated).await?;
    let row_count = run.rows.len() as i64;
    Ok(Json(AskBody {
        sql: validated,
        columns: run.columns,
        rows: run.rows,
        row_count,
        truncated: run.truncated,
    }))
}

// ── POST /api/ai/discount-recommendation (R10) ──────────────────────────────

#[derive(Deserialize)]
pub struct DiscountRecRequest {
    product_id: Uuid,
    account_id: Uuid,
    qty: i32,
    discount_pct: f64,
}

#[derive(Serialize)]
pub struct CompSample {
    account_name: String,
    ordered_on: NaiveDate,
    product_sku: String,
    qty: i32,
    gross_cents: i64,
    discount_pct: f64,
}

#[derive(Serialize)]
pub struct Comparables {
    count: i64,
    family: String,
    industry: String,
    /// The deterministic size band, stated in the receipts: the order-of-
    /// magnitude (log10 bucket) of the LINE GROSS in cents.
    band_label: String,
    median_pct: Option<f64>,
    p25: Option<f64>,
    p75: Option<f64>,
    sample: Vec<CompSample>,
}

#[derive(Serialize)]
pub struct DiscountRecBody {
    comparables: Comparables,
    narrative: Option<String>,
    degraded: bool,
}

const REC_SYSTEM_PROMPT: &str = "\
You are a discount-governance assistant for an industrial filtration sales \
team. Given comparable historical order lines (same product family, customer \
industry, and order-size band) and a proposed discount, reply with ONE short \
sentence (under 40 words) saying whether the proposed discount is in line \
with the comparables, citing the median and the interquartile range. No \
preamble, no markdown.";

fn cents_to_dollars_label(cents: i64) -> String {
    format!("${}.{:02}", cents / 100, (cents % 100).abs())
}

pub async fn discount_recommendation(
    State(state): State<AppState>,
    user: SessionUser,
    body: Result<Json<DiscountRecRequest>, JsonRejection>,
) -> Result<Json<DiscountRecBody>, ApiError> {
    // Flag alone gates the ENDPOINT; the key gates only the narrative (R10).
    if !state.ai.discount_enabled() {
        return Err(ApiError::AiUnavailable(
            "the discount recommender is disabled",
        ));
    }

    let Json(req) = body.map_err(|e| ApiError::Invalid(format!("invalid request body: {e}")))?;
    if req.qty <= 0 {
        return Err(ApiError::Invalid("qty must be > 0".into()));
    }
    if !(0.0..=100.0).contains(&req.discount_pct) {
        return Err(ApiError::Invalid(
            "discount_pct must be between 0 and 100".into(),
        ));
    }

    let mut tx = rls_tx(&state.pool, &user).await?;

    // The account must be visible (RLS 404) — its industry keys the cohort.
    let account = sqlx::query!(
        r#"SELECT industry FROM accounts WHERE id = $1"#,
        req.account_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    // The product keys the family and the size band (catalog, not RLS).
    let product = sqlx::query!(
        r#"SELECT family, list_price_cents FROM products WHERE id = $1"#,
        req.product_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::Invalid("unknown product".into()))?;

    // The deterministic band: order of magnitude of the line gross in cents,
    // computed exactly (digit count, no float log at the boundaries).
    let request_gross = product.list_price_cents * i64::from(req.qty);
    let band = i32::try_from(request_gross.max(1).to_string().len() - 1).unwrap_or(0);
    let band_low = 10_i64.pow(band as u32);
    let band_high = 10_i64.pow(band as u32 + 1) - 1;
    let band_label = format!(
        "{}–{} line gross",
        cents_to_dollars_label(band_low),
        cents_to_dollars_label(band_high)
    );

    // Comparables under the CALLER'S RLS: a rep's cohort comes from their own
    // scope only — disclosed behavior, not a bug (R10). "Won" lines = order
    // history (an order line IS a won line in this schema).
    let stats = sqlx::query!(
        r#"SELECT count(*) AS "n!",
                  percentile_cont(0.5) WITHIN GROUP (ORDER BY f.discount_pct::float8)
                      AS "median?",
                  percentile_cont(0.25) WITHIN GROUP (ORDER BY f.discount_pct::float8)
                      AS "p25?",
                  percentile_cont(0.75) WITHIN GROUP (ORDER BY f.discount_pct::float8)
                      AS "p75?"
           FROM v_order_facts f
           JOIN accounts a2 ON a2.id = f.account_id
           WHERE f.family = $1
             AND a2.industry = $2
             AND floor(log(GREATEST(f.gross_cents, 1)::numeric))::int = $3"#,
        product.family,
        account.industry,
        band
    )
    .fetch_one(&mut *tx)
    .await?;

    let sample = sqlx::query_as!(
        CompSample,
        r#"SELECT f.account_name AS "account_name!", f.ordered_on AS "ordered_on!",
                  f.product_sku AS "product_sku!", f.qty AS "qty!",
                  f.gross_cents AS "gross_cents!",
                  f.discount_pct::float8 AS "discount_pct!"
           FROM v_order_facts f
           JOIN accounts a2 ON a2.id = f.account_id
           WHERE f.family = $1
             AND a2.industry = $2
             AND floor(log(GREATEST(f.gross_cents, 1)::numeric))::int = $3
           ORDER BY f.ordered_on DESC, f.order_line_id
           LIMIT 10"#,
        product.family,
        account.industry,
        band
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    // Narrative from the R8 seam when a key is present; without one (or on a
    // vendor failure) the raw comparables stand alone — the spec's exact
    // degradation, never an error.
    let narrative = if state.ai.api_key.is_some() {
        let digest = format!(
            "Proposed discount: {:.2}% on {} × qty {} (line gross {}).\n\
             Comparables (family {}, industry {}, band {}): {} lines, \
             median {}%, p25 {}%, p75 {}%.",
            req.discount_pct,
            product.family,
            req.qty,
            cents_to_dollars_label(request_gross),
            product.family,
            account.industry,
            band_label,
            stats.n,
            stats
                .median
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "n/a".into()),
            stats
                .p25
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "n/a".into()),
            stats
                .p75
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "n/a".into()),
        );
        client::complete(&state.ai, REC_SYSTEM_PROMPT, &digest)
            .await
            .ok()
    } else {
        None
    };
    let degraded = narrative.is_none();

    Ok(Json(DiscountRecBody {
        comparables: Comparables {
            count: stats.n,
            family: product.family,
            industry: account.industry,
            band_label,
            median_pct: stats.median,
            p25: stats.p25,
            p75: stats.p75,
            sample,
        },
        narrative,
        degraded,
    }))
}
