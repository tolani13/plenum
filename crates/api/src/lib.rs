//! PLENUM API library. The binary in main.rs is a thin shell over this so
//! integration tests can build the exact same router in-process.

pub mod ai;
pub mod auth;
pub mod error;
pub mod rls;
pub mod routes;
pub mod state;

/// Dev-only default matching docker-compose.yml; .env overrides. The API
/// connects ONLY as plenum_app — RLS applies to every query it makes.
pub const DEFAULT_APP_URL: &str = "postgres://plenum_app:plenum_dev_app_pw@localhost:5434/plenum";
