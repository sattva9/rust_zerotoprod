use askama_axum::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_flash::IncomingFlashes;
use uuid::Uuid;

use crate::utils::read_flash_messages;

#[derive(Template)]
#[template(path = "admin/newsletter.html")]
struct NewsletterIssue {
    msg: String,
    idempotency_key: Uuid,
}

pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashes,
) -> Result<Response, StatusCode> {
    let msg = read_flash_messages(&flash_messages);
    let idempotency_key = Uuid::new_v4();

    Ok((
        flash_messages,
        NewsletterIssue {
            msg,
            idempotency_key,
        },
    )
        .into_response())
}
