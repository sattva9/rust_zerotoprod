use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::Flash;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    domain::{Subscriber, SubscriberEmail, SubscriberName},
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500},
    AppState,
};

#[derive(Deserialize)]
pub struct FormData {
    title: String,
    content: String,
    idempotency_key: String,
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
    let FormData {
        title,
        content,
        idempotency_key,
    } = form.0;
    let user_id = *user_id.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let mut transaction = match try_processing(&state.pg_connection_pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            let flash = success_message(flash);
            return Ok((flash, saved_response).into_response());
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &content)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;
    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = Redirect::to("/admin/issues").into_response();
    let response = save_response(transaction, &idempotency_key, user_id, response)
        .await
        .map_err(e500)?;
    let flash = success_message(flash);
    Ok((flash, response).into_response())
}

fn success_message(flash: Flash) -> Flash {
    flash.info("The newsletter issue has been accepted - emails will go out shortly.")
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

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            content,
            published_at
        )
        VALUES ($1, $2, $3, now())
        "#,
        newsletter_issue_id,
        title,
        content,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
