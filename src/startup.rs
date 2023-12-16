use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, publish_newsletter, subscribe};
use crate::AppState;
use axum::{
    routing::{get, post, IntoMakeService},
    Router,
};
use hyper::server::conn::AddrIncoming;
use hyper::{Body, Request, Response};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::request_id::MakeRequestUuid;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tower_http::ServiceBuilderExt;
use tracing::{field::Empty, info, Span};

type Server = hyper::Server<AddrIncoming, IntoMakeService<Router>>;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub fn build(configuration: Settings) -> Result<Self, io::Error> {
        let pg_connection_pool = get_connection_pool(&configuration.database);
        let email_client = Arc::new(
            EmailClient::new(configuration.email)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
        );
        let app_state = AppState {
            pg_connection_pool,
            email_client,
            application_base_url: Arc::new(configuration.application.base_url),
        };

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).expect("Failed to bind port");
        let port = listener.local_addr()?.port();
        let server = run(listener, app_state)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    }
}

pub fn run(listener: TcpListener, app_state: crate::AppState) -> Result<Server, io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
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
                        .on_response(|response: &Response<_>, latency: Duration, span: &Span| {
                            let latency = latency.as_millis();
                            span.record("http.status_code", response.status().as_u16());
                            span.record("latency_in_ms", latency);
                            info!("Processed request in {latency}ms");
                        })
                        .on_failure(DefaultOnFailure::new()),
                )
                .propagate_x_request_id(),
        );

    let server = hyper::Server::from_tcp(listener)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .serve(app.into_make_service());
    Ok(server)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
