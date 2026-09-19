//! Süreç service'i (MASTER PLAN §12-16): grup ataması, durum geçişleri,
//! bağımlılık çözümleme ve olay üretimi. Her durum değişikliği TEK transaction:
//! status update (optimistic lock) + event + downstream READY. Event'siz
//! status değişikliği kod yoluyla mümkün değildir.

use crate::domain::entities::{
    EventType, ProcessDependency, ProcessEvent, ProcessExecution, ProcessGroup,
    ProcessGroupStep, ProcessStatus, ProcessTemplate, User, WorkItem,
};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::domain::services::process_engine::{
    self, should_promote_to_ready, unblock_target, Transition,
};
use crate::infrastructure::repositories::section_repository::SectionRepository;
use crate::infrastructure::repositories::process_repository::{
    dependencies_for_work_item, EventRepository, ExecutionRepository, NewEvent, ProcessCatalogRepository,
    StatusPatch,
};
use crate::infrastructure::repositories::work_item_repository::WorkItemRepository;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Yetki yardımcıları (kaynak bazlı — MASTER PLAN §56)
// ---------------------------------------------------------------------------

fn ensure_manager(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::CreateWorkItem) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

fn ensure_viewer(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

/// Operasyonel aksiyon (start/pause/...): ADMIN/PM her zaman;
/// WORKER/TEAM_LEADER yalnızca kendisine atanmış ya da takımının işinde.
async fn can_operate(
    tx: &mut SqliteConnection,
    actor: &User,
    exec: &ProcessExecution,
) -> Result<bool, DomainError> {
    if can(actor.role, Action::UpdateWorkItem) {
        return Ok(true); // ADMIN | PM
    }
    if exec.assigned_user_id == Some(actor.id) {
        return Ok(true);
    }
    if let Some(team_id) = exec.assigned_team_id {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
        )
        .bind(team_id)
        .bind(actor.id)
        .fetch_one(&mut *tx)
        .await?;
        return Ok(count > 0);
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Şablonlar ve gruplar
// ---------------------------------------------------------------------------

pub async fn list_templates(
    pool: &SqlitePool,
    actor: &User,
) -> Result<Vec<ProcessTemplate>, DomainError> {
    ensure_viewer(actor)?;
    ProcessCatalogRepository::list_templates(pool, actor.workspace_id).await
}

pub async fn create_template(
    pool: &SqlitePool,
    actor: &User,
    name: &str,
    code: &str,
    description: Option<&str>,
) -> Result<ProcessTemplate, DomainError> {
    ensure_manager(actor)?;
    let name = name.trim();
    let code = code.trim();
    if name.is_empty() || code.is_empty() {
        return Err(DomainError::Validation {
            message: "Şablon adı ve kodu zorunludur.".into(),
        });
    }
    ProcessCatalogRepository::insert_template(pool, actor.workspace_id, name, code, description).await
}

/// Grup oluşturma girdisi — adımlar index bazlı verilir, bağımlılıklar index çiftleri.
pub struct NewProcessGroup {
    pub name: String,
    pub description: Option<String>,
    /// (template_id, required)
    pub steps: Vec<(Uuid, bool)>,
    /// (step_index, depends_on_step_index) — 0 tabanlı
    pub dependencies: Vec<(usize, usize)>,
}

pub async fn create_group(
    pool: &SqlitePool,
    actor: &User,
    input: NewProcessGroup,
) -> Result<ProcessGroup, DomainError> {
    ensure_manager(actor)?;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation {
            message: "Grup adı zorunludur.".into(),
        });
    }
    if input.steps.is_empty() {
        return Err(DomainError::Validation {
            message: "Grup en az bir adım içermelidir.".into(),
        });
    }

    // Şablonlar workspace'te mi?
    for (template_id, _) in &input.steps {
        ProcessCatalogRepository::find_template(pool, actor.workspace_id, *template_id).await?;
    }
    // Bağımlılık indeksleri geçerli mi?
    let n = input.steps.len();
    for (step, dep) in &input.dependencies {
        if *step >= n || *dep >= n {
            return Err(DomainError::Validation {
                message: "Bağımlılık indeksleri adım sayısının dışında.".into(),
            });
        }
        if step == dep {
            return Err(DomainError::Validation {
                message: "Bir adım kendisine bağımlı olamaz.".into(),
            });
        }
    }

    let mut tx = pool.begin().await?;
    let group =
        ProcessCatalogRepository::insert_group(&mut *tx, actor.workspace_id, name, input.description.as_deref()).await?;
    let steps = ProcessCatalogRepository::insert_steps(&mut *tx, group.id, &input.steps).await?;
    for (step_idx, dep_idx) in &input.dependencies {
        ProcessCatalogRepository::insert_dependency(
            &mut *tx,
            steps[*step_idx].id,
            steps[*dep_idx].id,
            ProcessStatus::Completed,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(group)
}

pub async fn list_groups(
    pool: &SqlitePool,
    actor: &User,
) -> Result<Vec<ProcessGroup>, DomainError> {
    ensure_viewer(actor)?;
    ProcessCatalogRepository::list_groups(pool, actor.workspace_id).await
}

pub async fn group_detail(
    pool: &SqlitePool,
    actor: &User,
    group_id: Uuid,
) -> Result<(Vec<ProcessGroupStep>, Vec<ProcessDependency>), DomainError> {
    ensure_viewer(actor)?;
    ProcessCatalogRepository::group_detail(pool, actor.workspace_id, group_id).await
}

// ---------------------------------------------------------------------------
// Grup atama — execution'ların doğumu
// ---------------------------------------------------------------------------

pub async fn assign_group(
    pool: &SqlitePool,
    actor: &User,
    work_item_id: Uuid,
    group_id: Uuid,
) -> Result<Vec<ProcessExecution>, DomainError> {
    ensure_manager(actor)?;

    // İş kalemi bu workspace'te mi?
    let _item: WorkItem = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, work_item_id).await?;
    let group = ProcessCatalogRepository::find_group(pool, actor.workspace_id, group_id).await?;
    let (steps, _deps) = ProcessCatalogRepository::group_detail(pool, actor.workspace_id, group.id).await?;

    if ExecutionRepository::count_active_by_work_item(pool, actor.workspace_id, work_item_id).await? > 0 {
        return Err(DomainError::Conflict {
            message: "Bu iş kalemine zaten atanmış aktif bir süreç grubu var.".into(),
        });
    }

    let mut tx = pool.begin().await?;
    let created = ExecutionRepository::insert_many(&mut *tx, actor.workspace_id, work_item_id, group.id, &steps).await?;

    // Her execution için CREATED olayı
    for exec in &created {
        EventRepository::insert(
            &mut *tx,
            actor.workspace_id,
            &NewEvent {
                process_execution_id: exec.id,
                event_type: EventType::Created,
                previous_status: None,
                new_status: Some(ProcessStatus::Pending),
                user_id: actor.id,
                team_id: None,
                note: Some(format!("Süreç grubu atandı: {}", group.name)),
                metadata: None,
            },
        )
        .await?;
    }

    // İlk READY'ler: bağımlılığı olmayan adımlar hemen READY olur
    let promoted = promote_pending(&mut *tx, actor, work_item_id).await?;
    tx.commit().await?;

    let _ = promoted;
    Ok(created)
}

/// PENDING execution'lardan bağımlılıkları karşılananları READY'a çeker
/// (+ READY olayları). Transaction İÇİNDE çağrılır.
async fn promote_pending(
    tx: &mut SqliteConnection,
    actor: &User,
    work_item_id: Uuid,
) -> Result<Vec<Uuid>, DomainError> {
    let rows = sqlx::query_as::<_, ProcessExecution>(
        "SELECT * FROM process_executions WHERE work_item_id = ?1 AND deleted_at IS NULL",
    )
    .bind(work_item_id)
    .fetch_all(&mut *tx)
    .await?;

    let deps = dependencies_for_work_item(&mut *tx, work_item_id).await?;
    let mut promoted = Vec::new();

    for exec in rows {
        if exec.status != ProcessStatus::Pending {
            continue;
        }
        let dep_list = deps.get(&exec.process_group_step_id).cloned().unwrap_or_default();
        if !should_promote_to_ready(ProcessStatus::Pending, &dep_list) {
            continue;
        }
        let patch = StatusPatch {
            status: ProcessStatus::Ready,
            status_before_block: None,
            ready_at: Some(crate::shared::now()),
            started_at: None,
            completed_at: None,
            expected_version: exec.version,
        };
        let updated = ExecutionRepository::update_status_tx(tx, &exec, &patch).await?;
        EventRepository::insert(
            &mut *tx,
            exec.workspace_id,
            &NewEvent {
                process_execution_id: exec.id,
                event_type: EventType::Ready,
                previous_status: Some(ProcessStatus::Pending),
                new_status: Some(ProcessStatus::Ready),
                user_id: actor.id,
                team_id: None,
                note: None,
                metadata: None,
            },
        )
        .await?;
        promoted.push(updated.id);
    }
    Ok(promoted)
}

// ---------------------------------------------------------------------------
// Durum geçişleri (action endpoint'ler)
// ---------------------------------------------------------------------------

pub struct TransitionOutcome {
    pub execution: ProcessExecution,
    /// Bu geçişin tetiklediği downstream READY promosyonları
    pub promoted: Vec<Uuid>,
}

/// Genel geçiş işleyicisi: doğrula → geçir → olay → (complete ise) downstream.
pub async fn apply_transition(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    action: Transition,
    note: Option<String>,
) -> Result<TransitionOutcome, DomainError> {
    let mut tx = pool.begin().await?;

    // Execution'ı kilitlemek üzere tx üzerinden oku
    let exec = sqlx::query_as::<_, ProcessExecution>(
        "SELECT * FROM process_executions WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
    )
    .bind(execution_id)
    .bind(actor.workspace_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DomainError::NotFound)?;

    // Kaynak bazlı yetki: worker yalnız kendi/takım işini işletebilir
    if !can_operate(&mut *tx, actor, &exec).await? {
        return Err(DomainError::Forbidden);
    }

    // Motor kuralı
    let (new_status, before_block) = match action {
        Transition::Unblock => {
            let target = unblock_target(exec.status_before_block)
                .map_err(|e| DomainError::Validation { message: e.to_string() })?;
            (target, None)
        }
        _ => process_engine::transition(exec.status, action)
            .map_err(|e| DomainError::Validation { message: e.to_string() })?,
    };

    // Geçiş yaması
    let patch = StatusPatch {
        status: new_status,
        status_before_block: before_block,
        ready_at: (new_status == ProcessStatus::Ready).then(crate::shared::now),
        started_at: (action == Transition::Start).then(crate::shared::now),
        completed_at: (action == Transition::Complete).then(crate::shared::now),
        expected_version: exec.version,
    };
    let updated = ExecutionRepository::update_status_tx(&mut *tx, &exec, &patch).await?;

    // Olay — status değişikliği OLAYSIZ asla olmaz (MASTER PLAN §71.5)
    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: action.event_type(),
            previous_status: Some(exec.status),
            new_status: Some(new_status),
            user_id: actor.id,
            team_id: exec.assigned_team_id,
            note,
            metadata: None,
        },
    )
    .await?;

    // complete sonrası: bağımlılıkları karşılanan PENDING'ler READY olur
    let promoted = if action == Transition::Complete {
        promote_pending(&mut *tx, actor, exec.work_item_id).await?
    } else {
        Vec::new()
    };

    tx.commit().await?;
    Ok(TransitionOutcome {
        execution: updated,
        promoted,
    })
}

