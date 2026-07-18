//! Rust mirrors of the Postgres enums from migrations/0001_enums.sql.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Rep,
    RegionalManager,
    Vp,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Rep => "rep",
            UserRole::RegionalManager => "regional_manager",
            UserRole::Vp => "vp",
            UserRole::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "account_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Customer,
    Prospect,
    AtRisk,
    Dormant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "opp_stage", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OppStage {
    Lead,
    Qualified,
    Quoted,
    Negotiation,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "quote_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum QuoteStatus {
    Draft,
    PendingApproval,
    Approved,
    Sent,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "signal_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    ReorderDue,
    DefectionRisk,
    Conquest,
    DiscountAnomaly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "signal_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SignalStatus {
    Open,
    Assigned,
    Actioned,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "product_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProductKind {
    Capital,
    Consumable,
    Part,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "activity_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Call,
    Visit,
    Email,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "region", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Region {
    Northeast,
    Southeast,
    Midwest,
    SouthCentral,
    Mountain,
    West,
    CanadaE,
    CanadaW,
}
