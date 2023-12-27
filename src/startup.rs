use crate::authentication::reject_anonymous_users;
use crate::configuration::{DatabaseSettings, RedisSettings, Settings};
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, home, log_out,
    login, login_form, publish_newsletter, publish_newsletter_form, subscribe,
};
use crate::{AppState, HmacSecret};

use axum::middleware;
use axum::routing::post;
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::request_id::MakeRequestUuid;
use tower_http::ServiceBuilderExt;
use tower_sessions::fred::clients::RedisClient;
use tower_sessions::fred::interfaces::ClientLike;
use tower_sessions::fred::types::RedisConfig;
use tower_sessions::RedisStore;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    routing::get,
    serve::Serve,
    Router,
};
use std::time::Duration;
use tokio::net::TcpListener;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tracing::{field::Empty, info, Span};

use axum::error_handling::HandleErrorLayer;
use tower_sessions::{Expiry, SessionManagerLayer};

type Server = Serve<Router, Router>;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> anyhow::Result<Self> {
        let pg_connection_pool = get_connection_pool(&configuration.database);
        let email_client = Arc::new(configuration.email.client()?);
        let flash_key = axum_flash::Key::from(
            configuration
                .application
                .hmac_secret
                .expose_secret()
                .as_bytes(),
        );
        let redis_client = redis_client(configuration.redis).await?;
        let app_state = AppState {
            pg_connection_pool,
            email_client,
            application_base_url: Arc::new(configuration.application.base_url),
            hmac_secret: Arc::new(HmacSecret(configuration.application.hmac_secret)),
            flash_config: axum_flash::Config::new(flash_key),
        };

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to bind port");
        let port = listener.local_addr()?.port();
        let server = run(listener, app_state, redis_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        self.server.await.map_err(|e| anyhow::anyhow!(e))
    }
}

pub fn run(
    listener: TcpListener,
    app_state: crate::AppState,
    redis_client: RedisClient,
) -> anyhow::Result<Server> {
    let session_store = RedisStore::new(redis_client);
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
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(
            SessionManagerLayer::new(session_store)
                .with_secure(true)
                .with_expiry(Expiry::OnInactivity(time::Duration::seconds(900))),
        )
        .propagate_x_request_id();

    let admin_routes = Router::new()
        .route("/dashboard", get(admin_dashboard))
        .route("/password", get(change_password_form).post(change_password))
        .route("/logout", post(log_out))
        .route(
            "/newsletters",
            get(publish_newsletter_form).post(publish_newsletter),
        )
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            reject_anonymous_users,
        ));

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/", get(home))
        .route("/login", get(login_form).post(login))
        .nest("/admin", admin_routes)
        .with_state(app_state)
        .layer(service);

    Ok(axum::serve(listener, app))
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

async fn redis_client(config: RedisSettings) -> anyhow::Result<RedisClient> {
    let redis_config =
        RedisConfig::from_url(config.uri.expose_secret()).map_err(|e| anyhow::anyhow!(e))?;
    let redis_client = RedisClient::new(redis_config, None, None, None);

    redis_client.connect();
    redis_client
        .wait_for_connect()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(redis_client)
}
