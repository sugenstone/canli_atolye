//! Ortak yardımcılar: zaman ve ID üretimi (docs/architecture.md §24).

use chrono::Utc;
use uuid::Uuid;

/// Yeni UUIDv7 (zaman sıralı, uygulama tarafında üretilen ID).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

/// Şu anki UTC zamanı. Tüm timestamp'ler DB'de UTC saklanır.
pub fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}
