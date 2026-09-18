//! Process Engine — state machine + bağımlılık çözümleme (MASTER PLAN §12-16).
//!
//! Saf fonksiyonlardan oluşur; DB erişimi yoktur. Service katmanı bu kuralları
//! transaction içinde uygular. Geçiş matrisi architecture.md §12.3 ile birebir.

use crate::domain::entities::{EventType, ProcessStatus};

/// Durum değiştiren aksiyonlar (action endpoint'lerle birebir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Start,
    Pause,
    Resume,
    Complete,
    Block,
    Unblock,
    Cancel,
}

impl Transition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Transition::Start => "start",
            Transition::Pause => "pause",
            Transition::Resume => "resume",
            Transition::Complete => "complete",
            Transition::Block => "block",
            Transition::Unblock => "unblock",
            Transition::Cancel => "cancel",
        }
    }

    /// Aksiyonun ürettiği olay (status değiştirmeyenler service'te ayrı ele alınır).
    pub fn event_type(&self) -> EventType {
        match self {
            Transition::Start => EventType::Started,
            Transition::Pause => EventType::Paused,
            Transition::Resume => EventType::Resumed,
            Transition::Complete => EventType::Completed,
            Transition::Block => EventType::Blocked,
            Transition::Unblock => EventType::Unblocked,
            Transition::Cancel => EventType::Cancelled,
        }
    }
}

/// Geçiş matrisi: (mevcut durum, aksiyon) → hedef durum.
/// Hedef, Some(new_status, status_before_block_to_write) döner.
///
/// Matris (architecture.md §12.3):
/// ```text
///               start    pause    resume   complete block    unblock  cancel
/// PENDING       –        –        –        –        –        –        ✓
/// READY         ✓        –        –        –        ✓        –        ✓
/// IN_PROGRESS   –        ✓        –        ✓        ✓        –        ✓
/// PAUSED        –        –        ✓        –        –        –        –
/// BLOCKED       –        –        –        –        –        ✓        –
/// COMPLETED     –        –        –        –        –        –        –
/// CANCELLED     –        –        –        –        –        –        –
/// ```
pub fn transition(
    current: ProcessStatus,
    action: Transition,
) -> Result<(ProcessStatus, Option<ProcessStatus>), InvalidTransition> {
    use ProcessStatus as S;

    let next = match (current, action) {
        // start: yalnızca READY'den
        (S::Ready, Transition::Start) => S::InProgress,
        // pause: yalnızca IN_PROGRESS'tan
        (S::InProgress, Transition::Pause) => S::Paused,
        // resume: yalnızca PAUSED'dan
        (S::Paused, Transition::Resume) => S::InProgress,
        // complete: yalnızca IN_PROGRESS'tan
        (S::InProgress, Transition::Complete) => S::Completed,
        // block: READY veya IN_PROGRESS'tan; dönülecek durum saklanır
        (S::Ready, Transition::Block) => {
            return Ok((S::Blocked, Some(S::Ready)));
        }
        (S::InProgress, Transition::Block) => {
            return Ok((S::Blocked, Some(S::InProgress)));
        }
        // unblock: kaydedilen duruma dön
        (S::Blocked, Transition::Unblock) => {
            return Err(InvalidTransition::UnblockNeedsBefore); // service status_before_block ile çağırır
        }
        // cancel: PENDING/READY/IN_PROGRESS'tan
        (S::Pending, Transition::Cancel)
        | (S::Ready, Transition::Cancel)
        | (S::InProgress, Transition::Cancel) => S::Cancelled,
        // geri kalan tüm kombinasyonlar geçersiz
        _ => return Err(InvalidTransition::Invalid { from: current, action }),
    };

    Ok((next, None))
}

/// unblock: hedef, execution'ın `status_before_block` alanından gelir.
pub fn unblock_target(
    before: Option<ProcessStatus>,
) -> Result<ProcessStatus, InvalidTransition> {
    match before {
        Some(status @ (ProcessStatus::Ready | ProcessStatus::InProgress)) => Ok(status),
        other => Err(InvalidTransition::InvalidUnblockTarget(other)),
    }
}

/// Geçersiz geçiş hatası — service DomainError::Validation'a çevirir.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvalidTransition {
    #[error("{action:?} aksiyonu {from:?} durumundan uygulanamaz.")]
    Invalid {
        from: ProcessStatus,
        action: Transition,
    },
    #[error("unblock yalnızca BLOCKED durumundan ve kayıtlı önceki durum ile yapılır.")]
    UnblockNeedsBefore,
    #[error("Geçersiz unblock hedefi: {0:?}")]
    InvalidUnblockTarget(Option<ProcessStatus>),
}

/// Bir adımın bağımlılıkları karşılanmış mı?
/// `dependencies`: (bağımlı olunan execution durumu, gereken durum) listesi.
/// Boş liste → bağımlılık yok → hazır (grup atamasında ilk adımlar READY olur).
pub fn dependencies_satisfied(dependencies: &[(ProcessStatus, ProcessStatus)]) -> bool {
    dependencies
        .iter()
        .all(|(actual, required)| actual == required)
}

