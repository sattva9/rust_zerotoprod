use axum::{
    body::Body,
    extract::{FromRequestParts, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    RequestExt,
};
use axum_flash::Flash;
use std::ops::Deref;
use uuid::Uuid;

use crate::{session_state::TypedSession, utils::e500, AppState};

#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    state: State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let mut parts = request.extract_parts().await.map_err(e500)?;
    let session = TypedSession::from_request_parts(&mut parts, &state.0)
        .await
        .unwrap();
    let flash = Flash::from_request_parts(&mut parts, &state.0)
        .await
        .map_err(e500)?;
    match session.get_user_id().map_err(e500)? {
        Some(user_id) => {
            request.extensions_mut().insert(UserId(user_id));
            let response = next.run(request).await;
            Ok(response)
        }
        None => Ok((flash.error("Session expired!"), Redirect::to("/login")).into_response()),
    }
}
