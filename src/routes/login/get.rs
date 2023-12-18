use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::{AppState, HmacSecret};

#[derive(Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> anyhow::Result<String> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));
        let mut mac =
            Hmac::<sha3::Sha3_256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(state: State<AppState>, query: Option<Query<QueryParams>>) -> Response {
    let error_html = match query {
        None => "".into(),
        Some(query) => match query.0.verify(&state.hmac_secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", html_escape::encode_text(&error))
            }
            Err(e) => {
                tracing::warn!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to verify query parameters using the HMAC tag"
                );
                "".into()
            }
        },
    };
    let body = format!(
        r#"<!doctype html>
    <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <title>Login</title>
        </head>
        <body>
            {error_html}
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
    Html(body).into_response()
}
