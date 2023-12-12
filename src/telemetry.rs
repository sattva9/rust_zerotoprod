use opentelemetry_otlp::WithExportConfig;
use tonic::metadata::MetadataMap;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::{self, MakeWriter};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::configuration::TelemetrySettings;

pub fn get_tracing_subscriber<Sink>(
    _name: String,
    env_filter: String,
    sink: Sink,
    settings: &TelemetrySettings,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let mut meta_data = MetadataMap::new();
    meta_data.insert(
        "x-honeycomb-team",
        settings
            .api_key
            .parse()
            .expect("Failed to parse honeycomb api key"),
    );
    let open_telemetry_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_metadata(meta_data)
                .with_endpoint(&settings.endpoint)
                .with_tls_config(Default::default()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("failed to get opentelemetry tracer");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(open_telemetry_tracer);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    // let formatting_layer = BunyanFormattingLayer::new(name, sink);
    let formatting_layer = fmt::layer().with_writer(sink);

    Registry::default()
        .with(env_filter)
        .with(formatting_layer)
        .with(telemetry_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
