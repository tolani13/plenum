//! PLENUM API binary. Connects ONLY as plenum_app (RLS applies), fails fast
//! with a plain-language message when the database is absent or unseeded,
//! serves on BIND_ADDR (default 127.0.0.1:5777), shuts down gracefully.

use api::state::AppState;
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

    // Fail fast, in plain language, if the schema is missing or empty.
    match sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
    {
        Ok(0) => return Err("database is empty — run: cargo run --bin seed".into()),
        Ok(_) => {}
        Err(_) => {
            return Err(
                "database not seeded — run: docker compose up -d  then  cargo run --bin seed"
                    .into(),
            )
        }
    }

    let app = routes::app(AppState { pool }, cookie_secure);
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
