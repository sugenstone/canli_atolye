//! Durum özetleme kuralları (matris hücreleri, dashboard KPI'ları).
//! Saf fonksiyonlar — unit testli.

use crate::domain::entities::ProcessStatus;

/// Bir iş kaleminin execution durumlarından TEK özet durumu üretir.
/// Öncelik (en kritik önce): BLOCKED > IN_PROGRESS > PAUSED > READY >
/// PENDING > COMPLETED(hepsi). None → süreç yok (boş hücre).
///
/// CANCELLED'lar özeti etkilemez; kalanların hepsi COMPLETED ise COMPLETED.
pub fn summarize_statuses(statuses: &[ProcessStatus]) -> Option<ProcessStatus> {
    use ProcessStatus as S;
    if statuses.is_empty() {
        return None;
    }
    let live: Vec<S> = statuses.iter().copied().filter(|s| *s != S::Cancelled).collect();
    if live.is_empty() {
        return Some(S::Cancelled);
    }
    if live.contains(&S::Blocked) {
        return Some(S::Blocked);
    }
    if live.contains(&S::InProgress) {
        return Some(S::InProgress);
    }
    if live.contains(&S::Paused) {
        return Some(S::Paused);
    }
    if live.contains(&S::Ready) {
        return Some(S::Ready);
    }
    if live.iter().all(|s| *s == S::Completed) {
        return Some(S::Completed);
    }
    Some(S::Pending) // kalan kombinasyon: PENDING(±COMPLETED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ProcessStatus as S;

    #[test]
    fn empty_means_no_process() {
        assert_eq!(summarize_statuses(&[]), None);
    }

    #[test]
    fn priority_blocked_first() {
        assert_eq!(
            summarize_statuses(&[S::Completed, S::InProgress, S::Blocked]),
            Some(S::Blocked)
        );
    }

    #[test]
    fn in_progress_beats_lower() {
        assert_eq!(summarize_statuses(&[S::Pending, S::InProgress]), Some(S::InProgress));
        assert_eq!(summarize_statuses(&[S::Paused, S::InProgress]), Some(S::InProgress));
    }

    #[test]
    fn paused_when_no_active() {
        assert_eq!(summarize_statuses(&[S::Paused, S::Pending]), Some(S::Paused));
    }

    #[test]
    fn ready_when_waiting_ready() {
        assert_eq!(summarize_statuses(&[S::Ready, S::Pending]), Some(S::Ready));
    }

    #[test]
    fn completed_only_when_all_done() {
        assert_eq!(summarize_statuses(&[S::Completed, S::Completed]), Some(S::Completed));
        assert_eq!(summarize_statuses(&[S::Completed, S::Pending]), Some(S::Pending));
    }

    #[test]
    fn cancelled_ignored() {
        assert_eq!(summarize_statuses(&[S::Cancelled, S::Ready]), Some(S::Ready));
        assert_eq!(summarize_statuses(&[S::Cancelled]), Some(S::Cancelled));
    }
}
