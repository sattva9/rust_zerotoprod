use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_flash::Flash;

use crate::session_state::TypedSession;

pub async fn log_out(session: TypedSession, flash: Flash) -> Result<Response, StatusCode> {
    session.log_out();
    Ok((
        flash.info("You have successfully logged out."),
        Redirect::to("/login"),
    )
        .into_response())
}
