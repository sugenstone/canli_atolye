//! Faz 6 insight service'i: dashboard özeti, matris, üretim akışı,
//! aktivite. Tümü RBAC + workspace izolasyonlu.

use crate::domain::entities::{Section, User};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::domain::services::aggregate::summarize_statuses;
use crate::infrastructure::repositories::insight_repository::{
    InsightRepository, ProcessSummaryRow, WorkItemSummary,
};
use crate::infrastructure::repositories::project_repository::ProjectRepository;
use crate::infrastructure::repositories::section_repository::SectionRepository;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MatrixCell {
    pub status: String,
    pub item_count: i64,
    pub label: String,
    /// Hücrenin section kimliği (drawer açmak için)
    pub section_id: Uuid,
    /// Hücrede geciken (LATE) execution var mı — MASTER PLAN §35
    pub late: bool,
}

/// Sütun = pozisyon (her satırın kendi çocukları sırayla dizilir;
/// "Daire 1..N" gibi tekrar eden adlar tek sütun başlığına indirgenir).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatrixCol {
    pub position: i64,
    pub label: String,
}

#[derive(Debug, serde::Serialize)]
pub struct MatrixResponse {
    pub parent: Section,
    pub rows: Vec<Section>,
    pub cols: Vec<MatrixCol>,
    /// cells[row_id][position]
    pub cells: std::collections::HashMap<String, std::collections::HashMap<String, MatrixCell>>,
}

#[derive(Debug, serde::Serialize)]
pub struct DashboardSummary {
    pub work_items: WorkItemSummary,
    pub processes: Vec<ProcessSummaryRow>,
}

