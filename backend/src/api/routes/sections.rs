//! Section endpoint'leri (docs/architecture.md §15).

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::{
    BulkSectionRequest, CloneSectionRequest, CreateSectionRequest, SectionDto,
    SectionTreeResponse, UpdateSectionRequest,
};
use crate::application::services::section_service as service;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

/// GET /api/v1/projects/{project_id}/sections/tree
pub async fn tree(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Json<SectionTreeResponse>> {
    Ok(Json(service::tree(&state.pool, &auth.user, project_id).await?))
}

/// POST /api/v1/projects/{project_id}/sections
pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateSectionRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<SectionDto>)> {
    let section = service::create(&state.pool, &auth.user, project_id, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(section.into())))
}

/// POST /api/v1/projects/{project_id}/sections/bulk
pub async fn bulk_create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<BulkSectionRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Vec<SectionDto>>)> {
    let created = service::bulk_create(&state.pool, &auth.user, project_id, req).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(created.into_iter().map(Into::into).collect()),
    ))
}

/// POST /api/v1/sections/{section_id}/clone
pub async fn clone(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(section_id): Path<Uuid>,
    Json(req): Json<CloneSectionRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Vec<SectionDto>>)> {
    let created = service::clone(&state.pool, &auth.user, section_id, req).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(created.into_iter().map(Into::into).collect()),
    ))
}

/// PATCH /api/v1/sections/{section_id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(section_id): Path<Uuid>,
    Json(req): Json<UpdateSectionRequest>,
) -> ApiResult<Json<SectionDto>> {
    Ok(Json(
        service::update(&state.pool, &auth.user, section_id, req)
            .await?
            .into(),
    ))
}

/// DELETE /api/v1/sections/{section_id} — soft delete (confirm'lu UI aksiyonu)
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(section_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    service::delete(&state.pool, &auth.user, section_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
