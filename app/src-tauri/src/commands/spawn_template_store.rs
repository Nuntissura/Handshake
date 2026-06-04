//! Durable per-session SPAWN TEMPLATE persistence for ROI#3 STATE RECOVERY
//! (Resume-From-Session), WP-KERNEL-004.
//!
//! ## Why this exists (the load-bearing gap)
//!
//! Nothing persisted today can reconstruct a swarm spawn. `SessionSummary`
//! (`session_transcript.rs`) carries only id/kind/timestamps/model/provider; the
//! FR `SessionSpawned` event carries process/parent/instance and grouping
//! (`swarm_id` / `worktree_id`) but NOT artifact_path / sha256 /
//! cloud_model_name / exact BYOK provider / working_dir / isolation_tier; and
//! `build_spawn_request` mints a FRESH `ModelId` per spawn so the request content
//! is never recoverable from the live instance id. Therefore "resume this
//! recorded session with its original config" REQUIRES persisting a spawn
//! TEMPLATE at spawn time, keyed by the session's composite `instance_id`.
//!
//! ## Relationship to the calendar `SpawnTemplate`
//!
//! This deliberately MIRRORS the calendar scheduler's persistence pattern
//! (`swarm_schedule_store.rs`): an atomic temp+rename JSON file under
//! `app_data_root`, a `schema_version`, missing-file => empty, corrupt =>
//! typed error. It reuses the calendar's `TemplateProvider` /
//! `TemplateRuntimeBinding` enums verbatim (they already model exactly the
//! local-vs-cloud lane + the candle/llama binding).
//!
//! It does NOT reuse the calendar `SpawnTemplate` TYPE verbatim, on purpose: that
//! type lacks `working_dir` and `isolation_tier`, BOTH of which are load-bearing
//! for resume fidelity (the operator asked to recreate "the same provider /
//! model / artifact / worktree / working_dir / isolation"). A dedicated,
//! resume-complete [`SessionSpawnTemplate`] adds those two fields plus an
//! `origin_session_id` (the lineage root) and a `captured_at` stamp, and is keyed
//! by the live composite `instance_id` rather than by a schedule id. A local
//! `TemplateIsolationTier` is defined here because the calendar never modelled
//! isolation; it round-trips 1:1 against the swarm IPC isolation tier.
//!
//! ## Disk-agnostic [GLOBAL-PORTABILITY]
//!
//! The path is derived SOLELY from the caller-supplied `app_data_root`, never
//! hardcoded, identical to `swarm_schedule_store`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use handshake_core::model_runtime::WarmVmSnapshotManifest;
use serde::{Deserialize, Serialize};

// Reuse the calendar's provider + runtime-binding enums verbatim: they already
// model the local/byok_cloud/official_cli lane + the candle/llama binding, and
// reusing them keeps the two spawn-template stores semantically aligned.
pub use super::swarm_schedule_store::{
    TemplateLocalExecutionMode, TemplateProvider, TemplateRuntimeBinding,
};

/// File name (under `app_data_root`) the per-session spawn templates persist to.
pub const SPAWN_TEMPLATES_FILE: &str = "session_spawn_templates.json";

/// Current on-disk schema version. Bumped if the persisted shape changes so a
/// future loader can migrate rather than silently mis-parse.
pub const SPAWN_TEMPLATES_SCHEMA_VERSION: u32 = 3;

/// Operator-intended isolation tier captured on a resume template. Mirrors the
/// swarm IPC `SwarmIsolationTierIpc` (and through it
/// `handshake_core::sandbox::adapter::IsolationTier`) 1:1. The calendar template
/// never modelled isolation, so this is defined locally. Resume re-applies it
/// to the rebuilt request exactly as the original spawn recorded
/// it. Tier3 local llama.cpp is load-bearing when the sandbox registry is wired;
/// other combinations remain attribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateIsolationTier {
    Tier1Container,
    Tier2Syscall,
    Tier3Microvm,
}

/// Exact BYOK provider flavor captured on a resume template. Optional for
/// backward compatibility with pre-v2 templates and non-BYOK providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateByokCloudProvider {
    Anthropic,
    #[serde(rename = "openai", alias = "open_ai")]
    OpenAi,
}

