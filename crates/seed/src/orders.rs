//! 3.5 years of orders ending 2026-07. Three generators:
//!   capital lumps  — rare, large, higher discount; one per in-window
//!                    commissioning of an ours-unit, dated just before it.
//!   cadence        — per cartridge-bearing unit: qty ≈ cartridge_count every
//!                    expected_changeout_months ± jitter; grain gets a Q3
//!                    quantity bump; Ridgeline goes silent after 2025-08.
//!   noise          — per-site parts/service orders every month; this is the
//!                    volume knob that lands the >15,000 gate.

use chrono::{Datelike, NaiveDate};
use domain::{net_unit_cents, AccountStatus, ProductKind};
use rand::rngs::StdRng;
use uuid::Uuid;

use crate::data::{AccountRow, OrderLineRow, OrderRow, ProductRow, SiteRow, UnitRow, UserRow};
use crate::products::idx_by_sku;
use crate::story_beats as beats;
use crate::units::SOURCE_OURS;
use crate::util::{
    add_days, add_months, chance, day_in_month, det_uuid, pick, pick_i32, pick_i64, seed_today,
    window_start,
};

pub struct OrdersOut {
    pub orders: Vec<OrderRow>,
    pub order_lines: Vec<OrderLineRow>,
    /// Per-unit (by index) latest cartridge order date — becomes
    /// installed_units.last_filter_order_on.
    pub last_filter: Vec<Option<NaiveDate>>,
}

/// Monthly parts/service order intensity per site, by industry.
fn noise_lambda(industry: &str) -> i32 {
    match industry {
        "metals" => 8,
        "grain" => 6,
        "automotive" => 6,
        "chemical" => 6,
        "pharma" => 5,
        "mining" => 5,
        "woodworking" => 5,
        "packaging" => 4,
        "food_bev" => 4,
        other => panic!("unknown industry {other}"),
    }
}

/// Grain noise multiplier for July–September (harvest season), percent.
const GRAIN_Q3_NOISE_PCT: i32 = 160;
/// Grain cadence quantity multiplier in Q3, percent.
const GRAIN_Q3_QTY_PCT: i32 = 120;

/// Family-realistic discount in basis points. Pharma pays premium (low
/// discounts); Stat-Safe is premium product; the leakage rep (beat 2) adds a
/// deep bonus discount to everything he touches.
fn line_bp(
    rng: &mut StdRng,
    kind: ProductKind,
    industry: &str,
    statsafe: bool,
    leakage_rep: bool,
) -> i64 {
    let mut bp = match kind {
        ProductKind::Capital => 800 + pick_i64(rng, 0, 1_000),
        ProductKind::Consumable => 200 + pick_i64(rng, 0, 800),
        ProductKind::Part => pick_i64(rng, 0, 600),
        ProductKind::Service => pick_i64(rng, 0, 400),
    };
    if industry == "pharma" {
        bp -= 300;
    }
    if statsafe {
        bp -= 200;
    }
    bp = bp.max(0);
    if leakage_rep {
        bp += if kind == ProductKind::Capital {
            beats::LEAKAGE_CAPITAL_BONUS_BP
        } else {
            beats::LEAKAGE_BONUS_BP
        };
        bp = bp.min(beats::LEAKAGE_CLAMP_BP);
    }
    bp
}

struct Emitter<'a> {
    products: &'a [ProductRow],
    orders: Vec<OrderRow>,
    lines: Vec<OrderLineRow>,
}

impl Emitter<'_> {
    fn order(
        &mut self,
        rng: &mut StdRng,
        account: &AccountRow,
        site_id: Uuid,
        date: NaiveDate,
    ) -> Uuid {
        let id = det_uuid(rng);
        self.orders.push(OrderRow {
            id,
            account_id: account.id,
            site_id,
            territory_id: account.territory_id,
            rep_id: account.rep_id,
            ordered_on: date,
        });
        id
    }

    fn line(&mut self, rng: &mut StdRng, order_id: Uuid, product_idx: usize, qty: i32, bp: i64) {
        let product = &self.products[product_idx];
        self.lines.push(OrderLineRow {
            id: det_uuid(rng),
            order_id,
            product_id: product.id,
            qty: qty.max(1),
            list_unit_cents: product.list_price_cents,
            net_unit_cents: net_unit_cents(product.list_price_cents, bp),
            discount_bp: bp,
        });
    }
}

fn bump_last(last: &mut [Option<NaiveDate>], unit_idx: usize, date: NaiveDate) {
    let newer = match last[unit_idx] {
        None => true,
        Some(existing) => existing < date,
    };
    if newer {
        last[unit_idx] = Some(date);
    }
}

