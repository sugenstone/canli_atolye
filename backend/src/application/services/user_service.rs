//! Kullanıcı yönetimi service'i: RBAC + workspace izolasyonu + validasyon.

use crate::application::dto::{CreateUserRequest, UpdateUserRequest};
use crate::domain::entities::{Role, User};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::user_repository::{
    NewUser, UserPatch, UserRepository,
};
use sqlx::SqlitePool;
use uuid::Uuid;

const MIN_PASSWORD_LEN: usize = 8;

pub async fn list(pool: &SqlitePool, actor: &User, workspace_id: Uuid) -> Result<Vec<User>, DomainError> {
    ensure_member_lister(actor, workspace_id)?;
    Ok(UserRepository { pool }.list_by_workspace(workspace_id).await?)
}

pub async fn create(
    pool: &SqlitePool,
    actor: &User,
    workspace_id: Uuid,
    req: CreateUserRequest,
) -> Result<User, DomainError> {
    ensure_manager(actor, workspace_id)?;

    let email = req.email.trim().to_lowercase();
    if !email.contains('@') || !email.contains('.') {
        return Err(DomainError::Validation { message: "Geçerli bir e-posta girin.".into() });
    }
    if req.password.len() < MIN_PASSWORD_LEN {
        return Err(DomainError::Validation {
            message: format!("Şifre en az {MIN_PASSWORD_LEN} karakter olmalı."),
        });
    }
    if req.full_name.trim().is_empty() {
        return Err(DomainError::Validation { message: "Ad soyad zorunludur.".into() });
    }

    let hash = super::auth_service::hash_password(&req.password)?;
    let created = UserRepository { pool }
        .insert(NewUser {
            workspace_id,
            email,
            password_hash: hash,
            full_name: req.full_name.trim().to_string(),
            role: req.role,
        })
        .await?;
    let _ = crate::infrastructure::repositories::notification_repository::AuditRepository::log(
        pool, actor.workspace_id, actor.id, "user.created", Some("USER"),
        Some(&created.id.to_string()), Some(&format!("\"email\":\"{}\"", created.email)),
    ).await;
    Ok(created)
}

pub async fn update(
    pool: &SqlitePool,
    actor: &User,
    workspace_id: Uuid,
    user_id: Uuid,
    req: UpdateUserRequest,
) -> Result<User, DomainError> {
    ensure_manager(actor, workspace_id)?;

    // İzolasyon: hedef kullanıcı aynı workspace'te mi?
    let target = UserRepository { pool }.find_by_id(user_id).await?;
    if target.workspace_id != workspace_id {
        return Err(DomainError::UserNotFound);
    }
    // Son admin'in yetkisi kendi eliyle kaldırılmasın (kilitlenme koruması).
    if target.role == Role::Admin
        && (req.role.is_some_and(|r| r != Role::Admin) || req.active == Some(false))
        && target.id == actor.id
    {
        return Err(DomainError::Conflict {
            message: "Kendi admin yetkinizi kaldıramazsınız.".into(),
        });
    }

    UserRepository { pool }
        .update(
            user_id,
            &UserPatch {
                full_name: req.full_name,
                role: req.role,
                active: req.active,
            },
        )
        .await
}

/// Pasifleştirme (soft delete): kayıt ve geçmişi korunur.
pub async fn deactivate(
    pool: &SqlitePool,
    actor: &User,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<User, DomainError> {
    ensure_manager(actor, workspace_id)?;
    if user_id == actor.id {
        return Err(DomainError::Conflict {
            message: "Kendinizi pasifleştiremezsiniz.".into(),
        });
    }
    let target = UserRepository { pool }.find_by_id(user_id).await?;
    if target.workspace_id != workspace_id {
        return Err(DomainError::UserNotFound);
    }
    UserRepository { pool }.deactivate(user_id).await
}

fn ensure_manager(actor: &User, workspace_id: Uuid) -> Result<(), DomainError> {
    if actor.workspace_id != workspace_id {
        return Err(DomainError::Forbidden);
    }
    if !can(actor.role, Action::ManageUsers) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

/// Üye listesi görüntüleme: ADMIN + PM (atama UI'ının ihtiyacı).
fn ensure_member_lister(actor: &User, workspace_id: Uuid) -> Result<(), DomainError> {
    if actor.workspace_id != workspace_id {
        return Err(DomainError::Forbidden);
    }
    if !can(actor.role, Action::ManageUsers) && !can(actor.role, Action::UpdateProject) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}
