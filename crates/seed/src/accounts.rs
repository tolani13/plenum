//! ~48 accounts (some corporate parents with 2–4 sites), industry-distributed
//! like the real book: metals-heavy, pharma high-value, grain seasonal.
//! Sites and contacts hang off each account. Rep ownership is pinned per
//! account (story control: the leakage rep owns the whales).

use chrono::NaiveDate;
use domain::AccountStatus;
use rand::rngs::StdRng;

use crate::data::{AccountRow, ContactRow, SiteRow, TerritoryRow, UserRow};
use crate::util::{day_in_month, det_uuid, pick, pick_i32, pick_i64};

pub struct AccountSpec {
    pub name: &'static str,
    pub industry: &'static str,
    pub territory: &'static str,
    pub status: AccountStatus,
    pub sites: usize,
    pub parent: Option<&'static str>,
    pub rep: &'static str,
}

const fn spec(
    name: &'static str,
    industry: &'static str,
    territory: &'static str,
    status: AccountStatus,
    sites: usize,
    parent: Option<&'static str>,
    rep: &'static str,
) -> AccountSpec {
    AccountSpec {
        name,
        industry,
        territory,
        status,
        sites,
        parent,
        rep,
    }
}

use AccountStatus::{AtRisk, Customer, Dormant, Prospect};

