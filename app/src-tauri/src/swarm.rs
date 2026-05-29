#[cfg(all(debug_assertions, feature = "swarm_ipc"))]
use handshake_core::test_harness::{run_swarm_scenario, SwarmReport};

pub const KERNEL_SWARM_RUN_IPC_CHANNEL: &str = "kernel.swarm.run";

#[cfg(all(debug_assertions, feature = "swarm_ipc"))]
#[tauri::command]
pub async fn kernel_swarm_run(scenario_id: String, n: usize) -> Result<SwarmReport, String> {
    let _ = KERNEL_SWARM_RUN_IPC_CHANNEL;
    run_swarm_scenario(&scenario_id, n)
        .await
        .map_err(|error| error.to_string())
}
