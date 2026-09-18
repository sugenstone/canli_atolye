//! SSE event hub (MASTER PLAN §100-101): proje bazlı yayın kanalları.
//! Publish yalnızca transaction commit SONRASI yapılır (rollback olan işlem
//! event üretmez). Frontend tek merkezî EventSource ile bağlanır.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SseEvent {
    pub event: String,
    pub project_id: uuid::Uuid,
    pub work_item_id: Option<uuid::Uuid>,
    pub process_execution_id: Option<uuid::Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct EventHub {
    channels: Mutex<HashMap<uuid::Uuid, broadcast::Sender<SseEvent>>>,
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHub {
    pub fn new() -> Self {
        Self { channels: Mutex::new(HashMap::new()) }
    }

    /// Projenin kanalına abone ol (yoksa oluştur). Olay akışını döndürür.
    pub fn subscribe(&self, project_id: uuid::Uuid) -> broadcast::Receiver<SseEvent> {
        let mut channels = self.channels.lock().unwrap();
        channels
            .entry(project_id)
            .or_insert_with(|| broadcast::channel(64).0)
            .subscribe()
    }

    /// Projeye olay yayınla. Abone yoksa sessizce yok sayılır.
    pub fn publish(&self, event: SseEvent) {
        let channels = self.channels.lock().unwrap();
        if let Some(sender) = channels.get(&event.project_id) {
            // hata (alıcı kalmadı) önemli değil
            let _ = sender.send(event);
        }
    }
}

/// Yayın yardımcıları — handler'larda commit sonrası çağrılır.
pub fn process_event_name(event_type: &str) -> String {
    format!("process.{}", event_type.to_lowercase().replace('_', "."))
}