/// The book of business. Duplicate-ish names (Keystone pair, Vantage pair) and
/// the story-beat accounts (Ridgeline Grain, Alpenglow) are deliberate — see
/// story_beats.rs.
pub const ACCOUNT_SPECS: [AccountSpec; 48] = [
    // SE-1 — all owned by Serena Estes (the P0-2 isolation rep).
    spec(
        "Ridgeline Grain Cooperative",
        "grain",
        "SE-1",
        AtRisk,
        1,
        None,
        "Serena Estes",
    ),
    spec(
        "Savannah Metal Systems",
        "metals",
        "SE-1",
        Customer,
        1,
        None,
        "Serena Estes",
    ),
    spec(
        "Blue Ridge Fabrication",
        "metals",
        "SE-1",
        Customer,
        2,
        None,
        "Serena Estes",
    ),
    spec(
        "Coastal Chem Processing",
        "chemical",
        "SE-1",
        Customer,
        1,
        None,
        "Serena Estes",
    ),
    spec(
        "Palmetto Wood Products",
        "woodworking",
        "SE-1",
        Customer,
        1,
        None,
        "Serena Estes",
    ),
    spec(
        "Peachtree Packaging",
        "packaging",
        "SE-1",
        Prospect,
        1,
        None,
        "Serena Estes",
    ),
    // NE-1
    spec(
        "Keystone Coatings",
        "metals",
        "NE-1",
        Customer,
        1,
        None,
        "Nora Ellery",
    ),
    spec(
        "Keystone Coating Co.",
        "metals",
        "NE-1",
        Customer,
        1,
        None,
        "Nathan Eastman",
    ),
    spec(
        "Meridian Biologics",
        "pharma",
        "NE-1",
        Customer,
        2,
        None,
        "Nora Ellery",
    ),
    spec(
        "Harbor Steel Works",
        "metals",
        "NE-1",
        Customer,
        1,
        None,
        "Nora Ellery",
    ),
    spec(
        "Granite State Grinding",
        "metals",
        "NE-1",
        Customer,
        1,
        None,
        "Nathan Eastman",
    ),
    spec(
        "Liberty Auto Components",
        "automotive",
        "NE-1",
        Customer,
        3,
        None,
        "Nathan Eastman",
    ),
    spec(
        "Back Bay Brewing Supply",
        "food_bev",
        "NE-1",
        Prospect,
        1,
        None,
        "Nora Ellery",
    ),
    // MW-1
    spec(
        "Prairie Gold Feed & Seed",
        "grain",
        "MW-1",
        Customer,
        1,
        None,
        "Miles Webb",
    ),
    spec(
        "Harvest Ridge Co-op",
        "grain",
        "MW-1",
        Customer,
        1,
        None,
        "Mia Winters",
    ),
    spec(
        "Great Lakes Laser Works",
        "metals",
        "MW-1",
        Customer,
        2,
        None,
        "Miles Webb",
    ),
    spec(
        "Clearwater Pharma",
        "pharma",
        "MW-1",
        Customer,
        1,
        None,
        "Mia Winters",
    ),
    spec(
        "Motor City Stampings",
        "automotive",
        "MW-1",
        Customer,
        2,
        None,
        "Miles Webb",
    ),
    spec(
        "Heartland Mills",
        "grain",
        "MW-1",
        Customer,
        1,
        None,
        "Mia Winters",
    ),
    spec(
        "Superior Abrasives",
        "metals",
        "MW-1",
        Prospect,
        1,
        None,
        "Miles Webb",
    ),
    // SC-1 — all Sam Cole.
    spec(
        "Sunbelt Grain Partners",
        "grain",
        "SC-1",
        Customer,
        1,
        None,
        "Sam Cole",
    ),
    spec(
        "Golden Plains Milling",
        "grain",
        "SC-1",
        Customer,
        1,
        None,
        "Sam Cole",
    ),
    spec(
        "Lone Star Thermal Spray",
        "metals",
        "SC-1",
        Customer,
        1,
        None,
        "Sam Cole",
    ),
    spec(
        "Gulf Coast Chemical",
        "chemical",
        "SC-1",
        Customer,
        2,
        None,
        "Sam Cole",
    ),
    spec(
        "Armadillo Auto Parts",
        "automotive",
        "SC-1",
        Customer,
        1,
        None,
        "Sam Cole",
    ),
    spec(
        "Bayou Packaging Group",
        "packaging",
        "SC-1",
        Dormant,
        1,
        None,
        "Sam Cole",
    ),
    // MT-1 — all Mona Tate. Alpenglow is the conquest-prospect story beat.
    spec(
        "Alpenglow Pharmaceuticals",
        "pharma",
        "MT-1",
        Prospect,
        1,
        None,
        "Mona Tate",
    ),
    spec(
        "Big Sky Grain",
        "grain",
        "MT-1",
        Customer,
        1,
        None,
        "Mona Tate",
    ),
    spec(
        "Rocky Mountain Mining Supply",
        "mining",
        "MT-1",
        Customer,
        1,
        None,
        "Mona Tate",
    ),
    spec(
        "Summit Plasma Cutting",
        "metals",
        "MT-1",
        Customer,
        1,
        None,
        "Mona Tate",
    ),
    spec(
        "Wasatch Woodworks",
        "woodworking",
        "MT-1",
        Customer,
        1,
        None,
        "Mona Tate",
    ),
    // W-1 — Wes Turner (the leakage rep) owns the whales.
    spec(
        "Vantage Metalworks",
        "metals",
        "W-1",
        Customer,
        3,
        None,
        "Wes Turner",
    ),
    spec(
        "Vantage Metalworks Coastal",
        "metals",
        "W-1",
        Customer,
        1,
        Some("Vantage Metalworks"),
        "Wes Turner",
    ),
    spec(
        "Vantage Metal Works",
        "metals",
        "W-1",
        Customer,
        1,
        None,
        "Willa Reyes",
    ),
    spec(
        "Ironhold Alloys",
        "metals",
        "W-1",
        Customer,
        2,
        None,
        "Wes Turner",
    ),
    spec(
        "Pacific Crest Pharma",
        "pharma",
        "W-1",
        Customer,
        1,
        None,
        "Willa Reyes",
    ),
    spec(
        "Sierra Auto Systems",
        "automotive",
        "W-1",
        Customer,
        1,
        None,
        "Willa Reyes",
    ),
    spec(
        "Redwood Timber Products",
        "woodworking",
        "W-1",
        Dormant,
        1,
        None,
        "Willa Reyes",
    ),
    // CE-1 — Celine Roy + Dana Cross (Dana spans CE-1 and CW-1).
    spec(
        "Maple Leaf Metal Fab",
        "metals",
        "CE-1",
        Customer,
        2,
        None,
        "Celine Roy",
    ),
    spec(
        "St. Lawrence Chemicals",
        "chemical",
        "CE-1",
        Customer,
        1,
        None,
        "Celine Roy",
    ),
    spec(
        "Ontario Grain Exchange",
        "grain",
        "CE-1",
        Customer,
        1,
        None,
        "Celine Roy",
    ),
    spec(
        "Quebec Precision Machining",
        "metals",
        "CE-1",
        Customer,
        1,
        None,
        "Dana Cross",
    ),
    spec(
        "Trillium Packaging",
        "packaging",
        "CE-1",
        Prospect,
        1,
        None,
        "Dana Cross",
    ),
    // CW-1 — Cole Brandt + Dana Cross.
    spec(
        "Chinook Fabrication",
        "metals",
        "CW-1",
        Customer,
        1,
        None,
        "Cole Brandt",
    ),
    spec(
        "Foothills Feed & Grain",
        "grain",
        "CW-1",
        Customer,
        1,
        None,
        "Cole Brandt",
    ),
    spec(
        "Cascadia Mining Co.",
        "mining",
        "CW-1",
        Customer,
        2,
        None,
        "Cole Brandt",
    ),
    spec(
        "Pacific Rim Auto",
        "automotive",
        "CW-1",
        Customer,
        1,
        None,
        "Dana Cross",
    ),
    spec(
        "Glacier Wood Systems",
        "woodworking",
        "CW-1",
        Prospect,
        1,
        None,
        "Dana Cross",
    ),
];

