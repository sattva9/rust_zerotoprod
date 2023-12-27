use std::collections::HashMap;

use askama_axum::{IntoResponse, Template};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use axum_flash::IncomingFlashes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    utils::{e500, read_flash_messages},
    AppState,
};

const IN_PROGRESS: &str = "IN PROGRESS";

#[derive(Template)]
#[template(path = "admin/newsletter_issues.html")]
struct NewsletterProgress {
    msg: String,
    issues: HashMap<Uuid, NewsletterIssue>,
}

struct NewsletterIssue {
    newsletter_issue_id: Uuid,
    title: String,
    content: String,
    published_at: String,
    status: String,
}

pub async fn issues(
    state: State<AppState>,
    flash_messages: IncomingFlashes,
) -> Result<Response, StatusCode> {
    let msg = read_flash_messages(&flash_messages);
    let issues = get_all_newsletter_issues(&state.pg_connection_pool)
        .await
        .map_err(e500)?;

    Ok((flash_messages, NewsletterProgress { msg, issues }).into_response())
}

async fn get_all_newsletter_issues(
    pg_pool: &PgPool,
) -> anyhow::Result<HashMap<Uuid, NewsletterIssue>> {
    let mut all_issues: HashMap<Uuid, NewsletterIssue> = sqlx::query_as_unchecked!(
        NewsletterIssue,
        r#"
        SELECT newsletter_issue_id, title, content, published_at, 'PUBLISHED' as status
        FROM newsletter_issues
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?
    .into_iter()
    .map(|issue| (issue.newsletter_issue_id, issue))
    .collect();

    let running_issues: Vec<Uuid> = sqlx::query!(
        r#"
        SELECT DISTINCT newsletter_issue_id
        FROM issue_delivery_queue
        "#,
    )
    .fetch_all(pg_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?
    .into_iter()
    .map(|r| r.newsletter_issue_id)
    .collect();

    running_issues.into_iter().for_each(|issue_id| {
        if let Some(issue) = all_issues.get_mut(&issue_id) {
            issue.status = IN_PROGRESS.to_owned();
        }
    });

    Ok(all_issues)
}

#[derive(Template)]
#[template(path = "admin/newsletter_issue.html")]
struct NewsletterIssueMeta {
    issue: NewsletterIssue,
}

pub async fn issue(state: State<AppState>, path: Path<Uuid>) -> Result<Response, StatusCode> {
    let newsletter_issue_id = path.0;
    let issue = get_issue(&state.pg_connection_pool, newsletter_issue_id)
        .await
        .map_err(e500)?;

    Ok(NewsletterIssueMeta { issue }.into_response())
}

async fn get_issue(pg_pool: &PgPool, newsletter_issue_id: Uuid) -> anyhow::Result<NewsletterIssue> {
    let issue = sqlx::query_as_unchecked!(
        NewsletterIssue,
        r#"
        SELECT newsletter_issue_id, title, content, published_at, '' as status
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        newsletter_issue_id
    )
    .fetch_one(pg_pool)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(issue)
}
