use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::error_chain_fmt;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> Response {
        match &self {
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
            Self::UnknownToken => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, parameters))]
pub async fn confirm(
    state: State<AppState>,
    parameters: Query<Parameters>,
) -> Result<Response, ConfirmationError> {
    let id =
        get_subscriber_id_from_token(&state.pg_connection_pool, &parameters.subscription_token)
            .await?;

    match id {
        None => Err(ConfirmationError::UnknownToken),
        Some(subscriber_id) => {
            confirm_subscriber(&state.pg_connection_pool, subscriber_id).await?;
            Ok((StatusCode::OK, "Subscribed Successfully!!").into_response())
        }
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pg_pool, subscriber_id))]
pub async fn confirm_subscriber(pg_pool: &PgPool, subscriber_id: Uuid) -> anyhow::Result<()> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pg_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        anyhow::anyhow!("Failed to update confirmation status. {e}")
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
) -> anyhow::Result<Option<Uuid>> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pg_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        anyhow::anyhow!("Failed to get data from postgres. {e}")
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
