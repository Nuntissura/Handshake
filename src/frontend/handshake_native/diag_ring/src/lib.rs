//! `handshake-diag-ring` — the shared-memory seqlock SPSC ring + typed-allowlist `DiagEvent`
//! schema that is the substrate of Handshake's three-tier diagnostic decoupling
//! (Master Spec v02.196 §5.8.6 internal_diagnostics + §6.13.4 Palmistry).
//!
//! This crate is PURE SUBSTRATE: no egui, no tokio, no reqwest, no backend. It is a SHARED crate
//! that BOTH the Handshake binary (Tier 2 writer, MT-082/MT-084) and the external Palmistry watcher
//! binary (Tier 3 reader, MT-089/MT-090/MT-091) link via a relative `path = ` dependency, so the
//! ring protocol and the record layout are compiled identically into both processes — which is what
//! makes the cross-process shared map ABI-stable.
//!
//! # What this crate provides
//!
//! - [`ring::DiagRingWriter`] — the wait-free single-producer side (the UI thread). It never blocks
//!   on the reader; it overwrites unread slots by design so a frozen reader can never back-pressure
//!   Handshake.
//! - [`ring::DiagRingReader`] — the single-consumer side (Palmistry). It maps the SAME backing file,
//!   validates the header (`magic`/`version`/`record_size`/`capacity`), and reads under a BOUNDED
//!   seqlock retry so it can observe a freeze without ever tearing or stalling the writer.
//! - [`schema::DiagEvent`] — the typed-allowlist record: every field is a fixed-width integer; there
//!   is NO text/blob field, and `bytemuck::Pod` enforces that at compile time.
//!
//! See the `ring` and `schema` module docs for the seqlock protocol, memory ordering, and the
//! cross-process backing-file pattern.
//!
//! # Also in this crate: the HBR-INT-009 three-tier EVIDENCE format (MT-095)
//!
//! [`three_tier_evidence`] is a SEPARATE concern from the runtime ring above: it is the
//! build/test-time governance EVIDENCE record (`ThreeTierDiagnosticWiringRecord`) a WP/MT emits to
//! prove how an observable behavior is wired across the three tiers
//! (`FLIGHT_RECORDER`/`INTERNAL_DIAGNOSTICS`/`PALMISTRY`). It lives in THIS crate — not in either
//! product binary — because it is the only crate both binaries already depend on, so any WP (and the
//! WP-016 retrofit) can emit it without a new dependency edge. Unlike [`schema::DiagEvent`], this
//! record is a JSON file (never enters the shared-memory ring) and carries governance identifiers, not
//! the typed-allowlist POD telemetry; see the module docs for the privacy distinction.

pub mod ring;
pub mod schema;
pub mod three_tier_evidence;

// Public re-exports so downstream crates can `use handshake_diag_ring::{DiagRingWriter, ...}`.
pub use ring::{
    default_backing_path, DiagRingReader, DiagRingWriter, Heartbeat, DEFAULT_CAPACITY, HEADER_SIZE,
    RECORD_SLOT_SIZE, RING_MAGIC, RING_VERSION, SEQLOCK_READ_RETRY_BOUND,
};
pub use schema::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity, DIAG_EVENT_SIZE};
pub use three_tier_evidence::{
    format_rfc3339_utc, run_at_now, DiagTier, EmitError, ThreeTierDiagnosticWiringRecord,
    TierWiring, ValidationError, WiringStatus, EVIDENCE_FILE_NAME,
};
