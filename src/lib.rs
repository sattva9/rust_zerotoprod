use sqlx::PgPool;

pub mod configuration;
pub mod domain;
pub mod routes;
pub mod startup;
pub mod telemetry;

#[derive(Clone)]
pub struct AppState {
    pub pg_connection_pool: PgPool,
}
