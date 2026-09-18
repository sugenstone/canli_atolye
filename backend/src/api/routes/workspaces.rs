//! Workspace endpoint'leri (Faz 1: okuma ağırlıklı; oluşturma seed üzerinden).

use crate::api::error::{ApiError, ApiResult};
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::WorkspaceDto;
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::workspace_repository::WorkspaceRepository;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

/// GET /api/v1/workspaces — ADMIN tüm workspace'leri listeler.
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<WorkspaceDto>>> {
    if !can(auth.user.role, Action::ManageWorkspace) {
        return Err(ApiError::from(DomainError::Forbidden));
    }
    let workspaces = WorkspaceRepository { pool: &state.pool }
        .list()
        .await?;
    Ok(Json(workspaces.into_iter().map(Into::into).collect()))
}

/// GET /api/v1/workspaces/{id} — üyeler kendi workspace'ini görebilir.
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<WorkspaceDto>> {
    if auth.user.workspace_id != workspace_id
        && !can(auth.user.role, Action::ManageWorkspace)
    {
        return Err(ApiError::from(DomainError::Forbidden));
    }
    let workspace = WorkspaceRepository { pool: &state.pool }
        .find_by_id(workspace_id)
        .await?;
    Ok(Json(workspace.into()))
}
