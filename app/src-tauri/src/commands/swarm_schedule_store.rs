//! Durable persistence for calendar swarm schedules + their spawn templates.
//!
//! TRACK 1 (calendar app-wiring) persistence layer. A registered schedule is a
//! `(SwarmSchedule, SpawnTemplate)` pair: the schedule carries the cron + the
//! `ScheduledAction` (rank-7 core), and the template describes WHAT a scheduled
//! spin-up actually launches (provider, model artifact/sha or cloud model name,
//! runtime binding, swarm_id, worktree_id). The pair is serialized to a single
//! JSON file under `app_data_root` so registered schedules survive an app
//! restart: on startup the app LOADS this file, RE-ARMS the scheduler, and
//! applies the catch-up policy (skip-with-FR-note) for fires that were due while
//! the app was closed.
//!
//! The store is deliberately a plain, atomic-write JSON file (not a DB): the
//! schedule set is tiny (operator-authored), and a JSON file is trivially
//! inspectable, disk-agnostic (path is derived from `app_data_root`), and round
//! trips through `serde`. Writes are atomic (temp file + rename) so a crash
//! mid-write cannot corrupt the live schedule set.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use handshake_core::swarm_orchestration::{ScheduledAction, SwarmSchedule};
use serde::{Deserialize, Serialize};

/// File name (under `app_data_root`) the registered schedules persist to.
pub const SWARM_SCHEDULES_FILE: &str = "swarm_schedules.json";

/// Current on-disk schema version. Bumped if the persisted shape changes so a
/// future loader can migrate rather than silently mis-parse.
pub const SWARM_SCHEDULES_SCHEMA_VERSION: u32 = 1;

/// The spawn TEMPLATE stored alongside a schedule: WHAT a scheduled spin-up
/// launches. This is the orchestrator's spawn-template decision made durable —
/// every field the app needs to reconstruct a `SpawnRequest` for the fire.
///
/// Serializable (unlike `SpawnRequest`, which carries a non-serializable
/// `Duration`-bearing builder chain plus runtime-only fields); the app maps this
/// template + the fire's `time_box`/`swarm_id` into a real `SpawnRequest`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpawnTemplate {
    /// Provider lane: `local`, `byok_cloud`, or `official_cli`.
    pub provider: TemplateProvider,
    /// Local: the on-disk model artifact path (safetensors / GGUF).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    /// Local: expected sha256 hex of the artifact (the integrity gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_expected: Option<String>,
    /// Local: runtime binding (`candle` | `llama_cpp`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_binding: Option<TemplateRuntimeBinding>,
    /// Cloud: allowlisted cloud model name (e.g. `claude-sonnet-4`, `gpt-4o`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_model_name: Option<String>,
    /// Which concurrent instance index of this model to spawn (default 0).
    #[serde(default)]
    pub instance: u32,
    /// Worktree binding for the spawned session (board swimlane / per-worktree
    /// recovery). Carried into the `SpawnRequest` via `with_worktree`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
    /// Parent session id for ledger lineage. Defaults to a schedule tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateProvider {
    Local,
    ByokCloud,
    OfficialCli,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateRuntimeBinding {
    Candle,
    LlamaCpp,
}

/// One registered schedule = a `SwarmSchedule` (cron + action) plus its
/// `SpawnTemplate`. For a `Teardown` action the template may be a minimal
/// placeholder (teardown only needs the swarm id, which lives on the action),
/// but it is stored uniformly so the round-trip is total.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredSchedule {
    /// Flattened `SwarmSchedule` fields (cron + action) — stored as a mirror so
    /// the file is self-describing and does not depend on `SwarmSchedule`'s
    /// internal serde shape (the core type is not `Serialize`).
    pub id: String,
    pub cron: String,
    pub summary: String,
    pub action: PersistedAction,
    pub template: SpawnTemplate,
    /// When this schedule was registered (UTC). Used only for operator display.
    #[serde(default = "Utc::now")]
    pub registered_at: DateTime<Utc>,
}

