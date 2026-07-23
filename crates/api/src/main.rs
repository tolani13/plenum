//! PLENUM API binary. Connects ONLY as plenum_app (RLS applies), fails fast
//! with a plain-language message when the database is absent or unseeded,
//! serves on BIND_ADDR (default 127.0.0.1:5777), shuts down gracefully.

use api::state::{AiConfig, AppState};
use api::{routes, DEFAULT_APP_URL};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("\nAPI FAILED TO START: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app_url = std::env::var("APP_DATABASE_URL").unwrap_or_else(|_| DEFAULT_APP_URL.to_string());
    let cookie_secure = match std::env::var("COOKIE_SECURE") {
        // Default true: production posture. Localhost dev sets false in .env.
        Err(_) => true,
        Ok(v) => v
            .trim()
            .parse::<bool>()
            .map_err(|_| "COOKIE_SECURE must be true or false")?,
    };
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:5777".to_string());
    // P4: the AI env keys, read once. The key value never leaves AppState;
    // only its PRESENCE is logged.
    let ai = AiConfig::from_env()?;
    tracing::info!(
        ask_enabled = ai.ask_enabled(),
        discount_enabled = ai.discount_enabled(),
        model = %ai.model,
        key_present = ai.api_key.is_some(),
        "AI configuration loaded"
    );

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&app_url)
        .await
        .map_err(|e| {
            format!(
                "cannot connect to the database.\n  \
                 Is the container up? Run: docker compose up -d\n  ({e})"
            )
        })?;

    // Deploy unit (env-gated; dev default = false, byte-identical behavior).
    // On Render there is one managed database user and no initdb hook, so the
    // API applies the embedded migrations itself on boot — idempotent, safe
    // on every redeploy. Two consequences handled here and only here:
    //   · the migrations GRANT to the `plenum_app` role (0007+); dev's docker
    //     initdb creates it, a managed database does not — ensure it exists
    //     first (NOLOGIN: nothing connects as it in prod, it only has to be
    //     grantable);
    //   · a freshly-migrated database has a schema but no world yet — that is
    //     the DESIGNED state between first deploy and the one-off seed job,
    //     so it must serve (health checks, login screen), not exit.
    let migrate_on_boot = matches!(
        std::env::var("MIGRATE_ON_BOOT"),
        Ok(v) if v.trim().eq_ignore_ascii_case("true")
    );
    if migrate_on_boot {
        sqlx::query(
            "DO $$ BEGIN \
               IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'plenum_app') THEN \
                 CREATE ROLE plenum_app NOLOGIN; \
               END IF; \
             END $$;",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("could not ensure the plenum_app role exists: {e}"))?;
        sqlx::migrate!("../../migrations").run(&pool).await?;
        tracing::info!("migrations current (MIGRATE_ON_BOOT=true)");
    }

    // Fail fast, in plain language, if the schema is missing or empty — the
    // dev posture. Under MIGRATE_ON_BOOT the empty world is expected until
    // the seed job runs, so the service warns and serves instead.
    match sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
    {
        Ok(0) if migrate_on_boot => tracing::warn!(
            "schema is current but the world is EMPTY — run the seed job \
             (the documented one-off) to load the demo world"
        ),
        Ok(0) => return Err("database is empty — run: cargo run --bin seed".into()),
        Ok(_) => {}
        Err(e) if migrate_on_boot => {
            return Err(format!(
                "migrations ran but the schema is unreadable — check DATABASE_URL \
                 permissions ({e})"
            )
            .into())
        }
        Err(_) => {
            return Err(
                "database not seeded — run: docker compose up -d  then  cargo run --bin seed"
                    .into(),
            )
        }
    }

    let app = routes::app(AppState { pool, ai }, cookie_secure);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| format!("cannot bind {bind_addr}: {e} — is another process on the port?"))?;

    tracing::info!("PLENUM API listening on {bind_addr}");
    println!("PLENUM API listening on {bind_addr} (ctrl-c to stop)");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    println!("PLENUM API stopped");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("shutdown signal received");
}
