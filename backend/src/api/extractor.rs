//! `AuthUser` extractor: cookie'deki session token'ından kullanıcıyı çözer.
//! Bunu kullanan her endpoint kimlik doğrulaması gerektirir (401/403).

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::application::services::auth_service;
use crate::domain::entities::{User, Workspace};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

/// Kimliği doğrulanmış istek context'i.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
    pub workspace: Workspace,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, &())
            .await
            .unwrap_or_default();

        let token = jar
            .get(auth_service::SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(ApiError::from(crate::domain::errors::DomainError::SessionExpired))?;

        let (user, workspace) = auth_service::authenticate_by_token(&state.pool, &token)
            .await
            .map_err(ApiError::from)?;

        tracing::Span::current().record("user_id", tracing::field::display(user.id));
        Ok(AuthUser { user, workspace })
    }
}

/// Cookie'ye session token'ı yazar. Secure flag config'den (prod: true).
/// max_age yok: süre otoritesi DB'deki `expires_at` kolonudur.
pub fn build_session_cookie(token: &str, secure: bool) -> axum_extra::extract::cookie::Cookie<'static> {
    use axum_extra::extract::cookie::{Cookie, SameSite};

    Cookie::build((auth_service::SESSION_COOKIE, token.to_string()))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}
