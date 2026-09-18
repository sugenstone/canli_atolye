//! Kullanıcı repository. `password_hash` asla dışarı serileştirilmez;
//! API DTO'ları (application/dto) hash'i dışlar.

use crate::domain::entities::{Role, User};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Yeni kullanıcı oluşturma girdisi (hash service katmanında üretilir).
pub struct NewUser {
    pub workspace_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub role: Role,
}

pub struct UserPatch {
    pub full_name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

pub struct UserRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> UserRepository<'a> {
    pub async fn insert(&self, input: NewUser) -> Result<User, DomainError> {
        let user = User {
            id: new_id(),
            workspace_id: input.workspace_id,
            email: input.email.to_lowercase(),
            password_hash: input.password_hash,
            full_name: input.full_name,
            role: input.role,
            active: true,
            created_at: now(),
            updated_at: now(),
        };
        sqlx::query(
            "INSERT INTO users (id, workspace_id, email, password_hash, full_name, role, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(user.id)
        .bind(user.workspace_id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(user.role)
        .bind(user.active)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(self.pool)
        .await
        .map_err(|e| map_unique_violation(e, &user.email))?;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<User, DomainError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?
            .ok_or(DomainError::UserNotFound)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?1")
            .bind(email.to_lowercase())
            .fetch_optional(self.pool)
            .await
            .map_err(DomainError::from)
    }

    /// Workspace izolasyonu: liste her zaman workspace sınırlı.
    pub async fn list_by_workspace(&self, workspace_id: Uuid) -> Result<Vec<User>, DomainError> {
        Ok(sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE workspace_id = ?1 ORDER BY created_at",
        )
        .bind(workspace_id)
        .fetch_all(self.pool)
        .await?)
    }

    pub async fn update(&self, id: Uuid, patch: &UserPatch) -> Result<User, DomainError> {
        let existing = self.find_by_id(id).await?;
        let updated = User {
            full_name: patch.full_name.clone().unwrap_or(existing.full_name),
            role: patch.role.unwrap_or(existing.role),
            active: patch.active.unwrap_or(existing.active),
            updated_at: now(),
            ..existing
        };
        sqlx::query(
            "UPDATE users SET full_name = ?2, role = ?3, active = ?4, updated_at = ?5
             WHERE id = ?1",
        )
        .bind(id)
        .bind(&updated.full_name)
        .bind(updated.role)
        .bind(updated.active)
        .bind(updated.updated_at)
        .execute(self.pool)
        .await?;
        Ok(updated)
    }

    /// Soft pasifleştirme: geçmiş kayıtlar korunur (silme yok).
    pub async fn deactivate(&self, id: Uuid) -> Result<User, DomainError> {
        self.update(id, &UserPatch { full_name: None, role: None, active: Some(false) })
            .await
    }
}

fn map_unique_violation(err: sqlx::Error, email: &str) -> DomainError {
    match &err {
        sqlx::Error::Database(db_err) if db_err.message().contains("UNIQUE") => {
            tracing::warn!(email, "e-posta zaten kayıtlı");
            DomainError::EmailTaken
        }
        _ => DomainError::Database(err),
    }
}
