// Client-side discount verdict — mirrors domain::ApprovalTier / role_can_decide
// (crates/domain/src/discount.rs). This is ADVISORY UI (the builder's live
// verdict, the detail screen's action gating); the server recomputes on submit
// and enforces on approve/reject, so this must never disagree with the server.

import type { DiscountPolicy, Role } from "../lib/types";

export type VerdictTier = "self" | "manager" | "vp";

export function verdictTier(worstPct: number, policy: DiscountPolicy): VerdictTier {
  if (worstPct <= policy.self_max_pct) return "self";
  if (worstPct <= policy.manager_max_pct) return "manager";
  return "vp";
}

export function verdictLabel(tier: VerdictTier, policy: DiscountPolicy): string {
  switch (tier) {
    case "self":
      return `Self-approve at ≤${policy.self_max_pct}%`;
    case "manager":
      return "Needs manager approval";
    case "vp":
      return "Needs VP approval";
  }
}

/** Mirror of domain::role_can_decide — a self-tier quote is never decided via
 *  the approval path (it auto-approves at submit). */
export function roleCanDecide(role: Role, tier: VerdictTier): boolean {
  if (tier === "manager")
    return role === "regional_manager" || role === "vp" || role === "admin";
  if (tier === "vp") return role === "vp" || role === "admin";
  return false;
}
