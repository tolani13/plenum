//! GET /api/data-quality — the P5 (R2) panel feed: deterministic SQL finders
//! for the seeded mess (spec §9 beat 5 — "mess is information"), read-only.
//!
//! Four finders, all naturally RLS-scoped (accounts / orders are RLS tables;
//! everything joins through them), so a rep sees only her own scope's mess —
//! the seeded trio is only guaranteed complete at VP view, and the panel
//! says so:
//!   1. duplicate-ish account names — normalized comparison (lower, strip
//!      punctuation, drop legal-suffix words, strip a trailing plural 's'
//!      per word, join): catches "Keystone Coatings" / "Keystone Coating
//!      Co." and "Vantage Metalworks" / "Vantage Metal Works" without
//!      flagging the legitimately distinct "Vantage Metalworks Coastal"
//!      (its extra word survives normalization). Pure SQL — no pg_trgm, no
//!      extension (R2).
//!   2. installed units with a cartridge but NULL expected_changeout_months
//!      — cadence math cannot run; the account 360 renders the CADENCE
//!      UNKNOWN chip for exactly these rows.
//!   3. order lines at 100% discount (net 0 — the comped inspection).
//!   4. accounts with zero sites — trivially cheap disclosed addition;
//!      the seed plants none, so its designed state is the clean one.

use axum::extract::State;
use axum::Json;
use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::rls::rls_tx;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DuplicatePair {
    name_key: String,
    a_id: Uuid,
    a_name: String,
    a_territory_code: String,
    b_id: Uuid,
    b_name: String,
    b_territory_code: String,
}

#[derive(Serialize)]
pub struct NullCadenceUnit {
    unit_id: Uuid,
    serial: String,
    account_id: Uuid,
    account_name: String,
    territory_code: String,
    cartridge_sku: Option<String>,
}

#[derive(Serialize)]
pub struct FullDiscountLine {
    order_line_id: Uuid,
    order_id: Uuid,
    ordered_on: NaiveDate,
    account_id: Uuid,
    account_name: String,
    territory_code: String,
    product_sku: String,
    qty: i32,
    list_unit_cents: i64,
}

#[derive(Serialize)]
pub struct ZeroSiteAccount {
    account_id: Uuid,
    account_name: String,
    territory_code: String,
}

#[derive(Serialize)]
pub struct DataQualityBody {
    duplicate_names: Vec<DuplicatePair>,
    null_cadence_units: Vec<NullCadenceUnit>,
    full_discount_lines: Vec<FullDiscountLine>,
    zero_site_accounts: Vec<ZeroSiteAccount>,
}

pub async fn data_quality(
    State(state): State<AppState>,
    user: SessionUser,
) -> Result<Json<DataQualityBody>, ApiError> {
    let mut tx = rls_tx(&state.pool, &user).await?;

    // Finder 1 — duplicate-ish names. The normalization key per account:
    // lower → strip non-alphanumerics (keeping word breaks) → drop legal
    // suffix words → strip one trailing 's' per word → concatenate in word
    // order. The pair join is (name, id)-ordered so each pair appears once,
    // deterministically, even if two accounts share an exact name.
    let duplicate_names = sqlx::query_as!(
        DuplicatePair,
        r#"WITH norm AS (
               SELECT a.id, a.name, t.code AS territory_code,
                      (SELECT COALESCE(string_agg(regexp_replace(w.word, 's$', ''),
                                                  '' ORDER BY w.ord), '')
                       FROM unnest(string_to_array(
                                regexp_replace(lower(a.name), '[^a-z0-9 ]', '', 'g'),
                                ' ')) WITH ORDINALITY AS w(word, ord)
                       WHERE w.word <> ''
                         AND w.word NOT IN ('co', 'corp', 'inc', 'llc', 'ltd',
                                            'company', 'corporation')
                      ) AS name_key
               FROM accounts a
               JOIN territories t ON t.id = a.territory_id
           )
           SELECT n1.name_key AS "name_key!",
                  n1.id AS "a_id!", n1.name AS "a_name!",
                  n1.territory_code AS "a_territory_code!",
                  n2.id AS "b_id!", n2.name AS "b_name!",
                  n2.territory_code AS "b_territory_code!"
           FROM norm n1
           JOIN norm n2 ON n1.name_key = n2.name_key
                       AND n1.name_key <> ''
                       AND (n1.name, n1.id) < (n2.name, n2.id)
           ORDER BY n1.name_key, n1.name"#
    )
    .fetch_all(&mut *tx)
    .await?;

    // Finder 2 — cadence unknown: cartridge-bearing units missing
    // expected_changeout_months (the same predicate the 360's CADENCE
    // UNKNOWN chip renders on).
    let null_cadence_units = sqlx::query_as!(
        NullCadenceUnit,
        r#"SELECT iu.id AS "unit_id!", iu.serial AS "serial!",
                  a.id AS "account_id!", a.name AS "account_name!",
                  t.code AS "territory_code!",
                  cp.sku AS "cartridge_sku?"
           FROM installed_units iu
           JOIN sites s ON s.id = iu.site_id
           JOIN accounts a ON a.id = s.account_id
           JOIN territories t ON t.id = a.territory_id
           LEFT JOIN products cp ON cp.id = iu.cartridge_product_id
           WHERE iu.expected_changeout_months IS NULL
             AND iu.cartridge_product_id IS NOT NULL
           ORDER BY a.name, iu.serial"#
    )
    .fetch_all(&mut *tx)
    .await?;

    // Finder 3 — the 100% line (net 0; passes the price CHECK by
    // construction).
    let full_discount_lines = sqlx::query_as!(
        FullDiscountLine,
        r#"SELECT ol.id AS "order_line_id!", o.id AS "order_id!",
                  o.ordered_on AS "ordered_on!",
                  a.id AS "account_id!", a.name AS "account_name!",
                  t.code AS "territory_code!",
                  p.sku AS "product_sku!", ol.qty AS "qty!",
                  ol.list_unit_cents AS "list_unit_cents!"
           FROM order_lines ol
           JOIN orders o ON o.id = ol.order_id
           JOIN accounts a ON a.id = o.account_id
           JOIN territories t ON t.id = a.territory_id
           JOIN products p ON p.id = ol.product_id
           WHERE ol.discount_pct = 100
           ORDER BY o.ordered_on, ol.id"#
    )
    .fetch_all(&mut *tx)
    .await?;

    // Finder 4 — accounts with zero sites (disclosed addition; designed
    // empty on the seed).
    let zero_site_accounts = sqlx::query_as!(
        ZeroSiteAccount,
        r#"SELECT a.id AS "account_id!", a.name AS "account_name!",
                  t.code AS "territory_code!"
           FROM accounts a
           JOIN territories t ON t.id = a.territory_id
           WHERE NOT EXISTS (SELECT 1 FROM sites s WHERE s.account_id = a.id)
           ORDER BY a.name"#
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(DataQualityBody {
        duplicate_names,
        null_cadence_units,
        full_discount_lines,
        zero_site_accounts,
    }))
}
