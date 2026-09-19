//! Proje yönetimi service'i.

use crate::application::dto::{CreateProjectRequest, UpdateProjectRequest};
use crate::domain::entities::{Project, ProjectStatus, User};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::project_repository::{
    NewProject, ProjectPatch, ProjectRepository,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn list(
    pool: &SqlitePool,
    actor: &User,
    status: Option<ProjectStatus>,
) -> Result<Vec<Project>, DomainError> {
    if !can(actor.role, Action::ViewProjects) {
        return Err(DomainError::Forbidden);
    }
    ProjectRepository { pool }
        .list_by_workspace(actor.workspace_id, status)
        .await
}

pub async fn get(pool: &SqlitePool, actor: &User, project_id: Uuid) -> Result<Project, DomainError> {
    if !can(actor.role, Action::ViewProjects) {
        return Err(DomainError::Forbidden);
    }
    ProjectRepository { pool }
        .find_in_workspace(actor.workspace_id, project_id)
        .await
}

pub async fn create(
    pool: &SqlitePool,
    actor: &User,
    req: CreateProjectRequest,
) -> Result<Project, DomainError> {
    if !can(actor.role, Action::CreateProject) {
        return Err(DomainError::Forbidden);
    }
    if req.name.trim().is_empty() || req.code.trim().is_empty() {
        return Err(DomainError::Validation {
            message: "Proje adı ve kodu zorunludur.".into(),
        });
    }
    let project = ProjectRepository { pool }
        .insert(
            actor.workspace_id,
            NewProject {
                name: req.name.trim().to_string(),
                code: req.code.trim().to_uppercase(),
                description: req.description,
            },
        )
        .await?;
    let _ = crate::infrastructure::repositories::notification_repository::AuditRepository::log(
        pool, actor.workspace_id, actor.id, "project.created", Some("PROJECT"),
        Some(&project.id.to_string()), None,
    ).await;
    Ok(project)
}

pub async fn update(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
    req: UpdateProjectRequest,
) -> Result<Project, DomainError> {
    if !can(actor.role, Action::UpdateProject) {
        return Err(DomainError::Forbidden);
    }
    if let (Some(start), Some(end)) = (req.planned_start_date, req.planned_end_date) {
        if end < start {
            return Err(DomainError::Validation {
                message: "Planlanan bitiş, başlangıçtan önce olamaz.".into(),
            });
        }
    }
    ProjectRepository { pool }
        .update(
            actor.workspace_id,
            project_id,
            &ProjectPatch {
                name: req.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
                description: req.description,
                status: req.status,
                planned_start_date: req.planned_start_date.map(Some),
                planned_end_date: req.planned_end_date.map(Some),
            },
        )
        .await
}

/// Arşivleme confirm gerektiren kritik aksiyondur; yalnızca ADMIN/PM (RBAC).
pub async fn archive(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<Project, DomainError> {
    if !can(actor.role, Action::ArchiveProject) {
        return Err(DomainError::Forbidden);
    }
    let archived = ProjectRepository { pool }
        .archive(actor.workspace_id, project_id)
        .await?;
    let _ = crate::infrastructure::repositories::notification_repository::AuditRepository::log(
        pool, actor.workspace_id, actor.id, "project.archived", Some("PROJECT"),
        Some(&archived.id.to_string()), None,
    ).await;
    Ok(archived)
}
