pub mod builder;
pub mod capsule;
pub mod injection;
pub mod ipc;
pub mod persistence;
pub mod policy_table;

pub use builder::{
    BuildContext, BuilderError, CapsuleBuilder, FemsError, FemsMtHandoffRetriever, FemsRetriever,
    RetrievedItem,
};
pub use capsule::{
    CapsuleAuditEntry, CapsuleAuditLog, DegradationTier, MemoryCapsule, MemoryCapsuleError,
    RetrievalPolicy, TaskType,
};
pub use injection::{
    attach_capsule_to_generate_request, render_capsule_prompt, CapsuleFlightRecorderEvent,
    CapsuleHandle, CapsuleInjectedEvent, CapsuleInjector, CapsuleSuppressedEvent,
    FemsFlightRecorder, FemsFlightRecorderError, InjectionDecision, InjectionError,
    MemoryCapsuleInjection, MemoryInjectionReceipt, ModelCallContext, ModelCallContextSource,
    SharedCapsuleInjector, SkipReason, FR_EVT_CAPSULE_INJECTED, FR_EVT_CAPSULE_SUPPRESSED,
};
pub use ipc::{
    CapsuleIpcError, CapsuleIpcService, CapsuleRecordStore, CapsuleSummary, GetCapsuleRequest,
    GetCapsuleResponse, ListRecentCapsulesRequest, ListRecentCapsulesResponse,
    MemoryCapsuleIpcStore, MemoryIpcError, MemoryIpcService, SuppressCapsuleRequest,
    SuppressItemRequest, SuppressionReceipt, MEMORY_CAPSULE_GET_COMMAND,
    MEMORY_CAPSULE_LIST_RECENT_COMMAND, MEMORY_CAPSULE_SUPPRESS_ACTION_ID,
    MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND, MEMORY_CAPSULE_SUPPRESS_INPUT_SCHEMA_ID,
    MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND, MEMORY_CAPSULE_SUPPRESS_PAYLOAD_SCHEMA_ID,
};
pub use persistence::{
    CapsuleOutcome, CapsuleRecord, CapsuleRecorder, KernelActionRejection, KernelActionSubmission,
    KernelActionSubmitter, RecordReceipt, RecorderError, WriteBoxV1Envelope,
    KERNEL_ACTION_REQUEST_SCHEMA_ID, MEMORY_CAPSULE_RECORD_ACTION_ID,
    MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID, MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID,
    MEMORY_WRITE_BOX_SCHEMA_ID, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
};
pub use policy_table::{CapsulePolicyTable, RETRIEVAL_SCORING_FORMULA_V0};
