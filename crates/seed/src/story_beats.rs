//! ════════════════════════ THE STORY-BEAT SECTION ════════════════════════
//! Spec §9: these five beats are seeded ON PURPOSE. They are demo script
//! material, not bugs to fix. Later phases (signals, leaderboards, quotes UI)
//! depend on this exact data existing.
//!
//!   1. Ridgeline Grain (SE-1, grain): steady cartridge cadence that goes
//!      silent — last_filter_order_on ≈ 2025-08, eleven months before the
//!      seeded "today" (2026-07). The defection alarm the demo opens on.
//!   2. Wes Turner (W-1): league-leading gross, league-worst discount
//!      leakage. He owns the whale accounts and every line he touches gets a
//!      deep extra discount.
//!   3. Alpenglow Pharmaceuticals (MT-1, pharma, PROSPECT): three
//!      competitor-source installed units, no cartridge relationship with us
//!      — pure conquest fuel.
//!   4. One quote in status pending_approval, worst line at 28% discount,
//!      approver = the VP. The governance walkthrough.
//!   5. Deliberate mess: duplicate-ish account names (Keystone Coatings /
//!      Keystone Coating Co. and Vantage Metalworks / Vantage Metal Works),
//!      exactly 2 units with NULL expected_changeout_months, exactly one
//!      order line at 100% discount (net 0 — passes the price CHECK).

use chrono::NaiveDate;
use domain::{net_unit_cents, ApprovalTier, DiscountPolicy, OppStage, QuoteStatus, UserRole};
use rand::rngs::StdRng;
use rust_decimal::Decimal;

use crate::data::{AccountRow, OppRow, ProductRow, QuoteLineRow, QuoteRow, UnitRow, UserRow};
use crate::products::{brand_code, idx_by_sku, COMPETITOR_BRANDS};
use crate::util::{add_days, det_uuid, pick_i32, pick_i64};

// ── Beat 1: Ridgeline Grain goes silent ─────────────────────────────────────

pub const RIDGELINE_ACCOUNT: &str = "Ridgeline Grain Cooperative";

/// No cartridge order after this date. Noise (parts/service) continues — the
/// account still buys gaskets while the filter spend goes to a will-fit
/// competitor. That contrast IS the story.
pub fn ridgeline_silence_cutoff() -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 8, 31).expect("valid date")
}

/// Every Ridgeline unit gets one final cartridge order in this month so
/// last_filter_order_on lands in 2025-08 deterministically.
pub fn ridgeline_final_order_date(rng: &mut StdRng) -> NaiveDate {
    NaiveDate::from_ymd_opt(2025, 8, pick_i32(rng, 3, 25) as u32).expect("valid date")
}

// ── Beat 2: the leakage rep ─────────────────────────────────────────────────

pub const LEAKAGE_REP: &str = "Wes Turner";
/// Extra discount on every non-capital line Wes touches (basis points).
pub const LEAKAGE_BONUS_BP: i64 = 900;
/// Capital lines get even deeper leakage.
pub const LEAKAGE_CAPITAL_BONUS_BP: i64 = 1_600;
/// Ceiling so ordinary lines stay below the quote beat's 28% headline.
pub const LEAKAGE_CLAMP_BP: i64 = 2_500;
/// The whale accounts that hand Wes league-leading gross volume.
pub const WHALE_ACCOUNTS: [&str; 3] = [
    "Vantage Metalworks",
    "Vantage Metalworks Coastal",
    "Ironhold Alloys",
];
/// Extra noise orders per whale site per month.
pub const WHALE_NOISE_BONUS: i32 = 3;

// ── Beat 3: the Alpenglow conquest prospect ─────────────────────────────────

pub const ALPENGLOW_ACCOUNT: &str = "Alpenglow Pharmaceuticals";

