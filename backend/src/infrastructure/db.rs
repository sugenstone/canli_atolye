//! SQLite pool kurulumu + migration (docs/architecture.md §7, §9).
//!
//! Cross-DB kuralı: SQLite yalnızca development. Kod, PostgreSQL'e geçişi
//! zorlamayacak şekilde yazılır (vendor SQL yok, UUID/timestamp tipleri
//! Rust tarafında normalize edilir).

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

/// Pool oluştur. SQLite için `data/` klasörü otomatik hazırlanır,
/// foreign_keys + WAL pragma'ları bağlantı başına açılır.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // sqlite://data/app.db?mode=rwc → relative path için çalışma dizininde data/ hazırla
    if let Some(path) = strip_sqlite_prefix(database_url) {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let connect_options: SqliteConnectOptions = database_url
        .parse::<SqliteConnectOptions>()?
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
}

/// `sqlite://data/app.db?mode=rwc` → `data/app.db` (query'siz, yol yoksa None).
fn strip_sqlite_prefix(url: &str) -> Option<String> {
    let rest = url.strip_prefix("sqlite://")?;
    let path = rest.split('?').next()?;
    if path.is_empty() || path == ":memory:" {
        None
    } else {
        Some(path.to_string())
    }
}

/// Migration'ları çalıştır (version control'deki `migrations/` klasöründen).
pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_prefix_and_query() {
        assert_eq!(
            strip_sqlite_prefix("sqlite://data/app.db?mode=rwc"),
            Some("data/app.db".into())
        );
        assert_eq!(strip_sqlite_prefix("sqlite://x.db"), Some("x.db".into()));
        assert_eq!(strip_sqlite_prefix("sqlite://?mode=memory"), None);
    }
}
