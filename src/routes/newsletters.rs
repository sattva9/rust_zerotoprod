use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sqlx::PgPool;

use crate::domain::{Subscriber, SubscriberEmail, SubscriberName};
use crate::AppState;

use super::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        match self {
            Self::UnexpectedError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "").into_response(),
        }
    }
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: String,
}

pub async fn publish_newsletter(
    state: State<AppState>,
    body: axum::Json<BodyData>,
) -> Result<Response, PublishError> {
    let subscribers = get_confirmed_subscribers(&state.pg_connection_pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(&subscriber, &body.title, &body.content)
                    .await?;
            }
            Err(e) => {
                tracing::warn!(error.cause_chain = ?e, "Skipping a confirmed subscriber. Their stored contact details are invalid")
            }
        }
    }
    Ok((StatusCode::OK, "").into_response())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pg_pool))]
async fn get_confirmed_subscribers(
    pg_pool: &PgPool,
) -> anyhow::Result<Vec<anyhow::Result<Subscriber>>> {
    let confirmed_subscribers =
        sqlx::query!(r#" SELECT email, name FROM subscriptions WHERE status = 'confirmed' "#,)
            .fetch_all(pg_pool)
            .await?
            .into_iter()
            .map(|r| {
                match (
                    SubscriberEmail::parse(r.email),
                    SubscriberName::parse(r.name),
                ) {
                    (Ok(email), Ok(name)) => Ok(Subscriber { email, name }),
                    other => Err(anyhow::anyhow!("{:?}", other)),
                }
            })
            .collect();
    Ok(confirmed_subscribers)
}
