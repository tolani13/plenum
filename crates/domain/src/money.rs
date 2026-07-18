//! Money math. Discount is represented internally in basis points
//! (i64; 12.50% = 1250) so no float ever touches a money value.
//!
//! The database enforces, on quote_lines and order_lines:
//!     CHECK (net_unit_cents = round(list_unit_cents * (100 - discount_pct) / 100))
//! Postgres round() on NUMERIC is half-away-from-zero. For the non-negative
//! values used here that equals round-half-up, which is what the integer
//! formula below computes. Rust MUST compute net identically or the seed dies
//! on the CHECK.

use rust_decimal::Decimal;

/// Maximum representable discount: 100.00% = 10_000 basis points.
pub const MAX_DISCOUNT_BP: i64 = 10_000;

/// Net unit cents from list unit cents and a discount in basis points.
/// Integer math, round-half-up: net = (list * (10000 - bp) + 5000) / 10000.
///
/// # Panics
/// Panics if `discount_bp` is outside 0..=10_000 or `list_unit_cents` is
/// negative — both are seed-programming errors, not data conditions.
pub fn net_unit_cents(list_unit_cents: i64, discount_bp: i64) -> i64 {
    assert!(
        (0..=MAX_DISCOUNT_BP).contains(&discount_bp),
        "discount_bp out of range: {discount_bp}"
    );
    assert!(
        list_unit_cents >= 0,
        "negative list_unit_cents: {list_unit_cents}"
    );
    (list_unit_cents * (MAX_DISCOUNT_BP - discount_bp) + 5_000) / MAX_DISCOUNT_BP
}

/// Convert basis points to the NUMERIC(5,2) discount_pct column value.
/// 1250 bp -> Decimal 12.50.
pub fn discount_bp_to_pct(discount_bp: i64) -> Decimal {
    Decimal::new(discount_bp, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::RoundingStrategy;

    /// Reimplementation of the SQL CHECK formula using exact decimal
    /// arithmetic: round(list * (100 - discount_pct) / 100), with Postgres
    /// round() semantics (half away from zero).
    fn sql_check_net(list_unit_cents: i64, discount_bp: i64) -> i64 {
        let list = Decimal::from(list_unit_cents);
        let pct = discount_bp_to_pct(discount_bp);
        let raw = list * (Decimal::from(100) - pct) / Decimal::from(100);
        raw.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
            .to_i64()
            .expect("net fits i64")
    }

    /// Property test: the integer bp formula equals the SQL CHECK formula for
    /// every basis point 0..=10000 across a spread of list prices, including
    /// the seed's real price ranges and edge values.
    #[test]
    fn net_matches_sql_check_for_all_basis_points() {
        let list_values: &[i64] = &[
            0, 1, 2, 3, 49, 50, 51, 99, 100, 101, 999, 12_345, 15_000, 29_999, 45_000, 60_000,
            123_456, 999_999, 4_500_000, 18_500_000, 25_000_000,
        ];
        for &list in list_values {
            for bp in 0..=MAX_DISCOUNT_BP {
                let rust_net = net_unit_cents(list, bp);
                let sql_net = sql_check_net(list, bp);
                assert_eq!(
                    rust_net, sql_net,
                    "mismatch at list={list} bp={bp}: rust={rust_net} sql={sql_net}"
                );
            }
        }
    }

    #[test]
    fn known_values() {
        // 12.50% off 999 cents: exact 874.125 -> 874.
        assert_eq!(net_unit_cents(999, 1250), 874);
        // 25% off 10 cents: exact 7.5 -> rounds half-up to 8.
        assert_eq!(net_unit_cents(10, 2500), 8);
        // 100% discount -> net 0 (the seed's deliberate-mess line).
        assert_eq!(net_unit_cents(123_456, 10_000), 0);
        // 0% discount -> net == list.
        assert_eq!(net_unit_cents(123_456, 0), 123_456);
    }

    #[test]
    fn pct_conversion_is_two_decimal_places() {
        assert_eq!(discount_bp_to_pct(1250).to_string(), "12.50");
        assert_eq!(discount_bp_to_pct(0).to_string(), "0.00");
        assert_eq!(discount_bp_to_pct(10_000).to_string(), "100.00");
        assert_eq!(discount_bp_to_pct(2800).to_string(), "28.00");
    }
}
