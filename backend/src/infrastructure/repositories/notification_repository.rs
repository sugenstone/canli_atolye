//! Bildirim (MASTER PLAN §34) ve denetim kaydı (§59) repository'leri.

use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::SqlitePool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Bildirimler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Notification {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NotificationRepository;

impl NotificationRepository {
    async fn insert_for(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
        notification_type: &str,
        title: &str,
        message: &str,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO notifications (id, workspace_id, user_id, type, title, message, entity_type, entity_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(new_id())
        .bind(workspace_id)
        .bind(user_id)
        .bind(notification_type)
        .bind(title)
        .bind(message)
        .bind(entity_type)
        .bind(entity_id.map(|id| id.to_string()))
        .bind(now())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Bir kullanıcıya bildirim bırak.
    pub async fn notify_user(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
        notification_type: &str,
        title: &str,
        message: &str,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
    ) -> Result<(), DomainError> {
        Self::insert_for(pool, workspace_id, user_id, notification_type, title, message, entity_type, entity_id).await
    }

    /// Belirtilen rollerdeki tüm kullanıcılara bildirim bırak (ör. ADMIN'ler).
    pub async fn notify_roles(
        pool: &SqlitePool,
        workspace_id: Uuid,
        roles: &[&str],
        notification_type: &str,
        title: &str,
        message: &str,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
    ) -> Result<usize, DomainError> {
        let users: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM users WHERE workspace_id = ?1 AND active = 1 AND role IN (\
             SELECT value FROM json_each(?2))",
        )
        .bind(workspace_id)
        .bind(serde_json::to_string(roles).unwrap_or_default())
        .fetch_all(pool)
        .await?;
        for (user_id,) in &users {
            Self::insert_for(pool, workspace_id, *user_id, notification_type, title, message, entity_type, entity_id).await?;
        }
        Ok(users.len())
    }

    pub async fn list_for_user(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Notification>, DomainError> {
        Ok(sqlx::query_as::<_, Notification>(
            "SELECT id, workspace_id, user_id, type AS notification_type, title, message, entity_type, entity_id, read_at, created_at
             FROM notifications
             WHERE workspace_id = ?1 AND user_id = ?2
             ORDER BY created_at DESC LIMIT 50",
        )
        .bind(workspace_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn mark_read(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE notifications SET read_at = ?3 WHERE id = ?1 AND user_id = ?2 AND workspace_id = ?4 AND read_at IS NULL")
            .bind(notification_id)
            .bind(user_id)
            .bind(now())
            .bind(workspace_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn mark_all_read(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE notifications SET read_at = ?2 WHERE user_id = ?1 AND workspace_id = ?3 AND read_at IS NULL")
            .bind(user_id)
            .bind(now())
            .bind(workspace_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Denetim kayıtları (§59) — append-only
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AdminAuditLog {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// JOIN ile gelen kullanıcı adı
    #[sqlx(default)]
    #[serde(default)]
    pub full_name: String,
}

pub struct AuditRepository;

impl AuditRepository {
    pub async fn log(
        pool: &SqlitePool,
        workspace_id: Uuid,
        user_id: Uuid,
        action: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO admin_audit_logs (id, workspace_id, user_id, action, entity_type, entity_id, metadata, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(new_id())
        .bind(workspace_id)
        .bind(user_id)
        .bind(action)
        .bind(entity_type)
        .bind(entity_id)
        .bind(metadata)
        .bind(now())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list(
        pool: &SqlitePool,
        workspace_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AdminAuditLog>, DomainError> {
        Ok(sqlx::query_as::<_, AdminAuditLog>(
            "SELECT a.*, u.full_name AS \"full_name: String\"
             FROM admin_audit_logs a JOIN users u ON u.id = a.user_id
             WHERE a.workspace_id = ?1 ORDER BY a.timestamp DESC LIMIT ?2",
        )
        .bind(workspace_id)
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }
}
