//! Proje repository.

use crate::domain::entities::{Project, ProjectStatus};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct NewProject {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
}

pub struct ProjectPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProjectStatus>,
    pub planned_start_date: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub planned_end_date: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

pub struct ProjectRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> ProjectRepository<'a> {
    pub async fn insert(
        &self,
        workspace_id: Uuid,
        input: NewProject,
    ) -> Result<Project, DomainError> {
        let project = Project {
            id: new_id(),
            workspace_id,
            name: input.name,
            code: input.code,
            description: input.description,
            status: ProjectStatus::Draft,
            planned_start_date: None,
            planned_end_date: None,
            actual_start_date: None,
            actual_end_date: None,
            created_at: now(),
            updated_at: now(),
            deleted_at: None,
        };
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, name, code, description, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(project.id)
        .bind(project.workspace_id)
        .bind(&project.name)
        .bind(&project.code)
        .bind(&project.description)
        .bind(project.status)
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(self.pool)
        .await?;
        Ok(project)
    }

    /// Workspace izolasyonu: proje her zaman workspace sınırlı aranır,
    /// silinmiş (archived/soft-deleted) kayıtlar listede yer almaz.
    pub async fn find_in_workspace(
        &self,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Project, DomainError> {
        sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
        )
        .bind(project_id)
        .bind(workspace_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or(DomainError::ProjectNotFound)
    }

    pub async fn list_by_workspace(
        &self,
        workspace_id: Uuid,
        status: Option<ProjectStatus>,
    ) -> Result<Vec<Project>, DomainError> {
        match status {
            Some(status) => Ok(sqlx::query_as::<_, Project>(
                "SELECT * FROM projects
                 WHERE workspace_id = ?1 AND status = ?2 AND deleted_at IS NULL
                 ORDER BY created_at DESC",
            )
            .bind(workspace_id)
            .bind(status)
            .fetch_all(self.pool)
            .await?),
            None => Ok(sqlx::query_as::<_, Project>(
                "SELECT * FROM projects
                 WHERE workspace_id = ?1 AND deleted_at IS NULL
                 ORDER BY created_at DESC",
            )
            .bind(workspace_id)
            .fetch_all(self.pool)
            .await?),
        }
    }

    pub async fn update(
        &self,
        workspace_id: Uuid,
        project_id: Uuid,
        patch: &ProjectPatch,
    ) -> Result<Project, DomainError> {
        let existing = self.find_in_workspace(workspace_id, project_id).await?;
        let updated = Project {
            name: patch.name.clone().unwrap_or(existing.name),
            description: patch
                .description
                .clone()
                .or(existing.description.clone()),
            status: patch.status.unwrap_or(existing.status),
            planned_start_date: patch
                .planned_start_date
                .unwrap_or(existing.planned_start_date),
            planned_end_date: patch.planned_end_date.unwrap_or(existing.planned_end_date),
            updated_at: now(),
            ..existing
        };
        sqlx::query(
            "UPDATE projects
             SET name = ?2, description = ?3, status = ?4,
                 planned_start_date = ?5, planned_end_date = ?6, updated_at = ?7
             WHERE id = ?1",
        )
        .bind(project_id)
        .bind(&updated.name)
        .bind(&updated.description)
        .bind(updated.status)
        .bind(updated.planned_start_date)
        .bind(updated.planned_end_date)
        .bind(updated.updated_at)
        .execute(self.pool)
        .await?;
        Ok(updated)
    }

    /// Arşivleme = status ARCHIVED + soft delete (geçmiş korunur).
    pub async fn archive(
        &self,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Project, DomainError> {
        let mut project = self.find_in_workspace(workspace_id, project_id).await?;
        project.status = ProjectStatus::Archived;
        project.deleted_at = Some(now());
        project.updated_at = now();
        sqlx::query(
            "UPDATE projects SET status = ?2, deleted_at = ?3, updated_at = ?4 WHERE id = ?1",
        )
        .bind(project_id)
        .bind(project.status)
        .bind(project.deleted_at)
        .bind(project.updated_at)
        .execute(self.pool)
        .await?;
        Ok(project)
    }
}
