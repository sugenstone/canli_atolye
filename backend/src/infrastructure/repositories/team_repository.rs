//! Takım repository.

use crate::domain::entities::{Team, TeamMember, TeamRole};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct TeamRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> TeamRepository<'a> {
    pub async fn insert(
        &self,
        workspace_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Team, DomainError> {
        let team = Team {
            id: new_id(),
            workspace_id,
            name: name.to_string(),
            description: description.map(Into::into),
            created_at: now(),
            updated_at: now(),
        };
        sqlx::query(
            "INSERT INTO teams (id, workspace_id, name, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(team.id)
        .bind(team.workspace_id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(team.created_at)
        .bind(team.updated_at)
        .execute(self.pool)
        .await?;
        Ok(team)
    }

    /// Workspace izolasyonu: takım her zaman workspace sınırlı aranır.
    pub async fn find_in_workspace(
        &self,
        workspace_id: Uuid,
        team_id: Uuid,
    ) -> Result<Team, DomainError> {
        sqlx::query_as::<_, Team>(
            "SELECT * FROM teams WHERE id = ?1 AND workspace_id = ?2",
        )
        .bind(team_id)
        .bind(workspace_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or(DomainError::TeamNotFound)
    }

    pub async fn list_by_workspace(&self, workspace_id: Uuid) -> Result<Vec<Team>, DomainError> {
        Ok(sqlx::query_as::<_, Team>(
            "SELECT * FROM teams WHERE workspace_id = ?1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(self.pool)
        .await?)
    }

    pub async fn rename(
        &self,
        workspace_id: Uuid,
        team_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Team, DomainError> {
        let existing = self.find_in_workspace(workspace_id, team_id).await?;
        let updated = Team {
            name: name.to_string(),
            description: description.map(Into::into),
            updated_at: now(),
            ..existing
        };
        sqlx::query(
            "UPDATE teams SET name = ?2, description = ?3, updated_at = ?4 WHERE id = ?1",
        )
        .bind(team_id)
        .bind(&updated.name)
        .bind(&updated.description)
        .bind(updated.updated_at)
        .execute(self.pool)
        .await?;
        Ok(updated)
    }

    // --- Üyelik ---

    pub async fn add_member(
        &self,
        workspace_id: Uuid,
        team_id: Uuid,
        user_id: Uuid,
        role: TeamRole,
    ) -> Result<(), DomainError> {
        // Takım workspace'e ait mi? (izolasyon)
        self.find_in_workspace(workspace_id, team_id).await?;
        sqlx::query(
            "INSERT INTO team_members (team_id, user_id, role) VALUES (?1, ?2, ?3)
             ON CONFLICT (team_id, user_id) DO UPDATE SET role = excluded.role",
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_member(
        &self,
        workspace_id: Uuid,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        self.find_in_workspace(workspace_id, team_id).await?;
        sqlx::query("DELETE FROM team_members WHERE team_id = ?1 AND user_id = ?2")
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_members(
        &self,
        workspace_id: Uuid,
        team_id: Uuid,
    ) -> Result<Vec<TeamMember>, DomainError> {
        self.find_in_workspace(workspace_id, team_id).await?;
        Ok(sqlx::query_as::<_, TeamMember>(
            "SELECT team_id, user_id, role FROM team_members WHERE team_id = ?1",
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await?)
    }

    /// Kullanıcının üyesi olduğu takımlar (worker iş görünürlüğü için).
    pub async fn teams_of_user(&self, user_id: Uuid) -> Result<Vec<Team>, DomainError> {
        Ok(sqlx::query_as::<_, Team>(
            "SELECT t.* FROM teams t
             JOIN team_members tm ON tm.team_id = t.id
             WHERE tm.user_id = ?1
             ORDER BY t.name",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?)
    }
}
