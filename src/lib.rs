use sqlx::PgPool;

pub mod configuration;
pub mod routes;
pub mod startup;

#[derive(Clone)]
pub struct AppState {
    pub pg_connection_pool: PgPool,
}