//! Per-agent action attribution for the swarm action channel (WP-KERNEL-011 MT-028).
//!
//! When N agents steer the shell concurrently, every dispatched mutation must be ATTRIBUTABLE after the
//! fact: which agent clicked which widget, in what order. [`AttributedAction`] records that, and
//! [`ActionLog`] is the bounded, ordered, thread-safe ring buffer the per-connection
//! [`crate::mcp::session::McpSession`] appends to and a diagnostic tool drains.
//!
//! ## `agent_id`: a deterministic per-session label, NOT a security identity
//!
//! `agent_id` is the first [`AGENT_ID_HEX_LEN`] hex chars of `SHA-256(session_token_bytes)`. It is a
//! short, stable, deterministic handle for one MCP session — two requests on the same connection share
//! one `agent_id`; two different sessions get different ones with overwhelming probability. It is
//! derived from the token (not the token itself), so the log never records the secret. It is a
//! traceability label for post-hoc audit and debugging — it is NOT an authorization gate (the
//! [`crate::mcp::leases::LeaseRegistry`] is the gate) and NOT a security identity (a local agent that
//! read another's token could reproduce its `agent_id`; the binding-file permissions from MT-027 are
//! what protect the token).
//!
//! ## Why a `std::sync::Mutex` ring buffer (not async)
//!
//! Appending one entry is a fast, non-blocking operation with no `.await` inside, so a plain
//! `std::sync::Mutex<VecDeque<_>>` is correct and avoids holding any lock across an await — consistent
//! with the rest of the MT-027/028 shared state. The buffer is bounded ([`ACTION_LOG_CAPACITY`]); when
//! full, the OLDEST entry is evicted so a long-running swarm never grows the log unboundedly. A
//! monotonically increasing `seq` is stamped on every entry so a diagnostic reader can use
//! [`ActionLog::drain_since`] to consume incrementally without missing or double-counting events even
//! across evictions (the red-team "diagnostic tools may miss events" control).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// The number of hex chars of `SHA-256(token)` used as an `agent_id`. 8 hex chars = 32 bits = 4 billion
/// values; collision across the handful of live swarm sessions is negligible, and a short id is readable
/// in logs. Matches the contract's "first 8 hex chars" requirement.
pub const AGENT_ID_HEX_LEN: usize = 8;

/// Max entries retained in the [`ActionLog`] ring buffer. The contract floor is 1000; the red-team
/// minimum control raises it to 10,000 so a high-rate swarm does not evict entries a diagnostic reader
/// has not yet consumed. At ~200 bytes/entry this caps the log near ~2 MiB.
pub const ACTION_LOG_CAPACITY: usize = 10_000;

/// Derive the short, deterministic `agent_id` for a session from its token. This is
/// `SHA-256(token_bytes)` truncated to [`AGENT_ID_HEX_LEN`] lowercase hex chars. Deterministic: the same
/// token always yields the same id, so attribution is stable across a session's many requests.
pub fn agent_id_for_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    let mut out = String::with_capacity(AGENT_ID_HEX_LEN);
    for byte in digest.iter() {
        if out.len() >= AGENT_ID_HEX_LEN {
            break;
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out.truncate(AGENT_ID_HEX_LEN);
    out
}

/// One attributed, dispatched action — the unit the [`ActionLog`] records for post-hoc audit. Carries
/// who (`agent_id`), what (`op_name`), against which widget (`target_key` + `node_id`), and when
/// (`dispatched_at_utc`), plus the monotonic `seq` for incremental draining.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AttributedAction {
    /// Monotonic sequence number stamped by the log on append (1-based, never reused). Lets a reader
    /// resume from the last seq it saw via [`ActionLog::drain_since`].
    pub seq: u64,
    /// The short deterministic per-session id (see [`agent_id_for_token`]).
    pub agent_id: String,
    /// The tool / operation name (`"click_widget"`, `"set_value"`, `"list_widgets"`, …).
    pub op_name: String,
    /// The stable widget `author_id` the action targeted (empty for non-targeted ops like
    /// `list_widgets`).
    pub target_key: String,
    /// The resolved AccessKit `NodeId` value the action dispatched to (0 for non-targeted ops).
    pub node_id: u64,
    /// ISO-8601 UTC timestamp the action was recorded.
    pub dispatched_at_utc: String,
}

/// A bounded, ordered, thread-safe ring buffer of [`AttributedAction`]s. Cloneable (`Arc` inside) so the
/// same log is shared by every [`crate::mcp::session::McpSession`] and any diagnostic surface. Append is
/// `O(1)`; when at capacity the oldest entry is evicted.
#[derive(Clone, Default)]
pub struct ActionLog {
    inner: Arc<Mutex<LogState>>,
}

#[derive(Default)]
struct LogState {
    entries: VecDeque<AttributedAction>,
    /// The seq assigned to the NEXT appended entry (1-based). Never decreases, even after eviction, so
    /// seqs are globally unique for the life of the log.
    next_seq: u64,
}