/// Exactly three competitor-source units at the Alpenglow site. No cartridge
/// product (they do not buy filters from us — yet), so no cadence, no orders.
pub fn alpenglow_units(
    rng: &mut StdRng,
    site_id: uuid::Uuid,
    site_idx: usize,
    products: &[ProductRow],
    serial_seq: &mut usize,
) -> Vec<UnitRow> {
    let competitor_capitals: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == domain::ProductKind::Capital && p.cartridge_slots.is_none())
        .map(|(i, _)| i)
        .collect();
    (0..3)
        .map(|_| {
            let pi = competitor_capitals
                [pick_i64(rng, 0, competitor_capitals.len() as i64 - 1) as usize];
            let product = &products[pi];
            let brand = COMPETITOR_BRANDS
                .iter()
                .find(|b| product.family.starts_with(&brand_code(b)))
                .expect("competitor family maps to a brand");
            *serial_seq += 1;
            UnitRow {
                id: det_uuid(rng),
                site_id,
                product_id: product.id,
                serial: format!("SN-{}-{:05}", product.family, *serial_seq),
                commissioned_on: NaiveDate::from_ymd_opt(
                    pick_i32(rng, 2017, 2023),
                    pick_i32(rng, 1, 12) as u32,
                    pick_i32(rng, 1, 28) as u32,
                )
                .expect("valid date"),
                source: (*brand).to_string(),
                cartridge_count: pick_i32(rng, 8, 14),
                cartridge_product_id: None,
                expected_changeout_months: Some(pick_i32(rng, 6, 9)),
                last_filter_order_on: None,
                site_idx,
                cartridge_idx: None,
            }
        })
        .collect()
}

// ── Beat 4: the 28% quote in the VP's approval inbox ────────────────────────

pub const PENDING_QUOTE_ACCOUNT: &str = "Vantage Metalworks";
pub const PENDING_QUOTE_WORST_BP: i64 = 2_800; // 28.00% — the governance headline

/// One opportunity + one quote (pending_approval, approver = VP) with the
/// worst line at exactly 28% discount. Created by Wes, of course.
pub fn build_pending_quote(
    rng: &mut StdRng,
    accounts: &[AccountRow],
    users: &[UserRow],
    products: &[ProductRow],
) -> (OppRow, QuoteRow, Vec<QuoteLineRow>) {
    let account = accounts
        .iter()
        .find(|a| a.name == PENDING_QUOTE_ACCOUNT)
        .expect("whale account exists");
    let wes = users
        .iter()
        .find(|u| u.name == LEAKAGE_REP)
        .expect("leakage rep exists");
    let vp = users
        .iter()
        .find(|u| u.role == UserRole::Vp)
        .expect("VP exists");

    // (sku, qty, discount bp) — CAP-GS3-16 at 28% is the worst line.
    let line_specs: [(&str, i32, i64); 3] = [
        ("CAP-GS3-16", 1, PENDING_QUOTE_WORST_BP),
        ("SVC-COMMISSION", 1, 1_000),
        ("FLT-OPTI-GS3", 32, 1_200),
    ];

    let opp_id = det_uuid(rng);
    let quote_id = det_uuid(rng);
    let mut lines = Vec::new();
    let mut gross = 0_i64;
    for (sku, qty, bp) in line_specs {
        let product = &products[idx_by_sku(products, sku)];
        gross += product.list_price_cents * i64::from(qty);
        lines.push(QuoteLineRow {
            id: det_uuid(rng),
            quote_id,
            product_id: product.id,
            qty,
            list_unit_cents: product.list_price_cents,
            net_unit_cents: net_unit_cents(product.list_price_cents, bp),
            discount_bp: bp,
        });
    }

    let opp = OppRow {
        id: opp_id,
        account_id: account.id,
        territory_id: account.territory_id,
        owner_id: wes.id,
        stage: OppStage::Quoted,
        kind: "capital",
        amount_cents: gross,
        expected_close: Some(NaiveDate::from_ymd_opt(2026, 8, 15).expect("valid date")),
        lost_reason: None,
    };
    // R9 backfill: the worst line is 28%, so the verdict is VP-tier — derived
    // through the SAME domain logic the API's submit handler uses (proving the
    // seeded quote and a live-submitted one agree). submitted_at is set so the
    // VP inbox renders this quote as a completed submission.
    let policy = DiscountPolicy {
        self_max_pct: Decimal::new(1000, 2),    // 10.00
        manager_max_pct: Decimal::new(2500, 2), // 25.00
    };
    let worst = Decimal::new(PENDING_QUOTE_WORST_BP, 2); // 28.00
    let tier = ApprovalTier::from_worst(worst, &policy);
    let discount_policy_result = serde_json::json!({
        "verdict": tier.verdict_code(),
        "worst_line_discount_pct": worst.to_string(),
        "self_max_pct": policy.self_max_pct.to_string(),
        "manager_max_pct": policy.manager_max_pct.to_string(),
        "outcome_status": "pending_approval",
    });

    let quote = QuoteRow {
        id: quote_id,
        opportunity_id: opp_id,
        status: QuoteStatus::PendingApproval,
        approver_id: Some(vp.id),
        created_by: wes.id,
        created_at: NaiveDate::from_ymd_opt(2026, 7, 10)
            .expect("valid date")
            .and_hms_opt(14, 30, 0)
            .expect("valid time")
            .and_utc(),
        submitted_at: Some(
            NaiveDate::from_ymd_opt(2026, 7, 10)
                .expect("valid date")
                .and_hms_opt(14, 35, 0)
                .expect("valid time")
                .and_utc(),
        ),
        decided_at: None, // still pending — no decision yet
        decision_reason: None,
        discount_policy_result: Some(discount_policy_result),
    };
    (opp, quote, lines)
}

