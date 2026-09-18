//! Faz 6 sorgu repository'leri: dashboard aggregate, matris hücreleri,
//! üretim akışı kartları ve aktivite akışı. Tüm sorgular workspace sınırlı;
//! event tablosunu istemciye dökmez (MASTER PLAN §60).

use crate::domain::entities::ProcessStatus;
use crate::domain::errors::DomainError;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct InsightRepository;

/// Bir section'daki iş kalemlerinin execution durumları (hücre özeti için).
#[derive(Debug, sqlx::FromRow)]
pub struct CellRow {
    pub work_item_id: Uuid,
    pub work_item_name: String,
    pub status: ProcessStatus,
    pub planned_end_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ProcessSummaryRow {
    pub template_id: Uuid,
    pub template_name: String,
    pub total: i64,
    pub completed: i64,
    pub in_progress: i64,
    pub blocked: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkItemSummary {
    pub total: i64,
    pub completed: i64,
    pub in_progress: i64,
    pub blocked: i64,
    pub pending: i64,
    /// Geciken iş kalemleri (aktif + plan bitişi geçti — MASTER PLAN §35)
    pub late: i64,
    /// Bugün tamamlanan süreç adımı (MASTER PLAN §30 TV modu)
    pub completed_today: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct FlowCard {
    pub execution_id: Uuid,
    pub template_name: String,
    pub status: ProcessStatus,
    pub work_item_id: Uuid,
    pub work_item_name: String,
    pub section_id: Uuid,
    pub section_path: String,
    pub planned_end_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ActivityRow {
    pub timestamp: String,
    pub event_type: String,
    pub user_name: String,
    pub work_item_name: String,
    pub section_path: String,
    pub note: Option<String>,
}

impl InsightRepository {
    /// Proje iş kalemi + süreç özeti (KPI kartları ve ilerleme çubukları).
    pub async fn dashboard_summary(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<(WorkItemSummary, Vec<ProcessSummaryRow>), DomainError> {
        // İş kalemi başına özet durum (aggregation Rust'ta)
        let item_rows = sqlx::query_as::<_, (Uuid, ProcessStatus)>(
            "SELECT DISTINCT e.work_item_id, e.status
             FROM process_executions e
             WHERE e.workspace_id = ?1
               AND e.work_item_id IN (SELECT id FROM work_items WHERE project_id = ?2 AND deleted_at IS NULL)
               AND e.status != 'CANCELLED'",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        use std::collections::HashMap;
        let mut by_item: HashMap<Uuid, Vec<ProcessStatus>> = HashMap::new();
        for (item, status) in item_rows {
            by_item.entry(item).or_default().push(status);
        }
        let summaries: Vec<Option<ProcessStatus>> =
            by_item.values().map(|v| crate::domain::services::aggregate::summarize_statuses(v)).collect();

        let late_rows = sqlx::query_as::<_, (Uuid, Option<chrono::DateTime<chrono::Utc>>, ProcessStatus)>(
            "SELECT DISTINCT e.work_item_id, e.planned_end_at, e.status
             FROM process_executions e
             WHERE e.workspace_id = ?1
               AND e.work_item_id IN (SELECT id FROM work_items WHERE project_id = ?2 AND deleted_at IS NULL)
               AND e.status NOT IN ('COMPLETED', 'CANCELLED')
               AND e.planned_end_at IS NOT NULL",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?;
        let now = crate::shared::now();
        let late_items: std::collections::HashSet<Uuid> = late_rows
            .into_iter()
            .filter(|(_, planned, _)| planned.map(|p| p < now).unwrap_or(false))
            .map(|(item, _, _)| item)
            .collect();

        let today_start = crate::shared::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc())
            .unwrap_or_else(crate::shared::now);
        let (completed_today,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM process_executions e
             WHERE e.workspace_id = ?1
               AND e.work_item_id IN (SELECT id FROM work_items WHERE project_id = ?2 AND deleted_at IS NULL)
               AND e.status = 'COMPLETED' AND e.completed_at >= ?3",
        )
        .bind(workspace_id)
        .bind(project_id)
        .bind(today_start)
        .fetch_one(pool)
        .await?;

        let total_items: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM work_items WHERE workspace_id = ?1 AND project_id = ?2 AND deleted_at IS NULL",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let mut wi = WorkItemSummary { total: total_items.0, completed: 0, in_progress: 0, blocked: 0, pending: 0, late: late_items.len() as i64, completed_today };
        for s in summaries.into_iter().flatten() {
            match s {
                ProcessStatus::Completed => wi.completed += 1,
                ProcessStatus::InProgress => wi.in_progress += 1,
                ProcessStatus::Blocked => wi.blocked += 1,
                _ => wi.pending += 1,
            }
        }

        // Süreç şablonu bazında sayılar
        let process_rows = sqlx::query_as::<_, ProcessSummaryRow>(
            "SELECT t.id AS template_id, t.name AS template_name,
                    COUNT(*) AS total,
                    SUM(CASE WHEN e.status = 'COMPLETED' THEN 1 ELSE 0 END) AS completed,
                    SUM(CASE WHEN e.status = 'IN_PROGRESS' THEN 1 ELSE 0 END) AS \"in_progress\",
                    SUM(CASE WHEN e.status = 'BLOCKED' THEN 1 ELSE 0 END) AS blocked
             FROM process_executions e
             JOIN process_templates t ON t.id = e.process_template_id
             JOIN process_group_steps s ON s.id = e.process_group_step_id
             WHERE e.workspace_id = ?1
               AND e.work_item_id IN (SELECT id FROM work_items WHERE project_id = ?2 AND deleted_at IS NULL)
             GROUP BY t.id
             ORDER BY MIN(s.sort_order)",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok((wi, process_rows))
    }

    /// Bir section'daki hücre verisi: item adları + execution durumları.
    pub async fn cell_rows(
        pool: &SqlitePool,
        workspace_id: Uuid,
        section_id: Uuid,
    ) -> Result<Vec<CellRow>, DomainError> {
        Ok(sqlx::query_as::<_, CellRow>(
            "SELECT wi.id AS work_item_id, wi.name AS work_item_name, e.status, e.planned_end_at
             FROM work_items wi
             JOIN process_executions e ON e.work_item_id = wi.id AND e.status != 'CANCELLED'
             WHERE wi.workspace_id = ?1 AND wi.section_id = ?2 AND wi.deleted_at IS NULL
             ORDER BY wi.created_at",
        )
        .bind(workspace_id)
        .bind(section_id)
        .fetch_all(pool)
        .await?)
    }

    /// Üretim akışı kartları: execution + iş kalemi + bölüm yolu (iki seviye).
    pub async fn flow_cards(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<FlowCard>, DomainError> {
        Ok(sqlx::query_as::<_, FlowCard>(
            "SELECT e.id AS execution_id, t.name AS template_name, e.status,
                    wi.id AS work_item_id, wi.name AS work_item_name,
                    sec.id AS section_id,
                    COALESCE(parent.name || ' / ' || sec.name, sec.name) AS section_path,
                    e.planned_end_at
             FROM process_executions e
             JOIN process_templates t ON t.id = e.process_template_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.deleted_at IS NULL
             JOIN sections sec ON sec.id = wi.section_id
             LEFT JOIN sections parent ON parent.id = sec.parent_id
             WHERE e.workspace_id = ?1 AND wi.project_id = ?2 AND e.status NOT IN ('CANCELLED','COMPLETED')
             ORDER BY t.name, wi.name",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    /// Aktivite akışı: projenin son süreç olayları.
    pub async fn activity(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ActivityRow>, DomainError> {
        Ok(sqlx::query_as::<_, ActivityRow>(
            "SELECT pe.timestamp, pe.event_type,
                    u.full_name AS user_name,
                    wi.name AS work_item_name,
                    COALESCE(parent.name || ' / ' || sec.name, sec.name) AS section_path,
                    pe.note
             FROM process_events pe
             JOIN process_executions e ON e.id = pe.process_execution_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.deleted_at IS NULL
             JOIN sections sec ON sec.id = wi.section_id
             LEFT JOIN sections parent ON parent.id = sec.parent_id
             JOIN users u ON u.id = pe.user_id
             WHERE pe.workspace_id = ?1 AND wi.project_id = ?2
             ORDER BY pe.timestamp DESC
             LIMIT ?3",
        )
        .bind(workspace_id)
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }
}
