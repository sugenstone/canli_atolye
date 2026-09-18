//! İlk çalıştırma seed'i: boş veritabanına default workspace + admin kullanıcı.
//! Mevcut veri varsa dokunmaz (idempotent).

use crate::application::services::auth_service;
use crate::config::Config;
use crate::domain::entities::Role;
use crate::domain::errors::DomainError;
use crate::infrastructure::repositories::user_repository::{NewUser, UserRepository};
use crate::infrastructure::repositories::workspace_repository::WorkspaceRepository;
use sqlx::SqlitePool;

pub async fn ensure_seed(pool: &SqlitePool, cfg: &Config) -> Result<(), DomainError> {
    let workspaces = WorkspaceRepository { pool };
    if workspaces.count().await? > 0 {
        return Ok(());
    }

    let slug = slugify(&cfg.seed_workspace_name);
    let workspace = workspaces
        .insert(&cfg.seed_workspace_name, &slug, None)
        .await?;
    tracing::info!(workspace_id = %workspace.id, "seed workspace oluşturuldu");

    let password_hash = auth_service::hash_password(&cfg.seed_admin_password)?;
    let admin = UserRepository { pool }
        .insert(NewUser {
            workspace_id: workspace.id,
            email: cfg.seed_admin_email.to_lowercase(),
            password_hash,
            full_name: "Sistem Yöneticisi".into(),
            role: Role::Admin,
        })
        .await?;
    tracing::info!(admin_id = %admin.id, email = %admin.email, "seed admin kullanıcı oluşturuldu");
    Ok(())
}

/// Türkçe karakterleri ASCII'ye çevirip URL-safe slug üret.
fn slugify(input: &str) -> String {
    let mapped: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ç' => 'c', 'ğ' => 'g', 'ı' => 'i', 'ö' => 'o', 'ş' => 's', 'ü' => 'u',
            c => c,
        })
        .collect();
    let mut slug = String::new();
    let mut prev_dash = true; // baştaki tireleri engelle
    for c in mapped.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "workspace".into()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_turkish_names() {
        assert_eq!(slugify("Canlı Atölye"), "canli-atolye");
        assert_eq!(slugify("  Gardenia Projesi! "), "gardenia-projesi");
        assert_eq!(slugify("---"), "workspace");
        assert_eq!(slugify("Ünal Şahin"), "unal-sahin");
    }
}