// ---------------------------------------------------------------------------
// Sorgular
// ---------------------------------------------------------------------------

/// Execution + şablon adı + adım sırası — UI listesi için flat satır.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ExecutionRow {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub work_item_id: Uuid,
    pub process_template_id: Uuid,
    pub process_group_step_id: Uuid,
    pub status: ProcessStatus,
    pub status_before_block: Option<ProcessStatus>,
    pub version: i64,
    pub assigned_user_id: Option<Uuid>,
    pub assigned_team_id: Option<Uuid>,
    pub planned_start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub planned_end_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ready_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revision_no: i64,
    pub parent_execution_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub template_name: String,
    pub sort_order: i64,
}

/// İş kaleminin execution'ları + şablon adları (UI sıralı liste).
pub async fn list_by_work_item(
    pool: &SqlitePool,
    actor: &User,
    work_item_id: Uuid,
) -> Result<Vec<ExecutionRow>, DomainError> {
    ensure_viewer(actor)?;
    let _item = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, work_item_id).await?;
    Ok(sqlx::query_as::<_, ExecutionRow>(
        "SELECT e.*, t.name AS template_name, s.sort_order AS sort_order
         FROM process_executions e
         JOIN process_templates t ON t.id = e.process_template_id
         JOIN process_group_steps s ON s.id = e.process_group_step_id
         WHERE e.workspace_id = ?1 AND e.work_item_id = ?2 AND e.deleted_at IS NULL
         ORDER BY s.sort_order",
    )
    .bind(actor.workspace_id)
    .bind(work_item_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_execution(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
) -> Result<ProcessExecution, DomainError> {
    ensure_viewer(actor)?;
    ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await
}

pub async fn list_events(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
) -> Result<Vec<ProcessEvent>, DomainError> {
    ensure_viewer(actor)?;
    let exec = ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await?;
    EventRepository::list_by_execution(pool, exec.id).await
}

// ---------------------------------------------------------------------------
// Atama
// ---------------------------------------------------------------------------

pub async fn assign(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    user_id: Option<Uuid>,
    team_id: Option<Uuid>,
) -> Result<ProcessExecution, DomainError> {
    ensure_manager(actor)?;
    let exec = ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await?;

    // Hedef kullanıcı/ takım aynı workspace'te mi?
    if let Some(uid) = user_id {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?1 AND workspace_id = ?2")
                .bind(uid)
                .bind(actor.workspace_id)
                .fetch_one(pool)
                .await?;
        if count == 0 {
            return Err(DomainError::UserNotFound);
        }
    }
    if let Some(tid) = team_id {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM teams WHERE id = ?1 AND workspace_id = ?2")
                .bind(tid)
                .bind(actor.workspace_id)
                .fetch_one(pool)
                .await?;
        if count == 0 {
            return Err(DomainError::TeamNotFound);
        }
    }

    let was_unassigned = exec.assigned_user_id.is_none() && exec.assigned_team_id.is_none();
    ExecutionRepository::update_assignee(pool, exec.id, user_id, team_id, exec.version).await?;

    let mut tx = pool.begin().await?;
    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: if was_unassigned {
                EventType::Assigned
            } else {
                EventType::AssigneeChanged
            },
            previous_status: None,
            new_status: None,
            user_id: actor.id,
            team_id,
            note: None,
            metadata: None,
        },
    )
    .await?;
    tx.commit().await?;

    let mut updated = exec.clone();
    updated.assigned_user_id = user_id;
    updated.assigned_team_id = team_id;
    updated.version += 1;
    Ok(updated)
}

