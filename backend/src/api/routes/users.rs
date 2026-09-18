//! Kullanıcı endpoint'leri (ADMIN).

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::{CreateUserRequest, UpdateUserRequest, UserDto};
use crate::application::services::user_service;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

/// GET /api/v1/workspaces/{workspace_id}/users
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Vec<UserDto>>> {
    let users = user_service::list(&state.pool, &auth.user, workspace_id).await?;
    Ok(Json(users.into_iter().map(Into::into).collect()))
}

/// POST /api/v1/workspaces/{workspace_id}/users
pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<UserDto>)> {
    let user = user_service::create(&state.pool, &auth.user, workspace_id, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(user.into())))
}

/// PATCH /api/v1/users/{user_id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> ApiResult<Json<UserDto>> {
    let user = user_service::update(
        &state.pool,
        &auth.user,
        auth.user.workspace_id,
        user_id,
        req,
    )
    .await?;
    Ok(Json(user.into()))
}

/// DELETE /api/v1/users/{user_id} — pasifleştirme (soft).
pub async fn deactivate(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<UserDto>> {
    let user = user_service::deactivate(
        &state.pool,
        &auth.user,
        auth.user.workspace_id,
        user_id,
    )
    .await?;
    Ok(Json(user.into()))
}
