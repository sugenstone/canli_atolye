//! Faz 5 operasyon repository: bloke kataloğu, bloke kayıtları, dosya ekleri.

use crate::domain::entities::{Attachment, BlockReason, ProcessBlock};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

pub struct OperationRepository;

impl OperationRepository {
    // --- Bloke kataloğu ---

    pub async fn insert_reason(
        pool: &SqlitePool,
        workspace_id: Uuid,
        name: &str,
        description: Option<&str>,
        sort_order: i64,
    ) -> Result<BlockReason, DomainError> {
        let r = BlockReason {
            id: new_id(),
            workspace_id,
            name: name.into(),
            description: description.map(Into::into),
            active: true,
            sort_order,
            created_at: now(),
            updated_at: now(),
        };
        sqlx::query(
            "INSERT INTO block_reasons (id, workspace_id, name, description, active, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(r.id)
        .bind(r.workspace_id)
        .bind(&r.name)
        .bind(&r.description)
        .bind(r.active)
        .bind(r.sort_order)
        .bind(r.created_at)
        .bind(r.updated_at)
        .execute(pool)
        .await?;
        Ok(r)
    }

    pub async fn list_reasons(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<BlockReason>, DomainError> {
        Ok(sqlx::query_as::<_, BlockReason>(
            "SELECT * FROM block_reasons WHERE workspace_id = ?1 AND active = 1 ORDER BY sort_order, name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_reason(
        pool: &SqlitePool,
        workspace_id: Uuid,
        reason_id: Uuid,
    ) -> Result<BlockReason, DomainError> {
        sqlx::query_as::<_, BlockReason>(
            "SELECT * FROM block_reasons WHERE id = ?1 AND workspace_id = ?2 AND active = 1",
        )
        .bind(reason_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::Validation {
            message: "Geçersiz bloke nedeni.".into(),
        })
    }

    // --- Bloke kayıtları ---

    pub async fn insert_block(
        tx: &mut SqliteConnection,
        execution_id: Uuid,
        reason_id: Uuid,
        description: Option<&str>,
        created_by: Uuid,
    ) -> Result<Uuid, DomainError> {
        let id = new_id();
        sqlx::query(
            "INSERT INTO process_blocks (id, process_execution_id, reason_id, description, created_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(id)
        .bind(execution_id)
        .bind(reason_id)
        .bind(description)
        .bind(created_by)
        .bind(now())
        .execute(&mut *tx)
        .await?;
        Ok(id)
    }

    /// Execution'ın AÇIK (çözülmemiş) bloke kaydı.
    pub async fn open_block(
        tx: &mut SqliteConnection,
        execution_id: Uuid,
    ) -> Result<Option<ProcessBlock>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessBlock>(
            "SELECT * FROM process_blocks
             WHERE process_execution_id = ?1 AND resolved_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(execution_id)
        .fetch_optional(&mut *tx)
        .await?)
    }

    pub async fn resolve_block(
        tx: &mut SqliteConnection,
        block_id: Uuid,
        resolved_by: Uuid,
        resolution_note: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE process_blocks SET resolved_by = ?2, resolved_at = ?3, resolution_note = ?4
             WHERE id = ?1 AND resolved_at IS NULL",
        )
        .bind(block_id)
        .bind(resolved_by)
        .bind(now())
        .bind(resolution_note)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    pub async fn blocks_of_execution(
        pool: &SqlitePool,
        execution_id: Uuid,
    ) -> Result<Vec<ProcessBlock>, DomainError> {
        Ok(sqlx::query_as::<_, ProcessBlock>(
            "SELECT * FROM process_blocks WHERE process_execution_id = ?1 ORDER BY created_at",
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await?)
    }

    // --- Dosya ekleri ---

    pub async fn insert_attachment(
        pool: &SqlitePool,
        a: &Attachment,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO attachments (id, workspace_id, entity_type, entity_id, file_name, storage_key, mime_type, size, uploaded_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(a.id)
        .bind(a.workspace_id)
        .bind(&a.entity_type)
        .bind(a.entity_id)
        .bind(&a.file_name)
        .bind(&a.storage_key)
        .bind(&a.mime_type)
        .bind(a.size)
        .bind(a.uploaded_by)
        .bind(a.created_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_attachments(
        pool: &SqlitePool,
        workspace_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<Attachment>, DomainError> {
        Ok(sqlx::query_as::<_, Attachment>(
            "SELECT * FROM attachments
             WHERE workspace_id = ?1 AND entity_type = ?2 AND entity_id = ?3 AND deleted_at IS NULL
             ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_attachment(
        pool: &SqlitePool,
        workspace_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Attachment, DomainError> {
        sqlx::query_as::<_, Attachment>(
            "SELECT * FROM attachments WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
        )
        .bind(attachment_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    pub async fn soft_delete_attachment(
        pool: &SqlitePool,
        attachment_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE attachments SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL")
            .bind(attachment_id)
            .bind(now())
            .execute(pool)
            .await?;
        Ok(())
    }
}
