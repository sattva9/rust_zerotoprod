use axum::extract::State;
use axum::Form;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sqlx::types::uuid;
use serde::Deserialize;
use uuid::Uuid;
use crate::AppState;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(state: State<AppState>, form: Form<FormData>,) -> Response {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(), form.email, form.name, chrono::Utc::now(),
    )
        .execute(&state.pg_connection_pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "").into_response(),
        Err(e) => {
            println!("Failed to execute query: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}