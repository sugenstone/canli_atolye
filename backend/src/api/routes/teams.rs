//! Takım endpoint'leri.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::dto::{AddMemberRequest, CreateTeamRequest, UpdateTeamRequest};
use crate::application::services::team_service;
use crate::domain::entities::{Team, TeamMember};
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

/// GET /api/v1/teams
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<Team>>> {
    Ok(Json(team_service::list(&state.pool, &auth.user).await?))
}

/// POST /api/v1/teams
pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTeamRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Team>)> {
    let team = team_service::create(&state.pool, &auth.user, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(team)))
}

/// GET /api/v1/teams/{team_id}
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<Team>> {
    Ok(Json(team_service::get(&state.pool, &auth.user, team_id).await?))
}

/// PATCH /api/v1/teams/{team_id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<Uuid>,
    Json(req): Json<UpdateTeamRequest>,
) -> ApiResult<Json<Team>> {
    Ok(Json(team_service::update(&state.pool, &auth.user, team_id, req).await?))
}

/// GET /api/v1/teams/{team_id}/members
pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamMember>>> {
    Ok(Json(team_service::list_members(&state.pool, &auth.user, team_id).await?))
}

/// POST /api/v1/teams/{team_id}/members/{user_id}
pub async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((team_id, _user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AddMemberRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    team_service::add_member(&state.pool, &auth.user, team_id, req).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/v1/teams/{team_id}/members/{user_id}
pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    team_service::remove_member(&state.pool, &auth.user, team_id, user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