/// The resume-complete spawn TEMPLATE captured per session at spawn time. Every
/// field the app needs to reconstruct a `SwarmSpawnRequestIpc` for a resume.
///
/// Captured from the VALIDATED `SwarmSpawnRequestIpc` only AFTER a successful
/// spawn (so it only ever records a request the spawn path accepted). Blank
/// fields are stored as `None` (the same trimming rule the spawn build applies).
/// Cloud templates store only `cloud_model_name` (never a key — keys live in the
/// `OsKeychainSecretsVault`); local templates store `artifact_path` +
/// `sha256_expected` + `runtime_binding` plus a warm restore manifest when a
/// restored warm-VM spawn supplied one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionSpawnTemplate {
    /// Provider lane: `local`, `byok_cloud`, or `official_cli`.
    pub provider: TemplateProvider,
    /// Local: the on-disk model artifact path (safetensors / GGUF).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    /// Local: expected sha256 hex of the artifact (the integrity gate, preserved
    /// on resume — a changed/missing artifact fails resume with the real factory
    /// sha-mismatch error, never a silent or faked spawn).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_expected: Option<String>,
    /// Local: runtime binding (`candle` | `llama_cpp`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_binding: Option<TemplateRuntimeBinding>,
    /// Local: explicit execution substrate (`cold` | `warm_vm`). `None` means
    /// cold/default for backward-compatible templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_execution_mode: Option<TemplateLocalExecutionMode>,
    /// Local warm VM only: restored snapshot manifest captured from the spawn
    /// request so resume can replay the same warm-start path instead of dropping
    /// to a cold warm-VM boot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warm_vm_restore_manifest: Option<WarmVmSnapshotManifest>,
    /// Cloud: allowlisted cloud model name (e.g. `claude-sonnet-4`, `gpt-4o`).
    /// NEVER the BYOK key — resume re-resolves the key from the vault via the
    /// same lane, so a resume after key rotation/removal honestly fails
    /// `ProviderNotConfigured`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_model_name: Option<String>,
    /// Cloud: exact BYOK provider flavor when the operator selected one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byok_cloud_provider: Option<TemplateByokCloudProvider>,
    /// Which concurrent instance index the original session used (default 0).
    #[serde(default)]
    pub instance: u32,
    /// Operator-assigned swarm grouping (board swimlane / paired orchestration
    /// identity). Re-applied to the rebuilt request on resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swarm_id: Option<String>,
    /// Operator-assigned worktree binding (board swimlane / per-worktree
    /// recovery / paired orchestration). Re-applied to the rebuilt request on resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
    /// Operator-assigned on-disk place (attribution only; never executed/resolved).
    /// NOT in the calendar template — added here for resume fidelity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// Operator-intended isolation tier. NOT in the calendar template — added
    /// here for resume fidelity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation_tier: Option<TemplateIsolationTier>,
    /// Rank-6 committed-memory estimate captured from the successful spawn.
    /// Preserved on resume so a run with committed-memory admission enabled does
    /// not fail closed merely because the template dropped the estimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_memory_bytes: Option<u64>,
    /// The session this template was captured from (the lineage root for resume).
    /// On resume the rebuilt request's `parent_session_id` is set to this, so the
    /// FR `SessionSpawned.parent_session_id` records "resumed from X".
    pub origin_session_id: String,
    /// When this template was captured (UTC). Operator display only.
    #[serde(default = "Utc::now")]
    pub captured_at: DateTime<Utc>,
}

/// The persisted document: a schema version + the templates keyed by the
/// composite `instance_id` (`<model_id>#<instance>`). A `BTreeMap` keeps the file
/// deterministic regardless of insertion order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnTemplateDoc {
    pub schema_version: u32,
    #[serde(default)]
    pub templates: BTreeMap<String, SessionSpawnTemplate>,
}

impl Default for SpawnTemplateDoc {
    fn default() -> Self {
        Self {
            schema_version: SPAWN_TEMPLATES_SCHEMA_VERSION,
            templates: BTreeMap::new(),
        }
    }
}

