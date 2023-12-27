use askama_axum::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_flash::IncomingFlashes;

use crate::utils::read_flash_messages;

#[derive(Template)]
#[template(path = "admin/password.html")]
struct PasswordChange {
    msg: String,
}

pub async fn change_password_form(flash_messages: IncomingFlashes) -> Result<Response, StatusCode> {
    let msg = read_flash_messages(&flash_messages);

    Ok((flash_messages, PasswordChange { msg }).into_response())
}
