//! The process's one owner slot (GUI-DESKTOP-027, HBR-HOST-003: one authoritative GPU owner).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static TAKEN: AtomicBool = AtomicBool::new(false);

/// Held by the owner and by every device generation, so a second owner cannot open while a device of the first
/// still exists. Released with its last holder.
pub(crate) struct ProcessClaim {
    _private: (),
}

impl ProcessClaim {
    /// `None` when the process already has its owner.
    pub(crate) fn take() -> Option<Arc<Self>> {
        TAKEN
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Arc::new(Self { _private: () }))
    }
}

impl Drop for ProcessClaim {
    fn drop(&mut self) {
        TAKEN.store(false, Ordering::Release);
    }
}
