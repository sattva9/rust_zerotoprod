use std::sync::Arc;

use axum::extract::FromRef;
use domain::ApplicationBaseUrl;
use email_client::EmailClient;
use secrecy::Secret;
use sqlx::PgPool;

pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod idempotency;
pub mod issue_delivery_worker;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod utils;

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

#[derive(Clone)]
pub struct AppState {
    pub pg_connection_pool: PgPool,
    pub email_client: Arc<EmailClient>,
    pub application_base_url: Arc<ApplicationBaseUrl>,
    pub hmac_secret: Arc<HmacSecret>,
    pub flash_config: axum_flash::Config,
}

impl FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> axum_flash::Config {
        state.flash_config.clone()
    }
}
