use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use handshake_core::{
    process_ledger::{LedgerEventKind, ReclaimTrigger},
    test_harness::{SessionStep, SwarmHarness, SwarmScenario},
};
use uuid::Uuid;

#[tokio::test]
async fn swarm_harness_baseline_runs_two_independent_governed_sessions() {
    let report = SwarmHarness::new(2, BaselineScenario)
        .run()
        .await
        .expect("swarm harness run");

    assert_eq!(report.n, 2);
    assert_eq!(report.scenario_id, "baseline-n2");
    assert_eq!(report.sessions.len(), 2);
    assert_eq!(report.ledger_overflow_count, 0);
    assert!(report.contention_events.is_empty());
    assert!(report.total_duration_ms > 0);

    let mut session_ids = HashSet::new();
    for result in &report.sessions {
        let uuid = Uuid::parse_str(&result.session_id).expect("session id is UUID");
        assert_eq!(uuid.get_version_num(), 7);
        assert!(session_ids.insert(result.session_id.clone()));
        assert_eq!(result.steps_completed, 8);
        assert!(result.errors.is_empty());
        assert!(!result.foreign_marker_seen);
        assert_eq!(
            result.local_marker,
            format!("swarm-marker-session-{}", result.session_idx)
        );
        assert_eq!(
            result.reclaim_triggers,
            vec![
                ReclaimTrigger::Close,
                ReclaimTrigger::Failure,
                ReclaimTrigger::Stale,
                ReclaimTrigger::OperatorCancel,
            ]
        );
        assert_eq!(result.process_ledger_rows.len(), 2);
        assert!(result
            .process_ledger_rows
            .iter()
            .all(|row| row.parent_session_id() == Some(result.session_id.as_str())));
        assert!(result
            .process_ledger_rows
            .iter()
            .any(|row| row.kind() == LedgerEventKind::Start));
        assert!(result
            .process_ledger_rows
            .iter()
            .any(|row| row.kind() == LedgerEventKind::Stop));
    }
}

#[test]
fn swarm_harness_sources_do_not_use_thread_local_session_state() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let offenders = thread_local_offenders(manifest_dir.join("src"));

    assert!(
        offenders.is_empty(),
        "thread_local! is forbidden outside test infra for swarm session state: {offenders:?}"
    );
}

struct BaselineScenario;

impl SwarmScenario for BaselineScenario {
    fn scenario_id(&self) -> &str {
        "baseline-n2"
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        vec![
            SessionStep::OpenWorkspace {
                ws_id: format!("workspace-baseline-{session_idx}"),
            },
            SessionStep::MutateViaCatalog {
                action_id: "kernel.write_box.promote".to_string(),
                envelope_ref: format!("envelope://baseline/{session_idx}"),
            },
            SessionStep::ReadInspector,
            SessionStep::Reclaim {
                trigger: ReclaimTrigger::Close,
            },
            SessionStep::Reclaim {
                trigger: ReclaimTrigger::Failure,
            },
            SessionStep::Reclaim {
                trigger: ReclaimTrigger::Stale,
            },
            SessionStep::Reclaim {
                trigger: ReclaimTrigger::OperatorCancel,
            },
            SessionStep::CloseSession,
        ]
    }
}

fn thread_local_offenders(root: PathBuf) -> Vec<PathBuf> {
    let mut offenders = Vec::new();
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            if entry.file_type().expect("file type").is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            if source.contains("thread_local!") {
                offenders.push(path);
            }
        }
    }
    offenders.sort();
    offenders
}
