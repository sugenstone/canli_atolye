//! Auth endpoint'leri: login / logout / me.

use crate::api::error::ApiResult;
use crate::api::extractor::{build_session_cookie, AuthUser};
use crate::api::state::AppState;
use crate::application::dto::{LoginRequest, MeResponse};
use crate::application::services::auth_service;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use axum::{Json, response::IntoResponse};

/// POST /api/v1/auth/login → HttpOnly cookie set edilir.
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let (_user, _workspace, token) = auth_service::login(
        &state.pool,
        &state.config,
        req.email.trim(),
        &req.password,
        None, // user_agent: middleware katmanında doldurulacak
        None,
    )
    .await?;

    let cookie = build_session_cookie(&token, state.config.cookie_secure);
    Ok((jar.add(cookie), Json(serde_json::json!({ "ok": true }))))
}

/// POST /api/v1/auth/logout → session silinir, cookie temizlenir.
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<impl IntoResponse> {
    if let Some(cookie) = jar.get(auth_service::SESSION_COOKIE) {
        auth_service::logout(&state.pool, cookie.value()).await?;
    }
    let expired = build_session_cookie("", state.config.cookie_secure).into_owned();
    Ok((jar.remove(expired), Json(serde_json::json!({ "ok": true }))))
}

/// GET /api/v1/auth/me → kullanıcı + workspace.
pub async fn me(auth: AuthUser) -> ApiResult<Json<MeResponse>> {
    Ok(Json(MeResponse {
        user: auth.user.into(),
        workspace: auth.workspace.into(),
    }))
}
