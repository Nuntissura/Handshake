pub mod bitemporal;
pub mod builder;
pub mod calibration;
pub mod capsule;
pub mod hygiene;
pub mod injection;
pub mod ipc;
pub mod outcome_feedback;
pub mod persistence;
pub mod persistence_postgres;
pub mod pinned_core;
pub mod policy_table;
pub mod progressive_retrieval;
pub mod replay_eval;
pub mod retrieval_mode;
pub mod scoring;
pub mod trace_export;

pub use builder::{
    BuildContext, BuilderError, CapsuleBuilder, FemsError, FemsMtHandoffRetriever, FemsRetriever,
    PINNED_RETRIEVAL_HEADROOM, RetrievedItem, pinned_aware_retrieval_limit,
};
pub use capsule::{
    CapsuleAuditEntry, CapsuleAuditLog, DegradationTier, MemoryCapsule, MemoryCapsuleError,
    RetrievalPolicy, TaskType,
};
pub use injection::{
    CapsuleFlightRecorderEvent, CapsuleHandle, CapsuleInjectedEvent, CapsuleInjector,
    CapsuleSuppressedEvent, FR_EVT_CAPSULE_INJECTED, FR_EVT_CAPSULE_SUPPRESSED, FemsFlightRecorder,
    FemsFlightRecorderError, InjectionDecision, InjectionError, MemoryCapsuleInjection,
    MemoryInjectionReceipt, ModelCallContext, ModelCallContextSource, SharedCapsuleInjector,
    SkipReason, attach_capsule_to_generate_request, render_capsule_prompt,
};
pub use ipc::{
    CapsuleIpcError, CapsuleIpcService, CapsuleRecordStore, CapsuleSummary, GetCapsuleRequest,
    GetCapsuleResponse, ListRecentCapsulesRequest, ListRecentCapsulesResponse,
    MEMORY_CAPSULE_GET_COMMAND, MEMORY_CAPSULE_LIST_RECENT_COMMAND,
    MEMORY_CAPSULE_SUPPRESS_ACTION_ID, MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND,
    MEMORY_CAPSULE_SUPPRESS_INPUT_SCHEMA_ID, MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND,
    MEMORY_CAPSULE_SUPPRESS_PAYLOAD_SCHEMA_ID, MemoryCapsuleIpcStore, MemoryIpcError,
    MemoryIpcService, SuppressCapsuleRequest, SuppressItemRequest, SuppressionReceipt,
};
pub use persistence::{
    CapsuleOutcome, CapsuleRecord, CapsuleRecorder, KERNEL_ACTION_REQUEST_SCHEMA_ID,
    KernelActionRejection, KernelActionSubmission, KernelActionSubmitter,
    MEMORY_CAPSULE_RECORD_ACTION_ID, MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID,
    MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID, MEMORY_WRITE_BOX_SCHEMA_ID, RecordReceipt,
    RecorderError, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID, WriteBoxV1Envelope,
};
pub use persistence_postgres::{
    MEMORY_CAPSULE_AGGREGATE_TYPE, MEMORY_CAPSULE_SOURCE_COMPONENT, PostgresKernelActionSubmitter,
    PostgresMemoryCapsuleStore,
};
pub use pinned_core::{
    FR_EVT_MEMORY_PIN, FR_EVT_MEMORY_UNPIN, PIN_MEMORY_ACTION_ID, PIN_MEMORY_INPUT_SCHEMA_ID,
    PIN_MEMORY_PAYLOAD_SCHEMA_ID, PIN_MEMORY_RESULT_SCHEMA_ID, PinError, PinIpcService, PinReceipt,
    PinSubmitter, PinnedBudget, PinnedCoreSelector, PinnedItem, SetPinRequest,
    UNPIN_MEMORY_ACTION_ID,
};
pub use policy_table::{CapsulePolicyTable, RETRIEVAL_SCORING_FORMULA_V0};
