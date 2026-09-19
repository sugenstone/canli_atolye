//! Süreç motoru endpoint'leri (docs/architecture.md §15).
//! Durum değişiklikleri her zaman action endpoint üzerinden — business logic
//! service + engine'de çalışır.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::services::process_service as service;
use crate::domain::entities::{
    ProcessDependency, ProcessEvent, ProcessExecution, ProcessGroup, ProcessGroupStep,
    ProcessTemplate,
};
use crate::domain::services::process_engine::Transition;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- DTO'lar ---

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub code: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StepInput {
    pub template_id: Uuid,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct DependencyInput {
    /// 0 tabanlı adım indeksi
    pub step: usize,
    pub depends_on: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<StepInput>,
    #[serde(default)]
    pub dependencies: Vec<DependencyInput>,
}

#[derive(Debug, Serialize)]
pub struct GroupDetailDto {
    pub group: ProcessGroup,
    pub steps: Vec<ProcessGroupStep>,
    pub dependencies: Vec<ProcessDependency>,
}

#[derive(Debug, Deserialize)]
pub struct AssignGroupRequest {
    pub process_group_id: Uuid,
}

#[derive(Debug, Deserialize, Default)]
pub struct ExecutionFilter {
    pub work_item_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ActionRequest {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRequest {
    #[serde(default)]
    pub user_id: Option<Uuid>,
    #[serde(default)]
    pub team_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct TransitionDto {
    pub execution: ProcessExecution,
    /// bu geçişin READY'a çektiği downstream execution id'leri
    pub promoted: Vec<Uuid>,
}

// --- Şablonlar ---

/// GET /api/v1/process-templates
pub async fn list_templates(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<ProcessTemplate>>> {
    Ok(Json(service::list_templates(&state.pool, &auth.user).await?))
}

/// POST /api/v1/process-templates (ADMIN)
pub async fn create_template(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTemplateRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<ProcessTemplate>)> {
    let created = service::create_template(
        &state.pool,
        &auth.user,
        &req.name,
        &req.code,
        req.description.as_deref(),
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

// --- Gruplar ---

/// GET /api/v1/process-groups
pub async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<ProcessGroup>>> {
    Ok(Json(service::list_groups(&state.pool, &auth.user).await?))
}

/// POST /api/v1/process-groups (ADMIN) — adımlar sıralı, bağımlılıklar index bazlı
pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<ProcessGroup>)> {
    let created = service::create_group(
        &state.pool,
        &auth.user,
        service::NewProcessGroup {
            name: req.name,
            description: req.description,
            steps: req.steps.into_iter().map(|s| (s.template_id, s.required)).collect(),
            dependencies: req.dependencies.into_iter().map(|d| (d.step, d.depends_on)).collect(),
        },
    )
    .await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

/// GET /api/v1/process-groups/{group_id}
pub async fn group_detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<GroupDetailDto>> {
    let group = crate::infrastructure::repositories::process_repository::ProcessCatalogRepository::find_group(
        &state.pool, auth.user.workspace_id, group_id,
    )
    .await?;
    let (steps, dependencies) = service::group_detail(&state.pool, &auth.user, group_id).await?;
    Ok(Json(GroupDetailDto { group, steps, dependencies }))
}

// --- Atama ve execution'lar ---

/// POST /api/v1/work-items/{work_item_id}/assign-process-group
pub async fn assign_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(work_item_id): Path<Uuid>,
    Json(req): Json<AssignGroupRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Vec<ProcessExecution>>)> {
    let created =
        service::assign_group(&state.pool, &auth.user, work_item_id, req.process_group_id).await?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

/// GET /api/v1/process-executions?work_item_id=...
pub async fn list_executions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filter): Query<ExecutionFilter>,
) -> ApiResult<Json<serde_json::Value>> {
    let work_item_id = filter.work_item_id.ok_or_else(|| {
        crate::api::error::ApiError::from(crate::domain::errors::DomainError::Validation {
            message: "work_item_id zorunludur.".into(),
        })
    })?;
    let rows = service::list_by_work_item(&state.pool, &auth.user, work_item_id).await?;
    Ok(Json(serde_json::json!({ "work_item_id": work_item_id, "executions": rows })))
}

/// GET /api/v1/process-executions/{id}
pub async fn get_execution(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> ApiResult<Json<ProcessExecution>> {
    Ok(Json(service::get_execution(&state.pool, &auth.user, execution_id).await?))
}

/// GET /api/v1/process-executions/{id}/events — immutable geçmiş
pub async fn list_events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ProcessEvent>>> {
    Ok(Json(service::list_events(&state.pool, &auth.user, execution_id).await?))
}

/// POST /api/v1/process-executions/{id}/assign
fn was_unassigned_checked(v: &bool) -> Option<bool> {
    Some(*v)
}

pub async fn assign(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<AssignRequest>,
) -> ApiResult<Json<ProcessExecution>> {
    if req.user_id.is_none() && req.team_id.is_none() {
        return Err(crate::api::error::ApiError::from(
            crate::domain::errors::DomainError::Validation {
                message: "user_id veya team_id gerekli.".into(),
            },
        ));
    }
    Ok(Json(
        service::assign(&state.pool, &auth.user, execution_id, req.user_id, req.team_id).await?,
    ))
}

/// Aksiyon endpoint'lerini tek makro benzeri fabrikayla bağlarız.
macro_rules! action_handler {
    ($name:ident, $action:expr) => {
        pub async fn $name(
            State(state): State<AppState>,
            auth: AuthUser,
            Path(execution_id): Path<Uuid>,
            body: Option<Json<ActionRequest>>,
        ) -> ApiResult<Json<TransitionDto>> {
            let note = body.and_then(|Json(req)| req.note);
            let outcome =
                service::apply_transition(&state.pool, &auth.user, execution_id, $action, note).await?;
            // SSE: yalnızca commit sonrası yayın (MASTER PLAN §16.2)
            crate::api::routes::insights::publish_execution_event(
                &state,
                outcome.execution.work_item_id,
                Some(outcome.execution.id),
                $action.event_type().as_str(),
            )
            .await;
            Ok(Json(TransitionDto {
                execution: outcome.execution,
                promoted: outcome.promoted,
            }))
        }
    };
}

action_handler!(start, Transition::Start);
action_handler!(pause, Transition::Pause);
action_handler!(resume_, Transition::Resume);
action_handler!(complete, Transition::Complete);
action_handler!(cancel, Transition::Cancel);

// --- Faz 8: Planlama ---

#[derive(Debug, Deserialize)]
pub struct PlannedDatesRequest {
    #[serde(default)]
    pub planned_start_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    #[serde(default)]
    pub planned_end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

/// PATCH /api/v1/process-executions/{id}/planned-dates (ADMIN/PM)
pub async fn planned_dates(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<PlannedDatesRequest>,
) -> ApiResult<Json<ProcessExecution>> {
    let updated = service::set_planned_dates(
        &state.pool, &auth.user, execution_id, req.planned_start_at, req.planned_end_at,
    )
    .await?;
    crate::api::routes::insights::publish_execution_event(
        &state, updated.work_item_id, Some(updated.id), "PLANNED_DATE_CHANGED",
    ).await;
    Ok(Json(updated))
}

#[derive(Debug, Deserialize)]
pub struct DurationsResponse(pub serde_json::Value);

/// GET /api/v1/process-executions/{id}/durations
pub async fn durations(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> ApiResult<Json<crate::domain::services::duration::Durations>> {
    Ok(Json(service::durations(&state.pool, &auth.user, execution_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct BulkPlanRequest {
    #[serde(default)]
    pub template_id: Option<Uuid>,
    #[serde(default)]
    pub execution_ids: Option<Vec<Uuid>>,
    #[serde(default)]
    pub planned_start_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    #[serde(default)]
    pub planned_end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

/// POST /api/v1/projects/{project_id}/process-executions/bulk-plan (ADMIN/PM)
pub async fn bulk_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(req): Json<BulkPlanRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let count = service::bulk_plan(
        &state.pool,
        &auth.user,
        project_id,
        service::BulkPlanInput {
            template_id: req.template_id,
            execution_ids: req.execution_ids,
            planned_start_at: req.planned_start_at,
            planned_end_at: req.planned_end_at,
        },
    )
    .await?;
    Ok(Json(serde_json::json!({ "updated": count })))
}

// --- Toplu atama + revizyon (MASTER PLAN §23, §20) ---

#[derive(Debug, Deserialize)]
pub struct BulkAssignGroupRequest {
    pub parent_section_id: Uuid,
    pub process_group_id: Uuid,
}

/// POST /api/v1/projects/{project_id}/work-items/bulk-assign-process-group (ADMIN/PM)
pub async fn bulk_assign_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(_project_id): Path<Uuid>,
    Json(req): Json<BulkAssignGroupRequest>,
) -> ApiResult<axum::http::StatusCode> {
    use crate::application::services::process_service;
    let count = process_service::bulk_assign_group(&state.pool, &auth.user, req.parent_section_id, req.process_group_id).await?;
    if count > 0 {
        // SSE: toplu atama — projeye genel yenileme sinyali
        let _ = &state;
    }
    Ok(axum::http::StatusCode::CREATED)
}

#[derive(Debug, Deserialize)]
pub struct ReopenRequest {
    pub reason: String,
}

/// POST /api/v1/process-executions/{id}/reopen (ADMIN/PM, confirm'lu)
pub async fn reopen(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<ReopenRequest>,
) -> ApiResult<Json<ProcessExecution>> {
    use crate::application::services::process_service;
    let revision = process_service::reopen(&state.pool, &auth.user, execution_id, req.reason).await?;
    crate::api::routes::insights::publish_execution_event(
        &state, revision.work_item_id, Some(revision.id), "REOPENED",
    ).await;
    Ok(Json(revision))
}
