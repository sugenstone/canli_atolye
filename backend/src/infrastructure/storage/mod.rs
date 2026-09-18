//! FileStorage abstraction (MASTER PLAN §19, §108):
//! Domain yalnızca `storage_key` bilir; disk/S3 detayı burada izole edilir.
//! Development: LocalFileStorage. Production: S3FileStorage (sonraki faz).

use crate::domain::errors::DomainError;
use axum::body::Body;
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait FileStorage: Send + Sync {
    /// Baytları verilen anahtarla saklar.
    async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), DomainError>;
    /// Anahtardaki içeriği akıtar. Dosya yoksa NotFound.
    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError>;
    /// İçeriği siler (soft-delete sonrası temizlik).
    async fn delete(&self, key: &str) -> Result<(), DomainError>;
}

/// Geliştirme deposu: `data/uploads/{workspace}/{uuid}/{safe_name}`.
pub struct LocalFileStorage {
    root: PathBuf,
}

impl LocalFileStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_of(&self, key: &str) -> Result<PathBuf, DomainError> {
        // Anahtar enjeksiyona kapalı olmalı: yalnız [A-Za-z0-9/_-.] izinli
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.'))
            || key.contains("..")
        {
            return Err(DomainError::Validation {
                message: "Geçersiz depolama anahtarı.".into(),
            });
        }
        Ok(self.root.join(key))
    }
}

#[async_trait::async_trait]
impl FileStorage for LocalFileStorage {
    async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), DomainError> {
        let path = self.path_of(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DomainError::Conflict {
                    message: format!("Depolama dizini oluşturulamadı: {e}"),
                })?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| DomainError::Conflict {
                message: format!("Dosya yazılamadı: {e}"),
            })?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError> {
        let path = self.path_of(key)?;
        tokio::fs::read(&path)
            .await
            .map_err(|_| DomainError::NotFound)
    }

    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        let path = self.path_of(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(DomainError::Conflict {
                message: format!("Dosya silinemedi: {e}"),
            }),
        }
    }
}

/// axum response body'ye bayt akışı (Content-Type üstbilgisi handler'da set edilir).
pub fn bytes_to_body(bytes: Vec<u8>) -> Body {
    Body::from(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_keys() {
        let storage = LocalFileStorage::new("/tmp/uploads");
        assert!(storage.path_of("../etc/passwd").is_err());
        assert!(storage.path_of("a/../../b").is_err());
        assert!(storage.path_of("").is_err());
        assert!(storage.path_of("ws/file_1.pdf").is_ok());
        assert!(storage.path_of("ws/uuid/tezgah-01.jpg").is_ok());
    }
}
