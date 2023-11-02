use std::io;
use std::net::TcpListener;
use std::time::Duration;
use hyper::server::conn::AddrIncoming;
use axum::{Router, routing::{get, post, IntoMakeService}};
use hyper::{Body, Request, Response};
use tower::ServiceBuilder;
use tower_http::request_id::MakeRequestUuid;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, TraceLayer};
use tower_http::ServiceBuilderExt;
use tracing::{info, Span, field::Empty};
use crate::AppState;
use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, app_state: AppState) -> Result<hyper::Server<AddrIncoming, IntoMakeService<Router>>, io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid::default())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body> | {
                            let request_id = request.headers()
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
                .propagate_x_request_id()
        );

    let server = hyper::Server::from_tcp(listener)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .serve(app.into_make_service());
    Ok(server)
}