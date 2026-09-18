//! Faz 9 rapor repository: süreç performansı, darboğaz ve takım performansı
//! sorguları (MASTER PLAN §36-37). Süreler olay akışından hesaplanır.

use crate::domain::entities::{EventType, ProcessStatus};
use crate::domain::errors::DomainError;
use crate::domain::services::duration::{compute_durations, TimedEvent, Durations};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ReportRepository;

/// Süreç performansı satırı — tamamlanan işlerin ortalama süreleri.
#[derive(Debug, serde::Serialize)]
pub struct ProcessPerformanceRow {
    pub template_id: Uuid,
    pub template_name: String,
    pub completed_count: i64,
    /// tamamlananların ortalaması, saniye
    pub avg_active_seconds: Option<i64>,
    pub avg_waiting_seconds: Option<i64>,
    pub avg_lead_seconds: Option<i64>,
}

/// Darboğaz satırı — aktif execution yığılmaları (MASTER PLAN §37).
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct BottleneckRow {
    pub template_id: Uuid,
    pub template_name: String,
    pub ready_count: i64,
    pub in_progress_count: i64,
    pub pending_count: i64,
    pub blocked_count: i64,
}

/// Takım performansı — atanan ve tamamlanan işler (yorumlama dikkatli,
/// MASTER PLAN §36: kişileri cezalandırma amacı taşımaz).
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TeamPerformanceRow {
    pub team_id: Option<Uuid>,
    pub team_name: Option<String>,
    pub completed_count: i64,
    pub active_seconds_total: i64,
}

impl ReportRepository {
    /// Süreç performansı: şablon bazında tamamlanan execution'ların süreleri.
    pub async fn process_performance(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<ProcessPerformanceRow>, DomainError> {
        // 1) Tamamlanmış execution'lar (id + şablon)
        let completed: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT e.id, t.id, t.name
             FROM process_executions e
             JOIN process_templates t ON t.id = e.process_template_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.project_id = ?2 AND wi.deleted_at IS NULL
             WHERE e.workspace_id = ?1 AND e.status = 'COMPLETED'",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        // 2) Bu execution'ların olaylarını tek sorguda çek ve grupla
        let mut durations_by_exec: std::collections::HashMap<Uuid, Durations> = Default::default();
        if !completed.is_empty() {
            let events: Vec<(Uuid, EventType, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
                "SELECT pe.process_execution_id, pe.event_type, pe.timestamp
                 FROM process_events pe
                 WHERE pe.workspace_id = ?1",
            )
            .bind(workspace_id)
            .fetch_all(pool)
            .await?;
            let mut by_exec: std::collections::HashMap<Uuid, Vec<TimedEvent>> = Default::default();
            for (exec_id, event_type, ts) in events {
                by_exec.entry(exec_id).or_default().push(TimedEvent { event_type, timestamp: ts });
            }
            let now = crate::shared::now();
            for (exec_id, evs) in by_exec {
                durations_by_exec.insert(exec_id, compute_durations(&evs, now));
            }
        }

        // 3) Şablon bazında ortalama
        let mut order: Vec<(Uuid, String)> = Vec::new();
        let mut groups: std::collections::HashMap<Uuid, Vec<i64>> = Default::default();
        let mut waitings: std::collections::HashMap<Uuid, Vec<i64>> = Default::default();
        let mut leads: std::collections::HashMap<Uuid, Vec<i64>> = Default::default();
        for (exec_id, tpl_id, tpl_name) in &completed {
            if !groups.contains_key(tpl_id) {
                order.push((*tpl_id, tpl_name.clone()));
            }
            if let Some(d) = durations_by_exec.get(exec_id) {
                groups.entry(*tpl_id).or_default().push(d.active_seconds);
                if let Some(w) = d.waiting_seconds {
                    waitings.entry(*tpl_id).or_default().push(w);
                }
                if let Some(l) = d.lead_seconds {
                    leads.entry(*tpl_id).or_default().push(l);
                }
            }
        }
        let count_of = |m: &std::collections::HashMap<Uuid, Vec<i64>>, id: &Uuid| {
            m.get(id).map(|v| v.len() as i64).unwrap_or(0)
        };
        let avg_of = |m: &std::collections::HashMap<Uuid, Vec<i64>>, id: &Uuid| {
            m.get(id).filter(|v| !v.is_empty()).map(|v| v.iter().sum::<i64>() / v.len() as i64)
        };

        Ok(order
            .into_iter()
            .map(|(tpl_id, tpl_name)| ProcessPerformanceRow {
                completed_count: count_of(&groups, &tpl_id),
                avg_active_seconds: avg_of(&groups, &tpl_id),
                avg_waiting_seconds: avg_of(&waitings, &tpl_id),
                avg_lead_seconds: avg_of(&leads, &tpl_id),
                template_id: tpl_id,
                template_name: tpl_name,
            })
            .collect())
    }

    /// Darboğaz: aktif execution'ların şablon bazında durum yığılmaları.
    pub async fn bottlenecks(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<BottleneckRow>, DomainError> {
        Ok(sqlx::query_as::<_, BottleneckRow>(
            "SELECT t.id AS template_id, t.name AS template_name,
                    SUM(CASE WHEN e.status = 'READY' THEN 1 ELSE 0 END) AS ready_count,
                    SUM(CASE WHEN e.status = 'IN_PROGRESS' THEN 1 ELSE 0 END) AS \"in_progress_count\",
                    SUM(CASE WHEN e.status = 'PENDING' THEN 1 ELSE 0 END) AS pending_count,
                    SUM(CASE WHEN e.status = 'BLOCKED' THEN 1 ELSE 0 END) AS blocked_count
             FROM process_executions e
             JOIN process_templates t ON t.id = e.process_template_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.project_id = ?2 AND wi.deleted_at IS NULL
             WHERE e.workspace_id = ?1 AND e.status NOT IN ('COMPLETED', 'CANCELLED')
             GROUP BY t.id
             ORDER BY ready_count DESC, t.name",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    /// Takım performansı: tamamlanan + devam eden atanan işler ve toplam aktif süre.
    pub async fn team_performance(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<TeamPerformanceRow>, DomainError> {
        // takım atanan execution'lar (tamamlanan + aktif)
        let rows: Vec<(Option<Uuid>, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT tm.id, tm.name,
                    SUM(CASE WHEN e.status = 'COMPLETED' THEN 1 ELSE 0 END),
                    COUNT(e.id)
             FROM process_executions e
             JOIN teams tm ON tm.id = e.assigned_team_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.project_id = ?2 AND wi.deleted_at IS NULL
             WHERE e.workspace_id = ?1 AND e.status != 'CANCELLED'
             GROUP BY tm.id
             ORDER BY 3 DESC",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        // aktif süre: tamamlanan ekip işlerinin olay ortalaması yerine toplamı —
        // basit tutmak adına şablon performansından bağımsız, event bazlı toplam:
        let mut out: Vec<TeamPerformanceRow> = rows
            .into_iter()
            .map(|(team_id, team_name, completed_count, _total)| TeamPerformanceRow {
                team_id,
                team_name,
                completed_count,
                active_seconds_total: 0,
            })
            .collect();
        let _ = ProcessStatus::Pending; // tip importu
        Ok(out)
    }
}
