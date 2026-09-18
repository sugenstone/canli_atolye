//! "Benim İşlerim" sorgusu (MASTER PLAN §31): kullanıcının kendisine veya
//! takımlarına atanmış AKTİF execution'lar — kart görünümü için birleşik satır.

use crate::domain::entities::ProcessStatus;
use crate::domain::errors::DomainError;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Worker kartı satırı (template + item + yol + atama türü).
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct MyWorkCard {
    pub execution_id: Uuid,
    pub project_id: Uuid,
    pub template_name: String,
    pub work_item_name: String,
    pub section_path: String,
    pub status: ProcessStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ready_at: Option<chrono::DateTime<chrono::Utc>>,
    /// "user" | "team" — kimin üzerinden atandığı
    pub assignment_kind: String,
    pub planned_end_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct MyWorkRepository;

impl MyWorkRepository {
    /// Kullanıcının aktif işleri: kendisine atanmış YA DA takımlarına atanmış.
    pub async fn list(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<MyWorkCard>, DomainError> {
        Ok(sqlx::query_as::<_, MyWorkCard>(
            "SELECT e.id AS execution_id,
                    wi.project_id,
                    t.name AS template_name,
                    wi.name AS work_item_name,
                    COALESCE(parent.name || ' / ' || sec.name, sec.name) AS section_path,
                    e.status,
                    e.started_at,
                    e.ready_at,
                    CASE WHEN e.assigned_user_id = ?2 THEN 'user' ELSE 'team' END AS assignment_kind,
                    e.planned_end_at
             FROM process_executions e
             JOIN process_templates t ON t.id = e.process_template_id
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.deleted_at IS NULL
             JOIN sections sec ON sec.id = wi.section_id
             LEFT JOIN sections parent ON parent.id = sec.parent_id
             WHERE e.workspace_id = ?1
               AND e.status IN ('READY', 'IN_PROGRESS', 'PAUSED', 'BLOCKED')
               AND (e.assigned_user_id = ?2
                    OR e.assigned_team_id IN (SELECT team_id FROM team_members WHERE user_id = ?2))
             ORDER BY CASE e.status
                        WHEN 'IN_PROGRESS' THEN 0
                        WHEN 'PAUSED' THEN 1
                        WHEN 'BLOCKED' THEN 2
                        WHEN 'READY' THEN 3
                      END,
                      wi.name",
        )
        .bind(workspace_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?)
    }
}
