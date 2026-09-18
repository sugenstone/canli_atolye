//! Paylaşılan uygulama state'i.

use crate::config::Config;
use crate::infrastructure::realtime::EventHub;
use crate::infrastructure::storage::FileStorage;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub storage: Arc<dyn FileStorage>,
    pub hub: Arc<EventHub>,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Config, storage: Arc<dyn FileStorage>) -> Self {
        Self { pool, config, storage, hub: Arc::new(EventHub::new()) }
    }
}
