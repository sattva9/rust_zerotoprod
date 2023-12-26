use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::Flash;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::{Subscriber, SubscriberEmail, SubscriberName},
    utils::e500,
    AppState,
};

#[derive(Deserialize)]
pub struct FormData {
    title: String,
    content: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(state, flash, user_id, form),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    state: State<AppState>,
    flash: Flash,
    user_id: Extension<UserId>,
    form: Form<FormData>,
) -> Result<Response, StatusCode> {
    let subscribers = get_confirmed_subscribers(&state.pg_connection_pool)
        .await
        .map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(&subscriber, &form.title, &form.content)
                    .await
                    .map_err(e500)?;
            }
            Err(_) => {
                tracing::warn!(
                    "Skipping a confirmed subscriber. Their stored contact details are invalid"
                )
            }
        }
    }
    let flash = flash.info("The newsletter issue has been published!");
    Ok((flash, Redirect::to("/admin/newsletters")).into_response())
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
