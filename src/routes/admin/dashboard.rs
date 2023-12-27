use anyhow::Context;
use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
};
use axum_flash::Flash;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, utils::e500, AppState};

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct AdminDashboard {
    username: String,
}

pub async fn admin_dashboard(
    state: State<AppState>,
    flash: Flash,
    user_id: Extension<UserId>,
) -> Result<Response, StatusCode> {
    let user_id = *user_id.0;
    let username = get_username(&state.pg_connection_pool, user_id)
        .await
        .map_err(e500)?;

    Ok((flash, AdminDashboard { username }).into_response())
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
