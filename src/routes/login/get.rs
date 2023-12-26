use axum::response::{Html, IntoResponse, Response};
use axum_flash::IncomingFlashes;

use crate::utils::read_flash_messages;

pub async fn login_form(flash_messages: IncomingFlashes) -> Response {
    let msg_html = read_flash_messages(&flash_messages);

    let body = format!(
        r#"<!doctype html>
    <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <title>Login</title>
        </head>
        <body>
            {msg_html}
            <form action="/login" method="post">
                <label>
                    Username
                    <input
                        type="text"
                        placeholder="Enter Username"
                        name="username"
                    />
                </label>
                <label>
                    Password
                    <input
                        type="password"
                        placeholder="Enter Password"
                        name="password"
                    />
                </label>
                <button type="submit">Login</button>
            </form>
        </body>
    </html>
"#
    );
    (flash_messages, Html(body)).into_response()
}
