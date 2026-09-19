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

// ---------------------------------------------------------------------------
// Bildirimler (§34) · Arama (§33) · CSV dışa aktarım (§23) · Denetim (§59)
// ---------------------------------------------------------------------------

use crate::infrastructure::repositories::notification_repository::{
    AdminAuditLog, AuditRepository, Notification, NotificationRepository,
};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// GET /api/v1/notifications — kullanıcının bildirimleri
pub async fn notifications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<axum::Json<Vec<Notification>>> {
    Ok(axum::Json(
        NotificationRepository::list_for_user(&state.pool, auth.user.workspace_id, auth.user.id).await?,
    ))
}

/// POST /api/v1/notifications/{id}/read
pub async fn read_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(notification_id): Path<Uuid>,
) -> ApiResult<axum::Json<serde_json::Value>> {
    NotificationRepository::mark_read(&state.pool, auth.user.workspace_id, auth.user.id, notification_id).await?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}

/// POST /api/v1/notifications/read-all
pub async fn read_all_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<axum::Json<serde_json::Value>> {
    NotificationRepository::mark_all_read(&state.pool, auth.user.workspace_id, auth.user.id).await?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SearchHit {
    pub kind: String,
    pub project_id: Uuid,
    pub section_id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub subtitle: String,
}

/// GET /api/v1/search?q= — iş kalemi adı/kod + bölüm adı/kod (§33)
pub async fn search(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<SearchQuery>,
) -> ApiResult<axum::Json<Vec<SearchHit>>> {
    let needle = format!("%{}%", query.q.trim().replace('%', ""));
    if query.q.trim().len() < 2 {
        return Ok(axum::Json(Vec::new()));
    }
    let hits: Vec<SearchHit> = sqlx::query_as(
        "SELECT 'work_item' AS kind, wi.project_id, sec.id AS section_id, wi.id AS work_item_id,
                wi.name AS title,
                COALESCE(parent.name || ' / ' || sec.name, sec.name) AS subtitle
         FROM work_items wi
         JOIN sections sec ON sec.id = wi.section_id
         LEFT JOIN sections parent ON parent.id = sec.parent_id
         WHERE wi.workspace_id = ?1 AND wi.deleted_at IS NULL
           AND (wi.name LIKE ?2 OR wi.code LIKE ?2)
         LIMIT 20",
    )
    .bind(auth.user.workspace_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await?;

    let section_hits: Vec<SearchHit> = sqlx::query_as(
        "SELECT 'section' AS kind, s.project_id, s.id AS section_id, NULL AS work_item_id,
                s.name AS title, COALESCE(p.name || ' / ' || s.name, s.name) AS subtitle
         FROM sections s LEFT JOIN sections p ON p.id = s.parent_id
         WHERE s.workspace_id = ?1 AND s.deleted_at IS NULL
           AND (s.name LIKE ?2 OR s.code LIKE ?2)
         LIMIT 10",
    )
    .bind(auth.user.workspace_id)
    .bind(&needle)
    .fetch_all(&state.pool)
    .await?;

    let mut all = hits;
    all.extend(section_hits);
    Ok(axum::Json(all))
}

/// GET /api/v1/projects/{project_id}/export.csv — iş kalemi listesi (BOM + ;)
pub async fn export_csv(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> ApiResult<Response> {
    service::activity(&state.pool, &auth.user, project_id, 1).await?; // yetki + proje kontrolü

    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT COALESCE(parent.name || ' / ' || sec.name, sec.name),
                wi.name,
                COALESCE(t.name, ''),
                wi.priority,
                COALESCE((SELECT GROUP_CONCAT(t2.name || ': ' || e.status)
                          FROM process_executions e JOIN process_templates t2 ON t2.id = e.process_template_id
                          WHERE e.work_item_id = wi.id AND e.status != 'CANCELLED'), '')
         FROM work_items wi
         JOIN sections sec ON sec.id = wi.section_id
         LEFT JOIN sections parent ON parent.id = sec.parent_id
         LEFT JOIN work_item_types t ON t.id = wi.work_item_type_id
         WHERE wi.workspace_id = ?1 AND wi.project_id = ?2 AND wi.deleted_at IS NULL
         ORDER BY sec.sort_order, wi.created_at",
    )
    .bind(auth.user.workspace_id)
    .bind(project_id)
    .fetch_all(&state.pool)
    .await?;

    let mut csv = String::from("\u{FEFF}B\u{00f6}l\u{00fc}m;\u{0130}\u{015f} Kalemi;Tip;\u{00d6}ncelik;S\u{00fc}re\u{00e7}ler\r\n");
    for (path, item, type_, priority, processes) in rows {
        csv.push_str(&format!(
            "{};{};{};{};{}\r\n",
            path.replace(';', ","),
            item.replace(';', ","),
            type_,
            priority,
            processes.replace(';', ",")
        ));
    }
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"is-kalemleri-{project_id}.csv\""),
            ),
        ],
        csv,
    )
        .into_response())
}

/// GET /api/v1/audit-logs (ADMIN) — son yönetimsel işlemler
pub async fn audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<axum::Json<Vec<AdminAuditLog>>> {
    use crate::domain::entities::Role;
    if auth.user.role != Role::Admin {
        return Err(crate::api::error::ApiError::from(
            crate::domain::errors::DomainError::Forbidden,
        ));
    }
    Ok(axum::Json(AuditRepository::list(&state.pool, auth.user.workspace_id, 50).await?))
}
