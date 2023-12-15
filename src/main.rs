use zerotoprod::configuration::get_configuration;
use zerotoprod::startup::Application;
use zerotoprod::telemetry::init_subscriber;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    init_subscriber(
        "zerotoprod".into(),
        "info".into(),
        std::io::stdout,
        &configuration.telemetry,
    );

    let application = Application::build(configuration)?;
    application.run_until_stopped().await?;
    Ok(())
}