fn ensure_viewer(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

async fn ensure_project(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<(), DomainError> {
    ProjectRepository { pool }
        .find_in_workspace(actor.workspace_id, project_id)
        .await
        .map(|_| ())
}

pub async fn dashboard_summary(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<DashboardSummary, DomainError> {
    ensure_viewer(actor)?;
    ensure_project(pool, actor, project_id).await?;
    let (work_items, processes) =
        InsightRepository::dashboard_summary(pool, actor.workspace_id, project_id).await?;
    Ok(DashboardSummary { work_items, processes })
}

/// Matris: parent'ın çocukları satır, torunlarının birleşimi sütun.
/// Hücre = sütun section'ındaki iş kalemlerinin özet durumu (MASTER PLAN §27).
pub async fn matrix(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
    parent_id: Uuid,
) -> Result<MatrixResponse, DomainError> {
    ensure_viewer(actor)?;
    ensure_project(pool, actor, project_id).await?;

    let parent = SectionRepository::new()
        .find_in_workspace(pool, actor.workspace_id, parent_id)
        .await?;
    if parent.project_id != project_id {
        return Err(DomainError::NotFound);
    }

    let all = SectionRepository::new()
        .list_by_project(pool, actor.workspace_id, project_id)
        .await?;
    use std::collections::HashMap;
    let mut by_parent: HashMap<Uuid, Vec<Section>> = HashMap::new();
    for s in &all {
        if let Some(p) = s.parent_id {
            by_parent.entry(p).or_default().push(s.clone());
        }
    }

    let rows = by_parent.remove(&parent_id).unwrap_or_default();
    if rows.is_empty() {
        return Err(DomainError::Validation {
            message: "Bu bölümün altında satır olacak bölüm yok.".into(),
        });
    }

    // Sütunlar: POZİSYON bazlı — en çok çocuğu olan satırın uzunluğu kadar;
    // etiket, o pozisyondaki en sık görülen ad (ör. "Daire 1").
    let col_count = rows
        .iter()
        .filter_map(|r| by_parent.get(&r.id))
        .map(|c| c.len())
        .max()
        .unwrap_or(0);
    let mut cols: Vec<MatrixCol> = Vec::with_capacity(col_count);
    for pos in 0..col_count {
        let mut counts: std::collections::HashMap<&str, usize> = Default::default();
        for row in &rows {
            if let Some(children) = by_parent.get(&row.id) {
                if let Some(child) = children.get(pos) {
                    *counts.entry(child.name.as_str()).or_default() += 1;
                }
            }
        }
        let label = counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(name, _)| name.to_string())
            .unwrap_or_else(|| format!("Sütun {}", pos + 1));
        cols.push(MatrixCol { position: pos as i64 + 1, label });
    }

    // Hücreler: (row, pozisyon) → row'un o pozisyondaki çocuğunun özeti
    let mut cells: std::collections::HashMap<String, std::collections::HashMap<String, MatrixCell>> =
        Default::default();
    for row in &rows {
        let children = by_parent.get(&row.id).cloned().unwrap_or_default();
        for (pos, col_section) in children.iter().enumerate() {
            let cell_rows =
                InsightRepository::cell_rows(pool, actor.workspace_id, col_section.id).await?;
            if cell_rows.is_empty() {
                continue;
            }
            use std::collections::HashMap as Map;
            let mut by_item: Map<Uuid, Vec<crate::domain::entities::ProcessStatus>> = Map::new();
            let mut names: Map<Uuid, String> = Map::new();
            for r in &cell_rows {
                by_item.entry(r.work_item_id).or_default().push(r.status);
                names.insert(r.work_item_id, r.work_item_name.clone());
            }
            let statuses: Vec<_> = by_item.values().map(|v| summarize_statuses(v)).collect();
            let summary = summarize_statuses(
                &statuses.iter().filter_map(|s| *s).collect::<Vec<_>>(),
            )
            .unwrap_or(crate::domain::entities::ProcessStatus::Pending);
            let now = crate::shared::now();
            let late = cell_rows.iter().any(|r| {
                r.status != crate::domain::entities::ProcessStatus::Completed
                    && r.planned_end_at.map(|p| p < now).unwrap_or(false)
            });
            let cell = MatrixCell {
                status: summary.to_string(),
                item_count: by_item.len() as i64,
                label: names.values().cloned().next().unwrap_or_default(),
                section_id: col_section.id,
                late,
            };
            cells
                .entry(row.id.to_string())
                .or_default()
                .insert((pos + 1).to_string(), cell);
        }
    }

        Ok(MatrixResponse { parent, rows, cols, cells })
}

pub async fn flow(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<Vec<crate::infrastructure::repositories::insight_repository::FlowCard>, DomainError> {
    ensure_viewer(actor)?;
    ensure_project(pool, actor, project_id).await?;
    InsightRepository::flow_cards(pool, actor.workspace_id, project_id).await
}

pub async fn activity(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::infrastructure::repositories::insight_repository::ActivityRow>, DomainError> {
    ensure_viewer(actor)?;
    ensure_project(pool, actor, project_id).await?;
    InsightRepository::activity(pool, actor.workspace_id, project_id, limit.clamp(1, 200)).await
}

// ---------------------------------------------------------------------------
// Faz 9: Raporlama
// ---------------------------------------------------------------------------

use crate::infrastructure::repositories::report_repository::{
    ReportRepository, BottleneckRow, ProcessPerformanceRow, TeamPerformanceRow,
};

#[derive(Debug, serde::Serialize)]
pub struct ProjectReport {
    pub work_items: WorkItemSummary,
    pub process_performance: Vec<ProcessPerformanceRow>,
    pub bottlenecks: Vec<BottleneckRow>,
    pub team_performance: Vec<TeamPerformanceRow>,
}

/// Basit eşik bazlı darboğaz yorumu (MASTER PLAN §37: ilk sürüm için yeterli).
pub fn bottleneck_hint(rows: &[BottleneckRow]) -> Option<String> {
    let with_ready: Vec<&BottleneckRow> = rows.iter().filter(|r| r.ready_count > 0).collect();
    let max_ready = with_ready.iter().map(|r| r.ready_count).max()?;
    // en yüksek READY yığını, en az 2 katı ortalamadan fazlaysa darboğaz adayı
    let avg_ready = with_ready.iter().map(|r| r.ready_count).sum::<i64>() as f64
        / with_ready.len().max(1) as f64;
    let top = with_ready.iter().find(|r| r.ready_count == max_ready)?;
    if (max_ready as f64) >= avg_ready * 1.75 && max_ready >= 2 {
        Some(format!("Olası darboğaz: {} ({} iş hazır bekliyor)", top.template_name, max_ready))
    } else {
        None
    }
}

pub async fn project_report(
    pool: &sqlx::SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<ProjectReport, DomainError> {
    ensure_viewer(actor)?;
    ensure_project(pool, actor, project_id).await?;
    let (work_items, _) =
        InsightRepository::dashboard_summary(pool, actor.workspace_id, project_id).await?;
    Ok(ProjectReport {
        work_items,
        process_performance: ReportRepository::process_performance(pool, actor.workspace_id, project_id).await?,
        bottlenecks: ReportRepository::bottlenecks(pool, actor.workspace_id, project_id).await?,
        team_performance: ReportRepository::team_performance(pool, actor.workspace_id, project_id).await?,
    })
}
