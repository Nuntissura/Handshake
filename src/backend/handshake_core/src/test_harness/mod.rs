pub mod crdt_workspace;
pub mod cross_adapter;
pub mod escape_attempts;
pub mod invariants;
pub mod scenarios;
pub mod session;
pub mod swarm;

#[cfg(all(feature = "surreal-test-support", feature = "runtime-full", feature = "duckdb-flight-recorder"))]
pub mod ac9_role_authority;

pub use invariants::{
    HbrSwarmInvariantFail, HbrSwarmLoopCapReceipt, HbrSwarmLoopCounter, FR_EVT_LOOP_CAP,
    HBR_SWARM_002_LOOP_CAP, HBR_SWARM_INVARIANT_FAIL,
};
pub use scenarios::{
    default_scenario_registry, run_swarm_scenario, BuiltinSwarmScenario, RegisteredSwarmScenario,
    ScenarioRegistry,
};
pub use session::{SessionResult, SessionStep};
pub use swarm::{
    ContentionEvent, SwarmCrdtWorkspaceSummary, SwarmHarness, SwarmHarnessError, SwarmReport,
    SwarmScenario, SHARED_LEASE_RESOURCE,
};

#[cfg(feature = "surreal-test-support")]
pub mod surreal_test_store_support;

#[cfg(feature = "surreal-test-support")]
pub mod swarm_lane_diagnostics;

#[cfg(feature = "test-utils")]
pub mod model_runtime_selection;
