use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use axum_flash::IncomingFlashes;

use crate::utils::read_flash_messages;

pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashes,
) -> Result<Response, StatusCode> {
    let msg_html = read_flash_messages(&flash_messages);

    let body = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Publish Newsletter Issue</title>
        </head>
        <body>
        {msg_html}
        <form action="/admin/newsletters" method="post">
            <label>Title:<br>
                <input
                    type="text"
                    placeholder="Enter the issue title"
                    name="title"
                >
            </label>
            <br>
            <label>Content:<br>
                <textarea
                    placeholder="Enter the content in plain text"
                    name="content"
                    rows="20"
                    cols="50"
                ></textarea>
            </label>
            <br>
            <button type="submit">Publish</button>
        </form>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
        </body>
        </html>
        "#,
    );
    Ok((flash_messages, Html(body)).into_response())
}
