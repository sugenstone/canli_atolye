//! Süreç repository: şablonlar, gruplar (+adım/bağımlılık), çalıştırmalar,
//! append-only olay kayıtları.

use crate::domain::entities::{
    EventType, ProcessDependency, ProcessEvent, ProcessExecution, ProcessGroup,
    ProcessGroupStep, ProcessStatus, ProcessTemplate,
};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::{Sqlite, SqliteConnection, SqlitePool};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Şablonlar ve gruplar
// ---------------------------------------------------------------------------

pub struct ProcessCatalogRepository;

impl ProcessCatalogRepository {
    // --- Templates ---

    pub async fn insert_template(
        pool: &SqlitePool,
        workspace_id: Uuid,
        name: &str,
        code: &str,
        description: Option<&str>,
    ) -> Result<ProcessTemplate, DomainError> {
        let t = ProcessTemplate {
            id: new_id(),
            workspace_id,
            name: name.into(),
            code: code.to_uppercase(),
            description: description.map(Into::into),
            default_duration: None,
            color: None,
            icon: None,
            active: true,
            created_at: now(),
            updated_at: now(),
        };
        match sqlx::query(
            "INSERT INTO process_templates (id, workspace_id, name, code, description, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(t.id)
        .bind(t.workspace_id)
        .bind(&t.name)
        .bind(&t.code)
        .bind(&t.description)
        .bind(t.active)
        .bind(t.created_at)
        .bind(t.updated_at)
        .execute(pool)
        .await
        {
            Ok(_) => Ok(t),
            Err(sqlx::Error::Database(db)) if db.message().contains("UNIQUE") => {
                Err(DomainError::Conflict {
                    message: "Bu kodla bir süreç şablonu zaten var.".into(),
                })
            }
            Err(e) => Err(DomainError::Database(e)),
        }
    }

    pub async fn list_templates(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<ProcessTemplate>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessTemplate>(
            "SELECT * FROM process_templates WHERE workspace_id = ?1 AND active = 1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_template(
        pool: &SqlitePool,
        workspace_id: Uuid,
        id: Uuid,
    ) -> Result<ProcessTemplate, DomainError> {
        sqlx::query_as::<_, ProcessTemplate>(
            "SELECT * FROM process_templates WHERE id = ?1 AND workspace_id = ?2 AND active = 1",
        )
        .bind(id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    // --- Groups ---

    pub async fn insert_group(
        tx: &mut SqliteConnection,
        workspace_id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<ProcessGroup, DomainError> {
        let g = ProcessGroup {
            id: new_id(),
            workspace_id,
            name: name.into(),
            description: description.map(Into::into),
            active: true,
            created_at: now(),
            updated_at: now(),
        };
        sqlx::query(
            "INSERT INTO process_groups (id, workspace_id, name, description, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(g.id)
        .bind(g.workspace_id)
        .bind(&g.name)
        .bind(&g.description)
        .bind(g.active)
        .bind(g.created_at)
        .bind(g.updated_at)
        .execute(&mut *tx)
        .await?;
        Ok(g)
    }

    /// Adımları sırayla ekle; index → step id haritasını döndür (bağımlılık kurulumu için).
    pub async fn insert_steps(
        tx: &mut SqliteConnection,
        group_id: Uuid,
        steps: &[(Uuid, bool)], // (template_id, required)
    ) -> Result<Vec<ProcessGroupStep>, DomainError> {
        let mut created = Vec::with_capacity(steps.len());
        for (i, (template_id, required)) in steps.iter().enumerate() {
            let step = ProcessGroupStep {
                id: new_id(),
                process_group_id: group_id,
                process_template_id: *template_id,
                sort_order: i as i64 + 1,
                required: *required,
            };
            sqlx::query(
                "INSERT INTO process_group_steps (id, process_group_id, process_template_id, sort_order, required)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(step.id)
            .bind(step.process_group_id)
            .bind(step.process_template_id)
            .bind(step.sort_order)
            .bind(step.required)
            .execute(&mut *tx)
            .await?;
            created.push(step);
        }
        Ok(created)
    }

    pub async fn insert_dependency(
        tx: &mut SqliteConnection,
        step_id: Uuid,
        depends_on: Uuid,
        required_status: ProcessStatus,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO process_dependencies (id, process_group_step_id, depends_on_process_group_step_id, required_status)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(new_id())
        .bind(step_id)
        .bind(depends_on)
        .bind(required_status)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    pub async fn list_groups(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<ProcessGroup>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessGroup>(
            "SELECT * FROM process_groups WHERE workspace_id = ?1 AND active = 1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_group(
        pool: &SqlitePool,
        workspace_id: Uuid,
        id: Uuid,
    ) -> Result<ProcessGroup, DomainError> {
        sqlx::query_as::<_, ProcessGroup>(
            "SELECT * FROM process_groups WHERE id = ?1 AND workspace_id = ?2 AND active = 1",
        )
        .bind(id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    /// Grubun adımları + bağımlılıkları (sıralı).
    pub async fn group_detail(
        pool: &SqlitePool,
        workspace_id: Uuid,
        group_id: Uuid,
    ) -> Result<(Vec<ProcessGroupStep>, Vec<ProcessDependency>), DomainError> {
        Self::find_group(pool, workspace_id, group_id).await?;
        let steps = sqlx::query_as::<_, ProcessGroupStep>(
            "SELECT * FROM process_group_steps WHERE process_group_id = ?1 ORDER BY sort_order",
        )
        .bind(group_id)
        .fetch_all(pool)
        .await?;
        let deps = sqlx::query_as::<_, ProcessDependency>(
            "SELECT * FROM process_dependencies WHERE process_group_step_id IN
             (SELECT id FROM process_group_steps WHERE process_group_id = ?1)",
        )
        .bind(group_id)
        .fetch_all(pool)
        .await?;
        Ok((steps, deps))
    }
}

// ---------------------------------------------------------------------------
// Çalıştırmalar
// ---------------------------------------------------------------------------

pub struct ExecutionRepository;

/// Durum değişikliği yaması — version'lu optimistic update ile uygulanır.
pub struct StatusPatch {
    pub status: ProcessStatus,
    pub status_before_block: Option<ProcessStatus>,
    pub ready_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expected_version: i64,
}

impl ExecutionRepository {
    /// Grup ataması: iş kalemi için tüm adımların execution'ları (tek transaction'da
    /// service tarafından çağrılır). Başlangıçta hepsi PENDING; ilk READY'ler
    /// service resolver'ı ile belirlenir.
    pub async fn insert_many(
        tx: &mut SqliteConnection,
        workspace_id: Uuid,
        work_item_id: Uuid,
        _group_id: Uuid,
        steps: &[ProcessGroupStep],
    ) -> Result<Vec<ProcessExecution>, DomainError> {
        let mut created = Vec::with_capacity(steps.len());
        for step in steps {
            let exec = ProcessExecution {
                id: new_id(),
                workspace_id,
                work_item_id,
                process_template_id: step.process_template_id,
                process_group_step_id: step.id,
                status: ProcessStatus::Pending,
                status_before_block: None,
                version: 0,
                assigned_user_id: None,
                assigned_team_id: None,
                planned_start_at: None,
                planned_end_at: None,
                ready_at: None,
                started_at: None,
                completed_at: None,
                revision_no: 0,
                parent_execution_id: None,
                created_at: now(),
                updated_at: now(),
                deleted_at: None,
            };
            sqlx::query(
                "INSERT INTO process_executions (id, workspace_id, work_item_id, process_template_id, process_group_step_id,
                        status, version, revision_no, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, ?7, ?7)",
            )
            .bind(exec.id)
            .bind(exec.workspace_id)
            .bind(exec.work_item_id)
            .bind(exec.process_template_id)
            .bind(exec.process_group_step_id)
            .bind(exec.status)
            .bind(exec.created_at)
            .execute(&mut *tx)
            .await?;
            created.push(exec);
        }
        Ok(created)
    }

    pub async fn find_in_workspace(
        pool: &SqlitePool,
        workspace_id: Uuid,
        id: Uuid,
    ) -> Result<ProcessExecution, DomainError> {
        sqlx::query_as::<_, ProcessExecution>(
            "SELECT * FROM process_executions WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    pub async fn list_by_work_item(
        pool: &SqlitePool,
        workspace_id: Uuid,
        work_item_id: Uuid,
    ) -> Result<Vec<ProcessExecution>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessExecution>(
            "SELECT * FROM process_executions
             WHERE workspace_id = ?1 AND work_item_id = ?2 AND deleted_at IS NULL
             ORDER BY created_at",
        )
        .bind(workspace_id)
        .bind(work_item_id)
        .fetch_all(pool)
        .await?)
    }

    /// İş kaleminin aktif (CANCELLED olmayan) execution'ı var mı?
    pub async fn count_active_by_work_item(
        pool: &SqlitePool,
        workspace_id: Uuid,
        work_item_id: Uuid,
    ) -> Result<i64, DomainError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM process_executions
             WHERE workspace_id = ?1 AND work_item_id = ?2 AND deleted_at IS NULL
               AND status NOT IN ('CANCELLED')",
        )
        .bind(workspace_id)
        .bind(work_item_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Optimistic-lock'lu durum güncellemesi. Etkilenen satır 0 → sürüm çakışması.
    pub async fn update_status_tx(
        tx: &mut SqliteConnection,
        exec: &ProcessExecution,
        patch: &StatusPatch,
    ) -> Result<ProcessExecution, DomainError> {
        let result = sqlx::query(
            "UPDATE process_executions
             SET status = ?2, status_before_block = ?3, ready_at = COALESCE(?4, ready_at),
                 started_at = COALESCE(?5, started_at), completed_at = COALESCE(?6, completed_at),
                 version = version + 1, updated_at = ?7
             WHERE id = ?1 AND version = ?8 AND deleted_at IS NULL",
        )
        .bind(exec.id)
        .bind(patch.status)
        .bind(patch.status_before_block)
        .bind(patch.ready_at)
        .bind(patch.started_at)
        .bind(patch.completed_at)
        .bind(now())
        .bind(patch.expected_version)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict {
                message: "Kayıt eşzamanlı değiştirildi; sayfayı yenileyip tekrar deneyin.".into(),
            });
        }
        let mut updated = exec.clone();
        updated.status = patch.status;
        updated.status_before_block = patch.status_before_block;
        if patch.ready_at.is_some() {
            updated.ready_at = patch.ready_at;
        }
        if patch.started_at.is_some() {
            updated.started_at = patch.started_at;
        }
        if patch.completed_at.is_some() {
            updated.completed_at = patch.completed_at;
        }
        updated.version += 1;
        Ok(updated)
    }

    /// Atama güncelle (version'lu).
    pub async fn update_assignee(
        pool: &SqlitePool,
        exec_id: Uuid,
        user_id: Option<Uuid>,
        team_id: Option<Uuid>,
        expected_version: i64,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE process_executions SET assigned_user_id = ?2, assigned_team_id = ?3,
                    version = version + 1, updated_at = ?4
             WHERE id = ?1 AND version = ?5 AND deleted_at IS NULL",
        )
        .bind(exec_id)
        .bind(user_id)
        .bind(team_id)
        .bind(now())
        .bind(expected_version)
        .execute(pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict {
                message: "Kayıt eşzamanlı değiştirildi; sayfayı yenileyip tekrar deneyin.".into(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Olaylar — append-only
// ---------------------------------------------------------------------------

pub struct EventRepository;

pub struct NewEvent {
    pub process_execution_id: Uuid,
    pub event_type: EventType,
    pub previous_status: Option<ProcessStatus>,
    pub new_status: Option<ProcessStatus>,
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub note: Option<String>,
    pub metadata: Option<String>,
}

impl EventRepository {
    pub async fn insert(
        executor: impl sqlx::Executor<'_, Database = Sqlite>,
        workspace_id: Uuid,
        e: &NewEvent,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO process_events (id, workspace_id, process_execution_id, event_type,
                    previous_status, new_status, user_id, team_id, timestamp, note, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?9)",
        )
        .bind(new_id())
        .bind(workspace_id)
        .bind(e.process_execution_id)
        .bind(e.event_type)
        .bind(e.previous_status)
        .bind(e.new_status)
        .bind(e.user_id)
        .bind(e.team_id)
        .bind(now())
        .bind(&e.note)
        .bind(&e.metadata)
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn list_by_execution(
        pool: &SqlitePool,
        execution_id: Uuid,
    ) -> Result<Vec<ProcessEvent>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessEvent>(
            "SELECT * FROM process_events WHERE process_execution_id = ?1 ORDER BY timestamp, created_at",
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await?)
    }
}

/// Work item'ın adımları için bağımlılık görünümü:
/// step_id → [(bağımlı olunan execution'ın DURUMU, GEREEKEN durum)]
pub async fn dependencies_for_work_item(
    tx: &mut SqliteConnection,
    work_item_id: Uuid,
) -> Result<std::collections::HashMap<Uuid, Vec<(ProcessStatus, ProcessStatus)>>, DomainError> {
    let dep_rows = sqlx::query_as::<_, (Uuid, ProcessStatus, ProcessStatus)>(
        "SELECT d.process_group_step_id, e2.status, d.required_status
         FROM process_dependencies d
         JOIN process_executions e
              ON e.process_group_step_id = d.process_group_step_id
             AND e.work_item_id = ?1 AND e.deleted_at IS NULL
         JOIN process_executions e2
              ON e2.process_group_step_id = d.depends_on_process_group_step_id
             AND e2.work_item_id = ?1 AND e2.deleted_at IS NULL",
    )
    .bind(work_item_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut by_step: std::collections::HashMap<Uuid, Vec<(ProcessStatus, ProcessStatus)>> =
        Default::default();
    for (step, actual, required) in dep_rows {
        by_step.entry(step).or_default().push((actual, required));
    }
    Ok(by_step)
}
