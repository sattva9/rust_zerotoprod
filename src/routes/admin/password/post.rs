use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_flash::Flash;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials, UserId},
    routes::admin::dashboard::get_username,
    utils::e500,
    AppState,
};

#[derive(Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    state: State<AppState>,
    flash: Flash,
    user_id: Extension<UserId>,
    form: Form<FormData>,
) -> Result<Response, StatusCode> {
    let user_id = *user_id.0;

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        let flash =
            flash.error("You entered two different new passwords - the field values must match.");
        return Ok((flash, Redirect::to("/admin/password")).into_response());
    }

    let username = get_username(&state.pg_connection_pool, user_id)
        .await
        .map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(&state.pg_connection_pool, credentials).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                let flash = flash.error("The current password is incorrect.");
                Ok((flash, Redirect::to("/admin/password")).into_response())
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(&state.pg_connection_pool, user_id, form.0.new_password)
        .await
        .map_err(e500)?;
    let flash = flash.info("Your password has been changed.");
    Ok((flash, Redirect::to("/admin/password")).into_response())
}