/// A JSON-file-backed store for per-session spawn templates. Disk-agnostic: the
/// path is derived from the caller-supplied `app_data_root`, never hardcoded.
#[derive(Clone, Debug)]
pub struct SpawnTemplateStore {
    path: PathBuf,
}

impl SpawnTemplateStore {
    /// Bind the store to `<app_data_root>/session_spawn_templates.json`.
    pub fn new(app_data_root: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_root.as_ref().join(SPAWN_TEMPLATES_FILE),
        }
    }

    /// Bind the store to an explicit file path (tests / alternate wirings).
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the persisted templates. A missing file is NOT an error — it yields
    /// an empty document (first run). A present-but-corrupt file returns an error
    /// so the caller can surface it rather than silently dropping templates.
    pub fn load(&self) -> Result<SpawnTemplateDoc, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                let doc = serde_json::from_slice::<SpawnTemplateDoc>(&bytes).map_err(|error| {
                    format!(
                        "spawn template store at {} is corrupt: {error}",
                        self.path.display()
                    )
                })?;
                self.normalize_loaded_doc(doc)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(SpawnTemplateDoc::default())
            }
            Err(error) => Err(format!(
                "failed to read spawn template store at {}: {error}",
                self.path.display()
            )),
        }
    }

    fn normalize_loaded_doc(&self, mut doc: SpawnTemplateDoc) -> Result<SpawnTemplateDoc, String> {
        match doc.schema_version {
            SPAWN_TEMPLATES_SCHEMA_VERSION => Ok(doc),
            // v1/v2 -> v3 only added optional fields with serde defaults.
            // Loading them as v3 is a lossless migration; the next save
            // persists v3.
            1 | 2 => {
                doc.schema_version = SPAWN_TEMPLATES_SCHEMA_VERSION;
                Ok(doc)
            }
            version if version > SPAWN_TEMPLATES_SCHEMA_VERSION => Err(format!(
                "spawn template store at {} uses unsupported schema_version {version}; this app supports {}",
                self.path.display(),
                SPAWN_TEMPLATES_SCHEMA_VERSION
            )),
            version => Err(format!(
                "spawn template store at {} uses unsupported schema_version {version}; expected {} or compatible v1/v2",
                self.path.display(),
                SPAWN_TEMPLATES_SCHEMA_VERSION
            )),
        }
    }

    /// Persist the document atomically (write a temp file in the same directory,
    /// then rename over the target) so a crash mid-write cannot corrupt the live
    /// template set. Creates the parent directory if needed. Mirrors
    /// `SwarmScheduleStore::save`.
    pub fn save(&self, doc: &SpawnTemplateDoc) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create spawn template store dir {}: {error}",
                    parent.display()
                )
            })?;
        }
        let mut doc = doc.clone();
        doc.schema_version = SPAWN_TEMPLATES_SCHEMA_VERSION;
        let json = serde_json::to_vec_pretty(&doc)
            .map_err(|error| format!("failed to serialize spawn templates: {error}"))?;
        let tmp = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, &json).map_err(|error| {
            format!(
                "failed to write temp spawn template store {}: {error}",
                tmp.display()
            )
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|error| {
            let _ = std::fs::remove_file(&tmp);
            format!(
                "failed to commit spawn template store {}: {error}",
                self.path.display()
            )
        })?;
        Ok(())
    }

    /// Read-modify-write a single template keyed by the composite `instance_id`,
    /// preserving every other stored template. Atomic via `save`. Idempotent: a
    /// second upsert of the same key REPLACES the prior value (re-resuming a
    /// session re-captures the current template).
    pub fn upsert(
        &self,
        instance_id: impl Into<String>,
        template: SessionSpawnTemplate,
    ) -> Result<(), String> {
        let mut doc = self.load()?;
        doc.templates.insert(instance_id.into(), template);
        self.save(&doc)
    }

    /// Fetch the stored template for a composite `instance_id`, or `None` when no
    /// template was captured for it (not resumable). A corrupt store surfaces as
    /// an error so the resume command reports it honestly rather than treating a
    /// readable-but-broken store as "not resumable".
    pub fn get(&self, instance_id: &str) -> Result<Option<SessionSpawnTemplate>, String> {
        Ok(self.load()?.templates.get(instance_id).cloned())
    }

    /// Best-effort presence probe for the `resumable` flag on a session summary.
    /// A load error => `false` so a corrupt/transient store can NEVER break
    /// `kernel_session_list` (the list stays honest: not-resumable rather than
    /// failing the whole listing).
    pub fn contains(&self, instance_id: &str) -> bool {
        self.load()
            .map(|doc| doc.templates.contains_key(instance_id))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_local(origin: &str) -> SessionSpawnTemplate {
        SessionSpawnTemplate {
            provider: TemplateProvider::Local,
            artifact_path: Some("D:/models/qwen.safetensors".to_string()),
            sha256_expected: Some("ab".repeat(32)),
            runtime_binding: Some(TemplateRuntimeBinding::Candle),
            local_execution_mode: Some(TemplateLocalExecutionMode::Cold),
            warm_vm_restore_manifest: None,
            cloud_model_name: None,
            byok_cloud_provider: None,
            instance: 2,
            swarm_id: Some("swarm-research".to_string()),
            worktree_id: Some("wt-research".to_string()),
            working_dir: Some("D:/work/wt-research".to_string()),
            isolation_tier: Some(TemplateIsolationTier::Tier3Microvm),
            committed_memory_bytes: Some(6 * 1024 * 1024 * 1024),
            origin_session_id: origin.to_string(),
            captured_at: Utc::now(),
        }
    }

    fn sample_cloud(origin: &str) -> SessionSpawnTemplate {
        SessionSpawnTemplate {
            provider: TemplateProvider::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            local_execution_mode: None,
            warm_vm_restore_manifest: None,
            cloud_model_name: Some("claude-sonnet-4".to_string()),
            byok_cloud_provider: Some(TemplateByokCloudProvider::Anthropic),
            instance: 0,
            swarm_id: None,
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
            committed_memory_bytes: None,
            origin_session_id: origin.to_string(),
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn missing_file_loads_an_empty_doc_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let loaded = store.load().expect("missing file -> empty doc");
        assert!(loaded.templates.is_empty());
        assert_eq!(loaded.schema_version, SPAWN_TEMPLATES_SCHEMA_VERSION);
        // contains() on an absent store is false (never an error).
        assert!(!store.contains("anything#0"));
        // get() on an absent store is Ok(None).
        assert_eq!(store.get("anything#0").expect("get ok"), None);
    }

    #[test]
    fn template_byok_provider_serializes_openai_and_reads_legacy_open_ai_alias() {
        assert_eq!(
            serde_json::to_string(&TemplateByokCloudProvider::OpenAi).expect("serialize"),
            "\"openai\""
        );
        assert_eq!(
            serde_json::from_str::<TemplateByokCloudProvider>("\"openai\"")
                .expect("deserialize openai"),
            TemplateByokCloudProvider::OpenAi
        );
        assert_eq!(
            serde_json::from_str::<TemplateByokCloudProvider>("\"open_ai\"")
                .expect("deserialize legacy alias"),
            TemplateByokCloudProvider::OpenAi
        );
    }

    #[test]
    fn v1_store_loads_as_lossless_v2_migration() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let key = "11111111-1111-7111-8111-111111111111#2";
        let mut doc = SpawnTemplateDoc::default();
        doc.schema_version = 1;
        doc.templates
            .insert(key.to_string(), sample_local("origin#0"));
        std::fs::write(
            store.path(),
            serde_json::to_vec_pretty(&doc).expect("serialize v1 doc"),
        )
        .expect("write v1 doc");

        let loaded = store.load().expect("v1 loads through migration");
        assert_eq!(loaded.schema_version, SPAWN_TEMPLATES_SCHEMA_VERSION);
        assert!(loaded.templates.contains_key(key));
    }

    #[test]
    fn future_schema_version_fails_closed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let mut doc = SpawnTemplateDoc::default();
        doc.schema_version = SPAWN_TEMPLATES_SCHEMA_VERSION + 1;
        std::fs::write(
            store.path(),
            serde_json::to_vec_pretty(&doc).expect("serialize future doc"),
        )
        .expect("write future doc");

        let err = store.load().expect_err("future version must not load");
        assert!(err.contains("unsupported schema_version"), "{err}");
        assert!(
            !store.contains("anything#0"),
            "list surfaces future schemas as not resumable instead of panicking"
        );
    }

    #[test]
    fn upsert_then_get_round_trips_local_through_a_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let key = "11111111-1111-7111-8111-111111111111#2";
        store.upsert(key, sample_local("origin#0")).expect("upsert");

        // A real file is on disk.
        assert!(store.path().exists());

        let got = store.get(key).expect("get ok").expect("present");
        assert_eq!(got.provider, TemplateProvider::Local);
        assert_eq!(
            got.artifact_path.as_deref(),
            Some("D:/models/qwen.safetensors")
        );
        assert_eq!(got.sha256_expected.as_deref(), Some(&"ab".repeat(32)[..]));
        assert_eq!(got.runtime_binding, Some(TemplateRuntimeBinding::Candle));
        assert_eq!(
            got.local_execution_mode,
            Some(TemplateLocalExecutionMode::Cold)
        );
        assert_eq!(got.instance, 2);
        // working_dir + isolation_tier + lineage survive the round-trip (the
        // two fields the calendar template lacked, plus the resume lineage root).
        assert_eq!(got.worktree_id.as_deref(), Some("wt-research"));
        assert_eq!(got.working_dir.as_deref(), Some("D:/work/wt-research"));
        assert_eq!(
            got.isolation_tier,
            Some(TemplateIsolationTier::Tier3Microvm)
        );
        assert_eq!(got.committed_memory_bytes, Some(6 * 1024 * 1024 * 1024));
        assert_eq!(got.origin_session_id, "origin#0");
        assert!(store.contains(key));
    }

    #[test]
    fn upsert_is_read_modify_write_and_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let a = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa#0";
        let b = "bbbbbbbb-bbbb-7bbb-8bbb-bbbbbbbbbbbb#0";

        store.upsert(a, sample_local("o#0")).expect("upsert a");
        // A second upsert of a DIFFERENT key must preserve the first (RMW).
        store.upsert(b, sample_cloud("o#0")).expect("upsert b");
        let doc = store.load().expect("load");
        assert_eq!(doc.templates.len(), 2, "RMW preserved both keys");

        // Re-upsert of the SAME key REPLACES (idempotent on key), not duplicates.
        let mut replaced = sample_cloud("o#0");
        replaced.cloud_model_name = Some("gpt-4o".to_string());
        store.upsert(b, replaced).expect("re-upsert b");
        let doc = store.load().expect("reload");
        assert_eq!(doc.templates.len(), 2, "re-upsert did not add a row");
        assert_eq!(
            doc.templates
                .get(b)
                .expect("b present")
                .cloud_model_name
                .as_deref(),
            Some("gpt-4o")
        );
    }

    #[test]
    fn cloud_template_stores_only_the_model_name_no_local_fields() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        let key = "cccccccc-cccc-7ccc-8ccc-cccccccccccc#0";
        store.upsert(key, sample_cloud("o#0")).expect("upsert");
        let got = store.get(key).expect("ok").expect("present");
        assert_eq!(got.provider, TemplateProvider::ByokCloud);
        assert_eq!(got.cloud_model_name.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(got.artifact_path, None);
        assert_eq!(got.sha256_expected, None);
        assert_eq!(got.runtime_binding, None);
        assert_eq!(got.local_execution_mode, None);
    }

    #[test]
    fn corrupt_file_surfaces_an_error_on_get_but_contains_is_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SpawnTemplateStore::new(tmp.path());
        std::fs::write(store.path(), b"{not valid json").expect("write garbage");
        let err = store.get("x#0").expect_err("corrupt must error");
        assert!(err.contains("corrupt"), "{err}");
        // contains() degrades to false on a corrupt store so kernel_session_list
        // never breaks (honest: surfaces as not-resumable, not a list failure).
        assert!(!store.contains("x#0"));
    }
}
