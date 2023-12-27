use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{utils::e500, AppState};

#[derive(Template)]
#[template(path = "admin/subscribers.html")]
struct Subscribers {
    subscribers: Vec<SubscriberMeta>,
}

struct SubscriberMeta {
    email: String,
    name: String,
    status: String,
    subscribed_at: DateTime<Utc>,
}

pub async fn subscribers_list(state: State<AppState>) -> Result<Response, StatusCode> {
    let subscribers = get_all_subscribers(&state.pg_connection_pool)
        .await
        .map_err(e500)?;
    Ok(Subscribers { subscribers }.into_response())
}

async fn get_all_subscribers(pg_pool: &PgPool) -> anyhow::Result<Vec<SubscriberMeta>> {
    let subscribers = sqlx::query_as_unchecked!(
        SubscriberMeta,
        r#"
        SELECT email, name, status, subscribed_at
        FROM subscriptions
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(subscribers)
}
