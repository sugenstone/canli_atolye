//! Faz 5 operasyon service'i: bloke (neden kataloğuyla), not ekleme ve
//! dosya ekleme (storage abstraction arkasında). MASTER PLAN §18, §19.

use crate::domain::entities::{
    Attachment, BlockReason, EventType, ProcessBlock, ProcessExecution, ProcessStatus, User,
};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::domain::services::process_engine;
use crate::infrastructure::repositories::operation_repository::OperationRepository;
use crate::infrastructure::repositories::process_repository::{
    EventRepository, ExecutionRepository, NewEvent, StatusPatch,
};
use crate::infrastructure::storage::FileStorage;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MB

/// İzin verilen MIME tipleri (MASTER PLAN §19 örnek listesi).
const ALLOWED_MIME: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/webp",
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/acad",        // DWG
    "application/dxf",         // DXF
    "application/octet-stream", // bilinmeyen ama uzantı izinli (aşağıda kontrol)
];

/// Uzantı bazlı ikincil kontrol (MIME beyanına tam güvenilmez).
fn extension_allowed(file_name: &str) -> bool {
    let ext = file_name.rsplit('.').next().map(|e| e.to_lowercase()).unwrap_or_default();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "pdf" | "xlsx" | "dwg" | "dxf"
    )
}

/// Dosya adını güvenli hale getir: yalnız harf/rakam/._- ve boşluk→alt çizgi.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            let c = match c {
                'ç' => 'c', 'ğ' => 'g', 'ı' => 'i', 'ö' => 'o', 'ş' => 's', 'ü' => 'u',
                'Ç' => 'C', 'Ğ' => 'G', 'İ' => 'I', 'Ö' => 'O', 'Ş' => 'S', 'Ü' => 'U',
                c => c,
            };
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(|c| c == '.' || c == '_');
    if trimmed.is_empty() {
        "dosya".into()
    } else {
        trimmed.to_string()
    }
}

// Yetki: worker kendi/takım işinde, ADMIN/PM her zaman (process_service ile aynı).
async fn can_operate(
    tx: &mut SqliteConnection,
    actor: &User,
    exec: &ProcessExecution,
) -> Result<bool, DomainError> {
    if can(actor.role, Action::UpdateWorkItem) {
        return Ok(true);
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

async fn load_exec_for_update(
    tx: &mut SqliteConnection,
    actor: &User,
    execution_id: Uuid,
) -> Result<ProcessExecution, DomainError> {
    sqlx::query_as::<_, ProcessExecution>(
        "SELECT * FROM process_executions WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
    )
    .bind(execution_id)
    .bind(actor.workspace_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DomainError::NotFound)
}

// ---------------------------------------------------------------------------
// Bloke kataloğu
// ---------------------------------------------------------------------------

pub async fn list_reasons(
    pool: &SqlitePool,
    actor: &User,
) -> Result<Vec<BlockReason>, DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    OperationRepository::list_reasons(pool, actor.workspace_id).await
}

pub async fn create_reason(
    pool: &SqlitePool,
    actor: &User,
    name: &str,
    description: Option<&str>,
) -> Result<BlockReason, DomainError> {
    if !can(actor.role, Action::ManageWorkItemTypes) {
        // katalog yönetimi ADMIN (workspace seviyesi)
        return Err(DomainError::Forbidden);
    }
    let name = name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation {
            message: "Neden adı zorunludur.".into(),
        });
    }
    let (max,): (Option<i64>,) = sqlx::query_as(
        "SELECT MAX(sort_order) FROM block_reasons WHERE workspace_id = ?1",
    )
    .bind(actor.workspace_id)
    .fetch_one(pool)
    .await?;
    OperationRepository::insert_reason(pool, actor.workspace_id, name, description, max.unwrap_or(0) + 1).await
}

// ---------------------------------------------------------------------------
// Bloke / çöz
// ---------------------------------------------------------------------------

/// "Sorun Bildir": reason zorunlu (MASTER PLAN §67), açıklama opsiyonel.
pub async fn block(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    reason_id: Uuid,
    description: Option<String>,
) -> Result<ProcessExecution, DomainError> {
    let reason = OperationRepository::find_reason(pool, actor.workspace_id, reason_id).await?;

    let mut tx = pool.begin().await?;
    let exec = load_exec_for_update(&mut *tx, actor, execution_id).await?;
    if !can_operate(&mut *tx, actor, &exec).await? {
        return Err(DomainError::Forbidden);
    }

    let (new_status, before_block) =
        process_engine::transition(exec.status, process_engine::Transition::Block)
            .map_err(|e| DomainError::Validation { message: e.to_string() })?;

    let patch = StatusPatch {
        status: new_status,
        status_before_block: before_block,
        ready_at: None,
        started_at: None,
        completed_at: None,
        expected_version: exec.version,
    };
    let updated = ExecutionRepository::update_status_tx(&mut *tx, &exec, &patch).await?;

    // Bloke kaydı
    OperationRepository::insert_block(
        &mut *tx,
        exec.id,
        reason.id,
        description.as_deref(),
        actor.id,
    )
    .await?;

    // Olay: not = neden adı; metadata'da neden kimliği
    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: EventType::Blocked,
            previous_status: Some(exec.status),
            new_status: Some(new_status),
            user_id: actor.id,
            team_id: exec.assigned_team_id,
            note: Some(reason.name.clone()),
            metadata: Some(format!(r#"{{"reason_id":"{0}"}}"#, reason.id)),
        },
    )
    .await?;

    tx.commit().await?;
    Ok(updated)
}

