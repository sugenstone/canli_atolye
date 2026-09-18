//! Kimlik doğrulama: Argon2id şifre doğrulama + server-side session.
//! Cookie'de rastgele token, DB'de SHA-256 hash'i (docs/architecture.md §23).

use crate::config::Config;
use crate::domain::entities::{User, Workspace};
use crate::domain::errors::DomainError;
use crate::infrastructure::repositories::session_repository::SessionRepository;
use crate::infrastructure::repositories::user_repository::UserRepository;
use crate::infrastructure::repositories::workspace_repository::WorkspaceRepository;
use crate::shared::now;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng};
use argon2::Argon2;
use chrono::Duration;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

pub const SESSION_COOKIE: &str = "atolye_session";

/// Argon2id ile şifre hash'le.
pub fn hash_password(password: &str) -> Result<String, DomainError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| DomainError::Validation {
            message: "Şifre hash'lenemedi".into(),
        })
}

/// Şifre doğrula; hash bozuksa veya eşleşmezse false.
pub fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Login: kullanıcı doğrula → session oluştur → (kullanıcı, workspace, cookie token).
pub async fn login(
    pool: &SqlitePool,
    cfg: &Config,
    email: &str,
    password: &str,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<(User, Workspace, String), DomainError> {
    let user = UserRepository { pool }
        .find_by_email(email)
        .await?
        .ok_or(DomainError::InvalidCredentials)?;

    if !user.active {
        return Err(DomainError::InvalidCredentials);
    }
    if !verify_password(&user.password_hash, password) {
        return Err(DomainError::InvalidCredentials);
    }

    let workspace = WorkspaceRepository { pool }
        .find_by_id(user.workspace_id)
        .await?;

    // Süresi dolmuş session'ları fırsatçı temizle
    let _ = SessionRepository { pool }.delete_expired().await;

    let token = generate_token();
    let expires_at = now() + Duration::hours(cfg.session_ttl_hours);
    SessionRepository { pool }
        .insert(
            &hash_token(&token),
            user.id,
            workspace.id,
            expires_at,
            user_agent,
            ip,
        )
        .await?;

    tracing::info!(user_id = %user.id, "oturum açıldı");
    Ok((user, workspace, token))
}

/// Cookie token'ından geçerli (User, Workspace) çıkar.
pub async fn authenticate_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<(User, Workspace), DomainError> {
    let session = SessionRepository { pool }
        .find_valid(&hash_token(token))
        .await?
        .ok_or(DomainError::SessionExpired)?;

    let user = UserRepository { pool }
        .find_by_id(session.user_id)
        .await?;
    if !user.active {
        return Err(DomainError::SessionExpired);
    }
    let workspace = WorkspaceRepository { pool }
        .find_by_id(user.workspace_id)
        .await?;
    Ok((user, workspace))
}

/// Logout: session'ı sil.
pub async fn logout(pool: &SqlitePool, token: &str) -> Result<(), DomainError> {
    SessionRepository { pool }
        .delete(&hash_token(token))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let hash = hash_password("s3cret-pass").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password(&hash, "s3cret-pass"));
        assert!(!verify_password(&hash, "wrong"));
    }

    #[test]
    fn malformed_hash_fails_closed() {
        assert!(!verify_password("not-a-hash", "x"));
    }

    #[test]
    fn token_hash_is_deterministic_and_not_plaintext() {
        let token = generate_token();
        let h1 = hash_token(&token);
        let h2 = hash_token(&token);
        assert_eq!(h1, h2);
        assert_ne!(h1, token);
        assert_eq!(h1.len(), 64);
    }
}