/// Serializable mirror of the core `ScheduledAction` (which is not `Serialize`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PersistedAction {
    SpinUp {
        swarm_id: String,
        /// Time-box in seconds (`None` = no box; the configured lease_ttl
        /// reaps it). Stored as seconds so the JSON is human-readable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        time_box_secs: Option<u64>,
    },
    Teardown {
        swarm_id: String,
    },
}

impl PersistedAction {
    pub fn from_core(action: &ScheduledAction) -> Self {
        match action {
            ScheduledAction::SpinUp { swarm_id, time_box } => PersistedAction::SpinUp {
                swarm_id: swarm_id.clone(),
                time_box_secs: time_box.map(|d| d.as_secs()),
            },
            ScheduledAction::Teardown { swarm_id } => PersistedAction::Teardown {
                swarm_id: swarm_id.clone(),
            },
        }
    }

    pub fn to_core(&self) -> ScheduledAction {
        match self {
            PersistedAction::SpinUp {
                swarm_id,
                time_box_secs,
            } => ScheduledAction::SpinUp {
                swarm_id: swarm_id.clone(),
                time_box: time_box_secs.map(Duration::from_secs),
            },
            PersistedAction::Teardown { swarm_id } => ScheduledAction::Teardown {
                swarm_id: swarm_id.clone(),
            },
        }
    }

    /// The swarm id this action targets (present on both variants).
    pub fn swarm_id(&self) -> &str {
        match self {
            PersistedAction::SpinUp { swarm_id, .. } => swarm_id,
            PersistedAction::Teardown { swarm_id } => swarm_id,
        }
    }
}

impl RegisteredSchedule {
    /// Build a `RegisteredSchedule` from a live `SwarmSchedule` + its template.
    pub fn new(schedule: &SwarmSchedule, template: SpawnTemplate) -> Self {
        Self {
            id: schedule.id.clone(),
            cron: schedule.cron.clone(),
            summary: schedule.summary.clone(),
            action: PersistedAction::from_core(&schedule.action),
            template,
            registered_at: Utc::now(),
        }
    }

    /// Reconstruct the core `SwarmSchedule` (cron + action) for re-arming.
    pub fn to_swarm_schedule(&self) -> SwarmSchedule {
        SwarmSchedule {
            id: self.id.clone(),
            cron: self.cron.clone(),
            summary: self.summary.clone(),
            action: self.action.to_core(),
        }
    }
}

/// The persisted document: a schema version + the list of registered schedules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmScheduleDoc {
    pub schema_version: u32,
    #[serde(default)]
    pub schedules: Vec<RegisteredSchedule>,
}

impl Default for SwarmScheduleDoc {
    fn default() -> Self {
        Self {
            schema_version: SWARM_SCHEDULES_SCHEMA_VERSION,
            schedules: Vec::new(),
        }
    }
}

/// A JSON-file-backed store for registered schedules. Disk-agnostic: the path is
/// derived from the caller-supplied `app_data_root`, never hardcoded.
#[derive(Clone, Debug)]
pub struct SwarmScheduleStore {
    path: PathBuf,
}

