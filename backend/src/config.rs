//! Uygulama konfigürasyonu: env okuma + başlangıçta doğrulama.
//! Kritik env eksikse uygulama başlamaz (MASTER PLAN §111).

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origin: Option<String>,
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    pub seed_workspace_name: String,
    pub seed_admin_email: String,
    pub seed_admin_password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Eksik zorunlu env değişkeni: {0}")]
    Missing(&'static str),
    #[error("Geçersiz env değeri {key}: {reason}")]
    Invalid { key: &'static str, reason: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // .env dosyası varsa yükle (development kolaylığı); yoksa sessiz geç.
        let _ = dotenvy::dotenv();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        if database_url.is_empty() {
            return Err(ConfigError::Invalid {
                key: "DATABASE_URL",
                reason: "boş olamaz".into(),
            });
        }

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .map_err(|_| ConfigError::Invalid {
                key: "PORT",
                reason: "16-bit sayı olmalı".into(),
            })?;

        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(Self {
            database_url,
            host,
            port,
            cors_origin: env::var("CORS_ORIGIN").ok(),
            cookie_secure,
            session_ttl_hours: 24 * 7, // 1 hafta
            seed_workspace_name: env::var("SEED_WORKSPACE_NAME")
                .unwrap_or_else(|_| "Canlı Atölye".into()),
            seed_admin_email: env::var("SEED_ADMIN_EMAIL")
                .unwrap_or_else(|_| "admin@canliatolye.local".into()),
            seed_admin_password: env::var("SEED_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "admin123".into()),
        })
    }
}
