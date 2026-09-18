//! Faz 6 endpoint'leri: dashboard-summary, matrix, flow, activity + SSE stream.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::services::insight_service as service;
use crate::infrastructure::realtime::{process_event_name, SseEvent};
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

/// GET /api/v1/my-work — kullanıcının aktif işleri (çalışan ekranı).
pub async fn my_work(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<axum::Json<Vec<crate::infrastructure::repositories::my_work_repository::MyWorkCard>>> {
    use crate::infrastructure::repositories::my_work_repository::MyWorkRepository;
    Ok(axum::Json(
        MyWorkRepository::list(&state.pool, auth.user.workspace_id, auth.user.id).await?,
    ))
}

/// GET /api/v1/projects/{project_id}/reports — Faz 9 raporu
pub async fn reports(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<axum::Json<crate::application::services::insight_service::ProjectReport>> {
    Ok(axum::Json(
        crate::application::services::insight_service::project_report(
            &state.pool, &auth.user, project_id,
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct MatrixQuery {
    pub parent: Uuid,
}

/// GET /api/v1/projects/{project_id}/dashboard-summary
pub async fn dashboard_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<axum::Json<service::DashboardSummary>> {
    Ok(axum::Json(
        service::dashboard_summary(&state.pool, &auth.user, project_id).await?,
    ))
}

/// GET /api/v1/projects/{project_id}/matrix?parent=...
pub async fn matrix(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<MatrixQuery>,
) -> ApiResult<axum::Json<service::MatrixResponse>> {
    Ok(axum::Json(
        service::matrix(&state.pool, &auth.user, project_id, query.parent).await?,
    ))
}

/// GET /api/v1/projects/{project_id}/flow
pub async fn flow(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<axum::Json<Vec<crate::infrastructure::repositories::insight_repository::FlowCard>>> {
    Ok(axum::Json(service::flow(&state.pool, &auth.user, project_id).await?))
}

#[derive(Debug, Deserialize, Default)]
pub struct ActivityQuery {
    pub limit: Option<i64>,
}

/// GET /api/v1/projects/{project_id}/activity
pub async fn activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ActivityQuery>,
) -> ApiResult<axum::Json<Vec<crate::infrastructure::repositories::insight_repository::ActivityRow>>> {
    Ok(axum::Json(
        service::activity(&state.pool, &auth.user, project_id, query.limit.unwrap_or(50)).await?,
    ))
}

/// GET /api/v1/projects/{project_id}/events/stream — SSE (MASTER PLAN §100).
/// Bağlantı açık kaldıkça sunucudan canlı olaylar akar; heartbeat 15 sn.
pub async fn stream(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    // Yetki + proje kontrolü bağlantı açılırken bir kez yapılır
    service::activity(&state.pool, &auth.user, project_id, 1).await?;

    let rx = state.hub.subscribe(project_id);
    let stream = futures_util::StreamExt::map(
        tokio_stream::wrappers::BroadcastStream::new(rx),
        |msg| {
        Ok(match msg {
            Ok(event) => Event::default()
                .event(process_event_name_from(&event))
                .data(serde_json::to_string(&event).unwrap_or_default()),
            Err(_) => Event::default().comment("subscriber lagged"),
        })
        },
    );

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

fn process_event_name_from(event: &SseEvent) -> String {
    process_event_name(&event.event)
}

/// Execution kimliğinden çözerek yayın (not ekleme gibi execution-bazlı aksiyonlar).
pub async fn publish_for_execution(
    state: &AppState,
    execution_id: Uuid,
    event_type: &str,
) {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT work_item_id FROM process_executions WHERE id = ?1 AND deleted_at IS NULL",
    )
    .bind(execution_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if let Some((work_item_id,)) = row {
        publish_execution_event(state, work_item_id, Some(execution_id), event_type).await;
    }
}

/// Aksiyon handler'larından yayın yardımcısı (commit sonrası çağrılır).
/// project_id execution'dan değil iş kaleminden çözülür (tek küçük sorgu).
pub async fn publish_execution_event(
    state: &AppState,
    work_item_id: Uuid,
    execution_id: Option<Uuid>,
    event_type: &str,
) {
    let project: Option<(Uuid,)> = sqlx::query_as(
        "SELECT project_id FROM work_items WHERE id = ?1 AND deleted_at IS NULL",
    )
    .bind(work_item_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if let Some((project_id,)) = project {
        state.hub.publish(SseEvent {
            event: event_type.to_string(),
            project_id,
            work_item_id: Some(work_item_id),
            process_execution_id: execution_id,
            timestamp: crate::shared::now(),
        });
    }
}
