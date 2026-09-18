//! Bootstrap: config → tracing → DB pool + migration → seed → router → serve.

use canli_atolye_backend::api;
use canli_atolye_backend::application::services::seed;
use canli_atolye_backend::config::Config;
use canli_atolye_backend::infrastructure::db;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Config — eksik kritik env varsa başlama.
    let config = Config::from_env().map_err(|e| {
        eprintln!("Konfigürasyon hatası: {e}");
        e
    })?;

    // 2. Structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "canli_atolye_backend=info,tower_http=info".into()),
        )
        .init();

    tracing::info!(database_url = %config.database_url, "Veritabanı bağlanıyor");

    // 3. DB pool + migrations
    let pool = db::init_pool(&config.database_url).await?;
    db::migrate(&pool).await?;
    tracing::info!("Migration'lar tamam");

    // 4. Boş veritabanını seed et (ilk workspace + admin kullanıcı)
    seed::ensure_seed(&pool, &config).await?;
    tracing::info!("Seed kontrolü tamam");

    // 5. Storage (development: local filesystem)
    let storage = std::sync::Arc::new(canli_atolye_backend::infrastructure::storage::LocalFileStorage::new(
        std::path::Path::new("data").join("uploads"),
    ));

    // 6. HTTP sunucu
    let state = api::state::AppState::new(pool.clone(), config.clone(), storage);
    let app = api::router(state);

    let addr = SocketAddr::new(
        config
            .host
            .parse()
            .expect("HOST geçerli bir IP olmalı"),
        config.port,
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "API dinliyor");
    axum::serve(listener, app).await?;
    Ok(())
}
