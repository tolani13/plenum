//! Discount governance (spec §7, architect ruling R3). The thresholds live in
//! the `discount_policy` table (seed-config, 10/25 by default) and are read per
//! request; this module is the PURE decision logic over them — no database, no
//! HTTP — so it lives in `domain` with unit tests, exactly like `period` and
//! `money`.
//!
//! The rule (spec §7): a quote's worst-line discount decides the approval tier.
//!   worst ≤ self_max      → SelfApprove (auto-approved on submit)
//!   self_max < worst ≤ mgr → Manager    (regional_manager / vp / admin decide)
//!   worst > manager_max   → Vp          (vp / admin only)
//!
//! Comparisons are on rust_decimal::Decimal — exact, no float anywhere near a
//! money-or-percent value (the money.rs doctrine).

use rust_decimal::Decimal;

use crate::enums::UserRole;

/// The two thresholds, read from `discount_policy`. numeric(5,2) → Decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscountPolicy {
    pub self_max_pct: Decimal,
    pub manager_max_pct: Decimal,
}

/// Which authority a quote's worst-line discount demands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalTier {
    SelfApprove,
    Manager,
    Vp,
}

impl ApprovalTier {
    /// The tier for a worst-line discount under a policy (boundaries inclusive
    /// on the low side: exactly self_max → SelfApprove, exactly manager_max →
    /// Manager, per spec §7's "≤10 / 10–25 / >25").
    pub fn from_worst(worst_pct: Decimal, policy: &DiscountPolicy) -> Self {
        if worst_pct <= policy.self_max_pct {
            ApprovalTier::SelfApprove
        } else if worst_pct <= policy.manager_max_pct {
            ApprovalTier::Manager
        } else {
            ApprovalTier::Vp
        }
    }

    /// The verdict string recorded in `quotes.discount_policy_result` and
    /// returned to the client (the receipts contract — the tier is on the face).
    pub fn verdict_code(&self) -> &'static str {
        match self {
            ApprovalTier::SelfApprove => "self_approved",
            ApprovalTier::Manager => "manager_approval",
            ApprovalTier::Vp => "vp_approval",
        }
    }
}

/// May a caller with `role` DECIDE (approve or reject) a pending quote whose
/// worst-line discount lands in `tier`? This is the R3 role gate — RLS alone
/// would let an in-territory rep approve their own quote, so the handler asks
/// this before every approve/reject.
///
/// SelfApprove is decision-closed here: a self-tier quote auto-approves at
/// submit and never reaches the approve/reject handlers, so no role may decide
/// one through the approval path (belt-and-braces).
pub fn role_can_decide(role: UserRole, tier: ApprovalTier) -> bool {
    match tier {
        ApprovalTier::SelfApprove => false,
        ApprovalTier::Manager => matches!(
            role,
            UserRole::RegionalManager | UserRole::Vp | UserRole::Admin
        ),
        ApprovalTier::Vp => matches!(role, UserRole::Vp | UserRole::Admin),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DiscountPolicy {
        DiscountPolicy {
            self_max_pct: Decimal::new(1000, 2),    // 10.00
            manager_max_pct: Decimal::new(2500, 2), // 25.00
        }
    }

    fn pct(hundredths: i64) -> Decimal {
        Decimal::new(hundredths, 2)
    }

    #[test]
    fn tiers_partition_the_range_with_inclusive_low_boundaries() {
        let p = policy();
        // ≤ 10.00 → self-approve, boundary inclusive.
        assert_eq!(ApprovalTier::from_worst(pct(0), &p), ApprovalTier::SelfApprove);
        assert_eq!(
            ApprovalTier::from_worst(pct(1000), &p),
            ApprovalTier::SelfApprove
        );
        // 10.01 .. 25.00 → manager, upper boundary inclusive.
        assert_eq!(
            ApprovalTier::from_worst(pct(1001), &p),
            ApprovalTier::Manager
        );
        assert_eq!(
            ApprovalTier::from_worst(pct(2500), &p),
            ApprovalTier::Manager
        );
        // > 25.00 → VP. The governance headline (28%) lands here.
        assert_eq!(ApprovalTier::from_worst(pct(2501), &p), ApprovalTier::Vp);
        assert_eq!(ApprovalTier::from_worst(pct(2800), &p), ApprovalTier::Vp);
        assert_eq!(ApprovalTier::from_worst(pct(10000), &p), ApprovalTier::Vp);
    }

    #[test]
    fn verdict_codes_are_stable() {
        assert_eq!(ApprovalTier::SelfApprove.verdict_code(), "self_approved");
        assert_eq!(ApprovalTier::Manager.verdict_code(), "manager_approval");
        assert_eq!(ApprovalTier::Vp.verdict_code(), "vp_approval");
    }

    #[test]
    fn role_gate_matches_spec_7() {
        use ApprovalTier::{Manager, SelfApprove, Vp};
        use UserRole::{Admin, RegionalManager, Rep, Vp as VpRole};

        // Manager tier (10–25%): RM/VP/admin decide; a rep never does.
        assert!(role_can_decide(RegionalManager, Manager));
        assert!(role_can_decide(VpRole, Manager));
        assert!(role_can_decide(Admin, Manager));
        assert!(!role_can_decide(Rep, Manager));

        // VP tier (>25%): only VP/admin — a regional manager is refused (the
        // adversarial "RM approves the 28% → 403" case).
        assert!(!role_can_decide(RegionalManager, Vp));
        assert!(role_can_decide(VpRole, Vp));
        assert!(role_can_decide(Admin, Vp));
        assert!(!role_can_decide(Rep, Vp));

        // Self-approve tier is never decided through the approval path.
        for role in [Rep, RegionalManager, VpRole, Admin] {
            assert!(!role_can_decide(role, SelfApprove));
        }
    }
}
