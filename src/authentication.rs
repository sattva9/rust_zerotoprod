use anyhow::Context;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use secrecy::ExposeSecret;
use secrecy::Secret;
use sha3::Digest;
use sqlx::PgPool;

use crate::routes::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pg_pool))]
pub async fn validate_credentials(
    pg_pool: &PgPool,
    credentials: Credentials,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new("$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string());

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pg_pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
pub fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pg_pool))]
pub async fn get_stored_credentials(
    username: &str,
    pg_pool: &PgPool,
) -> anyhow::Result<Option<(uuid::Uuid, Secret<String>)>> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pg_pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}

#[allow(unused)]
fn sha_password(password: Secret<String>) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(password.expose_secret().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[allow(unused)]
fn argon_password(password: Secret<String>, salt: &str) -> anyhow::Result<String> {
    let mut hasher = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).context("Failed to build Argon2 parameters")?,
    );

    let salt = SaltString::from_b64(salt).context("Failed to convert to salt")?;
    let password = hasher
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .context("Failed to hash password")?
        .to_string();
    Ok(password)
}
