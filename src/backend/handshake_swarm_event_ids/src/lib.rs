/// Self-contained FR-EVT-SWARM-* identifier table. Canonical case is
/// UPPER-KEBAB-CASE after the `FR-EVT-` prefix, matching the governance
/// registry convention exactly so a future fold into `FrEventId` is mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwarmFrEventId {
    SessionSpawned,
    SessionReady,
    SessionGenerating,
    SessionCancelled,
    SessionCompleted,
    SessionFailed,
    ResourceAllocated,
    ResourceEvicted,
    BreakerTripped,
    LeaseExpired,
    SpawnRejected,
    // rank-3: VM/sandbox worktree lifecycle (each emits one FR event so the
    // Flight Recorder can replay/audit per-worktree state and the board can
    // drill down by worktree).
    WorktreeCreated,
    WorktreeMounted,
    WorktreeReclaimed,
    WorktreeCleanupFailed,
    // rank-7 groundwork: calendar-scheduled spin-up / teardown fires.
    ScheduledSpinupFired,
    ScheduledTeardownFired,
}

impl SwarmFrEventId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionSpawned => "FR-EVT-SWARM-SESSION-SPAWNED",
            Self::SessionReady => "FR-EVT-SWARM-SESSION-READY",
            Self::SessionGenerating => "FR-EVT-SWARM-SESSION-GENERATING",
            Self::SessionCancelled => "FR-EVT-SWARM-SESSION-CANCELLED",
            Self::SessionCompleted => "FR-EVT-SWARM-SESSION-COMPLETED",
            Self::SessionFailed => "FR-EVT-SWARM-SESSION-FAILED",
            Self::ResourceAllocated => "FR-EVT-SWARM-RESOURCE-ALLOCATED",
            Self::ResourceEvicted => "FR-EVT-SWARM-RESOURCE-EVICTED",
            Self::BreakerTripped => "FR-EVT-SWARM-BREAKER-TRIPPED",
            Self::LeaseExpired => "FR-EVT-SWARM-LEASE-EXPIRED",
            Self::SpawnRejected => "FR-EVT-SWARM-SPAWN-REJECTED",
            Self::WorktreeCreated => "FR-EVT-SWARM-WORKTREE-CREATED",
            Self::WorktreeMounted => "FR-EVT-SWARM-WORKTREE-MOUNTED",
            Self::WorktreeReclaimed => "FR-EVT-SWARM-WORKTREE-RECLAIMED",
            Self::WorktreeCleanupFailed => "FR-EVT-SWARM-WORKTREE-CLEANUP-FAILED",
            Self::ScheduledSpinupFired => "FR-EVT-SWARM-SCHED-SPINUP-FIRED",
            Self::ScheduledTeardownFired => "FR-EVT-SWARM-SCHED-TEARDOWN-FIRED",
        }
    }

    pub fn all() -> &'static [SwarmFrEventId] {
        &[
            Self::SessionSpawned,
            Self::SessionReady,
            Self::SessionGenerating,
            Self::SessionCancelled,
            Self::SessionCompleted,
            Self::SessionFailed,
            Self::ResourceAllocated,
            Self::ResourceEvicted,
            Self::BreakerTripped,
            Self::LeaseExpired,
            Self::SpawnRejected,
            Self::WorktreeCreated,
            Self::WorktreeMounted,
            Self::WorktreeReclaimed,
            Self::WorktreeCleanupFailed,
            Self::ScheduledSpinupFired,
            Self::ScheduledTeardownFired,
        ]
    }

    pub fn from_str_id(s: &str) -> Option<Self> {
        Self::all().iter().copied().find(|id| id.as_str() == s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swarm_event_ids_round_trip() {
        for id in SwarmFrEventId::all() {
            assert_eq!(SwarmFrEventId::from_str_id(id.as_str()), Some(*id));
        }
    }

    #[test]
    fn swarm_event_ids_are_canonical_kebab() {
        for id in SwarmFrEventId::all() {
            let s = id.as_str();
            assert!(s.starts_with("FR-EVT-SWARM-"), "bad prefix: {s}");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'),
                "non-canonical char in {s}"
            );
        }
    }

    #[test]
    fn swarm_event_ids_include_worktree_and_scheduled_lifecycle() {
        // rank-3: the VM/worktree + calendar-scheduled lifecycle ids are wired
        // into the table (so coordinator/worktree/scheduler code can stamp them).
        let ids: std::collections::HashSet<&str> =
            SwarmFrEventId::all().iter().map(|i| i.as_str()).collect();
        for expected in [
            "FR-EVT-SWARM-WORKTREE-CREATED",
            "FR-EVT-SWARM-WORKTREE-MOUNTED",
            "FR-EVT-SWARM-WORKTREE-RECLAIMED",
            "FR-EVT-SWARM-WORKTREE-CLEANUP-FAILED",
            "FR-EVT-SWARM-SCHED-SPINUP-FIRED",
            "FR-EVT-SWARM-SCHED-TEARDOWN-FIRED",
        ] {
            assert!(
                ids.contains(expected),
                "missing FR-EVT-SWARM id: {expected}"
            );
        }
        // Every canonical string is unique (no two variants collide).
        assert_eq!(
            ids.len(),
            SwarmFrEventId::all().len(),
            "duplicate canonical FR-EVT-SWARM id string"
        );
    }
}
