use std::{collections::BTreeMap, sync::Arc};

use crate::process_ledger::ReclaimTrigger;

use super::{SessionStep, SwarmHarness, SwarmHarnessError, SwarmReport, SwarmScenario};

const N8_PERF_STEPS_PER_SESSION: usize = 100;
const LEASE_CONTENTION_STEPS_PER_SESSION: usize = 12;

#[derive(Clone)]
pub struct RegisteredSwarmScenario {
    inner: Arc<dyn SwarmScenario>,
}

impl RegisteredSwarmScenario {
    fn new<S>(scenario: S) -> Self
    where
        S: SwarmScenario,
    {
        Self {
            inner: Arc::new(scenario),
        }
    }
}

impl SwarmScenario for RegisteredSwarmScenario {
    fn scenario_id(&self) -> &str {
        self.inner.scenario_id()
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        self.inner.session_steps(session_idx)
    }
}

#[derive(Clone, Default)]
pub struct ScenarioRegistry {
    scenarios: BTreeMap<String, RegisteredSwarmScenario>,
}

impl ScenarioRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<S>(&mut self, scenario: S) -> Result<(), SwarmHarnessError>
    where
        S: SwarmScenario,
    {
        let scenario_id = scenario.scenario_id().trim();
        if scenario_id.is_empty() {
            return Err(SwarmHarnessError::InvalidConfig(
                "scenario id must not be empty".to_string(),
            ));
        }
        if self.scenarios.contains_key(scenario_id) {
            return Err(SwarmHarnessError::InvalidConfig(format!(
                "duplicate scenario id: {scenario_id}"
            )));
        }
        self.scenarios.insert(
            scenario_id.to_string(),
            RegisteredSwarmScenario::new(scenario),
        );
        Ok(())
    }

    pub fn scenario(&self, scenario_id: &str) -> Option<RegisteredSwarmScenario> {
        self.scenarios.get(scenario_id).cloned()
    }

    pub fn scenario_ids(&self) -> Vec<String> {
        self.scenarios.keys().cloned().collect()
    }

    pub async fn run(&self, scenario_id: &str, n: usize) -> Result<SwarmReport, SwarmHarnessError> {
        let scenario = self.scenario(scenario_id).ok_or_else(|| {
            SwarmHarnessError::InvalidConfig(format!("unknown swarm scenario: {scenario_id}"))
        })?;
        SwarmHarness::new(n, scenario).run().await
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinSwarmScenario {
    N8Perf,
    SessionCancel,
    LeaseContention,
}

impl BuiltinSwarmScenario {
    pub fn all() -> [Self; 3] {
        [Self::N8Perf, Self::SessionCancel, Self::LeaseContention]
    }
}

impl SwarmScenario for BuiltinSwarmScenario {
    fn scenario_id(&self) -> &str {
        match self {
            Self::N8Perf => "n8-perf",
            Self::SessionCancel => "session-cancel",
            Self::LeaseContention => "lease-contention",
        }
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        match self {
            Self::N8Perf => n8_perf_steps(session_idx),
            Self::SessionCancel => session_cancel_steps(session_idx),
            Self::LeaseContention => lease_contention_steps(session_idx),
        }
    }
}

pub fn default_scenario_registry() -> ScenarioRegistry {
    let mut registry = ScenarioRegistry::new();
    for scenario in BuiltinSwarmScenario::all() {
        registry
            .register(scenario)
            .expect("builtin swarm scenario ids are static and unique");
    }
    registry
}

pub async fn run_swarm_scenario(
    scenario_id: &str,
    n: usize,
) -> Result<SwarmReport, SwarmHarnessError> {
    default_scenario_registry().run(scenario_id, n).await
}

fn n8_perf_steps(session_idx: usize) -> Vec<SessionStep> {
    (0..N8_PERF_STEPS_PER_SESSION)
        .map(|op_idx| SessionStep::MutateViaCatalog {
            action_id: "kernel.write_box.promote".to_string(),
            envelope_ref: format!("envelope://n8-perf/{session_idx}/{op_idx}"),
        })
        .collect()
}

fn session_cancel_steps(session_idx: usize) -> Vec<SessionStep> {
    vec![
        SessionStep::OpenWorkspace {
            ws_id: format!("workspace-session-cancel-{session_idx}"),
        },
        SessionStep::MutateViaCatalog {
            action_id: "kernel.write_box.promote".to_string(),
            envelope_ref: format!("envelope://session-cancel/{session_idx}/prepare"),
        },
        SessionStep::Reclaim {
            trigger: ReclaimTrigger::OperatorCancel,
        },
        SessionStep::ReadInspector,
        SessionStep::CloseSession,
    ]
}

fn lease_contention_steps(session_idx: usize) -> Vec<SessionStep> {
    let mut steps = vec![SessionStep::OpenWorkspace {
        ws_id: "workspace-lease-contention".to_string(),
    }];
    for op_idx in 0..LEASE_CONTENTION_STEPS_PER_SESSION {
        steps.push(SessionStep::MutateViaCatalog {
            action_id: "kernel.role_mailbox_claim_lease.project".to_string(),
            envelope_ref: format!(
                "envelope://lease-contention/shared-resource/{session_idx}/{op_idx}"
            ),
        });
    }
    steps.push(SessionStep::ReadInspector);
    steps.push(SessionStep::CloseSession);
    steps
}
