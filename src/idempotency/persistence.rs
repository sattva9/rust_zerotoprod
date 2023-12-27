use axum::{
    body::{to_bytes, Body},
    http::StatusCode,
    response::Response,
};
use sqlx::{postgres::PgHasArrayType, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn get_saved_response(
    pg_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> anyhow::Result<Option<Response>> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        FROM idempotency
        WHERE
          user_id = $1 AND
          idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pg_pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut response_builder = Response::builder().status(status_code);
        for HeaderPairRecord { name, value } in r.response_headers {
            response_builder = response_builder.header(name, value);
        }
        let response = response_builder
            .body(Body::from(r.response_body))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize Response. {e}"))?;
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: Response,
) -> anyhow::Result<Response> {
    let (response_head, body) = response.into_parts();
    let status_code = response_head.status.as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers.len());
        for (name, value) in response_head.headers.iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };
    let body = to_bytes(body, usize::MAX).await.unwrap();

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let response = Response::from_parts(response_head, Body::from(body));
    Ok(response)
}

#[allow(clippy::large_enum_variant)]
pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}
pub async fn try_processing(
    pg_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> anyhow::Result<NextAction> {
    let mut transaction = pg_pool.begin().await?;
    let n_inserted_rows = sqlx::query!(
        r#"
            INSERT INTO idempotency (
                user_id,
                idempotency_key,
                created_at )
            VALUES ($1, $2, now())
            ON CONFLICT DO NOTHING
            "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pg_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
