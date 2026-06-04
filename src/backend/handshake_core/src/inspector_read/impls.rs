use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use super::trace_projection::TraceProjection;
use super::trait_def::{
    EventLedgerRow, InspectorReadV1, ModelLoadedRow, ProcessRow, SessionId, SessionStateRead,
    SessionSummary, WorkspaceId, WorkspaceStateRead,
};

const INSPECTOR_READ_FILES: &[&str] =
    &["mod.rs", "trait_def.rs", "impls.rs", "trace_projection.rs"];
const DENY_IMPORT_PREFIX: &str = "crate::kernel::";
const DEFAULT_DENY_IMPORT_SUFFIXES: &[&str] = &[
    "action_catalog",
    "direct_edit_guard",
    "kb003_promotion",
    "pre_use_kernel_acceptance_run",
    "validation::patch_proposal",
    "write_boxes",
];
const INTERIOR_MUTABILITY_NAMES: &[&str] = &[
    "Atomic",
    "Cell",
    "Mutex",
    "OnceCell",
    "RefCell",
    "RwLock",
    "UnsafeCell",
];

#[derive(Clone, Debug, Default)]
pub struct InspectorReadSnapshot {
    pub sessions: Vec<SessionSummary>,
    pub session_states: BTreeMap<SessionId, SessionStateRead>,
    pub event_ledger_tail: Vec<EventLedgerRow>,
    pub processes: Vec<ProcessRow>,
    pub workspace_states: BTreeMap<WorkspaceId, WorkspaceStateRead>,
    pub loaded_models: Vec<ModelLoadedRow>,
}

impl InspectorReadV1 for InspectorReadSnapshot {
    fn list_sessions(&self) -> Vec<SessionSummary> {
        self.sessions.clone()
    }

    fn session_state(&self, id: SessionId) -> Option<SessionStateRead> {
        self.session_states.get(&id).cloned()
    }

    fn event_ledger_tail(&self, n: usize) -> Vec<EventLedgerRow> {
        if n == 0 {
            return Vec::new();
        }
        let start = self.event_ledger_tail.len().saturating_sub(n);
        self.event_ledger_tail[start..].to_vec()
    }

    fn process_ledger_active(&self) -> Vec<ProcessRow> {
        self.processes
            .iter()
            .filter(|row| row.status.eq_ignore_ascii_case("running"))
            .cloned()
            .collect()
    }

    fn workspace_state_read(&self, ws_id: WorkspaceId) -> Option<WorkspaceStateRead> {
        self.workspace_states.get(&ws_id).cloned()
    }

    fn trace_projection(&self, session_id: SessionId) -> Option<TraceProjection> {
        TraceProjection::from_event_rows(session_id, &self.event_ledger_tail)
    }

    fn loaded_models(&self) -> Vec<ModelLoadedRow> {
        self.loaded_models.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectorReadIsolationError {
    ForbiddenImport { needle: String },
    MutableReceiver { needle: String },
    InteriorMutability { needle: String },
    ReadSource { path: PathBuf, message: String },
}

impl fmt::Display for InspectorReadIsolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForbiddenImport { needle } => {
                write!(f, "inspector_read forbidden import matched `{needle}`")
            }
            Self::MutableReceiver { needle } => {
                write!(f, "inspector_read mutable receiver matched `{needle}`")
            }
            Self::InteriorMutability { needle } => {
                write!(f, "inspector_read interior mutability matched `{needle}`")
            }
            Self::ReadSource { path, message } => {
                write!(f, "failed to read {}: {message}", path.display())
            }
        }
    }
}

impl Error for InspectorReadIsolationError {}

#[derive(Clone, Debug)]
pub struct InspectorReadIsolationRule {
    deny_imports: Vec<String>,
    deny_mutable_self_methods: bool,
    deny_interior_mutability: bool,
}

impl Default for InspectorReadIsolationRule {
    fn default() -> Self {
        Self {
            deny_imports: DEFAULT_DENY_IMPORT_SUFFIXES
                .iter()
                .map(|suffix| format!("{DENY_IMPORT_PREFIX}{suffix}"))
                .collect(),
            deny_mutable_self_methods: true,
            deny_interior_mutability: true,
        }
    }
}

impl InspectorReadIsolationRule {
    pub fn validate_source(&self, source: &str) -> Result<(), InspectorReadIsolationError> {
        for denied in &self.deny_imports {
            if source.contains(denied) {
                return Err(InspectorReadIsolationError::ForbiddenImport {
                    needle: denied.clone(),
                });
            }
        }
        if self.deny_mutable_self_methods {
            for needle in mutable_receiver_needles() {
                if source.contains(&needle) {
                    return Err(InspectorReadIsolationError::MutableReceiver { needle });
                }
            }
        }
        if self.deny_interior_mutability {
            for name in INTERIOR_MUTABILITY_NAMES {
                let generic_marker = format!("{name}<");
                let path_marker = format!("{name}::");
                if source.contains(&generic_marker) || source.contains(&path_marker) {
                    return Err(InspectorReadIsolationError::InteriorMutability {
                        needle: name.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn mutable_receiver_needles() -> [String; 3] {
    [
        format!("&mut {}", "self"),
        format!("{}: &mut", "self"),
        format!("mut {}", "self"),
    ]
}

pub fn validate_inspector_read_source_tree(
    manifest_dir: &Path,
) -> Result<(), InspectorReadIsolationError> {
    let rule = InspectorReadIsolationRule::default();
    for file_name in INSPECTOR_READ_FILES {
        let path = manifest_dir.join("src/inspector_read").join(file_name);
        let source =
            fs::read_to_string(&path).map_err(|error| InspectorReadIsolationError::ReadSource {
                path: path.clone(),
                message: error.to_string(),
            })?;
        rule.validate_source(&source)?;
    }
    Ok(())
}
