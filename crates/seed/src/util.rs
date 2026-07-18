//! Deterministic RNG helpers. Every random draw in the seed flows through the
//! one StdRng seeded with 20260717 — including every UUID — so each run
//! produces byte-identical business rows.

use chrono::{Days, Months, NaiveDate};
use rand::rngs::StdRng;
use rand::Rng;
use uuid::Uuid;

/// The fixed PRNG seed. Spec §9: every run tells the identical story.
pub const SEED: u64 = 20_260_717;

/// "Today" for the generated world; the order window ends here.
pub fn seed_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 15).expect("valid date")
}

/// Order history window start (3.5 years ending 2026-07).
pub fn window_start() -> NaiveDate {
    NaiveDate::from_ymd_opt(2023, 1, 1).expect("valid date")
}

/// Deterministic v4-shaped UUID from the seeded RNG. Server-side
/// gen_random_uuid() defaults exist for the app path; seed rows must be
/// identical run to run, so the seed never uses them.
pub fn det_uuid(rng: &mut StdRng) -> Uuid {
    let bytes: [u8; 16] = rng.gen();
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

/// Inclusive integer range draw.
pub fn pick_i64(rng: &mut StdRng, lo: i64, hi: i64) -> i64 {
    rng.gen_range(lo..=hi)
}

pub fn pick_i32(rng: &mut StdRng, lo: i32, hi: i32) -> i32 {
    rng.gen_range(lo..=hi)
}

/// True with probability `pct` percent.
pub fn chance(rng: &mut StdRng, pct: u32) -> bool {
    rng.gen_range(0..100) < pct
}

/// Deterministic element pick.
pub fn pick<'a, T>(rng: &mut StdRng, items: &'a [T]) -> &'a T {
    &items[rng.gen_range(0..items.len())]
}

/// date + n months (calendar-aware).
pub fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    if months >= 0 {
        date + Months::new(months as u32)
    } else {
        date - Months::new((-months) as u32)
    }
}

/// date + n days (n may be negative).
pub fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    if days >= 0 {
        date + Days::new(days as u64)
    } else {
        date - Days::new((-days) as u64)
    }
}

/// A day-of-month safe for every month.
pub fn day_in_month(rng: &mut StdRng) -> u32 {
    rng.gen_range(1..=28)
}
