use crate::domain::Subscriber;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::types::uuid;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(state: State<AppState>, form: Form<Subscriber>) -> Response {
    let mut transaction = match state.pg_connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &form).await {
        Ok(subscriber_id) => subscriber_id,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if subscriber_id.is_none() {
        return (StatusCode::OK, "").into_response();
    }
    let subscriber_id = subscriber_id.unwrap();

    let subscription_token = generate_subscription_token();
    if let Err(e) = store_token(&mut transaction, subscriber_id, &subscription_token).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) = transaction.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) = send_confirmation_email(&state, &form, &subscription_token).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (StatusCode::OK, "").into_response()
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &Subscriber,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        ON CONFLICT (email) DO NOTHING
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now(),
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    let result = sqlx::query!(
        r#"SELECT id from subscriptions where email=$1 and status != 'confirmed'"#,
        new_subscriber.email.as_ref(),
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute get id query: {e:?}");
        e
    })?;

    Ok(result.map(|r| r.id))
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(state, new_subscriber)
)]
pub async fn send_confirmation_email(
    state: &AppState,
    new_subscriber: &Subscriber,
    subscription_token: &str,
) -> Result<(), String> {
    let path = format!("subscriptions/confirm?subscription_token={subscription_token}");
    let confirmation_link = state.application_base_url.join(&path)?;

    let content = format!(
        "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link.as_str()
    );
    state
        .email_client
        .send_email(new_subscriber, "Welcome!", &content)
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
