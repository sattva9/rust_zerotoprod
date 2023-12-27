use askama_axum::Template;
use axum::response::{IntoResponse, Response};
use axum_flash::IncomingFlashes;

use crate::utils::read_flash_messages;

#[derive(Template)]
#[template(path = "subscribe.html")]
struct Subscribe {
    msg: String,
}

pub async fn subscribe_form(flash_messages: IncomingFlashes) -> Response {
    let msg = read_flash_messages(&flash_messages);
    (flash_messages, Subscribe { msg }).into_response()
}
