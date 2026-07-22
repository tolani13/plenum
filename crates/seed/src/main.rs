//! PLENUM deterministic seed. Runs migrations (the only thing that does),
//! wipes, regenerates the identical world every run (StdRng seed 20260717),
//! and prints: per-entity counts (queried back from the database), the
//! ORDERS TOTAL gate line, and the login table.

mod accounts;
mod data;
mod insert;
mod orders;
mod people;
mod products;
mod story_beats;
mod units;
mod util;

use chrono::Datelike;
use data::World;
use domain::UserRole;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sqlx::postgres::PgPoolOptions;

/// Dev-only default matching docker-compose.yml; .env overrides.
const DEFAULT_ADMIN_URL: &str = "postgres://plenum_admin:plenum_dev_admin_pw@localhost:5434/plenum";

// ════════════ WORLD GENERATION (R12 seam marker) ═══════════════════════════
// Everything below this line is the SYNTHETIC-WORLD side of the seed: pure
// in-memory generation from a fixed PRNG. In a production conversion this
// whole function is REPLACED by an ERP-extract reader (same World struct out);
// the DB-load side (insert::write_all and the derivation hooks in run()) is
// the part that survives as the loader.
fn generate_world() -> World {
    let mut rng = StdRng::seed_from_u64(util::SEED);
    let password_hash = people::demo_password_hash(&mut rng);
    let territories = people::build_territories(&mut rng);
    let (users, assignments) = people::build_users(&mut rng, &territories);
    let product_rows = products::build_products(&mut rng);
    let (account_rows, sites, contacts) = accounts::build_accounts(&mut rng, &territories, &users);
    let mut units = units::build_units(&mut rng, &account_rows, &sites, &product_rows);
    let orders_out = orders::build_orders(
        &mut rng,
        &account_rows,
        &sites,
        &units,
        &product_rows,
        &users,
    );
    for (unit, last) in units.iter_mut().zip(orders_out.last_filter.iter()) {
        unit.last_filter_order_on = *last;
    }
    let (opp, quote, quote_lines) =
        story_beats::build_pending_quote(&mut rng, &account_rows, &users, &product_rows);

    // Beat sanity: the duplicate-ish name pairs must both exist in the book.
    for (name_a, name_b) in story_beats::DUPLICATE_NAME_PAIRS {
        for name in [name_a, name_b] {
            assert!(
                account_rows.iter().any(|a| a.name == name),
                "duplicate-name beat account missing: {name}"
            );
        }
    }

    // Beat sanity: Ridgeline's most recent cartridge order must sit in
    // August 2025 (~11 months before the seeded today). Deterministic — if
    // this passes once it passes every run.
    let ridgeline_last = units
        .iter()
        .filter(|u| {
            account_rows[sites[u.site_idx].account_idx].name == story_beats::RIDGELINE_ACCOUNT
        })
        .filter_map(|u| u.last_filter_order_on)
        .max()
        .expect("Ridgeline has cartridge history");
    assert_eq!(
        (ridgeline_last.year(), ridgeline_last.month()),
        (2025, 8),
        "Ridgeline silence beat broken"
    );

    // P3 opportunity book (R2). A SEPARATE StdRng — keyed by SEED xor'd with a
    // documented constant — created and drawn AFTER every existing draw, so the
    // master `rng` sequence (and every frozen order/unit anchor) is untouched.
    let mut opp_rng = StdRng::seed_from_u64(util::SEED ^ story_beats::OPP_BOOK_STREAM_KEY);
    let mut opportunities = vec![opp];
    opportunities.extend(story_beats::build_opportunity_book(
        &mut opp_rng,
        &account_rows,
    ));

    World {
        territories,
        users,
        assignments,
        accounts: account_rows,
        sites,
        contacts,
        products: product_rows,
        units,
        opportunities,
        quotes: vec![quote],
        quote_lines,
        orders: orders_out.orders,
        order_lines: orders_out.order_lines,
        password_hash,
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("\nSEED FAILED: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_ADMIN_URL.to_string());

    println!("PLENUM seed — deterministic engine (seed {})", util::SEED);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .map_err(|e| {
            format!(
                "cannot connect to the database.\n  \
                 Is the container up? Run: docker compose up -d\n  ({e})"
            )
        })?;

    print!("applying migrations... ");
    sqlx::migrate!("../../migrations").run(&pool).await?;
    println!("ok");

    print!("generating world in memory... ");
    let world = generate_world();
    println!("ok");

    // ════════════ DB LOAD (R12 seam marker) ════════════════════════════════
    // From here down is the LOADER side: write the World, then derive
    // (rollups, signals). This path is the future ERP-extract-loader seam —
    // production swaps the World's source, not this load-and-derive shape.
    print!("writing (truncate + regenerate, one transaction)... ");
    insert::write_all(&pool, &world).await?;
    println!("ok");

    // Post-load ANALYZE: the wipe+reload leaves planner statistics stale until
    // autovacuum's next pass (up to ~a minute) — long enough for the first
    // post-reset clicks to plan without the 0013 index. Derivation-path
    // maintenance in the same spirit as refresh_rollups(); reads and writes
    // no table data, so no anchor can move.
    print!("analyzing (planner statistics)... ");
    sqlx::query("ANALYZE").execute(&pool).await?;
    println!("ok");

    // Counts come from the database, not from memory — no proof, no run.
    println!("\nrow counts (queried back from the database):");
    let tables = [
        "territories",
        "users",
        "territory_assignments",
        "accounts",
        "sites",
        "contacts",
        "products",
        "installed_units",
        "opportunities",
        "quotes",
        "quote_lines",
        "orders",
        "order_lines",
        "signals",
        "activities",
        "audit_log",
    ];
    let mut orders_total: i64 = 0;
    for table in tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await?;
        if table == "orders" {
            orders_total = count;
        }
        println!("  {table:<24}{count:>8}");
    }

    println!("\nORDERS TOTAL: {orders_total} (gate: >15000)");
    if orders_total <= 15_000 {
        return Err(format!("orders gate FAILED: {orders_total} <= 15000").into());
    }

    // P3 opportunity-book anchors (R2): deterministic count, checksum, and
    // per-stage distribution — re-checked at audit (precondition 6).
    let (opp_total, opp_checksum): (i64, i64) = sqlx::query_as(
        "SELECT count(*), COALESCE(sum(hashtext(id::text))::bigint, 0) FROM opportunities",
    )
    .fetch_one(&pool)
    .await?;
    println!("\nOPPORTUNITIES TOTAL: {opp_total} · checksum {opp_checksum}");
    let stage_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT stage::text, count(*) FROM opportunities GROUP BY stage ORDER BY stage",
    )
    .fetch_all(&pool)
    .await?;
    for (stage, n) in &stage_rows {
        println!("  stage {stage:<14}{n:>4}");
    }

    // P1: populate the mv_* rollups through the same refresh_rollups()
    // function the API's admin endpoint calls (seed connects as
    // plenum_admin, so this is a plain REFRESH). Data generation above is
    // untouched — rollups are derived, so determinism is unaffected.
    println!("\nrollups refreshed via refresh_rollups():");
    let rollups: Vec<(String, i64)> =
        sqlx::query_as("SELECT matview, row_count FROM refresh_rollups()")
            .fetch_all(&pool)
            .await?;
    for (matview, row_count) in &rollups {
        println!("  {matview:<24}{row_count:>8} rows");
    }

    // P4: derive signals through the SAME generate_signals() function the
    // admin endpoint calls (seed runs it as plenum_admin; the endpoint runs
    // it as plenum_app under an admin session). Signals are DERIVED, never
    // seeded as rows — the generators must find the story in the tables.
    //
    // CLOCK-DRIFTING anchors: every count below depends on CURRENT_DATE
    // (due windows, silence boundaries, the anomaly recency window). Two runs
    // the SAME day print identical counts; different days differ by design.
    // The frozen anchors above (row counts, checksums, mv counts) never move.
    println!("\nsignals generated via generate_signals():");
    let generated: Vec<(String, i64, i64, i64)> =
        sqlx::query_as("SELECT signal_type, inserted, updated, expired FROM generate_signals()")
            .fetch_all(&pool)
            .await?;
    let mut signals_total: i64 = 0;
    for (signal_type, inserted, updated, expired) in &generated {
        println!(
            "  {signal_type:<20}{inserted:>6} inserted  {updated:>4} updated  {expired:>4} expired"
        );
        signals_total += inserted;
    }
    println!("SIGNALS TOTAL: {signals_total} (clock-drifting: same-day reruns identical; other days differ by design)");

    println!(
        "\nLOGINS — password for every user: {}",
        people::DEMO_PASSWORD
    );
    println!("{:<30}| {:<17}| territories", "email", "role");
    println!("{}", "-".repeat(72));
    for user in &world.users {
        let territory_col =
            people::territory_display(user, &world.users, &world.assignments, &world.territories);
        let marker = if user.name == "Serena Estes" {
            "   <- the SE-1 rep (P0-2 rep-side check)"
        } else if user.role == UserRole::Vp {
            "   <- the VP (P0-2 all-territories check)"
        } else {
            ""
        };
        println!(
            "{:<30}| {:<17}| {}{}",
            user.email,
            user.role.as_str(),
            territory_col,
            marker
        );
    }
    Ok(())
}
