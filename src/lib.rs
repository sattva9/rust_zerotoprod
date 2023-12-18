use std::sync::Arc;

use domain::ApplicationBaseUrl;
use email_client::EmailClient;
use secrecy::Secret;
use sqlx::PgPool;

pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod startup;
pub mod telemetry;

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

#[derive(Clone)]
pub struct AppState {
    pub pg_connection_pool: PgPool,
    pub email_client: Arc<EmailClient>,
    pub application_base_url: Arc<ApplicationBaseUrl>,
    pub hmac_secret: Arc<HmacSecret>,
}
