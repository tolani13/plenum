//! In-memory row types. Fields prefixed with nothing are persisted; the few
//! `*_idx` fields are generation-time bookkeeping (indexes into World vecs)
//! and never hit the database.

use chrono::{DateTime, NaiveDate, Utc};
use domain::{AccountStatus, OppStage, ProductKind, QuoteStatus, Region, UserRole};
use uuid::Uuid;

pub struct TerritoryRow {
    pub id: Uuid,
    pub code: &'static str,
    pub name: &'static str,
    pub region: Region,
    pub quota_year_cents: i64,
}

pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub name: &'static str,
    pub role: UserRole,
    pub manager_id: Option<Uuid>,
}

pub struct AssignmentRow {
    pub user_id: Uuid,
    pub territory_id: Uuid,
}

pub struct AccountRow {
    pub id: Uuid,
    pub name: &'static str,
    pub industry: &'static str,
    pub status: AccountStatus,
    pub territory_id: Uuid,
    pub parent_account_id: Option<Uuid>,
    pub created: DateTime<Utc>,
    // bookkeeping
    pub rep_id: Uuid,
}

pub struct SiteRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub address: String,
    pub city: &'static str,
    pub state: &'static str,
    pub primary_contact_id: Option<Uuid>,
    // bookkeeping
    pub account_idx: usize,
}

pub struct ContactRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub name: String,
    pub title: &'static str,
    pub email: String,
    pub phone: String,
    pub role: &'static str,
}

pub struct ProductRow {
    pub id: Uuid,
    pub sku: String,
    pub name: String,
    pub family: String,
    pub kind: ProductKind,
    pub list_price_cents: i64,
    pub filter_fits: Vec<String>,
    // bookkeeping: cartridge slots on capital collector models
    pub cartridge_slots: Option<i32>,
}

pub struct UnitRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub product_id: Uuid,
    pub serial: String,
    pub commissioned_on: NaiveDate,
    pub source: String,
    pub cartridge_count: i32,
    pub cartridge_product_id: Option<Uuid>,
    pub expected_changeout_months: Option<i32>,
    pub last_filter_order_on: Option<NaiveDate>,
    // bookkeeping
    pub site_idx: usize,
    pub cartridge_idx: Option<usize>,
}

pub struct OppRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub territory_id: Uuid,
    pub owner_id: Uuid,
    pub stage: OppStage,
    pub kind: &'static str,
    pub amount_cents: i64,
    pub expected_close: Option<NaiveDate>,
    pub lost_reason: Option<&'static str>,
}

pub struct QuoteRow {
    pub id: Uuid,
    pub opportunity_id: Uuid,
    pub status: QuoteStatus,
    pub approver_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    // P3 (0011) additive columns. Only the seeded Wes quote carries a backfilled
    // verdict + submitted_at so the VP inbox renders it complete (R9).
    pub submitted_at: Option<DateTime<Utc>>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub discount_policy_result: Option<serde_json::Value>,
}

pub struct QuoteLineRow {
    pub id: Uuid,
    pub quote_id: Uuid,
    pub product_id: Uuid,
    pub qty: i32,
    pub list_unit_cents: i64,
    pub net_unit_cents: i64,
    pub discount_bp: i64,
}

pub struct OrderRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub site_id: Uuid,
    pub territory_id: Uuid,
    pub rep_id: Uuid,
    pub ordered_on: NaiveDate,
}

pub struct OrderLineRow {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub qty: i32,
    pub list_unit_cents: i64,
    pub net_unit_cents: i64,
    pub discount_bp: i64,
}

/// Everything the seed writes, generated fully in memory first.
pub struct World {
    pub territories: Vec<TerritoryRow>,
    pub users: Vec<UserRow>,
    pub assignments: Vec<AssignmentRow>,
    pub accounts: Vec<AccountRow>,
    pub sites: Vec<SiteRow>,
    pub contacts: Vec<ContactRow>,
    pub products: Vec<ProductRow>,
    pub units: Vec<UnitRow>,
    pub opportunities: Vec<OppRow>,
    pub quotes: Vec<QuoteRow>,
    pub quote_lines: Vec<QuoteLineRow>,
    pub orders: Vec<OrderRow>,
    pub order_lines: Vec<OrderLineRow>,
    /// argon2id hash shared by all demo users (hashed once — 17 users).
    pub password_hash: String,
}
