//! Territories, users, and territory assignments. 8 territories (one per
//! region), 12 reps + 3 regional managers + 1 VP + 1 admin. The SE-1 rep
//! (Serena Estes) carries only SE-1 — the clean isolation case for P0-2.
//! Dana Cross carries two territories (CE-1 + CW-1) — schema proof that a rep
//! can hold more than one.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use domain::{Region, UserRole};
use rand::rngs::StdRng;
use rand::Rng;

use crate::data::{AssignmentRow, TerritoryRow, UserRow};
use crate::util::det_uuid;

/// Every demo user shares this password (printed in the seed's login table).
pub const DEMO_PASSWORD: &str = "demo-plenum-2026";

/// The eight territory codes, one per region.
pub const TERRITORY_SPECS: [(&str, &str, Region, i64); 8] = [
    ("NE-1", "Northeast 1", Region::Northeast, 380_000_000),
    ("SE-1", "Southeast 1", Region::Southeast, 360_000_000),
    ("MW-1", "Midwest 1", Region::Midwest, 420_000_000),
    ("SC-1", "South Central 1", Region::SouthCentral, 340_000_000),
    ("MT-1", "Mountain 1", Region::Mountain, 260_000_000),
    ("W-1", "West 1", Region::West, 400_000_000),
    ("CE-1", "Canada East 1", Region::CanadaE, 240_000_000),
    ("CW-1", "Canada West 1", Region::CanadaW, 220_000_000),
];

/// (name, role, manager name, assigned territory codes)
/// Managers must appear before their reports so ids resolve in one pass.
/// RMs/VP/admin have no territory_assignments rows — their scope derives from
/// the manager chain (v_user_scope) or role.
const USER_SPECS: [(&str, UserRole, Option<&str>, &[&str]); 17] = [
    ("Valerie Price", UserRole::Vp, None, &[]),
    ("Priya Nair", UserRole::Admin, None, &[]),
    (
        "Rachel Moore",
        UserRole::RegionalManager,
        Some("Valerie Price"),
        &[],
    ),
    (
        "Marcus Reed",
        UserRole::RegionalManager,
        Some("Valerie Price"),
        &[],
    ),
    (
        "Renee Vega",
        UserRole::RegionalManager,
        Some("Valerie Price"),
        &[],
    ),
    (
        "Nora Ellery",
        UserRole::Rep,
        Some("Rachel Moore"),
        &["NE-1"],
    ),
    (
        "Nathan Eastman",
        UserRole::Rep,
        Some("Rachel Moore"),
        &["NE-1"],
    ),
    // THE SE-1 rep — P0-2's isolation subject. Carries only SE-1.
    (
        "Serena Estes",
        UserRole::Rep,
        Some("Rachel Moore"),
        &["SE-1"],
    ),
    ("Sam Cole", UserRole::Rep, Some("Rachel Moore"), &["SC-1"]),
    ("Miles Webb", UserRole::Rep, Some("Marcus Reed"), &["MW-1"]),
    ("Mia Winters", UserRole::Rep, Some("Marcus Reed"), &["MW-1"]),
    ("Mona Tate", UserRole::Rep, Some("Marcus Reed"), &["MT-1"]),
    // Dual-territory rep — proves territory_assignments is many-to-many.
    (
        "Dana Cross",
        UserRole::Rep,
        Some("Marcus Reed"),
        &["CE-1", "CW-1"],
    ),
    ("Wes Turner", UserRole::Rep, Some("Renee Vega"), &["W-1"]),
    ("Willa Reyes", UserRole::Rep, Some("Renee Vega"), &["W-1"]),
    ("Celine Roy", UserRole::Rep, Some("Renee Vega"), &["CE-1"]),
    ("Cole Brandt", UserRole::Rep, Some("Renee Vega"), &["CW-1"]),
];

pub fn email_for(name: &str) -> String {
    format!(
        "{}@plenum.demo",
        name.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(".")
    )
}

/// Hash the shared demo password once (argon2id). The salt comes from the
/// seeded RNG so the stored hash is identical run to run — acceptable for a
/// demo credential that is printed to the console anyway.
pub fn demo_password_hash(rng: &mut StdRng) -> String {
    let salt_bytes: [u8; 16] = rng.gen();
    let salt = SaltString::encode_b64(&salt_bytes).expect("salt encodes");
    Argon2::default()
        .hash_password(DEMO_PASSWORD.as_bytes(), &salt)
        .expect("argon2id hash succeeds")
        .to_string()
}

pub fn build_territories(rng: &mut StdRng) -> Vec<TerritoryRow> {
    TERRITORY_SPECS
        .iter()
        .map(|(code, name, region, quota)| TerritoryRow {
            id: det_uuid(rng),
            code,
            name,
            region: *region,
            quota_year_cents: *quota,
        })
        .collect()
}

pub fn build_users(
    rng: &mut StdRng,
    territories: &[TerritoryRow],
) -> (Vec<UserRow>, Vec<AssignmentRow>) {
    let mut users: Vec<UserRow> = Vec::with_capacity(USER_SPECS.len());
    let mut assignments = Vec::new();

    for (name, role, manager, codes) in USER_SPECS {
        let manager_id = manager.map(|m| {
            users
                .iter()
                .find(|u| u.name == m)
                .expect("manager listed before report")
                .id
        });
        let user = UserRow {
            id: det_uuid(rng),
            email: email_for(name),
            name,
            role,
            manager_id,
        };
        for code in codes {
            let territory = territories
                .iter()
                .find(|t| t.code == *code)
                .expect("known territory code");
            assignments.push(AssignmentRow {
                user_id: user.id,
                territory_id: territory.id,
            });
        }
        users.push(user);
    }
    (users, assignments)
}

/// Territory display string for the login table: reps show their assigned
/// codes; regional managers show the union of their reports' codes; vp/admin
/// show ALL.
pub fn territory_display(
    user: &UserRow,
    users: &[UserRow],
    assignments: &[AssignmentRow],
    territories: &[TerritoryRow],
) -> String {
    let codes_for = |user_id| {
        let mut codes: Vec<&str> = assignments
            .iter()
            .filter(|a| a.user_id == user_id)
            .map(|a| {
                territories
                    .iter()
                    .find(|t| t.id == a.territory_id)
                    .expect("assignment references territory")
                    .code
            })
            .collect();
        codes.sort_unstable();
        codes
    };
    match user.role {
        UserRole::Vp | UserRole::Admin => format!("ALL ({})", territories.len()),
        UserRole::Rep => codes_for(user.id).join("+"),
        UserRole::RegionalManager => {
            let mut codes: Vec<&str> = users
                .iter()
                .filter(|u| u.manager_id == Some(user.id))
                .flat_map(|report| codes_for(report.id))
                .collect();
            codes.sort_unstable();
            codes.dedup();
            codes.join("+")
        }
    }
}