/// Blokeyi kaldır: yalnızca ADMIN/PM (MASTER PLAN §14). Açık kaydı çöz.
pub async fn unblock(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    resolution_note: Option<String>,
) -> Result<ProcessExecution, DomainError> {
    if !can(actor.role, Action::UpdateWorkItem) {
        return Err(DomainError::Forbidden);
    }

    let mut tx = pool.begin().await?;
    let exec = load_exec_for_update(&mut *tx, actor, execution_id).await?;
    let target = process_engine::unblock_target(exec.status_before_block)
        .map_err(|e| DomainError::Validation { message: e.to_string() })?;

    let patch = StatusPatch {
        status: target,
        status_before_block: None,
        ready_at: (target == ProcessStatus::Ready).then(crate::shared::now),
        started_at: None,
        completed_at: None,
        expected_version: exec.version,
    };
    let updated = ExecutionRepository::update_status_tx(&mut *tx, &exec, &patch).await?;

    if let Some(open) = OperationRepository::open_block(&mut *tx, exec.id).await? {
        OperationRepository::resolve_block(&mut *tx, open.id, actor.id, resolution_note.as_deref())
            .await?;
    }

    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: EventType::Unblocked,
            previous_status: Some(exec.status),
            new_status: Some(target),
            user_id: actor.id,
            team_id: exec.assigned_team_id,
            note: resolution_note,
            metadata: None,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(updated)
}

pub async fn blocks_of_execution(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
) -> Result<Vec<ProcessBlock>, DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    let exec = ExecutionRepository::find_in_workspace(pool, actor.workspace_id, execution_id).await?;
    OperationRepository::blocks_of_execution(pool, exec.id).await
}

// ---------------------------------------------------------------------------
// Notlar — NOTE_ADDED olayı olarak (immutable akışın parçası)
// ---------------------------------------------------------------------------

