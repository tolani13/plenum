//! Database writer: wipe-first (TRUNCATE … RESTART IDENTITY CASCADE on all
//! seed-owned tables), then chunked multi-row inserts, all inside ONE
//! transaction on the admin connection.

use domain::discount_bp_to_pct;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::data::World;

/// Wipe order is irrelevant under CASCADE in a single statement; the list is
/// every seed-owned table. TRUNCATE does not fire row-level triggers, so the
/// wipe itself writes no audit rows.
const TRUNCATE_SQL: &str = "TRUNCATE TABLE \
    audit_log, activities, signals, order_lines, orders, quote_lines, quotes, \
    opportunities, installed_units, contacts, sites, accounts, products, \
    territory_assignments, territories, users \
    RESTART IDENTITY CASCADE";

const CHUNK: usize = 500;

pub async fn write_all(pool: &PgPool, w: &World) -> sqlx::Result<()> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    sqlx::query(TRUNCATE_SQL).execute(&mut *tx).await?;

    // territories
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO territories (id, code, name, region, quota_year_cents) ",
    );
    qb.push_values(&w.territories, |mut b, t| {
        b.push_bind(t.id)
            .push_bind(t.code)
            .push_bind(t.name)
            .push_bind(t.region)
            .push_bind(t.quota_year_cents);
    });
    qb.build().execute(&mut *tx).await?;

    // users (managers precede reports in the vec; FKs check at statement end)
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO users (id, email, password_hash, name, role, manager_id) ",
    );
    qb.push_values(&w.users, |mut b, u| {
        b.push_bind(u.id)
            .push_bind(&u.email)
            .push_bind(&w.password_hash)
            .push_bind(u.name)
            .push_bind(u.role)
            .push_bind(u.manager_id);
    });
    qb.build().execute(&mut *tx).await?;

    // territory_assignments
    let mut qb =
        QueryBuilder::<Postgres>::new("INSERT INTO territory_assignments (user_id, territory_id) ");
    qb.push_values(&w.assignments, |mut b, a| {
        b.push_bind(a.user_id).push_bind(a.territory_id);
    });
    qb.build().execute(&mut *tx).await?;

    // accounts (parents precede children in the vec)
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO accounts (id, name, industry, status, territory_id, parent_account_id, created) ",
    );
    qb.push_values(&w.accounts, |mut b, a| {
        b.push_bind(a.id)
            .push_bind(a.name)
            .push_bind(a.industry)
            .push_bind(a.status)
            .push_bind(a.territory_id)
            .push_bind(a.parent_account_id)
            .push_bind(a.created);
    });
    qb.build().execute(&mut *tx).await?;

    // sites (primary_contact_id set after contacts exist)
    let mut qb =
        QueryBuilder::<Postgres>::new("INSERT INTO sites (id, account_id, address, city, state) ");
    qb.push_values(&w.sites, |mut b, s| {
        b.push_bind(s.id)
            .push_bind(s.account_id)
            .push_bind(&s.address)
            .push_bind(s.city)
            .push_bind(s.state);
    });
    qb.build().execute(&mut *tx).await?;

    // contacts
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO contacts (id, site_id, name, title, email, phone, role) ",
    );
    qb.push_values(&w.contacts, |mut b, c| {
        b.push_bind(c.id)
            .push_bind(c.site_id)
            .push_bind(&c.name)
            .push_bind(c.title)
            .push_bind(&c.email)
            .push_bind(&c.phone)
            .push_bind(c.role);
    });
    qb.build().execute(&mut *tx).await?;

    // close the sites <-> contacts circle
    let pairs: Vec<_> = w
        .sites
        .iter()
        .filter_map(|s| s.primary_contact_id.map(|c| (s.id, c)))
        .collect();
    if !pairs.is_empty() {
        let mut qb = QueryBuilder::<Postgres>::new(
            "UPDATE sites SET primary_contact_id = v.contact_id FROM (",
        );
        qb.push_values(&pairs, |mut b, (site_id, contact_id)| {
            b.push_bind(site_id).push_bind(contact_id);
        });
        qb.push(") AS v(site_id, contact_id) WHERE sites.id = v.site_id");
        qb.build().execute(&mut *tx).await?;
    }

    // products
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO products (id, sku, name, family, kind, list_price_cents, filter_fits) ",
    );
    qb.push_values(&w.products, |mut b, p| {
        b.push_bind(p.id)
            .push_bind(&p.sku)
            .push_bind(&p.name)
            .push_bind(&p.family)
            .push_bind(p.kind)
            .push_bind(p.list_price_cents)
            .push_bind(&p.filter_fits);
    });
    qb.build().execute(&mut *tx).await?;

    // installed_units (filter_life_pct stays NULL — telemetry stretch)
    for chunk in w.units.chunks(CHUNK) {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO installed_units (id, site_id, product_id, serial, commissioned_on, \
             source, cartridge_count, cartridge_product_id, expected_changeout_months, \
             last_filter_order_on) ",
        );
        qb.push_values(chunk, |mut b, u| {
            b.push_bind(u.id)
                .push_bind(u.site_id)
                .push_bind(u.product_id)
                .push_bind(&u.serial)
                .push_bind(u.commissioned_on)
                .push_bind(&u.source)
                .push_bind(u.cartridge_count)
                .push_bind(u.cartridge_product_id)
                .push_bind(u.expected_changeout_months)
                .push_bind(u.last_filter_order_on);
        });
        qb.build().execute(&mut *tx).await?;
    }

    // opportunities (audit trigger fires; actor NULL on the admin path)
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO opportunities (id, account_id, territory_id, owner_id, stage, kind, \
         amount_cents, expected_close, lost_reason) ",
    );
    qb.push_values(&w.opportunities, |mut b, o| {
        b.push_bind(o.id)
            .push_bind(o.account_id)
            .push_bind(o.territory_id)
            .push_bind(o.owner_id)
            .push_bind(o.stage)
            .push_bind(o.kind)
            .push_bind(o.amount_cents)
            .push_bind(o.expected_close)
            .push_bind(o.lost_reason);
    });
    qb.build().execute(&mut *tx).await?;

    // quotes (audit trigger fires)
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO quotes (id, opportunity_id, status, approver_id, created_by, created_at) ",
    );
    qb.push_values(&w.quotes, |mut b, q| {
        b.push_bind(q.id)
            .push_bind(q.opportunity_id)
            .push_bind(q.status)
            .push_bind(q.approver_id)
            .push_bind(q.created_by)
            .push_bind(q.created_at);
    });
    qb.build().execute(&mut *tx).await?;

    // quote_lines (discount stored as NUMERIC(5,2); the CHECK re-verifies net)
    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO quote_lines (id, quote_id, product_id, qty, list_unit_cents, \
         net_unit_cents, discount_pct) ",
    );
    qb.push_values(&w.quote_lines, |mut b, l| {
        b.push_bind(l.id)
            .push_bind(l.quote_id)
            .push_bind(l.product_id)
            .push_bind(l.qty)
            .push_bind(l.list_unit_cents)
            .push_bind(l.net_unit_cents)
            .push_bind(discount_bp_to_pct(l.discount_bp));
    });
    qb.build().execute(&mut *tx).await?;

    // orders — chunked multi-row binds
    for chunk in w.orders.chunks(CHUNK) {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO orders (id, account_id, site_id, territory_id, rep_id, ordered_on) ",
        );
        qb.push_values(chunk, |mut b, o| {
            b.push_bind(o.id)
                .push_bind(o.account_id)
                .push_bind(o.site_id)
                .push_bind(o.territory_id)
                .push_bind(o.rep_id)
                .push_bind(o.ordered_on);
        });
        qb.build().execute(&mut *tx).await?;
    }

    // order_lines — chunked; every row passes the price-triplet CHECK
    for chunk in w.order_lines.chunks(CHUNK) {
        let mut qb = QueryBuilder::<Postgres>::new(
            "INSERT INTO order_lines (id, order_id, product_id, qty, list_unit_cents, \
             net_unit_cents, discount_pct) ",
        );
        qb.push_values(chunk, |mut b, l| {
            b.push_bind(l.id)
                .push_bind(l.order_id)
                .push_bind(l.product_id)
                .push_bind(l.qty)
                .push_bind(l.list_unit_cents)
                .push_bind(l.net_unit_cents)
                .push_bind(discount_bp_to_pct(l.discount_bp));
        });
        qb.build().execute(&mut *tx).await?;
    }

    tx.commit().await
}