// ── Beat 6 (P3): the deterministic opportunity book ─────────────────────────
// Ruling R2. §9 seeded exactly ONE opportunity (beat 4); a kanban with one
// card is not demoable. This book is generated from a SEPARATE StdRng stream
// (keyed by the master SEED xor'd with OPP_BOOK_STREAM_KEY) and appended AFTER
// every existing draw in main.rs, so the pre-existing streams — and every
// frozen order/unit anchor — are byte-for-byte untouched.
//
// Beat 6 proper is the Ridgeline win-back: the ~$34k/yr annuity from the demo
// script, seeded as an opportunity with NO quote attached (D. drafts it live —
// that is gate P3-1). Every other opp is a plausible pipeline card. Stages are
// lead→negotiation ONLY (no seeded won/lost — those are user actions).

/// Separate RNG stream key for the opportunity book (R2). XOR'd with util::SEED.
/// Documented so the isolation is auditable: change nothing before this and the
/// anchors cannot move.
pub const OPP_BOOK_STREAM_KEY: u64 = 0x0000_0003_0B00_C0DE; // "P3 opp book"

/// Ridgeline win-back: ~$34k/yr cartridge annuity going to a will-fit
/// competitor. SE-1, owner Serena, kind filter-program, stage qualified.
pub const RIDGELINE_WINBACK_KIND: &str = "filter-program";
pub const RIDGELINE_WINBACK_AMOUNT_CENTS: i64 = 3_400_000;

/// The pipeline book: (account name, stage, kind, gross amount cents). Curated
/// so the per-stage distribution is exact and auditable (precondition 6), while
/// uuids + expected_close jitter come from the isolated stream. Every account
/// here has ≥1 site (precondition 5) and its territory/owner are taken from the
/// account row (R2: territory_id ALWAYS == the account's territory).
const OPP_BOOK: [(&str, OppStage, &str, i64); 14] = [
    (
        "Blue Ridge Fabrication",
        OppStage::Negotiation,
        "capital",
        8_500_000,
    ),
    (
        "Coastal Chem Processing",
        OppStage::Lead,
        "filter-program",
        1_200_000,
    ),
    (
        "Meridian Biologics",
        OppStage::Qualified,
        "capital",
        14_500_000,
    ),
    (
        "Liberty Auto Components",
        OppStage::Negotiation,
        "retrofit",
        6_400_000,
    ),
    (
        "Great Lakes Laser Works",
        OppStage::Quoted,
        "capital",
        9_800_000,
    ),
    (
        "Motor City Stampings",
        OppStage::Lead,
        "filter-program",
        980_000,
    ),
    (
        "Heartland Mills",
        OppStage::Qualified,
        "filter-program",
        2_100_000,
    ),
    (
        "Lone Star Thermal Spray",
        OppStage::Quoted,
        "capital",
        7_800_000,
    ),
    (
        "Alpenglow Pharmaceuticals",
        OppStage::Qualified,
        "filter-program",
        4_000_000,
    ),
    (
        "Rocky Mountain Mining Supply",
        OppStage::Negotiation,
        "capital",
        7_800_000,
    ),
    ("Ironhold Alloys", OppStage::Quoted, "capital", 21_500_000),
    (
        "Pacific Crest Pharma",
        OppStage::Lead,
        "filter-program",
        1_500_000,
    ),
    (
        "Maple Leaf Metal Fab",
        OppStage::Qualified,
        "retrofit",
        5_400_000,
    ),
    (
        "Cascadia Mining Co.",
        OppStage::Negotiation,
        "capital",
        7_800_000,
    ),
];

