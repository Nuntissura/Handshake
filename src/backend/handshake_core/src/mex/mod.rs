//! Mechanical Extensions (MEX) Runtime (Spec §6.3.0, §11.8)
//!
//! Provides the PlannedOperation/EngineResult envelopes, global gate pipeline,
//! registry loader, runtime orchestrator, and conformance harness for
//! mechanical engines.

pub mod conformance;
pub mod envelope;
pub mod gates;
pub mod registry;
pub mod runtime;
pub mod supply_chain;

pub use conformance::{ConformanceCase, ConformanceHarness, ConformanceResult};
pub use envelope::{
    BudgetSpec, DeterminismLevel, EngineError, EngineResult, EngineStatus, EvidencePolicy,
    OutputSpec, PlannedOperation, ProvenanceRecord, POE_SCHEMA_VERSION,
};
pub use gates::{
    BudgetGate, CapabilityGate, DetGate, Gate, GateDenial, GatePipeline, IntegrityGate,
    ProvenanceGate, SchemaGate,
};
pub use registry::{EngineSpec, MexRegistry, OperationSpec};
pub use runtime::{EngineAdapter, MexRuntime, MexRuntimeError};
pub use supply_chain::{
    LicenseScanAllowlist, SecretScanAllowlist, SupplyChainAllowlists, SupplyChainEngineAdapter,
    SupplyChainReport, SupplyChainReportKind, TerminalServiceRunner, VulnScanAllowlist,
};