// ---------------------------------------------------------------------------
// Faz 8: Planlama — plan tarihleri, gecikme, süre kırılımı
// ---------------------------------------------------------------------------

/// Planlanan tarihleri güncelle (ADMIN/PM) + PLANNED_DATE_CHANGED olayı.
/// `None` gelen alan dokunulmaz; `Some(None)` temizler.
pub async fn set_planned_dates(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    planned_start_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    planned_end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
) -> Result<ProcessExecution, DomainError> {
    ensure_manager(actor)?;
    let exec = ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await?;

    let new_start = planned_start_at.unwrap_or(exec.planned_start_at);
    let new_end = planned_end_at.unwrap_or(exec.planned_end_at);
    if let (Some(s), Some(e)) = (new_start, new_end) {
        if e < s {
            return Err(DomainError::Validation {
                message: "Planlanan bitiş, başlangıçtan önce olamaz.".into(),
            });
        }
    }

    let result = sqlx::query(
        "UPDATE process_executions SET planned_start_at = ?2, planned_end_at = ?3,
                version = version + 1, updated_at = ?4
         WHERE id = ?1 AND version = ?5 AND deleted_at IS NULL",
    )
    .bind(exec.id)
    .bind(new_start)
    .bind(new_end)
    .bind(crate::shared::now())
    .bind(exec.version)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DomainError::Conflict {
            message: "Kayıt eşzamanlı değiştirildi; sayfayı yenileyip tekrar deneyin.".into(),
        });
    }

    let mut tx = pool.begin().await?;
    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: EventType::PlannedDateChanged,
            previous_status: None,
            new_status: None,
            user_id: actor.id,
            team_id: None,
            note: Some("Plan tarihleri güncellendi".into()),
            metadata: None,
        },
    )
    .await?;
    tx.commit().await?;

    let mut updated = exec.clone();
    updated.planned_start_at = new_start;
    updated.planned_end_at = new_end;
    updated.version += 1;
    Ok(updated)
}