fn city_pool(code: &str) -> &'static [(&'static str, &'static str)] {
    match code {
        "NE-1" => &[
            ("Boston", "MA"),
            ("Hartford", "CT"),
            ("Albany", "NY"),
            ("Providence", "RI"),
            ("Buffalo", "NY"),
            ("Manchester", "NH"),
        ],
        "SE-1" => &[
            ("Atlanta", "GA"),
            ("Charlotte", "NC"),
            ("Savannah", "GA"),
            ("Birmingham", "AL"),
            ("Greenville", "SC"),
            ("Chattanooga", "TN"),
        ],
        "MW-1" => &[
            ("Chicago", "IL"),
            ("Milwaukee", "WI"),
            ("Detroit", "MI"),
            ("Des Moines", "IA"),
            ("Minneapolis", "MN"),
            ("Columbus", "OH"),
        ],
        "SC-1" => &[
            ("Dallas", "TX"),
            ("Houston", "TX"),
            ("Oklahoma City", "OK"),
            ("Baton Rouge", "LA"),
            ("Little Rock", "AR"),
            ("San Antonio", "TX"),
        ],
        "MT-1" => &[
            ("Denver", "CO"),
            ("Boise", "ID"),
            ("Salt Lake City", "UT"),
            ("Billings", "MT"),
            ("Colorado Springs", "CO"),
        ],
        "W-1" => &[
            ("Los Angeles", "CA"),
            ("Oakland", "CA"),
            ("Portland", "OR"),
            ("Seattle", "WA"),
            ("Fresno", "CA"),
            ("Sacramento", "CA"),
        ],
        "CE-1" => &[
            ("Toronto", "ON"),
            ("Montreal", "QC"),
            ("Ottawa", "ON"),
            ("Hamilton", "ON"),
            ("Quebec City", "QC"),
        ],
        "CW-1" => &[
            ("Calgary", "AB"),
            ("Edmonton", "AB"),
            ("Vancouver", "BC"),
            ("Winnipeg", "MB"),
            ("Saskatoon", "SK"),
        ],
        _ => panic!("unknown territory code {code}"),
    }
}

const STREETS: [&str; 8] = [
    "Industrial Pkwy",
    "Commerce Dr",
    "Foundry Rd",
    "Enterprise Blvd",
    "Mill Creek Rd",
    "Fabrication Way",
    "Distribution Ave",
    "Prospect St",
];