/// PENDING → READY geçişi yapılabilmeli mi (resolver kararı).
pub fn should_promote_to_ready(
    current: ProcessStatus,
    dependencies: &[(ProcessStatus, ProcessStatus)],
) -> bool {
    current == ProcessStatus::Pending && dependencies_satisfied(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_statuses() -> Vec<ProcessStatus> {
        use ProcessStatus as S;
        vec![S::Pending, S::Ready, S::InProgress, S::Paused, S::Blocked, S::Completed, S::Cancelled]
    }

    const ALL_ACTIONS: [Transition; 7] = [
        Transition::Start,
        Transition::Pause,
        Transition::Resume,
        Transition::Complete,
        Transition::Block,
        Transition::Unblock,
        Transition::Cancel,
    ];

    /// Matrisin İZİN VERİLEN hücreleri — architecture.md §12.3 ile birebir.
    #[test]
    fn transition_matrix_allowed_cells() {
        use ProcessStatus as S;
        // (from, action) → (to, before_block)
        let allowed: Vec<(ProcessStatus, Transition, ProcessStatus)> = vec![
            (S::Ready, Transition::Start, S::InProgress),
            (S::InProgress, Transition::Pause, S::Paused),
            (S::Paused, Transition::Resume, S::InProgress),
            (S::InProgress, Transition::Complete, S::Completed),
            (S::Pending, Transition::Cancel, S::Cancelled),
            (S::Ready, Transition::Cancel, S::Cancelled),
            (S::InProgress, Transition::Cancel, S::Cancelled),
        ];
        for (from, action, expected) in allowed {
            let (to, before) = transition(from, action)
                .unwrap_or_else(|e| panic!("{from:?} + {action:?}: {e}"));
            assert_eq!(to, expected, "{from:?} + {action:?}");
            assert!(before.is_none(), "status_before_block yalnız block'ta yazılır");
        }
        // block her iki kaynaktan before yazar
        assert_eq!(
            transition(S::Ready, Transition::Block).unwrap(),
            (S::Blocked, Some(S::Ready))
        );
        assert_eq!(
            transition(S::InProgress, Transition::Block).unwrap(),
            (S::Blocked, Some(S::InProgress))
        );
    }

    /// Matrisin REDDEDİLEN hücreleri — kalan tüm kombinasyonlar hata vermeli.
    #[test]
    fn transition_matrix_rejects_everything_else() {
        use ProcessStatus as S;
        let allowed_set: Vec<(ProcessStatus, Transition)> = vec![
            (S::Ready, Transition::Start),
            (S::InProgress, Transition::Pause),
            (S::Paused, Transition::Resume),
            (S::InProgress, Transition::Complete),
            (S::Ready, Transition::Block),
            (S::InProgress, Transition::Block),
            (S::Blocked, Transition::Unblock),
            (S::Pending, Transition::Cancel),
            (S::Ready, Transition::Cancel),
            (S::InProgress, Transition::Cancel),
        ];
        for from in all_statuses() {
            for action in ALL_ACTIONS {
                let cell = (from, action);
                if allowed_set.contains(&cell) {
                    continue;
                }
                assert!(
                    transition(from, action).is_err(),
                    "{from:?} + {action:?} reddedilmeliydi"
                );
            }
        }
    }

    #[test]
fn terminal_states_are_frozen() {
        use ProcessStatus as S;
        for status in [S::Completed, S::Cancelled] {
            for action in ALL_ACTIONS {
                assert!(transition(status, action).is_err(), "{status:?} donmuş durum");
            }
        }
    }

    #[test]
    fn unblock_returns_recorded_status() {
        use ProcessStatus as S;
        assert_eq!(unblock_target(Some(S::Ready)).unwrap(), S::Ready);
        assert_eq!(unblock_target(Some(S::InProgress)).unwrap(), S::InProgress);
        assert!(unblock_target(None).is_err());
        assert!(unblock_target(Some(S::Completed)).is_err());
    }

    #[test]
    fn dependency_resolver_scenarios() {
        use ProcessStatus as S;
        // bağımlılık yok → hazır
        assert!(dependencies_satisfied(&[]));
        assert!(should_promote_to_ready(S::Pending, &[]));
        // tek bağımlılık tamam → hazır
        assert!(should_promote_to_ready(S::Pending, &[(S::Completed, S::Completed)]));
        // bağımlılık bekliyor → değil
        assert!(!should_promote_to_ready(S::Pending, &[(S::Pending, S::Completed)]));
        assert!(!should_promote_to_ready(S::Pending, &[(S::InProgress, S::Completed)]));
        // çoklu: biri tamam değil yeter değil
        assert!(!should_promote_to_ready(
            S::Pending,
            &[(S::Completed, S::Completed), (S::Ready, S::Completed)]
        ));
        // çoklu: hepsi tamam → hazır
        assert!(should_promote_to_ready(
            S::Pending,
            &[(S::Completed, S::Completed), (S::Completed, S::Completed)]
        ));
        // READY zaten geçilmiş → promote gerekmez (false ama hata değil)
        assert!(!should_promote_to_ready(S::Ready, &[]));
    }

    #[test]
    fn action_event_mapping() {
        assert_eq!(Transition::Start.event_type(), EventType::Started);
        assert_eq!(Transition::Complete.event_type(), EventType::Completed);
        assert_eq!(Transition::Block.event_type(), EventType::Blocked);
    }
}