pub fn build_orders(
    rng: &mut StdRng,
    accounts: &[AccountRow],
    sites: &[SiteRow],
    units: &[UnitRow],
    products: &[ProductRow],
    users: &[UserRow],
) -> OrdersOut {
    let wes_id = users
        .iter()
        .find(|u| u.name == beats::LEAKAGE_REP)
        .expect("leakage rep exists")
        .id;
    let today = seed_today();
    let start = window_start();

    // Dormant accounts stopped ordering somewhere mid-window (deterministic).
    let dormant_cutoffs: Vec<Option<NaiveDate>> = accounts
        .iter()
        .map(|a| {
            (a.status == AccountStatus::Dormant).then(|| add_months(start, pick_i32(rng, 14, 27)))
        })
        .collect();

    let mut em = Emitter {
        products,
        orders: Vec::new(),
        lines: Vec::new(),
    };
    let mut last_filter: Vec<Option<NaiveDate>> = vec![None; units.len()];

    let svc_commission = idx_by_sku(products, "SVC-COMMISSION");
    let parts_idxs: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == ProductKind::Part)
        .map(|(i, _)| i)
        .collect();
    let service_noise_idxs: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == ProductKind::Service && p.sku != "SVC-COMMISSION")
        .map(|(i, _)| i)
        .collect();

    // ── capital lumps ───────────────────────────────────────────────────────
    let capital_window_start = NaiveDate::from_ymd_opt(2023, 3, 1).expect("valid date");
    for unit in units {
        if unit.source != SOURCE_OURS || unit.commissioned_on < capital_window_start {
            continue;
        }
        let site = &sites[unit.site_idx];
        let account = &accounts[site.account_idx];
        let date = add_days(unit.commissioned_on, -pick_i64(rng, 30, 75));
        if date < start {
            continue;
        }
        let leakage = account.rep_id == wes_id;
        let capital_idx = products
            .iter()
            .position(|p| p.id == unit.product_id)
            .expect("unit product in catalog");
        let order_id = em.order(rng, account, site.id, date);
        let bp = line_bp(rng, ProductKind::Capital, account.industry, false, leakage);
        em.line(rng, order_id, capital_idx, 1, bp);
        let bp = line_bp(rng, ProductKind::Service, account.industry, false, leakage);
        em.line(rng, order_id, svc_commission, 1, bp);
        if chance(rng, 60) {
            let part_idx = *pick(rng, &parts_idxs);
            let bp = line_bp(rng, ProductKind::Part, account.industry, false, leakage);
            let qty = pick_i32(rng, 1, 2);
            em.line(rng, order_id, part_idx, qty, bp);
        }
    }

    // ── consumable cadence ──────────────────────────────────────────────────
    let ridgeline_cutoff = beats::ridgeline_silence_cutoff();
    for (unit_idx, unit) in units.iter().enumerate() {
        let Some(cart_idx) = unit.cartridge_idx else {
            continue;
        };
        let site = &sites[unit.site_idx];
        let account = &accounts[site.account_idx];
        if account.status == AccountStatus::Prospect {
            continue;
        }
        let leakage = account.rep_id == wes_id;
        let statsafe = products[cart_idx].sku.contains("STATSAFE");
        let is_grain = account.industry == "grain";

        match unit.expected_changeout_months {
            None => {
                // Beat 5: the two NULL-ECM units still order cartridges — on
                // an irregular, uninferable schedule. That is the data-quality
                // story: history exists, cadence math cannot run.
                for _ in 0..3 {
                    let date = NaiveDate::from_ymd_opt(
                        pick_i32(rng, 2023, 2025),
                        pick_i32(rng, 1, 12) as u32,
                        day_in_month(rng),
                    )
                    .expect("valid date");
                    let order_id = em.order(rng, account, site.id, date);
                    let bp = line_bp(
                        rng,
                        ProductKind::Consumable,
                        account.industry,
                        statsafe,
                        leakage,
                    );
                    em.line(rng, order_id, cart_idx, unit.cartridge_count, bp);
                    bump_last(&mut last_filter, unit_idx, date);
                }
            }
            Some(ecm) => {
                // Beat 1: Ridgeline's cadence stops dead after the cutoff.
                let stop = if account.name == beats::RIDGELINE_ACCOUNT {
                    ridgeline_cutoff
                } else if let Some(cut) = dormant_cutoffs[site.account_idx] {
                    cut
                } else {
                    today
                };
                let mut k: i32 = 1;
                loop {
                    let base = add_months(unit.commissioned_on, k * ecm);
                    if base > stop {
                        break;
                    }
                    k += 1;
                    let date = add_days(base, pick_i64(rng, -20, 20));
                    if date < start || date > stop {
                        continue;
                    }
                    if chance(rng, 8) {
                        continue; // missed cycle
                    }
                    let mut qty = (unit.cartridge_count + pick_i32(rng, -2, 2)).max(1);
                    if is_grain && (7..=9).contains(&date.month()) {
                        qty = ((qty * GRAIN_Q3_QTY_PCT) / 100).max(1);
                    }
                    let order_id = em.order(rng, account, site.id, date);
                    let bp = line_bp(
                        rng,
                        ProductKind::Consumable,
                        account.industry,
                        statsafe,
                        leakage,
                    );
                    em.line(rng, order_id, cart_idx, qty, bp);
                    bump_last(&mut last_filter, unit_idx, date);
                }
            }
        }
    }

    // ── Beat 1 (anchor): every Ridgeline cartridge unit places one final
    // order in August 2025 so last_filter_order_on lands there exactly.
    for (unit_idx, unit) in units.iter().enumerate() {
        let site = &sites[unit.site_idx];
        let account = &accounts[site.account_idx];
        if account.name != beats::RIDGELINE_ACCOUNT {
            continue;
        }
        let Some(cart_idx) = unit.cartridge_idx else {
            continue;
        };
        let statsafe = products[cart_idx].sku.contains("STATSAFE");
        let date = beats::ridgeline_final_order_date(rng);
        let order_id = em.order(rng, account, site.id, date);
        let bp = line_bp(
            rng,
            ProductKind::Consumable,
            account.industry,
            statsafe,
            false,
        );
        em.line(rng, order_id, cart_idx, unit.cartridge_count, bp);
        bump_last(&mut last_filter, unit_idx, date);
    }

    // ── parts/service noise ─────────────────────────────────────────────────
    let comped_inspect = idx_by_sku(products, beats::COMPED_LINE_SKU);
    let mut comped_done = false;
    for site in sites {
        let account = &accounts[site.account_idx];
        if account.status == AccountStatus::Prospect {
            continue;
        }
        let leakage = account.rep_id == wes_id;
        let whale = beats::WHALE_ACCOUNTS.contains(&account.name);
        let lambda_base =
            noise_lambda(account.industry) + if whale { beats::WHALE_NOISE_BONUS } else { 0 };
        for year in 2023..=2026 {
            for month in 1..=12u32 {
                if year == 2026 && month > 7 {
                    break;
                }
                let month_first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
                if let Some(cut) = dormant_cutoffs[site.account_idx] {
                    if month_first > cut {
                        continue; // dormant: went quiet at the cutoff
                    }
                }
                let mut lambda = lambda_base;
                if account.industry == "grain" && (7..=9).contains(&month) {
                    lambda = lambda * GRAIN_Q3_NOISE_PCT / 100;
                }
                let mut n = lambda + pick_i32(rng, -2, 2);
                if year == 2026 && month == 7 {
                    n /= 2; // half month — the window ends 2026-07-15
                }
                for _ in 0..n.max(0) {
                    let day = if year == 2026 && month == 7 {
                        pick_i32(rng, 1, 15) as u32
                    } else {
                        day_in_month(rng)
                    };
                    let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid date");
                    let order_id = em.order(rng, account, site.id, date);
                    for _ in 0..pick_i32(rng, 1, 2) {
                        let (product_idx, qty) = if chance(rng, 70) {
                            (*pick(rng, &parts_idxs), pick_i32(rng, 1, 4))
                        } else {
                            (*pick(rng, &service_noise_idxs), 1)
                        };
                        let bp = line_bp(
                            rng,
                            products[product_idx].kind,
                            account.industry,
                            false,
                            leakage,
                        );
                        em.line(rng, order_id, product_idx, qty, bp);
                    }
                    // Beat 5: exactly one order line at 100% discount — a
                    // comped inspection, net 0, passes the price CHECK.
                    if !comped_done
                        && account.name == beats::COMPED_LINE_ACCOUNT
                        && (year, month) == beats::COMPED_LINE_YEAR_MONTH
                    {
                        em.line(rng, order_id, comped_inspect, 1, beats::COMPED_LINE_BP);
                        comped_done = true;
                    }
                }
            }
        }
    }
    assert!(comped_done, "the 100%-discount comped line must be seeded");

    OrdersOut {
        orders: em.orders,
        order_lines: em.lines,
        last_filter,
    }
}
