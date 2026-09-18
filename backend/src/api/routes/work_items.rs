//! Work item endpoint'leri: tipler, iş kalemleri, dinamik özellikler.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::{
    BulkWorkItemRequest, CreatePropertyDefinitionRequest, CreateWorkItemRequest,
    CreateWorkItemTypeRequest, SetPropertyValueRequest, UpdateWorkItemRequest,
    WorkItemDetailDto,
};
use crate::application::services::work_item_service as service;
use crate::domain::entities::{PropertyDefinition, WorkItem, WorkItemType};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct SectionFilter {
    pub section_id: Option<Uuid>,
}

// --- Tipler ---

/// GET /api/v1/work-item-types
pub async fn list_types(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<WorkItemType>>> {
    Ok(Json(service::list_types(&state.pool, &auth.user).await?))
}

/// POST /api/v1/work-item-types (ADMIN)
pub async fn create_type(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateWorkItemTypeRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<WorkItemType>)> {
    let created = service::create_type(&state.pool, &auth.user, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

// --- İş kalemleri ---

/// GET /api/v1/projects/{project_id}/work-items?section_id=...
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(filter): Query<SectionFilter>,
) -> ApiResult<Json<Vec<WorkItem>>> {
    // section_id verilmezse projenin tüm iş kalemleri döner
    if let Some(section_id) = filter.section_id {
        return Ok(Json(
            service::list_by_section(&state.pool, &auth.user, section_id).await?,
        ));
    }
    Ok(Json(service::list_all(&state.pool, &auth.user, project_id).await?))
}

/// POST /api/v1/projects/{project_id}/work-items
pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateWorkItemRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<WorkItem>)> {
    let item = service::create(&state.pool, &auth.user, project_id, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(item)))
}

/// POST /api/v1/projects/{project_id}/work-items/bulk
pub async fn bulk_create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<BulkWorkItemRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Vec<WorkItem>>)> {
    let created = service::bulk_create(&state.pool, &auth.user, project_id, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

/// GET /api/v1/work-items/{item_id} — detay + özellik değerleri
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(item_id): Path<Uuid>,
) -> ApiResult<Json<WorkItemDetailDto>> {
    Ok(Json(service::get(&state.pool, &auth.user, item_id).await?))
}

/// PATCH /api/v1/work-items/{item_id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(item_id): Path<Uuid>,
    Json(req): Json<UpdateWorkItemRequest>,
) -> ApiResult<Json<WorkItem>> {
    Ok(Json(service::update(&state.pool, &auth.user, item_id, req).await?))
}

/// DELETE /api/v1/work-items/{item_id} — soft delete
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(item_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    service::delete(&state.pool, &auth.user, item_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Özellikler ---

/// GET /api/v1/property-definitions
pub async fn list_definitions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<PropertyDefinition>>> {
    Ok(Json(service::list_definitions(&state.pool, &auth.user).await?))
}

/// POST /api/v1/property-definitions (ADMIN)
pub async fn create_definition(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePropertyDefinitionRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<PropertyDefinition>)> {
    let created = service::create_definition(&state.pool, &auth.user, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

/// PUT /api/v1/work-items/{item_id}/property-values
pub async fn set_values(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(item_id): Path<Uuid>,
    Json(values): Json<Vec<SetPropertyValueRequest>>,
) -> ApiResult<Json<serde_json::Value>> {
    service::set_values(&state.pool, &auth.user, item_id, values).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
