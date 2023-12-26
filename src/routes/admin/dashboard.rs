use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Extension,
};
use axum_flash::Flash;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, utils::e500, AppState};

pub async fn admin_dashboard(
    state: State<AppState>,
    flash: Flash,
    user_id: Extension<UserId>,
) -> Result<Response, StatusCode> {
    let user_id = *user_id.0;
    let username = get_username(&state.pg_connection_pool, user_id)
        .await
        .map_err(e500)?;

    let body = format!(
        r#"<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {username}!</p>
        <p>Available actions:</p>
        <ol>
            <li><a href="/admin/password">Change password</a></li>
            <li><a href="/admin/newsletters">Send a newsletter</a></li>
            <li>
                <form name="logoutForm" action="/admin/logout" method="post">
                    <input type="submit" value="Logout">
                </form>
            </li>
        </ol>
    </body>
    </html>"#
    );
    Ok((flash, Html(body)).into_response())
}

#[tracing::instrument(name = "Get username", skip(pg_pool))]
pub async fn get_username(pg_pool: &PgPool, user_id: Uuid) -> anyhow::Result<String> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pg_pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
