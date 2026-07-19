// Payload shapes — mirror the P1 metrics handlers verbatim (crates/api/src/
// routes/metrics.rs). The frontend consumes these as-is; it NEVER widens,
// recomputes, or supplements what the caller's session returned. Money fields
// are integer cents; the *_pct / *_rate fields are display numbers or null.

export type Role = "rep" | "regional_manager" | "vp" | "admin";

export interface Me {
  id: string;
  name: string;
  email: string;
  role: Role;
  territories: string[];
}

export interface Page<T> {
  items: T[];
  limit: number;
  offset: number;
  total: number;
}

export interface TerritoryRow {
  territory_code: string;
  territory_name: string;
  gross_cents: number;
  net_cents: number;
  leakage_cents: number;
  leakage_pct: number | null;
  order_count: number;
  active_accounts: number;
  quota_attainment_pct: number | null;
}

export interface LeaderboardRow {
  rep_name: string;
  gross_cents: number;
  net_cents: number;
  leakage_pct: number | null;
  capital_gross_cents: number;
  capital_net_cents: number;
  consumable_gross_cents: number;
  consumable_net_cents: number;
  top_account_name: string | null;
}

export interface ItemRow {
  product_sku: string | null;
  product_name: string;
  family: string;
  kind: string;
  units: number;
  gross_cents: number;
  net_cents: number;
  attach_rate_pct: number | null;
}

export interface CustomerRow {
  account_name: string;
  gross_cents: number;
  net_cents: number;
  leakage_pct: number | null;
  capital_gross_cents: number;
  capital_net_cents: number;
  consumable_gross_cents: number;
  consumable_net_cents: number;
}

export interface CoverageRow {
  territory_code: string;
  territory_name: string;
  units_due: number;
  pct_covered: number | null;
  projected_consumable_gross_cents: number;
  projected_consumable_net_cents: number;
}

export interface DefectionRow {
  serial: string;
  site: string;
  account_name: string;
  territory_code: string;
  days_silent: number;
  expected_changeout_months: number;
  annual_consumable_value_cents: number;
  score: number;
}
