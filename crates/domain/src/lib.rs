//! Shared domain types: the Postgres enum mappings and the money math.
//! Money is BIGINT cents everywhere; discount is carried internally in basis
//! points (i64) and converted to a 2-decimal NUMERIC only at the database edge.
//! No f64 anywhere near money.

pub mod enums;
pub mod money;

pub use enums::*;
pub use money::*;
