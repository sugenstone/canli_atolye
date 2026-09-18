//! Proje endpoint'leri.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::{CreateProjectRequest, ProjectDto, UpdateProjectRequest};
use crate::application::services::project_service;
use crate::domain::entities::ProjectStatus;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct ProjectFilter {
    pub status: Option<String>,
}

fn parse_status(raw: Option<String>) -> Result<Option<ProjectStatus>, crate::api::error::ApiError> {
    match raw {
        None => Ok(None),
        Some(s) => s
            .parse::<ProjectStatus>()
            .map(Some)
            .map_err(|_| crate::api::error::ApiError::from(
                crate::domain::errors::DomainError::Validation {
                    message: format!("Geçersiz status filtresi: {s}"),
                },
            )),
    }
}

/// GET /api/v1/projects?status=ACTIVE
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filter): Query<ProjectFilter>,
) -> ApiResult<Json<Vec<ProjectDto>>> {
    let status = parse_status(filter.status)?;
    let projects = project_service::list(&state.pool, &auth.user, status).await?;
    Ok(Json(projects.into_iter().map(Into::into).collect()))
}

/// POST /api/v1/projects (ADMIN)
pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateProjectRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<ProjectDto>)> {
    let project = project_service::create(&state.pool, &auth.user, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(project.into())))
}

/// GET /api/v1/projects/{project_id}
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Json<ProjectDto>> {
    Ok(Json(project_service::get(&state.pool, &auth.user, project_id).await?.into()))
}

/// PATCH /api/v1/projects/{project_id} (ADMIN / PM)
pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<Json<ProjectDto>> {
    Ok(Json(
        project_service::update(&state.pool, &auth.user, project_id, req)
            .await?
            .into(),
    ))
}

/// POST /api/v1/projects/{project_id}/archive (ADMIN / PM, confirm'lu UI aksiyonu)
pub async fn archive(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Json<ProjectDto>> {
    Ok(Json(
        project_service::archive(&state.pool, &auth.user, project_id)
            .await?
            .into(),
    ))
}