fn book_account<'a>(accounts: &'a [AccountRow], name: &str) -> &'a AccountRow {
    accounts
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("opportunity-book account missing: {name}"))
}

/// The Ridgeline win-back beat + the 14-opp pipeline book. `rng` MUST be the
/// isolated opp-book stream (see OPP_BOOK_STREAM_KEY).
pub fn build_opportunity_book(rng: &mut StdRng, accounts: &[AccountRow]) -> Vec<OppRow> {
    let mut opps = Vec::with_capacity(OPP_BOOK.len() + 1);

    // Beat 6: Ridgeline win-back — qualified, no quote (D. drafts P3-1 live),
    // near-term close.
    let ridgeline = book_account(accounts, RIDGELINE_ACCOUNT);
    opps.push(OppRow {
        id: det_uuid(rng),
        account_id: ridgeline.id,
        territory_id: ridgeline.territory_id,
        owner_id: ridgeline.rep_id,
        stage: OppStage::Qualified,
        kind: RIDGELINE_WINBACK_KIND,
        amount_cents: RIDGELINE_WINBACK_AMOUNT_CENTS,
        expected_close: Some(add_days(
            NaiveDate::from_ymd_opt(2026, 9, 1).expect("valid date"),
            pick_i64(rng, 0, 30),
        )),
        lost_reason: None,
    });

    for (name, stage, kind, amount_cents) in OPP_BOOK {
        let account = book_account(accounts, name);
        opps.push(OppRow {
            id: det_uuid(rng),
            account_id: account.id,
            territory_id: account.territory_id,
            owner_id: account.rep_id,
            stage,
            kind,
            amount_cents,
            expected_close: Some(add_days(
                NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date"),
                pick_i64(rng, 0, 150),
            )),
            lost_reason: None,
        });
    }

    opps
}

// ── Beat 5: the deliberate mess ─────────────────────────────────────────────

/// Duplicate-ish account names, documented here, defined in accounts.rs:
///   "Keystone Coatings"  vs "Keystone Coating Co."  (both NE-1)
///   "Vantage Metalworks" vs "Vantage Metal Works"   (both W-1)
/// A future data-quality panel finds them — mess is information.
pub const DUPLICATE_NAME_PAIRS: [(&str, &str); 2] = [
    ("Keystone Coatings", "Keystone Coating Co."),
    ("Vantage Metalworks", "Vantage Metal Works"),
];

/// Exactly two units lose expected_changeout_months (first cartridge-bearing
/// unit of each account below). Cadence math cannot run for them — that is
/// the point.
pub const NULL_ECM_ACCOUNTS: [&str; 2] = ["Harbor Steel Works", "Gulf Coast Chemical"];

/// Exactly one order line at 100% discount (net 0): a comped inspection on
/// the first noise order for this account in this month.
pub const COMPED_LINE_ACCOUNT: &str = "Golden Plains Milling";
pub const COMPED_LINE_YEAR_MONTH: (i32, u32) = (2025, 3);
pub const COMPED_LINE_SKU: &str = "SVC-INSPECT";
pub const COMPED_LINE_BP: i64 = 10_000; // 100.00%
