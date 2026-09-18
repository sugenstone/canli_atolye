//! Workspace repository.

use crate::domain::entities::Workspace;
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct WorkspaceRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> WorkspaceRepository<'a> {
    pub async fn insert(
        &self,
        name: &str,
        slug: &str,
        description: Option<&str>,
    ) -> Result<Workspace, DomainError> {
        let ws = Workspace {
            id: new_id(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(Into::into),
            created_at: now(),
            updated_at: now(),
        };
        sqlx::query(
            "INSERT INTO workspaces (id, name, slug, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(ws.id)
        .bind(&ws.name)
        .bind(&ws.slug)
        .bind(&ws.description)
        .bind(ws.created_at)
        .bind(ws.updated_at)
        .execute(self.pool)
        .await?;
        Ok(ws)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Workspace, DomainError> {
        sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?
        .ok_or(DomainError::WorkspaceNotFound)
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Workspace>, DomainError> {
        Ok(sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces WHERE slug = ?1",
        )
        .bind(slug)
        .fetch_optional(self.pool)
        .await?)
    }

    pub async fn first(&self) -> Result<Option<Workspace>, DomainError> {
        Ok(sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(self.pool)
        .await?)
    }

    pub async fn list(&self) -> Result<Vec<Workspace>, DomainError> {
        Ok(sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces ORDER BY created_at",
        )
        .fetch_all(self.pool)
        .await?)
    }

    pub async fn count(&self) -> Result<i64, DomainError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.pool)
            .await?;
        Ok(count)
    }
}
