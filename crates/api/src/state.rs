use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    /// The plenum_app pool. There is no admin pool anywhere in the API —
    /// the admin connection belongs to migrations and the seed binary only.
    pub pool: PgPool,
}
