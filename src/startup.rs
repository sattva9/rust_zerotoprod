use std::io;
use std::net::TcpListener;
use hyper::server::conn::AddrIncoming;
use axum::{Router, routing::{get, post, IntoMakeService}};
use crate::AppState;
use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, app_state: AppState) -> Result<hyper::Server<AddrIncoming, IntoMakeService<Router>>, io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(app_state);

    let server = hyper::Server::from_tcp(listener)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .serve(app.into_make_service());
    Ok(server)
}