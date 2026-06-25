//! FEMS interop (WP-KERNEL-012 cluster E9 — Pillar 12 typed memory).
//!
//! This subtree is the editors' READ-ONLY consumer of the FEMS (Pillar 12) retrieval capsule:
//!
//! - [`memory_client`] — the typed read client + the deserialized Pillar 12 [`memory_client::MemoryPack`]
//!   model (3 kinds: episodic/semantic/procedural; provenance-first source; <=24 items hard-capped
//!   client-side; <=500 token advisory budget). It reuses the WP-011 `backend_client` shared reqwest
//!   pool + base URL (no second HTTP stack) and returns the typed blocker
//!   [`memory_client::MemoryClientError::EndpointMissing`] when the FEMS read route is absent — the
//!   DESIGNED primary path in the current handshake_core build, where the route does not exist (MT-063).
//! - [`relevant_memory_panel`] — the egui "Relevant Memory" side panel that renders the capsule
//!   provenance-first (grouped by kind, a "Go to source" affordance per item routed through the MT-030
//!   navigation seam) and shows a calm empty-state banner for the `EndpointMissing` typed blocker.
//!
//! MT-064 (memory-write proposals) and MT-065 (end-to-end proof against real PostgreSQL/EventLedger)
//! build ON this read-only consumer. The live pane dock (`app.rs` pane factory) + the MT-031
//! interaction-bus context subscription + the live MT-030 nav wiring land at E11 (MT-069), like the
//! other panes; MT-063 registers the pane factory now and proves the client/panel at the widget level.

pub mod memory_client;
// WP-KERNEL-012 MT-064 (E9 — FEMS memory-write proposal from the editor): turns the current editor
// selection into a typed, review-gated FEMS memory-write PROPOSAL (never a direct commit), submits it to
// the EXISTING review-gated FEMS write path, and emits an FR-EVT-MEM-001 (memory_write_proposed) event
// through the MT-036 NativeEditorEventEmitter on success. The proposal WRITE endpoint is ABSENT in the
// current handshake_core build (verified read-only), so `submit_proposal` returns the typed blocker
// `MissingEndpoint` and writes nothing — never a silent direct-memory fallback (the designed primary
// path). content_hash REUSES the MT-032 loom content-hash primitive (no second hashing scheme).
pub mod memory_proposal;
pub mod relevant_memory_panel;

pub use memory_client::{
    clamp_pack_items, MemoryClient, MemoryClientError, MemoryContext, MemoryItem, MemoryKind,
    MemoryPack, MemoryResult, MemorySource, MEMORY_PACK_MAX_ITEMS, MEMORY_PACK_TOKEN_BUDGET,
};

pub use relevant_memory_panel::{
    mem_item_author_id, mem_source_author_id, FnNavigationBus, MemoryNavTarget, NavigationBus,
    RelevantMemoryPanel, ENDPOINT_MISSING_BANNER, MEM_ITEM_AUTHOR_PREFIX, MEM_SOURCE_AUTHOR_PREFIX,
    NO_MEMORY_TEXT, RELEVANT_MEMORY_LIST_AUTHOR_ID, RELEVANT_MEMORY_PANEL_AUTHOR_ID,
};

pub use memory_proposal::{
    build_proposal, content_hash_of_selection, fems_class_author_id, proposal_path,
    register_propose_to_memory_command, submit_proposal, submit_proposal_and_emit,
    HandshakeCoreClient, MemoryClass, MemoryProposalError, MemorySourceProvenance,
    MemoryWriteProposal, ProposalAck, ProposeDialogOutcome, ProposeToMemoryDialog,
    FEMS_CLASS_AUTHOR_PREFIX, FEMS_PROPOSE_COMMAND_ID, FEMS_PROPOSE_COMMAND_LABEL,
    FEMS_PROPOSE_CONFIRM_AUTHOR_ID, FEMS_PROPOSE_DIALOG_AUTHOR_ID, PROPOSE_TO_MEMORY_COMMAND,
};
