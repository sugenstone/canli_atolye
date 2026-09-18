//! Session repository — cookie'deki token'ın SHA-256 hash'i saklanır,
//! token'ın kendisi asla DB'ye yazılmaz (docs/architecture.md §23).

use crate::domain::entities::Session;
use crate::domain::errors::DomainError;
use crate::shared::now;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SessionRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> SessionRepository<'a> {
    pub async fn insert(
        &self,
        token_hash: &str,
        user_id: Uuid,
        workspace_id: Uuid,
        expires_at: DateTime<Utc>,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, workspace_id, created_at, expires_at, user_agent, ip)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(token_hash)
        .bind(user_id)
        .bind(workspace_id)
        .bind(now())
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Token hash'ine ait, süresi geçmemiş aktif session'ı getir.
    pub async fn find_valid(&self, token_hash: &str) -> Result<Option<Session>, DomainError> {
        Ok(sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE id = ?1 AND expires_at > ?2",
        )
        .bind(token_hash)
        .bind(now())
        .fetch_optional(self.pool)
        .await?)
    }

    pub async fn delete(&self, token_hash: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?1")
            .bind(token_hash)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Süresi dolmuş session'ları temizle (login sırasında periyodik çağrılır).
    pub async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?1")
            .bind(now())
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
