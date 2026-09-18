//! Merkezi hata dönüşümü: DomainError → standart JSON hata formatı
//! (docs/architecture.md §19). Handler'larda dağınık map yok.

use crate::domain::errors::DomainError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "code": self.code,
            "message": self.message,
        }));
        (self.status, body).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        use DomainError as E;
        match err {
            // Kimlik
            E::InvalidCredentials => ApiError::new(
                StatusCode::UNAUTHORIZED,
                "AUTH_INVALID_CREDENTIALS",
                err.to_string(),
            ),
            E::SessionExpired => ApiError::new(
                StatusCode::UNAUTHORIZED,
                "AUTH_SESSION_EXPIRED",
                err.to_string(),
            ),
            E::Forbidden => ApiError::new(
                StatusCode::FORBIDDEN,
                "AUTH_FORBIDDEN",
                err.to_string(),
            ),

            // Kaynak yok
            E::NotFound
            | E::WorkspaceNotFound
            | E::UserNotFound
            | E::TeamNotFound
            | E::ProjectNotFound => {
                ApiError::new(StatusCode::NOT_FOUND, "NOT_FOUND", err.to_string())
            }

            // Doğrulama / çakışma
            E::Validation { .. } => ApiError::new(
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                err.to_string(),
            ),
            E::EmailTaken => ApiError::new(
                StatusCode::CONFLICT,
                "EMAIL_TAKEN",
                err.to_string(),
            ),
            E::Conflict { .. } => ApiError::new(
                StatusCode::CONFLICT,
                "CONFLICT",
                err.to_string(),
            ),

            // Altyapı: detayı logla, response'a sızdırma
            E::Database(source) => {
                tracing::error!(error = ?source, "veritabanı hatası");
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Sunucu hatası oluştu.",
                )
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        DomainError::Database(err).into()
    }
}

/// Handler kolaylığı: `Result<T, ApiError>`
pub type ApiResult<T> = Result<T, ApiError>;
