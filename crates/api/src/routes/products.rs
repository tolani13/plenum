//! GET /api/products — the catalog the quote builder's product picker reads.
//!
//! products is a GLOBAL catalog (no RLS, spec §4 lists exactly six RLS tables),
//! so this is auth-guarded but not territory-scoped — every session sees the
//! same catalog, which is correct: a product is not tenant data. The builder
//! needs it to show SKUs, names, families, and LIST PRICES (the money law: the
//! server still derives every stored price at draft time; the picker is display
//! only). Reads beyond the R-route-list, added openly for the §8 screen-7
//! builder — flagged in the session report.

use axum::extract::{Query, State};
use axum::Json;
use domain::ProductKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::SessionUser;
use crate::error::ApiError;
use crate::routes::common::parse_page;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ProductItem {
    id: Uuid,
    sku: String,
    name: String,
    family: String,
    kind: ProductKind,
    list_price_cents: i64,
}

#[derive(Serialize)]
pub struct ProductsPage {
    items: Vec<ProductItem>,
    limit: i64,
    offset: i64,
    total: i64,
}

#[derive(Deserialize)]
pub struct ProductParams {
    kind: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

pub async fn list_products(
    State(state): State<AppState>,
    _user: SessionUser,
    Query(params): Query<ProductParams>,
) -> Result<Json<ProductsPage>, ApiError> {
    let (limit, offset) = parse_page(params.limit, params.offset)?;
    let kind = match params.kind {
        None => None,
        Some(k) => {
            let low = k.trim().to_ascii_lowercase();
            if !["capital", "consumable", "part", "service"].contains(&low.as_str()) {
                return Err(ApiError::Invalid(
                    "kind must be capital, consumable, part, or service".into(),
                ));
            }
            Some(low)
        }
    };

    let items = sqlx::query_as!(
        ProductItem,
        r#"SELECT id, sku, name, family, kind AS "kind: ProductKind", list_price_cents
           FROM products
           WHERE ($1::text IS NULL OR kind::text = $1)
           ORDER BY kind, family, sku
           LIMIT $2 OFFSET $3"#,
        kind,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM products
           WHERE ($1::text IS NULL OR kind::text = $1)"#,
        kind
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ProductsPage {
        items,
        limit,
        offset,
        total,
    }))
}
