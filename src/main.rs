use std::net::TcpListener;
use sqlx::PgPool;
use zerotoprod::AppState;
use zerotoprod::configuration::get_configuration;
use zerotoprod::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    let pg_connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let app_state = AppState { pg_connection_pool };

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind port");
    run(listener, app_state)?.await.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
}