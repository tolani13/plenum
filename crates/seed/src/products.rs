//! Product catalog: ~10 capital SKUs across real Camfil families, ~20
//! cartridge SKUs (including Stat-Safe premium and replacement-brand SKUs
//! whose filter_fits include competitor family codes), parts, service.
//! Competitor collector models exist as catalog rows (kind=capital, family =
//! competitor family code, list price 0, never ordered) so installed_units
//! can reference a product for competitor-source units and the P4 conquest
//! cross-reference has a family key to join on.

use domain::ProductKind;
use rand::rngs::StdRng;

use crate::data::ProductRow;
use crate::util::det_uuid;

/// The competitor brand placeholders. Single const — D. may rename pre-demo;
/// every competitor SKU, family code, and installed_unit.source derives from
/// these two strings.
pub const COMPETITOR_BRANDS: [&str; 2] = ["Brand-X", "Brand-Y"];

/// "Brand-X" -> "BRANDX" (family-code prefix).
pub fn brand_code(brand: &str) -> String {
    brand
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

/// Camfil capital family codes.
pub const FAM_GS3: &str = "GS3";
pub const FAM_XFLO: &str = "XFLO";
pub const FAM_GSXP: &str = "GSXP";
pub const FAM_CAMTAIN: &str = "CAMTAIN";
pub const FAM_HIVAC: &str = "HIVAC";
pub const FAM_QPP: &str = "QPP";
pub const FAM_MIST: &str = "MIST";

struct Capital(&'static str, &'static str, &'static str, i64, i32);
struct Cartridge(
    &'static str,
    &'static str,
    &'static str,
    i64,
    &'static [&'static str],
);
struct Simple(&'static str, &'static str, i64, ProductKind);

pub fn build_products(rng: &mut StdRng) -> Vec<ProductRow> {
    let mut rows = Vec::new();

    // ~10 capital SKUs (sku, name, family, list cents, cartridge slots).
    let capitals = [
        Capital("CAP-GS3-8", "Gold Series III GS8", FAM_GS3, 6_500_000, 8),
        Capital(
            "CAP-GS3-16",
            "Gold Series III GS16",
            FAM_GS3,
            11_800_000,
            16,
        ),
        Capital(
            "CAP-GS3-32",
            "Gold Series III GS32",
            FAM_GS3,
            21_500_000,
            32,
        ),
        Capital(
            "CAP-XFLO-12",
            "Gold Series X-Flo 12",
            FAM_XFLO,
            9_800_000,
            12,
        ),
        Capital(
            "CAP-XFLO-24",
            "Gold Series X-Flo 24",
            FAM_XFLO,
            17_200_000,
            24,
        ),
        Capital("CAP-GSXP-6", "GSX P 6", FAM_GSXP, 5_400_000, 6),
        Capital(
            "CAP-CAMTAIN-8",
            "Gold Series Camtain 8",
            FAM_CAMTAIN,
            14_500_000,
            8,
        ),
        Capital(
            "CAP-HIVAC-4",
            "Gold Series High Vacuum 4",
            FAM_HIVAC,
            7_800_000,
            4,
        ),
        Capital("CAP-QPP-4", "Quad Pulse Package 4", FAM_QPP, 4_200_000, 4),
        Capital(
            "CAP-MIST-2",
            "EM-O Mist Collector 2",
            FAM_MIST,
            2_800_000,
            2,
        ),
    ];
    for Capital(sku, name, family, price, slots) in capitals {
        rows.push(ProductRow {
            id: det_uuid(rng),
            sku: sku.to_string(),
            name: name.to_string(),
            family: family.to_string(),
            kind: ProductKind::Capital,
            list_price_cents: price,
            filter_fits: vec![],
            cartridge_slots: Some(slots),
        });
    }

    // Competitor collector placeholders (never sold by us; price 0).
    let bx = brand_code(COMPETITOR_BRANDS[0]);
    let by = brand_code(COMPETITOR_BRANDS[1]);
    let competitor_capitals = [
        (
            format!("CMP-{bx}-DF3"),
            format!("{} Downflow DF-3 (competitor)", COMPETITOR_BRANDS[0]),
            format!("{bx}-DF"),
        ),
        (
            format!("CMP-{bx}-CM2"),
            format!(
                "{} Cartridge Module CM-2 (competitor)",
                COMPETITOR_BRANDS[0]
            ),
            format!("{bx}-CM"),
        ),
        (
            format!("CMP-{by}-DF4"),
            format!("{} Downflow DF-4 (competitor)", COMPETITOR_BRANDS[1]),
            format!("{by}-DF"),
        ),
        (
            format!("CMP-{by}-VP6"),
            format!("{} Vertical Pulse VP-6 (competitor)", COMPETITOR_BRANDS[1]),
            format!("{by}-VP"),
        ),
    ];
    for (sku, name, family) in competitor_capitals {
        rows.push(ProductRow {
            id: det_uuid(rng),
            sku,
            name,
            family,
            kind: ProductKind::Capital,
            list_price_cents: 0,
            filter_fits: vec![],
            cartridge_slots: None,
        });
    }

    // ~20 cartridge SKUs (sku, name, family, list cents, fits).
    let bx_df = format!("{bx}-DF");
    let bx_cm = format!("{bx}-CM");
    let by_df = format!("{by}-DF");
    let by_vp = format!("{by}-VP");
    let cartridges = [
        Cartridge(
            "FLT-OMNI-STD",
            "OmniPleat Standard Cartridge",
            "Filters-OmniPleat",
            28_500,
            &[FAM_GS3, FAM_GSXP],
        ),
        Cartridge(
            "FLT-OMNI-HE",
            "OmniPleat High-Efficiency Cartridge",
            "Filters-OmniPleat",
            34_500,
            &[FAM_GS3, FAM_GSXP],
        ),
        Cartridge(
            "FLT-OMNI-MERV15",
            "OmniPleat MERV 15 Cartridge",
            "Filters-OmniPleat",
            39_500,
            &[FAM_GS3, FAM_XFLO],
        ),
        Cartridge(
            "FLT-OPTI-GS3",
            "OptiCone Cartridge (GS III)",
            "Filters-OptiCone",
            31_500,
            &[FAM_GS3],
        ),
        Cartridge(
            "FLT-OPTI-CAM",
            "OptiCone Cartridge (Camtain)",
            "Filters-OptiCone",
            44_500,
            &[FAM_CAMTAIN],
        ),
        Cartridge(
            "FLT-OPTI-HV",
            "OptiCone Cartridge (High Vacuum)",
            "Filters-OptiCone",
            36_500,
            &[FAM_HIVAC],
        ),
        Cartridge(
            "FLT-OPTI-PTFE",
            "OptiCone PTFE Cartridge",
            "Filters-OptiCone",
            52_500,
            &[FAM_GS3, FAM_CAMTAIN],
        ),
        Cartridge(
            "FLT-XFLO-STD",
            "X-Flo Cartridge Standard",
            "Filters-XFlo",
            29_500,
            &[FAM_XFLO],
        ),
        Cartridge(
            "FLT-XFLO-HE",
            "X-Flo Cartridge High-Efficiency",
            "Filters-XFlo",
            35_500,
            &[FAM_XFLO],
        ),
        Cartridge(
            "FLT-TENKAY-STD",
            "Tenkay Cartridge",
            "Filters-Tenkay",
            24_500,
            &[FAM_GS3, FAM_XFLO],
        ),
        Cartridge(
            "FLT-QPP-STD",
            "Quad Pulse Cartridge",
            "Filters-QuadPulse",
            19_500,
            &[FAM_QPP],
        ),
        Cartridge(
            "FLT-MIST-EL",
            "Mist Collector Element",
            "Filters-Mist",
            16_500,
            &[FAM_MIST],
        ),
        Cartridge(
            "FLT-STATSAFE-GS3",
            "Stat-Safe Static-Dissipating (GS III)",
            "Filters-StatSafe",
            48_500,
            &[FAM_GS3, FAM_GSXP],
        ),
        Cartridge(
            "FLT-STATSAFE-XFLO",
            "Stat-Safe Static-Dissipating (X-Flo)",
            "Filters-StatSafe",
            47_500,
            &[FAM_XFLO],
        ),
    ];
    for Cartridge(sku, name, family, price, fits) in cartridges {
        rows.push(ProductRow {
            id: det_uuid(rng),
            sku: sku.to_string(),
            name: name.to_string(),
            family: family.to_string(),
            kind: ProductKind::Consumable,
            list_price_cents: price,
            filter_fits: fits.iter().map(|f| f.to_string()).collect(),
            cartridge_slots: None,
        });
    }

    // Replacement-brand SKUs: our filters that fit competitor collectors —
    // the conquest weapon. filter_fits carries competitor family codes.
    let replacements = [
        (
            format!("FLT-RB-{bx}-DF"),
            format!("Replacement Cartridge ({} DF)", COMPETITOR_BRANDS[0]),
            26_500i64,
            bx_df.clone(),
        ),
        (
            format!("FLT-RB-{bx}-CM"),
            format!("Replacement Cartridge ({} CM)", COMPETITOR_BRANDS[0]),
            27_500,
            bx_cm.clone(),
        ),
        (
            format!("FLT-RB-{by}-DF"),
            format!("Replacement Cartridge ({} DF)", COMPETITOR_BRANDS[1]),
            25_500,
            by_df.clone(),
        ),
        (
            format!("FLT-RB-{by}-VP"),
            format!("Replacement Cartridge ({} VP)", COMPETITOR_BRANDS[1]),
            28_900,
            by_vp.clone(),
        ),
        (
            format!("FLT-RB-{bx}-DFHE"),
            format!("Replacement HE Cartridge ({} DF)", COMPETITOR_BRANDS[0]),
            33_500,
            bx_df.clone(),
        ),
        (
            format!("FLT-RB-{by}-DFHE"),
            format!("Replacement HE Cartridge ({} DF)", COMPETITOR_BRANDS[1]),
            34_500,
            by_df.clone(),
        ),
    ];
    for (sku, name, price, fits) in replacements {
        rows.push(ProductRow {
            id: det_uuid(rng),
            sku,
            name,
            family: "Filters-Replacement-Brand".to_string(),
            kind: ProductKind::Consumable,
            list_price_cents: price,
            filter_fits: vec![fits],
            cartridge_slots: None,
        });
    }

    // Parts and service.
    let simples = [
        Simple(
            "PRT-VALVE-KIT",
            "Diaphragm Valve Kit",
            18_500,
            ProductKind::Part,
        ),
        Simple(
            "PRT-GASKET-SET",
            "Door Gasket Set",
            6_500,
            ProductKind::Part,
        ),
        Simple(
            "PRT-PULSE-BOARD",
            "Pulse Controller Board",
            42_500,
            ProductKind::Part,
        ),
        Simple(
            "PRT-HOPPER-KIT",
            "Hopper Discharge Kit",
            31_500,
            ProductKind::Part,
        ),
        Simple(
            "PRT-CAGE-VENTURI",
            "Cage & Venturi Set",
            9_800,
            ProductKind::Part,
        ),
        Simple(
            "SVC-INSPECT",
            "Preventive Inspection Visit",
            65_000,
            ProductKind::Service,
        ),
        Simple(
            "SVC-CHANGEOUT",
            "Filter Change-Out Service",
            120_000,
            ProductKind::Service,
        ),
        Simple(
            "SVC-COMMISSION",
            "Commissioning & Startup",
            350_000,
            ProductKind::Service,
        ),
    ];
    for Simple(sku, name, price, kind) in simples {
        rows.push(ProductRow {
            id: det_uuid(rng),
            sku: sku.to_string(),
            name: name.to_string(),
            family: if kind == ProductKind::Service {
                "Service"
            } else {
                "Accessories"
            }
            .to_string(),
            kind,
            list_price_cents: price,
            filter_fits: vec![],
            cartridge_slots: None,
        });
    }

    rows
}

/// Index of a product by exact SKU. Panics on unknown SKU — a seed bug.
pub fn idx_by_sku(products: &[ProductRow], sku: &str) -> usize {
    products
        .iter()
        .position(|p| p.sku == sku)
        .unwrap_or_else(|| panic!("unknown SKU {sku}"))
}

/// Indexes of consumable cartridges that fit the given family code.
pub fn cartridges_fitting(products: &[ProductRow], family: &str) -> Vec<usize> {
    products
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.kind == ProductKind::Consumable && p.filter_fits.iter().any(|f| f == family)
        })
        .map(|(i, _)| i)
        .collect()
}
