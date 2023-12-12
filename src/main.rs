use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zerotoprod::configuration::get_configuration;
use zerotoprod::startup::run;
use zerotoprod::telemetry::{get_tracing_subscriber, init_subscriber};
use zerotoprod::AppState;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    let subscriber = get_tracing_subscriber(
        "zerotoprod".into(),
        "info".into(),
        std::io::stdout,
        &configuration.telemetry,
    );
    init_subscriber(subscriber);

    let pg_connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let app_state = AppState { pg_connection_pool };

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind port");
    run(listener, app_state)?
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
}
