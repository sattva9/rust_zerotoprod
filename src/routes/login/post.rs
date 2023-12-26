use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_flash::Flash;
use secrecy::Secret;
use serde::Deserialize;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
    session_state::TypedSession,
    AppState,
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(state, flash, session, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    state: State<AppState>,
    flash: Flash,
    session: TypedSession,
    form: Form<FormData>,
) -> Result<Response, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(&state.pg_connection_pool, credentials).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.renew();
            if let Err(e) = session.insert_user_id(user_id) {
                let e = LoginError::UnexpectedError(e);
                return Ok(login_redirect(e, flash));
            }

            Ok((flash, Redirect::to("/admin/dashboard")).into_response())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };

            Ok(login_redirect(e, flash))
        }
    }
}

fn login_redirect(e: LoginError, flash: Flash) -> Response {
    (flash.error(e.to_string()), Redirect::to("/login")).into_response()
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match &self {
            Self::AuthError(_) => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}
