use anyhow::Context;
use axum::http::StatusCode;
use axum_flash::IncomingFlashes;
use std::fmt::Write;

pub fn e500<T>(e: T) -> StatusCode
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    tracing::error!("Internal Server error: {e:?}");
    StatusCode::INTERNAL_SERVER_ERROR
}

pub fn e400<T>(e: T) -> StatusCode
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    tracing::error!("Bad Request error: {e:?}");
    StatusCode::BAD_REQUEST
}

pub fn read_flash_messages(flash_messages: &IncomingFlashes) -> String {
    let mut msg_html = String::new();
    for (_, m) in flash_messages.iter() {
        let _ = writeln!(msg_html, "{}", m).with_context(|| "Failed to write flash messages");
    }
    msg_html
}
