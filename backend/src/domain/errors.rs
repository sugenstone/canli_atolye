//! Domain hataları → API katmanı bunları merkezi handler'da HTTP yanıtına çevirir
//! (docs/architecture.md §19). UI teknik hata göstermez; code bazlı mesaj eşler.

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    // --- Kimlik / yetki ---
    #[error("E-posta veya şifre hatalı.")]
    InvalidCredentials,
    #[error("Oturum bulunamadı veya süresi doldu.")]
    SessionExpired,
    #[error("Bu işlem için yetkiniz yok.")]
    Forbidden,
    #[error("Bu e-posta zaten kayıtlı.")]
    EmailTaken,

    // --- Kaynak yok ---
    #[error("Kayıt bulunamadı.")]
    NotFound,
    #[error("Workspace bulunamadı.")]
    WorkspaceNotFound,
    #[error("Kullanıcı bulunamadı.")]
    UserNotFound,
    #[error("Takım bulunamadı.")]
    TeamNotFound,
    #[error("Proje bulunamadı.")]
    ProjectNotFound,

    // --- Doğrulama ---
    #[error("Doğrulama hatası: {message}")]
    Validation { message: String },

    // --- Çakışma ---
    #[error("Kayıt zaten mevcut: {message}")]
    Conflict { message: String },

    // --- Altyapı ---
    #[error("Veritabanı hatası.")]
    Database(#[from] sqlx::Error),
}
