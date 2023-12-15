use once_cell::sync::Lazy;
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zerotoprod::configuration::{get_configuration, DatabaseSettings};
use zerotoprod::startup::{get_connection_pool, Application};
use zerotoprod::telemetry::init_subscriber;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    let configuration = get_configuration().expect("Failed to read configuration.");

    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout,
            &configuration.telemetry,
        );
    } else {
        init_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink,
            &configuration.telemetry,
        );
    };
});

pub struct TestApp {
    pub port: u16,
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> reqwest::Url {
        let body = serde_json::from_slice::<Value>(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        get_link(body["htmlContent"].as_str().unwrap())
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email.endpoint = email_server.uri();
        c
    };
    configure_database(&configuration.database).await;

    let application =
        Application::build(configuration.clone()).expect("Failed to build application.");
    let application_port = application.port();
    let address = format!("http://localhost:{}", application_port);
    tokio::spawn(application.run_until_stopped());

    TestApp {
        port: application_port,
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate teh database.");

    connection_pool
}
