//! Takım yönetimi service'i.

use crate::application::dto::{AddMemberRequest, CreateTeamRequest, UpdateTeamRequest};
use crate::domain::entities::{Team, TeamMember, TeamRole, User};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::team_repository::TeamRepository;
use crate::infrastructure::repositories::user_repository::UserRepository;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn list(pool: &SqlitePool, actor: &User) -> Result<Vec<Team>, DomainError> {
    if !can(actor.role, Action::ViewTeams) {
        return Err(DomainError::Forbidden);
    }
    TeamRepository { pool }
        .list_by_workspace(actor.workspace_id)
        .await
}

pub async fn create(
    pool: &SqlitePool,
    actor: &User,
    req: CreateTeamRequest,
) -> Result<Team, DomainError> {
    if !can(actor.role, Action::ManageTeams) {
        return Err(DomainError::Forbidden);
    }
    if req.name.trim().is_empty() {
        return Err(DomainError::Validation { message: "Takım adı zorunludur.".into() });
    }
    TeamRepository { pool }
        .insert(actor.workspace_id, req.name.trim(), req.description.as_deref())
        .await
}

pub async fn get(pool: &SqlitePool, actor: &User, team_id: Uuid) -> Result<Team, DomainError> {
    if !can(actor.role, Action::ViewTeams) {
        return Err(DomainError::Forbidden);
    }
    TeamRepository { pool }
        .find_in_workspace(actor.workspace_id, team_id)
        .await
}

pub async fn update(
    pool: &SqlitePool,
    actor: &User,
    team_id: Uuid,
    req: UpdateTeamRequest,
) -> Result<Team, DomainError> {
    if !can(actor.role, Action::ManageTeams) {
        return Err(DomainError::Forbidden);
    }
    if req.name.trim().is_empty() {
        return Err(DomainError::Validation { message: "Takım adı zorunludur.".into() });
    }
    TeamRepository { pool }
        .rename(actor.workspace_id, team_id, req.name.trim(), req.description.as_deref())
        .await
}

pub async fn add_member(
    pool: &SqlitePool,
    actor: &User,
    team_id: Uuid,
    req: AddMemberRequest,
) -> Result<(), DomainError> {
    if !can(actor.role, Action::ManageTeams) {
        return Err(DomainError::Forbidden);
    }
    // İzolasyon: üye aynı workspace'ten olmalı.
    let member_user = UserRepository { pool }.find_by_id(req.user_id).await?;
    if member_user.workspace_id != actor.workspace_id {
        return Err(DomainError::UserNotFound);
    }
    TeamRepository { pool }
        .add_member(actor.workspace_id, team_id, req.user_id, req.role.unwrap_or(TeamRole::Member))
        .await
}

pub async fn remove_member(
    pool: &SqlitePool,
    actor: &User,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<(), DomainError> {
    if !can(actor.role, Action::ManageTeams) {
        return Err(DomainError::Forbidden);
    }
    TeamRepository { pool }
        .remove_member(actor.workspace_id, team_id, user_id)
        .await
}

pub async fn list_members(
    pool: &SqlitePool,
    actor: &User,
    team_id: Uuid,
) -> Result<Vec<TeamMember>, DomainError> {
    if !can(actor.role, Action::ViewTeams) {
        return Err(DomainError::Forbidden);
    }
    TeamRepository { pool }
        .list_members(actor.workspace_id, team_id)
        .await
}