impl SwarmScheduleStore {
    /// Bind the store to `<app_data_root>/swarm_schedules.json`.
    pub fn new(app_data_root: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_root.as_ref().join(SWARM_SCHEDULES_FILE),
        }
    }

    /// Bind the store to an explicit file path (tests / alternate wirings).
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the persisted schedules. A missing file is NOT an error — it yields
    /// an empty document (first run). A present-but-corrupt file returns an
    /// error so the caller can surface it rather than silently dropping the
    /// operator's schedule set.
    pub fn load(&self) -> Result<SwarmScheduleDoc, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice::<SwarmScheduleDoc>(&bytes).map_err(|error| {
                format!(
                    "swarm schedule store at {} is corrupt: {error}",
                    self.path.display()
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(SwarmScheduleDoc::default())
            }
            Err(error) => Err(format!(
                "failed to read swarm schedule store at {}: {error}",
                self.path.display()
            )),
        }
    }

    /// Persist the document atomically (write a temp file in the same directory,
    /// then rename over the target) so a crash mid-write cannot corrupt the live
    /// schedule set. Creates the parent directory if needed.
    pub fn save(&self, doc: &SwarmScheduleDoc) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create schedule store dir {}: {error}", parent.display())
            })?;
        }
        let json = serde_json::to_vec_pretty(doc)
            .map_err(|error| format!("failed to serialize swarm schedules: {error}"))?;
        // Unique temp name in the same dir so the final rename is atomic on the
        // same filesystem.
        let tmp = self.path.with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, &json).map_err(|error| {
            format!("failed to write temp schedule store {}: {error}", tmp.display())
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|error| {
            // Best-effort cleanup of the temp file on rename failure.
            let _ = std::fs::remove_file(&tmp);
            format!(
                "failed to commit schedule store {}: {error}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spinup() -> RegisteredSchedule {
        let schedule = SwarmSchedule {
            id: "morning-research".to_string(),
            cron: "0 0 9 * * *".to_string(),
            summary: "Morning research swarm".to_string(),
            action: ScheduledAction::SpinUp {
                swarm_id: "research".to_string(),
                time_box: Some(Duration::from_secs(3600)),
            },
        };
        let template = SpawnTemplate {
            provider: TemplateProvider::Local,
            artifact_path: Some("D:/models/qwen.safetensors".to_string()),
            sha256_expected: Some("ab".repeat(32)),
            runtime_binding: Some(TemplateRuntimeBinding::Candle),
            cloud_model_name: None,
            instance: 0,
            worktree_id: Some("wt-research".to_string()),
            parent_session_id: Some("calendar".to_string()),
        };
        RegisteredSchedule::new(&schedule, template)
    }

    #[test]
    fn persisted_action_round_trips_core() {
        let spinup = ScheduledAction::SpinUp {
            swarm_id: "s".to_string(),
            time_box: Some(Duration::from_secs(120)),
        };
        assert_eq!(PersistedAction::from_core(&spinup).to_core(), spinup);

        let teardown = ScheduledAction::Teardown {
            swarm_id: "s".to_string(),
        };
        assert_eq!(PersistedAction::from_core(&teardown).to_core(), teardown);
    }

    #[test]
    fn registered_schedule_reconstructs_swarm_schedule() {
        let reg = sample_spinup();
        let sched = reg.to_swarm_schedule();
        assert_eq!(sched.id, "morning-research");
        assert_eq!(sched.cron, "0 0 9 * * *");
        assert_eq!(
            sched.action,
            ScheduledAction::SpinUp {
                swarm_id: "research".to_string(),
                time_box: Some(Duration::from_secs(3600)),
            }
        );
    }

    #[test]
    fn store_round_trips_through_a_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());
        // Missing file -> empty doc, not an error.
        let loaded = store.load().expect("load empty");
        assert!(loaded.schedules.is_empty());
        assert_eq!(loaded.schema_version, SWARM_SCHEDULES_SCHEMA_VERSION);

        let mut doc = SwarmScheduleDoc::default();
        doc.schedules.push(sample_spinup());
        store.save(&doc).expect("save");

        // Real file exists on disk.
        assert!(store.path().exists());

        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded.schedules.len(), 1);
        assert_eq!(reloaded.schedules[0], doc.schedules[0]);
        assert_eq!(reloaded.schedules[0].template.artifact_path.as_deref(), Some("D:/models/qwen.safetensors"));
    }

    #[test]
    fn corrupt_file_surfaces_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());
        std::fs::write(store.path(), b"{not valid json").expect("write garbage");
        let err = store.load().expect_err("corrupt must error");
        assert!(err.contains("corrupt"), "{err}");
    }
}
