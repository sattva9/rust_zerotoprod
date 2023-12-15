use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, parameters))]
pub async fn confirm(state: State<AppState>, parameters: Query<Parameters>) -> Response {
    let id = match get_subscriber_id_from_token(
        &state.pg_connection_pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response(),
    };
    match id {
        None => (StatusCode::UNAUTHORIZED, "").into_response(),
        Some(subscriber_id) => {
            match confirm_subscriber(&state.pg_connection_pool, subscriber_id).await {
                Ok(_) => (StatusCode::OK, "Subscribed Successfully!!").into_response(),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "").into_response(),
            }
        }
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pg_pool, subscriber_id))]
pub async fn confirm_subscriber(pg_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pg_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(pg_pool, subscription_token)
)]
pub async fn get_subscriber_id_from_token(
    pg_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pg_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
