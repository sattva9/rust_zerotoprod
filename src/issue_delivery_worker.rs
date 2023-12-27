use std::time::Duration;

use crate::{
    configuration::Settings,
    domain::{Subscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::get_connection_pool,
};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(newsletter_issue_id=tracing::field::Empty, subscriber_email=tracing::field::Empty),
    err
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionOutcome> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (transaction, issue_id, email, name) = task.unwrap();

    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));

    match (
        SubscriberEmail::parse(email.clone()),
        SubscriberName::parse(name),
    ) {
        (Ok(email), Ok(name)) => {
            let issue = get_issue(pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(&Subscriber { name, email }, &issue.title, &issue.content)
                .await
            {
                tracing::error!("Failed to deliver issue to a confirmed subscriber. Skipping. {e}",);
            }
        }
        other => {
            tracing::error!(
                    "Skipping a confirmed subscriber. Their stored contact details are invalid. {other:?}",
                );
        }
    }

    delete_task(transaction, issue_id, &email).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    pg_pool: &PgPool,
) -> anyhow::Result<Option<(PgTransaction, Uuid, String, String)>> {
    let mut transaction = pg_pool.begin().await?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email, name as subscriber_name
        FROM issue_delivery_queue as a INNER JOIN subscriptions as b
        ON a.subscriber_email=b.email
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await?;
    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id,
            r.subscriber_email,
            r.subscriber_name,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        issue_id,
        email
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(pg_pool: &PgPool, issue_id: Uuid) -> anyhow::Result<NewsletterIssue> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, content
        FROM newsletter_issues
        WHERE
            newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(pg_pool)
    .await?;
    Ok(issue)
}

async fn worker_loop(pg_pool: PgPool, email_client: EmailClient) -> anyhow::Result<()> {
    loop {
        match try_execute_task(&pg_pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_worker_until_stopped(configuration: Settings) -> anyhow::Result<()> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email.client()?;
    worker_loop(connection_pool, email_client).await
}
