use super::fold_manifest::KERNEL002_WP_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrdtLibrary {
    Yjs,
    Yrs,
    Loro,
    Automerge,
    ExistingProductDependencies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrdtSelection {
    YjsCompatibleUpdateLog,
    RejectForKernel002,
    DeferAsFutureAdapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelCrdtStorageModel {
    PostgresEventLedgerUpdateLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeIntegrationBoundary {
    TypeScriptYjsRustValidatedBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelCrdtAuthorityBoundary {
    CrdtIsPrePromotionEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrdtOptionAssessment {
    pub library: CrdtLibrary,
    pub decision: CrdtSelection,
    pub fit_notes: &'static str,
    pub rejected_or_deferred_because: &'static str,
    pub risks: &'static str,
    pub reuse_opportunities: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelCrdtAdr {
    pub wp_id: &'static str,
    pub decision: CrdtSelection,
    pub selected_approach: &'static str,
    pub option_assessments: &'static [CrdtOptionAssessment],
    pub storage_model: KernelCrdtStorageModel,
    pub storage_contract: &'static str,
    pub runtime_boundary: RuntimeIntegrationBoundary,
    pub rust_typescript_boundary: &'static str,
    pub authority_boundary: KernelCrdtAuthorityBoundary,
    pub schema_compatibility: &'static str,
    pub sync_model: &'static str,
    pub validation_plan: &'static [&'static str],
    pub research_sources: &'static [&'static str],
}

impl KernelCrdtAdr {
    pub fn option_for(&self, library: CrdtLibrary) -> Option<&'static CrdtOptionAssessment> {
        self.option_assessments
            .iter()
            .find(|option| option.library == library)
    }
}

pub fn kernel002_crdt_adr() -> KernelCrdtAdr {
    KernelCrdtAdr {
        wp_id: KERNEL002_WP_ID,
        decision: CrdtSelection::YjsCompatibleUpdateLog,
        selected_approach: "Use the existing frontend Yjs/Tiptap dependency path as the editor CRDT format, persist Yjs-compatible binary updates and snapshots as kernel write-box evidence, and validate/promotion-gate materialized state before EventLedger authority writes.",
        option_assessments: KERNEL002_CRDT_OPTIONS,
        storage_model: KernelCrdtStorageModel::PostgresEventLedgerUpdateLog,
        storage_contract: "Persist document_id, workspace_id, actor_id, actor_kind, crdt_site_id, schema_id, update_seq, update_bytes_ref, update_sha256, state_vector, base_snapshot_ref, materialized_projection_hash, EventLedger correlation id, promotion_box_id, validation_state, and replay metadata in Postgres. CRDT updates are evidence until a promotion action appends authority events.",
        runtime_boundary: RuntimeIntegrationBoundary::TypeScriptYjsRustValidatedBytes,
        rust_typescript_boundary: "TypeScript owns Y.Doc/Tiptap collaboration editing and emits binary update/state-vector payloads. Rust receives opaque Yjs-compatible bytes, stores hashes and replay metadata, validates schema/materialized projections through kernel guards, and may later use yrs for server-side replay without changing the wire format.",
        authority_boundary: KernelCrdtAuthorityBoundary::CrdtIsPrePromotionEvidence,
        schema_compatibility: "Every workspace binds a schema_id covering the Tiptap/ProseMirror schema, kernel write-box schema, and materialized projection schema. Mixed schema versions cannot promote until a validity guard confirms no unsupported node/mark loss and records the schema migration or denial.",
        sync_model: "Clients exchange Yjs updates/state vectors; backend stores append-only updates, bounded snapshots, state vectors, and compaction receipts. Sync may deliver updates in any order, but promotion reads a replayed/materialized state at a specific state_vector and validates it before authority effects.",
        validation_plan: KERNEL002_CRDT_VALIDATION_PLAN,
        research_sources: KERNEL002_CRDT_RESEARCH_SOURCES,
    }
}

const KERNEL002_CRDT_OPTIONS: &[CrdtOptionAssessment] = &[
    CrdtOptionAssessment {
        library: CrdtLibrary::Yjs,
        decision: CrdtSelection::YjsCompatibleUpdateLog,
        fit_notes: "Best fit for Kernel002 because the app already depends on yjs and Tiptap collaboration packages, Yjs has editor bindings for Tiptap/ProseMirror, update/state-vector APIs, persistence providers, and network-agnostic sync semantics.",
        rejected_or_deferred_because: "",
        risks: "Browser CRDT state can be mistaken for authority; schema drift can silently drop unsupported Tiptap nodes; server-side validation needs byte/hash discipline until Rust replay is wired.",
        reuse_opportunities: "Reuse app/package.json yjs dependency, Tiptap collaboration dependency, current REST boundary, and future Postgres/EventLedger write-box storage.",
    },
    CrdtOptionAssessment {
        library: CrdtLibrary::Yrs,
        decision: CrdtSelection::DeferAsFutureAdapter,
        fit_notes: "Yrs is compatible with the Yjs algorithm and binary protocol, making it the preferred future Rust-side replay/validation adapter if backend CRDT execution becomes necessary.",
        rejected_or_deferred_because: "No yrs dependency exists in handshake_core today; adding native replay now would widen MT-003 beyond ADR and storage-boundary selection.",
        risks: "Version compatibility and client-id width must stay aligned with JavaScript/Yjs; Rust replay must never promote directly around KernelActionCatalog and WriteBox rules.",
        reuse_opportunities: "Can validate persisted Yjs-compatible updates later without changing the TypeScript wire format or stored update log.",
    },
    CrdtOptionAssessment {
        library: CrdtLibrary::Loro,
        decision: CrdtSelection::RejectForKernel002,
        fit_notes: "Strong local-first CRDT with Rust, JavaScript, snapshots, changes, undo/redo, time travel, and tree/map/list/text containers.",
        rejected_or_deferred_because: "It would introduce a second CRDT format and new frontend/backend dependencies while the product already carries Yjs/Tiptap collaboration gravity.",
        risks: "Migration from Tiptap/Yjs to Loro would add conversion risk, schema mismatch risk, and more review surface before pre-use hardening is complete.",
        reuse_opportunities: "Revisit as a future adapter if tree/time-travel semantics become more valuable than Tiptap/Yjs ecosystem compatibility.",
    },
    CrdtOptionAssessment {
        library: CrdtLibrary::Automerge,
        decision: CrdtSelection::RejectForKernel002,
        fit_notes: "Mature local-first CRDT with Rust and JavaScript implementations, sync protocol, and efficient binary storage; automerge-repo has pluggable storage including Postgres-like backends.",
        rejected_or_deferred_because: "Automerge document/repo model does not align with existing Tiptap/Yjs dependencies and would require replacing editor collaboration plumbing.",
        risks: "Introducing automerge-repo concepts could blur Kernel002 write boxes with an external repository abstraction and increase promotion-boundary complexity.",
        reuse_opportunities: "Storage and compaction patterns are useful references for update-log compaction and backend-independent persistence design.",
    },
    CrdtOptionAssessment {
        library: CrdtLibrary::ExistingProductDependencies,
        decision: CrdtSelection::YjsCompatibleUpdateLog,
        fit_notes: "Local product already ships yjs and @tiptap/extension-collaboration dependencies; current editor persists full JSON blocks over REST and can be migrated toward Yjs updates without replacing the whole document surface.",
        rejected_or_deferred_because: "",
        risks: "Dependencies are present but not wired into TiptapEditor, so Kernel002 must explicitly prevent full-block REST replacement from becoming the hidden authority path.",
        reuse_opportunities: "Reuse existing Tiptap editor, document API, Postgres storage abstraction, DCC projection patterns, and write-box/promotion work in later MTs.",
    },
];

const KERNEL002_CRDT_VALIDATION_PLAN: &[&str] = &[
    "replay updates from Postgres by document_id/workspace_id to rebuild materialized state and verify stored update_sha256/state_vector metadata",
    "compare materialized Tiptap/ProseMirror JSON against schema_id and reject unsupported nodes, unsupported marks, stale schemas, or lossy projections",
    "prove CRDT merge only creates pre-promotion evidence; promotion gate must validate actor eligibility, state_vector freshness, idempotency, and schema compatibility before EventLedger authority events",
    "record denied promotion attempts with actionable denial evidence and retain CRDT workspace state for replay",
    "compact only through snapshot receipts that preserve promotion evidence, update hashes, state vectors, and replay metadata",
];

const KERNEL002_CRDT_RESEARCH_SOURCES: &[&str] = &[
    "https://docs.yjs.dev/api/document-updates",
    "https://docs.yjs.dev/",
    "https://github.com/yjs/yjs",
    "https://github.com/y-crdt/y-crdt",
    "https://docs.rs/yrs/latest/yrs/",
    "https://docs.rs/loro/latest/loro/",
    "https://www.loro.dev/docs/api/js",
    "https://automerge.org/automerge/automerge/",
    "https://automerge.org/docs/reference/concepts/",
    "https://automerge.org/docs/reference/under-the-hood/storage/",
    "https://tiptap.dev/docs/collaboration/getting-started/overview",
    "https://tiptap.dev/docs/editor/extensions/functionality/collaboration",
    "app/package.json",
    "app/src/components/TiptapEditor.tsx",
    "src/backend/handshake_core/Cargo.toml",
];