pub async fn add_note(
    pool: &SqlitePool,
    actor: &User,
    execution_id: Uuid,
    note: String,
) -> Result<(), DomainError> {
    let note = note.trim().to_string();
    if note.is_empty() {
        return Err(DomainError::Validation {
            message: "Not boş olamaz.".into(),
        });
    }

    let mut tx = pool.begin().await?;
    let exec = load_exec_for_update(&mut *tx, actor, execution_id).await?;
    if !can_operate(&mut *tx, actor, &exec).await? {
        return Err(DomainError::Forbidden);
    }

    EventRepository::insert(
        &mut *tx,
        exec.workspace_id,
        &NewEvent {
            process_execution_id: exec.id,
            event_type: EventType::NoteAdded,
            previous_status: None,
            new_status: None,
            user_id: actor.id,
            team_id: exec.assigned_team_id,
            note: Some(note),
            metadata: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Dosya ekleri — storage abstraction arkasında
// ---------------------------------------------------------------------------

fn ensure_entity_type(t: &str) -> Result<(), DomainError> {
    if matches!(t, "PROJECT" | "SECTION" | "WORK_ITEM" | "PROCESS_EXECUTION") {
        Ok(())
    } else {
        Err(DomainError::Validation {
            message: "Geçersiz entity_type.".into(),
        })
    }
}

/// Eklenen varlık bu workspace'te mi? (izolasyon)
async fn ensure_entity_in_workspace(
    pool: &SqlitePool,
    actor: &User,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(), DomainError> {
    let ok = match entity_type {
        "PROJECT" => sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM projects WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL"),
        "SECTION" => sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM sections WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL"),
        "WORK_ITEM" => sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM work_items WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL"),
        "PROCESS_EXECUTION" => sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM process_executions WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL"),
        _ => return Err(DomainError::Validation { message: "Geçersiz entity_type.".into() }),
    }
    .bind(entity_id)
    .bind(actor.workspace_id)
    .fetch_one(pool)
    .await?
    .0 > 0;
    if ok {
        Ok(())
    } else {
        Err(DomainError::NotFound)
    }
}

pub struct UploadInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    pub file_name: &'a str,
    pub mime_type: &'a str,
    pub bytes: Vec<u8>,
}

pub async fn upload(
    pool: &SqlitePool,
    storage: &dyn FileStorage,
    actor: &User,
    input: UploadInput<'_>,
) -> Result<Attachment, DomainError> {
    ensure_entity_type(input.entity_type)?;
    ensure_entity_in_workspace(pool, actor, input.entity_type, input.entity_id).await?;

    if input.bytes.is_empty() {
        return Err(DomainError::Validation {
            message: "Boş dosya yüklenemez.".into(),
        });
    }
    if input.bytes.len() > MAX_UPLOAD_BYTES {
        return Err(DomainError::Validation {
            message: "Dosya boyutu 10 MB sınırını aşıyor.".into(),
        });
    }
    if !extension_allowed(input.file_name) {
        return Err(DomainError::Validation {
            message: "İzin verilmeyen dosya türü (jpg, png, webp, pdf, xlsx, dwg, dxf).".into(),
        });
    }
    if !ALLOWED_MIME.contains(&input.mime_type) {
        return Err(DomainError::Validation {
            message: "İzin verilmeyen içerik türü.".into(),
        });
    }

    let safe_name = sanitize_file_name(input.file_name);
    let attachment = Attachment {
        id: crate::shared::new_id(),
        workspace_id: actor.workspace_id,
        entity_type: input.entity_type.into(),
        entity_id: input.entity_id,
        storage_key: format!("{}/{}/{}", actor.workspace_id, attachment_uuid(), safe_name),
        file_name: safe_name,
        mime_type: input.mime_type.into(),
        size: input.bytes.len() as i64,
        uploaded_by: actor.id,
        created_at: crate::shared::now(),
        deleted_at: None,
    };

    storage.put(&attachment.storage_key, input.bytes).await?;
    OperationRepository::insert_attachment(pool, &attachment).await?;

    // PROCESS_EXECUTION eklerinde FILE_ADDED olayı da üret
    if input.entity_type == "PROCESS_EXECUTION" {
        let mut tx = pool.begin().await?;
        EventRepository::insert(
            &mut *tx,
            actor.workspace_id,
            &NewEvent {
                process_execution_id: input.entity_id,
                event_type: EventType::FileAdded,
                previous_status: None,
                new_status: None,
                user_id: actor.id,
                team_id: None,
                note: Some(attachment.file_name.clone()),
                metadata: Some(format!(r#"{{"attachment_id":"{0}"}}"#, attachment.id)),
            },
        )
        .await?;
        tx.commit().await?;
    }

    Ok(attachment)
}

fn attachment_uuid() -> String {
    crate::shared::new_id().to_string()
}

pub async fn list_attachments(
    pool: &SqlitePool,
    actor: &User,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Vec<Attachment>, DomainError> {
    ensure_entity_type(entity_type)?;
    ensure_entity_in_workspace(pool, actor, entity_type, entity_id).await?;
    OperationRepository::list_attachments(pool, actor.workspace_id, entity_type, entity_id).await
}

pub async fn download(
    pool: &SqlitePool,
    storage: &dyn FileStorage,
    actor: &User,
    attachment_id: Uuid,
) -> Result<(Attachment, Vec<u8>), DomainError> {
    let attachment = OperationRepository::find_attachment(pool, actor.workspace_id, attachment_id).await?;
    let bytes = storage.get(&attachment.storage_key).await?;
    Ok((attachment, bytes))
}

pub async fn delete(
    pool: &SqlitePool,
    actor: &User,
    attachment_id: Uuid,
) -> Result<(), DomainError> {
    let attachment = OperationRepository::find_attachment(pool, actor.workspace_id, attachment_id).await?;
    // Yükleyen ya da ADMIN silebilir
    if attachment.uploaded_by != actor.id && !can(actor.role, Action::ManageWorkspace) {
        return Err(DomainError::Forbidden);
    }
    OperationRepository::soft_delete_attachment(pool, attachment.id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_turkish_and_unsafe_chars() {
        assert_eq!(sanitize_file_name("tezgah ölçü (1).jpg"), "tezgah_olcu__1_.jpg");
        assert_eq!(sanitize_file_name("../../etc/passwd"), "etc_passwd", "yol ayraçları alt çizgiye iner, içeriğe giremez");
        assert_eq!(sanitize_file_name("..."), "dosya");
        assert_eq!(sanitize_file_name("rapor-final_v2.PDF"), "rapor-final_v2.PDF");
    }

    #[test]
    fn extension_gate() {
        assert!(extension_allowed("foto.JPG"));
        assert!(extension_allowed("cizim.dwg"));
        assert!(!extension_allowed("script.exe"));
        assert!(!extension_allowed("belge.doc"));
        assert!(!extension_allowed("sikistirilmis.zip"));
    }
}
