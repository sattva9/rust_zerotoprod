use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, health_check, home, login, login_form, publish_newsletter, subscribe,
};
use crate::{AppState, HmacSecret};

use axum::routing::post;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::request_id::MakeRequestUuid;
use tower_http::ServiceBuilderExt;

use axum::{body::Body, http::Request, response::Response, routing::get, serve::Serve, Router};
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tracing::{field::Empty, info, Span};

type Server = Serve<Router, Router>;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> anyhow::Result<Self> {
        let pg_connection_pool = get_connection_pool(&configuration.database);
        let email_client = Arc::new(EmailClient::new(configuration.email)?);
        let app_state = AppState {
            pg_connection_pool,
            email_client,
            application_base_url: Arc::new(configuration.application.base_url),
            hmac_secret: Arc::new(HmacSecret(configuration.application.hmac_secret)),
        };

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to bind port");
        let port = listener.local_addr()?.port();
        let server = run(listener, app_state)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        self.server.await.map_err(|e| anyhow::anyhow!(e))
    }
}

pub fn run(listener: TcpListener, app_state: crate::AppState) -> anyhow::Result<Server> {
    let service = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .expect("Failed to get `x-request-id` from headers")
                        .to_str()
                        .expect("Failed to parse `HeaderValue` to `str`");

                    tracing::info_span!(
                        "HTTP request",
                        request_id = %request_id,
                        http.method = %request.method(),
                        http.target = %request.uri().path(),
                        http.status_code = Empty,
                        latency_in_ms = Empty,
                    )
                })
                .on_request(DefaultOnRequest::new())
                .on_response(|response: &Response, latency: Duration, span: &Span| {
                    let latency = latency.as_millis();
                    span.record("http.status_code", response.status().as_u16());
                    span.record("latency_in_ms", latency);
                    info!("Processed request in {latency}ms");
                })
                .on_failure(DefaultOnFailure::new()),
        )
        .propagate_x_request_id();

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .route("/", get(home))
        .route("/login", get(login_form).post(login))
        .with_state(app_state)
        .layer(service);

    Ok(axum::serve(listener, app))
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