const FIRST_NAMES: [&str; 16] = [
    "Jordan", "Casey", "Taylor", "Morgan", "Riley", "Avery", "Quinn", "Hayden", "Elena", "Marco",
    "Priit", "Sofia", "Derek", "Lena", "Omar", "Grace",
];

const LAST_NAMES: [&str; 16] = [
    "Hutchins",
    "Barlow",
    "Okafor",
    "Lindqvist",
    "Marsh",
    "Delgado",
    "Novak",
    "Iverson",
    "Chen",
    "Whitfield",
    "Sandoval",
    "Pruitt",
    "Kowalski",
    "Vance",
    "Herrera",
    "Bloom",
];

const CONTACT_ROLES: [(&str, &str); 4] = [
    ("buyer", "Purchasing Manager"),
    ("plant engineer", "Plant Engineer"),
    ("EHS", "EHS Coordinator"),
    ("maintenance", "Maintenance Supervisor"),
];

fn account_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn build_accounts(
    rng: &mut StdRng,
    territories: &[TerritoryRow],
    users: &[UserRow],
) -> (Vec<AccountRow>, Vec<SiteRow>, Vec<ContactRow>) {
    let mut accounts: Vec<AccountRow> = Vec::with_capacity(ACCOUNT_SPECS.len());
    let mut sites: Vec<SiteRow> = Vec::new();
    let mut contacts: Vec<ContactRow> = Vec::new();

    for account_spec in &ACCOUNT_SPECS {
        let territory = territories
            .iter()
            .find(|t| t.code == account_spec.territory)
            .expect("known territory");
        let rep = users
            .iter()
            .find(|u| u.name == account_spec.rep)
            .expect("known rep name");
        let parent_account_id = account_spec.parent.map(|p| {
            accounts
                .iter()
                .find(|a| a.name == p)
                .expect("parent listed before child")
                .id
        });
        let created_date = NaiveDate::from_ymd_opt(
            pick_i32(rng, 2011, 2023),
            pick_i32(rng, 1, 12) as u32,
            day_in_month(rng),
        )
        .expect("valid date");
        let account = AccountRow {
            id: det_uuid(rng),
            name: account_spec.name,
            industry: account_spec.industry,
            status: account_spec.status,
            territory_id: territory.id,
            parent_account_id,
            created: created_date
                .and_hms_opt(9, 0, 0)
                .expect("valid time")
                .and_utc(),
            rep_id: rep.id,
        };
        let account_idx = accounts.len();

        for _ in 0..account_spec.sites {
            let site_id = det_uuid(rng);
            let (city, state) = *pick(rng, city_pool(account_spec.territory));
            let address = format!("{} {}", pick_i64(rng, 100, 9800), pick(rng, &STREETS));

            let n_contacts = pick_i32(rng, 1, 3);
            let mut primary_contact_id = None;
            for c in 0..n_contacts {
                let first = *pick(rng, &FIRST_NAMES);
                let last = *pick(rng, &LAST_NAMES);
                let (role, title) = *pick(rng, &CONTACT_ROLES);
                let contact = ContactRow {
                    id: det_uuid(rng),
                    site_id,
                    name: format!("{first} {last}"),
                    title,
                    email: format!(
                        "{}.{}@{}.example.com",
                        first.to_lowercase(),
                        last.to_lowercase(),
                        account_slug(account_spec.name)
                    ),
                    phone: format!(
                        "+1-555-{:03}-{:04}",
                        pick_i64(rng, 100, 999),
                        pick_i64(rng, 0, 9999)
                    ),
                    role,
                };
                if c == 0 {
                    // First contact becomes the site's primary.
                    primary_contact_id = Some(contact.id);
                }
                contacts.push(contact);
            }
            sites.push(SiteRow {
                id: site_id,
                account_id: account.id,
                address,
                city,
                state,
                primary_contact_id,
                account_idx,
            });
        }
        accounts.push(account);
    }
    (accounts, sites, contacts)
}
