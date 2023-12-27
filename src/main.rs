use std::fmt::{Debug, Display};
use tokio::task::JoinError;

use zerotoprod::configuration::get_configuration;
use zerotoprod::issue_delivery_worker::run_worker_until_stopped;
use zerotoprod::startup::Application;
use zerotoprod::telemetry::init_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    init_subscriber(
        "zerotoprod".into(),
        "info".into(),
        std::io::stdout,
        &configuration.telemetry,
    );

    let application = Application::build(configuration.clone()).await?;
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));

    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{task_name} has exited")
        }
        Ok(Err(e)) => {
            tracing::error!("{task_name} failed. {e}")
        }
        Err(e) => {
            tracing::error!("{task_name} task failed to complete. {e}")
        }
    }
}
