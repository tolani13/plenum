//! ~220 installed units, commissioned 2010–2026, ~15% competitor-source.
//! Ours-units always know their cartridge; competitor units are ~40%
//! "converted" (we already sell them replacement cartridges) and otherwise
//! pure conquest targets with no cartridge relationship.

use chrono::NaiveDate;
use domain::{AccountStatus, ProductKind};
use rand::rngs::StdRng;

use crate::data::{AccountRow, ProductRow, SiteRow, UnitRow};
use crate::products::{brand_code, cartridges_fitting, COMPETITOR_BRANDS};
use crate::story_beats;
use crate::util::{chance, day_in_month, det_uuid, pick, pick_i32};

pub const SOURCE_OURS: &str = "ours";

fn industry_families(industry: &str) -> &'static [&'static str] {
    match industry {
        "metals" => &["GS3", "GS3", "XFLO", "HIVAC", "MIST"],
        "pharma" => &["CAMTAIN", "GS3"],
        "grain" => &["GS3", "GSXP", "QPP"],
        "automotive" => &["XFLO", "GS3", "MIST"],
        "chemical" => &["XFLO", "CAMTAIN"],
        "mining" => &["HIVAC", "GS3"],
        "woodworking" => &["GSXP", "QPP"],
        "packaging" => &["QPP", "GSXP"],
        "food_bev" => &["GSXP"],
        other => panic!("unknown industry {other}"),
    }
}

fn industry_unit_range(industry: &str, whale: bool) -> (i32, i32) {
    if whale {
        return (5, 7);
    }
    match industry {
        "metals" => (4, 6),
        "grain" => (3, 5),
        "pharma" | "automotive" => (3, 4),
        "chemical" => (2, 4),
        "mining" => (2, 3),
        "woodworking" => (2, 3),
        _ => (1, 2),
    }
}

fn industry_ecm_range(industry: &str) -> (i32, i32) {
    match industry {
        "metals" => (4, 6),
        "grain" => (5, 7),
        "pharma" => (6, 9),
        _ => (5, 8),
    }
}

/// Commissioning year distribution: a mature installed base with a live
/// recent-sales tail (2023+ commissions become in-window capital orders).
fn commission_date(rng: &mut StdRng) -> NaiveDate {
    let r = pick_i32(rng, 0, 99);
    let year = if r < 20 {
        pick_i32(rng, 2010, 2014)
    } else if r < 55 {
        pick_i32(rng, 2015, 2019)
    } else if r < 75 {
        pick_i32(rng, 2020, 2022)
    } else {
        pick_i32(rng, 2023, 2026)
    };
    let month = if year == 2026 {
        pick_i32(rng, 1, 5)
    } else {
        pick_i32(rng, 1, 12)
    };
    NaiveDate::from_ymd_opt(year, month as u32, day_in_month(rng)).expect("valid date")
}

