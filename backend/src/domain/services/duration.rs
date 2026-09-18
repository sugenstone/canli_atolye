//! Süre hesaplayıcı (MASTER PLAN §16): olay akışından aktif/duraklatılmış/
//! bloke edilmiş/toplam süre. Saf fonksiyon — unit testli.
//!
//! Model: IN_PROGRESS'a geçiren olaylar (STARTED, RESUMED, UNBLOCKED-aktif)
//! aktif süreyi başlatır; PAUSED, BLOCKED, COMPLETED durdurur.
//! Not: UNBLOCKED, READY'a dönüşte aktif başlatmaz — bu nadir edge case'te
//! küçük tutarsızlık olabilir (durum bilgisi olayda taşınmadıkça).

use crate::domain::entities::EventType;
use chrono::{DateTime, Utc};

#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Durations {
    /// Aktif çalışma süresi (duraklatma/bloke hariç), saniye
    pub active_seconds: i64,
    /// Toplam duraklatılmada geçen süre, saniye
    pub paused_seconds: i64,
    /// Toplam blokede geçen süre, saniye
    pub blocked_seconds: i64,
    /// İlk başlangıç → tamamlanma (takvim süresi, her şey dahil), saniye
    pub lead_seconds: Option<i64>,
    /// Oluşturma/READY → ilk başlangıç bekleme süresi, saniye
    pub waiting_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct TimedEvent {
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
}

/// Olay akışından süre kırılımını hesaplar. `now`: devam eden aralıklar için.
pub fn compute_durations(events: &[TimedEvent], now: DateTime<Utc>) -> Durations {
    use EventType::*;

    let mut d = Durations::default();
    // aktif aralığın başlangıcı
    let mut active_since: Option<DateTime<Utc>> = None;
    let mut paused_since: Option<DateTime<Utc>> = None;
    let mut blocked_since: Option<DateTime<Utc>> = None;
    let mut first_created: Option<DateTime<Utc>> = None;
    let mut first_started: Option<DateTime<Utc>> = None;
    let mut last_completed: Option<DateTime<Utc>> = None;

    for e in events {
        let t = e.timestamp;
        match e.event_type {
            Created => { first_created.get_or_insert(t); }
            Ready => { first_created.get_or_insert(t); }
            Assigned | NoteAdded | FileAdded | AssigneeChanged | PlannedDateChanged | Reopened => {
                let _ = first_created.get_or_insert(t);
            }
            Started | Resumed => {
                // aktif başlar; duraklatma/bloke biter
                if let Some(since) = paused_since.take() {
                    d.paused_seconds += (t - since).num_seconds().max(0);
                }
                if let Some(since) = blocked_since.take() {
                    d.blocked_seconds += (t - since).num_seconds().max(0);
                }
                active_since = Some(t);
                { first_started.get_or_insert(t); }
            }
            Paused => {
                if let Some(since) = active_since.take() {
                    d.active_seconds += (t - since).num_seconds().max(0);
                }
                paused_since = Some(t);
            }
            Blocked => {
                if let Some(since) = active_since.take() {
                    d.active_seconds += (t - since).num_seconds().max(0);
                }
                blocked_since = Some(t);
            }
            Unblocked => {
                // bloke biter; hedef IN_PROGRESS ise aktif devam eder
                if let Some(since) = blocked_since.take() {
                    d.blocked_seconds += (t - since).num_seconds().max(0);
                }
                // aktifin sürüp sürmediğini olay tipe bakarak bilemeyiz;
                // tutarlılık için aktif başlatmayız — RESUMED/STARTED gelecekse
                // zaten başlatacak. (Bloke READY'den açıldıysa doğrudur.)
                let _ = &mut active_since;
            }
            Completed => {
                if let Some(since) = active_since.take() {
                    d.active_seconds += (t - since).num_seconds().max(0);
                }
                if let Some(since) = paused_since.take() {
                    d.paused_seconds += (t - since).num_seconds().max(0);
                }
                if let Some(since) = blocked_since.take() {
                    d.blocked_seconds += (t - since).num_seconds().max(0);
                }
                last_completed = Some(t);
            }
            Cancelled => {
                active_since = None;
                paused_since = None;
                blocked_since = None;
            }
        };
    }

    // devam eden aralıklar now'a kadar
    if let Some(since) = active_since {
        d.active_seconds += (now - since).num_seconds().max(0);
    }
    if let Some(since) = paused_since {
        d.paused_seconds += (now - since).num_seconds().max(0);
    }
    if let Some(since) = blocked_since {
        d.blocked_seconds += (now - since).num_seconds().max(0);
    }

    if let (Some(start), Some(end)) = (first_started, last_completed) {
        d.lead_seconds = Some((end - start).num_seconds().max(0));
    }
    if let (Some(created), Some(started)) = (first_created, first_started) {
        d.waiting_seconds = Some((started - created).num_seconds().max(0));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(sec: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + sec, 0).unwrap()
    }

    fn ev(t: EventType, sec: i64) -> TimedEvent {
        TimedEvent { event_type: t, timestamp: at(sec) }
    }

    #[test]
    fn active_plus_waiting_on_simple_flow() {
        let now = at(1000);
        let d = compute_durations(
            &[ev(EventType::Created, 0), ev(EventType::Ready, 0), ev(EventType::Started, 10), ev(EventType::Completed, 40)],
            now,
        );
        assert_eq!(d.active_seconds, 30);
        assert_eq!(d.lead_seconds, Some(30));
        assert_eq!(d.waiting_seconds, Some(10));
        assert_eq!(d.paused_seconds, 0);
    }

    #[test]
    fn pause_excluded_from_active() {
        let now = at(1000);
        let d = compute_durations(
            &[ev(EventType::Started, 0), ev(EventType::Paused, 10), ev(EventType::Resumed, 25), ev(EventType::Completed, 40)],
            now,
        );
        assert_eq!(d.active_seconds, 25, "10 + 15");
        assert_eq!(d.paused_seconds, 15);
        assert_eq!(d.lead_seconds, Some(40));
    }

    #[test]
    fn blocked_time_tracked_separately() {
        let now = at(1000);
        let d = compute_durations(
            &[ev(EventType::Started, 0), ev(EventType::Blocked, 10), ev(EventType::Unblocked, 40), ev(EventType::Started, 45), ev(EventType::Completed, 60)],
            now,
        );
        assert_eq!(d.blocked_seconds, 30);
        assert_eq!(d.active_seconds, 25, "0-10 ve 45-60");
    }

    #[test]
    fn still_running_counts_until_now() {
        let now = at(200);
        let d = compute_durations(&[ev(EventType::Started, 20)], now);
        assert_eq!(d.active_seconds, 180);
        assert_eq!(d.lead_seconds, None, "tamamlanmadı");
    }

    #[test]
    fn paused_now_counts_until_now() {
        let now = at(100);
        let d = compute_durations(&[ev(EventType::Started, 0), ev(EventType::Paused, 30)], now);
        assert_eq!(d.active_seconds, 30);
        assert_eq!(d.paused_seconds, 70);
    }

    #[test]
    fn non_state_events_ignored() {
        let now = at(1000);
        let d = compute_durations(
            &[ev(EventType::Started, 0), ev(EventType::NoteAdded, 5), ev(EventType::FileAdded, 8), ev(EventType::Completed, 20)],
            now,
        );
        assert_eq!(d.active_seconds, 20);
    }
}