/// Toplu plan: şablon bazlı (proje kapsamındaki aktif execution'lara) veya
/// kimlik listesiyle. Her biri için olay üretilir.
pub struct BulkPlanInput {
    pub template_id: Option<Uuid>,
    pub execution_ids: Option<Vec<Uuid>>,
    pub planned_start_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub planned_end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

pub async fn bulk_plan(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
    input: BulkPlanInput,
) -> Result<usize, DomainError> {
    ensure_manager(actor)?;

    let targets: Vec<Uuid> = if let Some(template_id) = input.template_id {
        sqlx::query_as::<_, (Uuid,)>(
            "SELECT e.id FROM process_executions e
             JOIN work_items wi ON wi.id = e.work_item_id AND wi.project_id = ?2 AND wi.deleted_at IS NULL
             WHERE e.workspace_id = ?1 AND e.process_template_id = ?3
               AND e.status NOT IN ('COMPLETED', 'CANCELLED') AND e.deleted_at IS NULL",
        )
        .bind(actor.workspace_id)
        .bind(project_id)
        .bind(template_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(id,)| id)
        .collect()
    } else if let Some(ids) = &input.execution_ids {
        ids.clone()
    } else {
        return Err(DomainError::Validation {
            message: "template_id veya execution_ids gerekli.".into(),
        });
    };

    if targets.is_empty() {
        return Ok(0);
    }
    if let (Some(Some(s)), Some(Some(e))) = (input.planned_start_at, input.planned_end_at) {
        if e < s {
            return Err(DomainError::Validation {
                message: "Planlanan bitiş, başlangıçtan önce olamaz.".into(),
            });
        }
    }

    let mut count = 0usize;
    for id in targets {
        if set_planned_dates(pool, actor, id, input.planned_start_at.clone(), input.planned_end_at.clone()).await.is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

/// Süre kırılımı (MASTER PLAN §16): olaylardan aktif/duraklatılmış/bloke süre.
pub async fn durations(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
) -> Result<crate::domain::services::duration::Durations, DomainError> {
    ensure_viewer(actor)?;
    let exec = ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await?;
    let events = EventRepository::list_by_execution(pool, exec.id).await?;
    let timed: Vec<crate::domain::services::duration::TimedEvent> = events
        .into_iter()
        .map(|e| crate::domain::services::duration::TimedEvent {
            event_type: e.event_type,
            timestamp: e.timestamp,
        })
        .collect();
    Ok(crate::domain::services::duration::compute_durations(&timed, crate::shared::now()))
}

// ---------------------------------------------------------------------------
// Toplu süreç grubu atama (MASTER PLAN §23) ve revizyon (§20)
// ---------------------------------------------------------------------------

/// Üst bölümün altındaki YAPRAK bölümdeki, henüz süreci olmayan tüm iş
/// kalemlerine grubu ata (idempotent). Tek transaction; kaç item'a atandığını döner.
pub async fn bulk_assign_group(
    pool: &SqlitePool,
    actor: &User,
    parent_section_id: Uuid,
    group_id: Uuid,
) -> Result<usize, DomainError> {
    ensure_manager(actor)?;

    let parent = SectionRepository::new()
        .find_in_workspace(pool, actor.workspace_id, parent_section_id)
        .await?;
    let group = ProcessCatalogRepository::find_group(pool, actor.workspace_id, group_id).await?;
    let (steps, _deps) = ProcessCatalogRepository::group_detail(pool, actor.workspace_id, group.id).await?;

    // yapraklar → o bölümlerdeki iş kalemleri
    let all_sections = SectionRepository::new()
        .list_by_project(pool, actor.workspace_id, parent.project_id)
        .await?;
    let leaves = crate::infrastructure::repositories::work_item_repository::leaves_under(&all_sections, &parent);
    let leaf_ids: Vec<Uuid> = leaves.iter().map(|s| s.id).collect();
    let items = WorkItemRepository::list_by_sections(pool, actor.workspace_id, &leaf_ids).await?;

    // zaten aktif süreci olanları çıkar
    let with_process: std::collections::HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
        "SELECT DISTINCT work_item_id FROM process_executions
         WHERE workspace_id = ?1 AND status != 'CANCELLED' AND deleted_at IS NULL",
    )
    .bind(actor.workspace_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id,)| id)
    .collect();
    let targets: Vec<_> = items.into_iter().filter(|i| !with_process.contains(&i.id)).collect();
    if targets.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;
    let mut count = 0usize;
    for item in &targets {
        let created = ExecutionRepository::insert_many(&mut *tx, actor.workspace_id, item.id, group.id, &steps).await?;
        for exec in &created {
            EventRepository::insert(
                &mut *tx,
                actor.workspace_id,
                &NewEvent {
                    process_execution_id: exec.id,
                    event_type: EventType::Created,
                    previous_status: None,
                    new_status: Some(ProcessStatus::Pending),
                    user_id: actor.id,
                    team_id: None,
                    note: Some(format!("Süreç grubu atandı: {}", group.name)),
                    metadata: None,
                },
            )
            .await?;
        }
        promote_pending(&mut *tx, actor, item.id).await?;
        count += 1;
    }
    tx.commit().await?;
    Ok(count)
}

/// Revizyon: tamamlanmış süreci yeniden aç — orijinal kayıt korunur,
/// yeni revision_no'lu execution PENDING olarak doğar (MASTER PLAN §20).
pub async fn reopen(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    reason: String,
) -> Result<ProcessExecution, DomainError> {
    ensure_manager(actor)?;
    let reason = reason.trim().to_string();
    if reason.is_empty() {
        return Err(DomainError::Validation {
            message: "Revizyon nedeni zorunludur.".into(),
        });
    }

    let mut tx = pool.begin().await?;
    let exec = sqlx::query_as::<_, ProcessExecution>(
        "SELECT * FROM process_executions WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
    )
    .bind(execution_id)
    .bind(actor.workspace_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DomainError::NotFound)?;

    if exec.status != ProcessStatus::Completed {
        return Err(DomainError::Validation {
            message: "Yalnızca tamamlanmış süreçler yeniden açılabilir.".into(),
        });
    }

    // aynı adıma zaten açık bir revizyon var mı?
    let (open_rev,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM process_executions
         WHERE work_item_id = ?1 AND process_group_step_id = ?2
           AND status NOT IN ('COMPLETED','CANCELLED') AND deleted_at IS NULL",
    )
    .bind(exec.work_item_id)
    .bind(exec.process_group_step_id)
    .fetch_one(&mut *tx)
    .await?;
    if open_rev > 0 {
        return Err(DomainError::Conflict {
            message: "Bu adımın zaten aktif bir çalışması var.".into(),
        });
    }

    // yeni revision execution
    let revision = ProcessExecution {
        id: crate::shared::new_id(),
        workspace_id: exec.workspace_id,
        work_item_id: exec.work_item_id,
        process_template_id: exec.process_template_id,
        process_group_step_id: exec.process_group_step_id,
        status: ProcessStatus::Pending,
        status_before_block: None,
        version: 0,
        assigned_user_id: exec.assigned_user_id,
        assigned_team_id: exec.assigned_team_id,
        planned_start_at: None,
        planned_end_at: None,
        ready_at: None,
        started_at: None,
        completed_at: None,
        revision_no: exec.revision_no + 1,
        parent_execution_id: Some(exec.id),
        created_at: crate::shared::now(),
        updated_at: crate::shared::now(),
        deleted_at: None,
    };
    sqlx::query(
        "INSERT INTO process_executions (id, workspace_id, work_item_id, process_template_id, process_group_step_id,
                status, version, assigned_user_id, assigned_team_id, revision_no, parent_execution_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?9, ?10, ?11, ?11)",
    )
    .bind(revision.id)
    .bind(revision.workspace_id)
    .bind(revision.work_item_id)
    .bind(revision.process_template_id)
    .bind(revision.process_group_step_id)
    .bind(revision.status)
    .bind(revision.assigned_user_id)
    .bind(revision.assigned_team_id)
    .bind(revision.revision_no)
    .bind(revision.parent_execution_id)
    .bind(revision.created_at)
    .execute(&mut *tx)
    .await?;

    // eskiye REOPENED işareti + yenine REOPENED doğumu
    EventRepository::insert(&mut *tx, exec.workspace_id, &NewEvent {
        process_execution_id: exec.id,
        event_type: EventType::Reopened,
        previous_status: Some(ProcessStatus::Completed),
        new_status: None,
        user_id: actor.id,
        team_id: None,
        note: Some(reason.clone()),
        metadata: Some(format!(r#"{{"new_execution_id":"{0}"}}"#, revision.id)),
    }).await?;
    EventRepository::insert(&mut *tx, revision.workspace_id, &NewEvent {
        process_execution_id: revision.id,
        event_type: EventType::Reopened,
        previous_status: None,
        new_status: Some(ProcessStatus::Pending),
        user_id: actor.id,
        team_id: None,
        note: Some(reason),
        metadata: Some(format!(r#"{{"parent_execution_id":"{0}"}}"#, exec.id)),
    }).await?;

    // bağımlılıkları karşılanıyorsa hemen READY
    promote_pending(&mut *tx, actor, revision.work_item_id).await?;
    tx.commit().await?;
    Ok(revision)
}
