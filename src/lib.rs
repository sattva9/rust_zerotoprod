use axum::{Form, Router, routing::{get, post, IntoMakeService}};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::io;
use std::net::TcpListener;
use hyper::server::conn::AddrIncoming;
use serde::Deserialize;

pub fn run(listener: TcpListener) -> Result<hyper::Server<AddrIncoming, IntoMakeService<Router>>, io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));

    let server = hyper::Server::from_tcp(listener)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .serve(app.into_make_service());
    Ok(server)
}

async fn health_check() -> Response {
    (StatusCode::OK, "").into_response()
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

async fn subscribe(_data: Form<FormData>) -> Response {
    (StatusCode::OK, "").into_response()
}