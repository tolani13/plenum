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

// ── P3 CRM operational core — mirror the crates/api/src/routes payloads ──────

export type OppStage =
  | "lead"
  | "qualified"
  | "quoted"
  | "negotiation"
  | "won"
  | "lost";

export type QuoteStatus =
  | "draft"
  | "pending_approval"
  | "approved"
  | "sent"
  | "accepted"
  | "rejected";

export type ActivityKind = "call" | "visit" | "email" | "note";

export interface OppRow {
  id: string;
  account_id: string;
  account_name: string;
  territory_code: string;
  owner_id: string;
  owner_name: string;
  stage: OppStage;
  kind: string;
  amount_cents: number;
  expected_close: string | null;
  lost_reason: string | null;
  has_approved_quote: boolean;
  approved_quote_net_cents: number | null;
}

export interface BookedOrder {
  id: string;
  ordered_on: string;
  gross_cents: number;
  net_cents: number;
}

export interface StageResult {
  opportunity: OppRow;
  booked_order: BookedOrder | null;
  accepted_quote_id: string | null;
}

export interface ProductItem {
  id: string;
  sku: string;
  name: string;
  family: string;
  kind: string;
  list_price_cents: number;
}

export interface DiscountPolicy {
  self_max_pct: number;
  manager_max_pct: number;
}

export interface QuoteLineOut {
  id: string;
  product_id: string;
  product_sku: string;
  product_name: string;
  family: string;
  qty: number;
  list_unit_cents: number;
  net_unit_cents: number;
  discount_pct: number;
  line_gross_cents: number;
  line_net_cents: number;
}

export interface QuoteDetail {
  id: string;
  opportunity_id: string;
  account_id: string;
  account_name: string;
  territory_code: string;
  status: QuoteStatus;
  created_by: string;
  created_by_name: string;
  approver_id: string | null;
  approver_name: string | null;
  created_at: string;
  submitted_at: string | null;
  decided_at: string | null;
  decision_reason: string | null;
  discount_policy_result: {
    verdict: string;
    worst_line_discount_pct: string;
    self_max_pct: string;
    manager_max_pct: string;
    outcome_status: string;
  } | null;
  worst_discount_pct: number | null;
  gross_cents: number;
  net_cents: number;
  leakage_cents: number;
  lines: QuoteLineOut[];
}

export interface QuoteListRow {
  id: string;
  opportunity_id: string;
  account_name: string;
  status: QuoteStatus;
  worst_discount_pct: number | null;
  created_by_name: string;
  created_at: string;
  submitted_at: string | null;
  gross_cents: number;
  net_cents: number;
}

export interface AuditRow {
  id: string;
  action: string;
  actor_id: string | null;
  actor_name: string | null;
  at: string;
  before_status: string | null;
  after_status: string | null;
}

export interface AccountSite {
  id: string;
  address: string;
  city: string;
  state: string;
}

export interface AccountContact {
  id: string;
  site_id: string;
  name: string;
  title: string | null;
  email: string | null;
  phone: string | null;
  role: string | null;
}

export interface UnitTimeline {
  id: string;
  serial: string;
  family: string;
  source: string;
  site_label: string;
  commissioned_on: string;
  expected_changeout_months: number | null;
  last_filter_order_on: string | null;
  cartridge_sku: string | null;
  cartridge_name: string | null;
  cartridge_count: number;
}

export interface AccountOrderBrief {
  id: string;
  ordered_on: string;
  gross_cents: number;
  net_cents: number;
}

export interface AccountOppBrief {
  id: string;
  stage: OppStage;
  kind: string;
  amount_cents: number;
  expected_close: string | null;
  owner_name: string;
  has_approved_quote: boolean;
}

export interface AccountActivity {
  id: string;
  kind: ActivityKind;
  body: string;
  occurred_at: string;
  rep_name: string;
}

export interface AccountDetail {
  id: string;
  name: string;
  industry: string;
  status: string;
  territory_code: string;
  territory_name: string;
  parent_account_id: string | null;
  created: string;
  cumulative: {
    gross_cents: number;
    net_cents: number;
    leakage_cents: number;
    leakage_pct: number | null;
  };
  sites: AccountSite[];
  contacts: AccountContact[];
  units: UnitTimeline[];
  recent_orders: AccountOrderBrief[];
  opportunities: AccountOppBrief[];
  activities: {
    items: AccountActivity[];
    limit: number;
    offset: number;
    total: number;
  };
  /** P4 (R14): the account's signals — active first, score DESC, capped 20 —
   *  in the same enriched shape as GET /api/signals. */
  signals: SignalRow[];
}

export interface AccountItem {
  id: string;
  name: string;
  industry: string;
  status: string;
  territory_code: string;
  parent_account_id: string | null;
  created: string;
}

// ── P4 Signals + AI — mirror crates/api/src/routes/signals.rs + src/ai ──────

export type SignalType =
  | "reorder_due"
  | "defection_risk"
  | "conquest"
  | "discount_anomaly";

export type SignalStatus = "open" | "assigned" | "actioned" | "dismissed";

/** The receipts contract: label + detail render on the card; weight is the
 *  raw numeric term behind the label (days/months/dollars/pct). */
export interface SignalReason {
  label: string;
  weight: number | null;
  detail: string;
}

export interface SignalRow {
  id: string;
  type: SignalType;
  status: SignalStatus;
  score: number;
  reasons: SignalReason[];
  account_id: string;
  account_name: string;
  territory_code: string;
  site_id: string | null;
  site_label: string | null;
  installed_unit_id: string | null;
  serial: string | null;
  cartridge_product_id: string | null;
  cartridge_sku: string | null;
  cartridge_count: number | null;
  annual_value_cents: number | null;
  order_line_id: string | null;
  assigned_to: string | null;
  assignee_name: string | null;
  outcome: string | null;
  dismissed_reason: string | null;
  opened_at: string;
  assigned_at: string | null;
  actioned_at: string | null;
  dismissed_at: string | null;
}

export interface SignalsSummary {
  total: number;
  by_type: {
    reorder_due: number;
    defection_risk: number;
    conquest: number;
    discount_anomaly: number;
  };
  territories: {
    territory_id: string;
    territory_code: string;
    open_count: number;
  }[];
}

export interface AssigneeRow {
  id: string;
  name: string;
  role: Role;
}

export interface AiStatus {
  ask: boolean;
  discount: boolean;
}

export interface AskResult {
  sql: string;
  columns: string[];
  rows: (string | number | boolean | null)[][];
  row_count: number;
  truncated: boolean;
}

export interface CompSample {
  account_name: string;
  ordered_on: string;
  product_sku: string;
  qty: number;
  gross_cents: number;
  discount_pct: number;
}

export interface DiscountRec {
  comparables: {
    count: number;
    family: string;
    industry: string;
    band_label: string;
    median_pct: number | null;
    p25: number | null;
    p75: number | null;
    sample: CompSample[];
  };
  narrative: string | null;
  degraded: boolean;
}
