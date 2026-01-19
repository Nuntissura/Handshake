use std::path::Path;
use std::{env, fs};

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::capability_registry_workflow::{
    repo_root_from_manifest_dir, run_capability_registry_workflow, CapabilityRegistryWorkflowError,
    CapabilityRegistryWorkflowParams,
};
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use uuid::Uuid;

fn usage_string() -> String {
    "usage: capability_registry_workflow run --policy-decision-id <id> [--model-id <id>] [--reviewer-id <id>] [--approve]"
        .to_string()
}

fn take_flag_value(
    args: &mut Vec<String>,
    flag: &str,
) -> Result<String, CapabilityRegistryWorkflowError> {
    if args.is_empty() {
        return Err(CapabilityRegistryWorkflowError::MissingFlagValue {
            flag: flag.to_string(),
        });
    }
    Ok(args.remove(0))
}

fn init_flight_recorder(
    repo_root: &Path,
) -> Result<DuckDbFlightRecorder, CapabilityRegistryWorkflowError> {
    let data_dir = repo_root.join("data");
    fs::create_dir_all(&data_dir).map_err(|source| CapabilityRegistryWorkflowError::CreateDir {
        path: data_dir.clone(),
        source,
    })?;
    let fr_db_path = data_dir.join("flight_recorder.db");
    DuckDbFlightRecorder::new_on_path(&fr_db_path, 7).map_err(|source| {
        CapabilityRegistryWorkflowError::FlightRecorderInit {
            message: source.to_string(),
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), String> {
    run().await.map_err(|e| e.to_string())
}

async fn run() -> Result<(), CapabilityRegistryWorkflowError> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err(CapabilityRegistryWorkflowError::Usage {
            usage: usage_string(),
        });
    }
    let cmd = args.remove(0);
    if cmd != "run" {
        return Err(CapabilityRegistryWorkflowError::UnsupportedCommand { command: cmd });
    }

    let mut policy_decision_id: Option<String> = None;
    let mut reviewer_id: Option<String> = None;
    let mut model_id: Option<String> = None;
    let mut approve = false;
    while let Some(flag) = args.first().cloned() {
        args.remove(0);
        match flag.as_str() {
            "--policy-decision-id" => {
                policy_decision_id = Some(take_flag_value(&mut args, "--policy-decision-id")?);
            }
            "--reviewer-id" => {
                reviewer_id = Some(take_flag_value(&mut args, "--reviewer-id")?);
            }
            "--model-id" => {
                model_id = Some(take_flag_value(&mut args, "--model-id")?);
            }
            "--approve" => approve = true,
            other => {
                return Err(CapabilityRegistryWorkflowError::UnknownFlag {
                    flag: other.to_string(),
                })
            }
        }
    }

    let Some(policy_decision_id) = policy_decision_id else {
        return Err(CapabilityRegistryWorkflowError::PolicyDecisionIdRequired);
    };
    let model_id = model_id
        .or_else(|| env::var("OLLAMA_MODEL").ok())
        .unwrap_or_else(|| "llama3".to_string());

    let repo_root = repo_root_from_manifest_dir()?;
    let flight_recorder = init_flight_recorder(&repo_root)?;
    let trace_id = Uuid::new_v4();

    let registry = CapabilityRegistry::new();
    let params = CapabilityRegistryWorkflowParams {
        trace_id,
        policy_decision_id,
        model_id,
        reviewer_id,
        approve,
        job_id: None,
        workflow_id: None,
    };

    run_capability_registry_workflow(&repo_root, &registry, &flight_recorder, params).await?;
    Ok(())
}
