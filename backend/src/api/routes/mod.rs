//! `/api/v1` route kayıtları.

pub mod auth;
pub mod insights;
pub mod operations;
pub mod processes;
pub mod projects;
pub mod sections;
pub mod teams;
pub mod users;
pub mod work_items;
pub mod workspaces;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::api::state::AppState;

pub fn v1_router() -> Router<AppState> {
    Router::new()
        // Auth
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        // Workspaces
        .route("/workspaces", get(workspaces::list))
        .route("/workspaces/{workspace_id}", get(workspaces::get_one))
        // Kullanıcılar
        .route(
            "/workspaces/{workspace_id}/users",
            get(users::list).post(users::create),
        )
        .route("/users/{user_id}", patch(users::update).delete(users::deactivate))
        // Takımlar
        .route("/teams", get(teams::list).post(teams::create))
        .route("/teams/{team_id}", get(teams::get_one).patch(teams::update))
        .route("/teams/{team_id}/members", get(teams::list_members))
        .route(
            "/teams/{team_id}/members/{user_id}",
            post(teams::add_member).delete(teams::remove_member),
        )
        // Projeler
        .route("/projects", get(projects::list).post(projects::create))
        .route(
            "/projects/{project_id}",
            get(projects::get_one).patch(projects::update),
        )
        .route("/projects/{project_id}/archive", post(projects::archive))
        // Bölümler (recursive sections)
        .route(
            "/projects/{project_id}/sections/tree",
            get(sections::tree),
        )
        .route(
            "/projects/{project_id}/sections",
            post(sections::create),
        )
        .route(
            "/projects/{project_id}/sections/bulk",
            post(sections::bulk_create),
        )
        .route("/sections/{section_id}", patch(sections::update).delete(sections::delete))
        .route("/sections/{section_id}/clone", post(sections::clone))
        // İş kalemi tipleri
        .route("/work-item-types", get(work_items::list_types).post(work_items::create_type))
        // İş kalemleri
        .route("/projects/{project_id}/work-items", get(work_items::list).post(work_items::create))
        .route("/projects/{project_id}/work-items/bulk", post(work_items::bulk_create))
        .route("/work-items/{item_id}", get(work_items::get_one).patch(work_items::update).delete(work_items::delete))
        .route("/work-items/{item_id}/property-values", put(work_items::set_values))
        // Dinamik özellik tanımları
        .route("/property-definitions", get(work_items::list_definitions).post(work_items::create_definition))
        // Süreç motoru
        .route("/process-templates", get(processes::list_templates).post(processes::create_template))
        .route("/process-groups", get(processes::list_groups).post(processes::create_group))
        .route("/process-groups/{group_id}", get(processes::group_detail))
        .route(
            "/work-items/{work_item_id}/assign-process-group",
            post(processes::assign_group),
        )
        .route("/process-executions", get(processes::list_executions))
        .route("/process-executions/{execution_id}", get(processes::get_execution))
        .route("/process-executions/{execution_id}/events", get(processes::list_events))
        .route("/process-executions/{execution_id}/assign", post(processes::assign))
        .route("/process-executions/{execution_id}/start", post(processes::start))
        .route("/process-executions/{execution_id}/pause", post(processes::pause))
        .route("/process-executions/{execution_id}/resume", post(processes::resume_))
        .route("/process-executions/{execution_id}/complete", post(processes::complete))
        .route("/process-executions/{execution_id}/block", post(operations::block))
        .route("/process-executions/{execution_id}/unblock", post(operations::unblock))
        .route("/process-executions/{execution_id}/cancel", post(processes::cancel))
        // Faz 5 operasyonlar
        .route("/block-reasons", get(operations::list_reasons).post(operations::create_reason))
        .route("/process-executions/{execution_id}/notes", post(operations::add_note))
        .route("/process-executions/{execution_id}/blocks", get(operations::list_blocks))
        .route("/attachments", post(operations::upload).get(operations::list))
        .route("/attachments/{attachment_id}/download", get(operations::download))
        .route("/attachments/{attachment_id}", delete(operations::delete))
        // Faz 6: yönetici insight'ları + SSE
        .route("/projects/{project_id}/dashboard-summary", get(insights::dashboard_summary))
        .route("/projects/{project_id}/matrix", get(insights::matrix))
        .route("/projects/{project_id}/flow", get(insights::flow))
        .route("/projects/{project_id}/activity", get(insights::activity))
        .route("/projects/{project_id}/events/stream", get(insights::stream))
        .route("/my-work", get(insights::my_work))
        .route("/projects/{project_id}/reports", get(insights::reports))
        // Faz 8: planlama
        .route("/process-executions/{execution_id}/planned-dates", patch(processes::planned_dates))
        .route("/process-executions/{execution_id}/durations", get(processes::durations))
        .route("/projects/{project_id}/process-executions/bulk-plan", post(processes::bulk_plan))
}