impl ActionLog {
    /// A fresh, empty log with the default capacity.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an attributed action, stamping the next monotonic `seq` and `dispatched_at_utc`. Evicts the
    /// oldest entry if the buffer is at [`ACTION_LOG_CAPACITY`]. Returns the assigned seq. Never blocks on
    /// an await; recovers a poisoned lock so one panicking reader cannot wedge the log.
    pub fn record(&self, agent_id: &str, op_name: &str, target_key: &str, node_id: u64) -> u64 {
        let mut state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let seq = state.next_seq + 1;
        state.next_seq = seq;
        let entry = AttributedAction {
            seq,
            agent_id: agent_id.to_owned(),
            op_name: op_name.to_owned(),
            target_key: target_key.to_owned(),
            node_id,
            dispatched_at_utc: now_utc_iso8601(),
        };
        if state.entries.len() >= ACTION_LOG_CAPACITY {
            state.entries.pop_front();
        }
        state.entries.push_back(entry);
        seq
    }

    /// Snapshot ALL currently retained entries (oldest first). For diagnostics that want the full
    /// in-memory window; for incremental consumption prefer [`Self::drain_since`].
    pub fn drain_log(&self) -> Vec<AttributedAction> {
        let state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        state.entries.iter().cloned().collect()
    }

    /// All retained entries with `seq > after_seq` (oldest first). A diagnostic reader passes the last
    /// seq it consumed; passing 0 returns the whole retained window. This lets a reader keep up with a
    /// high-rate swarm without missing or re-reading entries (red-team incremental-cursor control).
    pub fn drain_since(&self, after_seq: u64) -> Vec<AttributedAction> {
        let state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        state
            .entries
            .iter()
            .filter(|e| e.seq > after_seq)
            .cloned()
            .collect()
    }

    /// Number of entries currently retained (after eviction). Useful for assertions and diagnostics.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .entries
            .len()
    }

    /// True when no entries are retained.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Current UTC time as an ISO-8601 string. Uses `std::time::SystemTime` (no chrono dependency added) and
/// renders `…Z`. Monotonic ordering of entries is guaranteed by `seq`, not by this timestamp, so a
/// coarse wall-clock string is sufficient for the audit label.
fn now_utc_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Render as "<unix_secs>.<nanos>Z" — a sortable, parseable instant without a date library. The
    // action log is an in-memory diagnostic surface, not a persisted audit record, so this is adequate.
    format!("{}.{:09}Z", dur.as_secs(), dur.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_is_deterministic_and_8_hex() {
        let a = agent_id_for_token("session-secret");
        let b = agent_id_for_token("session-secret");
        assert_eq!(a, b, "same token -> same agent_id");
        assert_eq!(a.len(), AGENT_ID_HEX_LEN);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_tokens_get_different_ids() {
        let a = agent_id_for_token("token-a");
        let b = agent_id_for_token("token-b");
        assert_ne!(a, b, "distinct tokens -> distinct agent_ids");
    }

    #[test]
    fn record_stamps_monotonic_seq_and_fields() {
        let log = ActionLog::new();
        let s1 = log.record("aaaaaaaa", "click_widget", "btn", 10);
        let s2 = log.record("bbbbbbbb", "set_value", "field", 11);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        let entries = log.drain_log();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].agent_id, "aaaaaaaa");
        assert_eq!(entries[0].op_name, "click_widget");
        assert_eq!(entries[0].target_key, "btn");
        assert_eq!(entries[0].node_id, 10);
        assert!(entries[0].dispatched_at_utc.ends_with('Z'));
    }

    #[test]
    fn drain_since_returns_only_newer_entries() {
        let log = ActionLog::new();
        log.record("a", "click_widget", "x", 1);
        let after = log.record("a", "click_widget", "y", 2);
        log.record("a", "click_widget", "z", 3);
        let newer = log.drain_since(after);
        assert_eq!(newer.len(), 1, "only the entry with seq > `after`");
        assert_eq!(newer[0].target_key, "z");
    }

    #[test]
    fn ring_buffer_evicts_oldest_but_keeps_seq_monotonic() {
        let log = ActionLog::new();
        // Append capacity + 5 entries; len caps at capacity, seq keeps climbing past it.
        for _ in 0..(ACTION_LOG_CAPACITY + 5) {
            log.record("a", "click_widget", "x", 1);
        }
        assert_eq!(log.len(), ACTION_LOG_CAPACITY, "len capped at capacity");
        let entries = log.drain_log();
        // Oldest retained seq is 6 (1..=5 were evicted); newest is capacity+5.
        assert_eq!(entries.first().unwrap().seq, 6);
        assert_eq!(
            entries.last().unwrap().seq,
            (ACTION_LOG_CAPACITY + 5) as u64
        );
    }
}
