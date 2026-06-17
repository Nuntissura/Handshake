//! WP-KERNEL-009 MT-254 DebugAdapterCore — adapter registry (honesty gate).
//!
//! The registry exposes EXACTLY the adapters that drive a real process today. It
//! is the single source of truth the UI/API read so there are never dead
//! dropdown entries for adapters that do not run yet. Adding a future adapter
//! (Python `debugpy`, `lldb`) is a code change here AND a working
//! implementation — never a config-only stub.

use serde::{Deserialize, Serialize};

use crate::debug_adapter::protocol::AdapterKind;

/// A listable adapter descriptor for the picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterDescriptor {
    pub kind: AdapterKind,
    pub id: String,
    pub display_name: String,
    /// Always true for listed adapters: a listed adapter is a runnable adapter.
    pub runnable: bool,
}

/// The adapters Handshake ships AND can run right now. ONLY Node today.
///
/// This is the negative-check anchor: the API/UI list MUST equal this, so a
/// reviewer can prove there is no `python`/`lldb`/disabled entry.
pub fn listable_adapters() -> Vec<AdapterDescriptor> {
    vec![AdapterDescriptor {
        kind: AdapterKind::Node,
        id: AdapterKind::Node.as_str().to_string(),
        display_name: AdapterKind::Node.display_name().to_string(),
        runnable: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_exactly_node_and_nothing_disabled() {
        let adapters = listable_adapters();
        assert_eq!(adapters.len(), 1, "only Node ships today");
        assert_eq!(adapters[0].kind, AdapterKind::Node);
        assert!(adapters[0].runnable, "listed adapters must be runnable");
        // Negative: no python/lldb/disabled entries leak in.
        assert!(adapters
            .iter()
            .all(|a| a.id != "python" && a.id != "lldb" && a.runnable));
    }
}