pub fn build_units(
    rng: &mut StdRng,
    accounts: &[AccountRow],
    sites: &[SiteRow],
    products: &[ProductRow],
) -> Vec<UnitRow> {
    let mut units: Vec<UnitRow> = Vec::new();
    let mut serial_seq: usize = 0;

    for (site_idx, site) in sites.iter().enumerate() {
        let account = &accounts[site.account_idx];

        if account.status == AccountStatus::Prospect {
            // Prospects own no units we know about — except the Alpenglow
            // conquest beat, which is exactly three competitor units.
            if account.name == story_beats::ALPENGLOW_ACCOUNT {
                units.extend(story_beats::alpenglow_units(
                    rng,
                    site.id,
                    site_idx,
                    products,
                    &mut serial_seq,
                ));
            }
            continue;
        }

        let whale = story_beats::WHALE_ACCOUNTS.contains(&account.name);
        let (lo, hi) = industry_unit_range(account.industry, whale);
        let n_units = pick_i32(rng, lo, hi);
        let (ecm_lo, ecm_hi) = industry_ecm_range(account.industry);

        for _ in 0..n_units {
            let competitor = chance(rng, 15);
            if competitor {
                let competitor_capitals: Vec<usize> = products
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.kind == ProductKind::Capital && p.cartridge_slots.is_none())
                    .map(|(i, _)| i)
                    .collect();
                let pi = *pick(rng, &competitor_capitals);
                let product = &products[pi];
                let brand = COMPETITOR_BRANDS
                    .iter()
                    .find(|b| product.family.starts_with(&brand_code(b)))
                    .expect("competitor family maps to a brand");
                let converted = chance(rng, 40);
                let cartridge_idx = if converted {
                    let fitting = cartridges_fitting(products, &product.family);
                    if fitting.is_empty() {
                        None
                    } else {
                        Some(*pick(rng, &fitting))
                    }
                } else {
                    None
                };
                serial_seq += 1;
                units.push(UnitRow {
                    id: det_uuid(rng),
                    site_id: site.id,
                    product_id: product.id,
                    serial: format!("SN-{}-{:05}", product.family, serial_seq),
                    commissioned_on: commission_date(rng),
                    source: (*brand).to_string(),
                    cartridge_count: pick_i32(rng, 6, 16),
                    cartridge_product_id: cartridge_idx.map(|i| products[i].id),
                    expected_changeout_months: Some(pick_i32(rng, ecm_lo, ecm_hi)),
                    last_filter_order_on: None, // backfilled from real orders
                    site_idx,
                    cartridge_idx,
                });
            } else {
                let family = *pick(rng, industry_families(account.industry));
                let capitals: Vec<usize> = products
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        p.kind == ProductKind::Capital
                            && p.family == family
                            && p.cartridge_slots.is_some()
                    })
                    .map(|(i, _)| i)
                    .collect();
                let pi = *pick(rng, &capitals);
                let product = &products[pi];
                let fitting = cartridges_fitting(products, family);
                assert!(!fitting.is_empty(), "every Camfil family has a cartridge");
                // Pharma leans premium OptiCone/PTFE; grain leans Stat-Safe
                // (static-dissipating — combustible dust country).
                let preferred: Vec<usize> = match account.industry {
                    "pharma" if chance(rng, 70) => fitting
                        .iter()
                        .copied()
                        .filter(|&i| products[i].sku.contains("OPTI"))
                        .collect(),
                    "grain" if chance(rng, 40) => fitting
                        .iter()
                        .copied()
                        .filter(|&i| products[i].sku.contains("STATSAFE"))
                        .collect(),
                    _ => vec![],
                };
                let pool = if preferred.is_empty() {
                    &fitting
                } else {
                    &preferred
                };
                let cartridge_idx = Some(*pick(rng, pool));
                serial_seq += 1;
                units.push(UnitRow {
                    id: det_uuid(rng),
                    site_id: site.id,
                    product_id: product.id,
                    serial: format!("SN-{}-{:05}", family, serial_seq),
                    commissioned_on: commission_date(rng),
                    source: SOURCE_OURS.to_string(),
                    cartridge_count: product.cartridge_slots.expect("capital SKU has slots"),
                    cartridge_product_id: cartridge_idx.map(|i| products[i].id),
                    expected_changeout_months: Some(pick_i32(rng, ecm_lo, ecm_hi)),
                    last_filter_order_on: None, // backfilled from real orders
                    site_idx,
                    cartridge_idx,
                });
            }
        }
    }

    // Beat 5: exactly two units lose expected_changeout_months — the first
    // cartridge-bearing unit of each NULL_ECM account.
    for account_name in story_beats::NULL_ECM_ACCOUNTS {
        let unit = units
            .iter_mut()
            .find(|u| {
                accounts[sites[u.site_idx].account_idx].name == account_name
                    && u.cartridge_idx.is_some()
            })
            .expect("NULL_ECM account has a cartridge-bearing unit");
        unit.expected_changeout_months = None;
    }

    units
}
