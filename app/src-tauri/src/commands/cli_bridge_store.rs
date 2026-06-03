//! Durable persistence for the operator's Official-CLI bridge configuration.
//!
//! The official-CLI cloud lane (Claude Code / Codex CLI / any-CLI) is a complete
//! swarm cloud runtime (commit b9ae06f3): `CliBridgeModelRuntime` streams the
//! CLI subprocess stdout as live tokens, `CliBridgeCloudRuntimeBuilder` builds it
//! on the `ProviderKind::OfficialCli` lane, and the production factory dispatches
//! to it. The ONLY missing piece is an operator-facing config: the production app
//! sets `official_cli: None` because nothing tells it the CLI executable + args.
//!
//! This store is that config made durable. It mirrors `swarm_schedule_store.rs`
//! byte-for-byte in posture: a single atomic-write JSON file under
//! `app_data_root` (disk-agnostic, never a hardcoded path per [GLOBAL-PORTABILITY-004]),
//! a `SCHEMA_VERSION`, missing-file -> default (honest "unconfigured"), and
//! present-but-corrupt -> error (so a bad file surfaces rather than silently
//! dropping the operator's config).
//!
//! CAPABILITY/SECRET POSTURE: the CLI bridge authenticates via the operator's own
//! CLI login (`claude auth login`, the codex login flow, ...). The kernel stores
//! NO API key. The config is executable path + args template + model allowlist +
//! timeout + (optional) non-secret env vars only. The spawner additionally scrubs
//! secret-bearing inherited env names at spawn time (`official_cli_bridge.rs`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use handshake_core::model_runtime::cloud::{CliBridgeConfig, CliKind, CliOutputFormat};
use serde::{Deserialize, Serialize};

/// File name (under `app_data_root`) the CLI-bridge config persists to.
pub const CLI_BRIDGE_CONFIG_FILE: &str = "cli_bridge_config.json";

/// Current on-disk schema version. Bumped if the persisted shape changes so a
/// future loader can migrate rather than silently mis-parse.
pub const CLI_BRIDGE_CONFIG_SCHEMA_VERSION: u32 = 1;

/// Serializable mirror of the core `CliKind` (`official_cli_bridge.rs:45-50`),
/// which is not `Serialize`. Serialised as snake_case so the frontend receives
/// `"claude_code"` etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredCliKind {
    ClaudeCode,
    CodexCli,
    GeminiCli,
    Other,
}

impl StoredCliKind {
    pub fn to_core(self) -> CliKind {
        match self {
            StoredCliKind::ClaudeCode => CliKind::ClaudeCode,
            StoredCliKind::CodexCli => CliKind::CodexCli,
            StoredCliKind::GeminiCli => CliKind::GeminiCli,
            StoredCliKind::Other => CliKind::Other,
        }
    }

    pub fn from_core(kind: CliKind) -> Self {
        match kind {
            CliKind::ClaudeCode => StoredCliKind::ClaudeCode,
            CliKind::CodexCli => StoredCliKind::CodexCli,
            CliKind::GeminiCli => StoredCliKind::GeminiCli,
            CliKind::Other => StoredCliKind::Other,
        }
    }
}

/// Serializable mirror of the core `CliOutputFormat` (`official_cli_bridge.rs:63-68`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredOutputFormat {
    Json,
    RawText,
    JsonStream,
}

impl StoredOutputFormat {
    pub fn to_core(self) -> CliOutputFormat {
        match self {
            StoredOutputFormat::Json => CliOutputFormat::Json,
            StoredOutputFormat::RawText => CliOutputFormat::RawText,
            StoredOutputFormat::JsonStream => CliOutputFormat::JsonStream,
        }
    }

    pub fn from_core(fmt: CliOutputFormat) -> Self {
        match fmt {
            CliOutputFormat::Json => StoredOutputFormat::Json,
            CliOutputFormat::RawText => StoredOutputFormat::RawText,
            CliOutputFormat::JsonStream => StoredOutputFormat::JsonStream,
        }
    }
}

/// The persisted CLI-bridge configuration document. Serialised camelCase so the
/// frontend IPC layer consumes it directly (mirrors `CloudLaneSummary`).
///
/// `configured = false` is the honest unconfigured default: the production
/// factory leaves `official_cli: None` and a CLI swarm spawn reports
/// `ProviderNotConfigured` until the operator saves a real config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliBridgeConfigDoc {
    pub schema_version: u32,
    /// `false` => the official_cli swarm lane stays disabled (None).
    pub configured: bool,
    pub cli_kind: StoredCliKind,
    /// Stored as a String (not PathBuf) so the JSON is disk-agnostic + portable.
    pub executable_path: String,
    /// MUST contain a `{prompt}`-bearing element; may contain `{model}`.
    pub args_template: Vec<String>,
    pub output_format: StoredOutputFormat,
    /// Allowlisted CLI model names (the per-spawn `engine_origin` gate).
    pub model_allowlist: Vec<String>,
    pub working_dir: Option<String>,
    pub timeout_seconds: u64,
    /// NON-secret env vars only. The spawner scrubs secret-bearing inherited
    /// names regardless (`official_cli_bridge.rs`), and config.env_vars is
    /// applied AFTER that scrub as an intentional operator opt-in.
    pub env_vars: BTreeMap<String, String>,
    pub updated_at_utc: Option<String>,
}

impl Default for CliBridgeConfigDoc {
    fn default() -> Self {
        Self {
            schema_version: CLI_BRIDGE_CONFIG_SCHEMA_VERSION,
            configured: false,
            cli_kind: StoredCliKind::Other,
            executable_path: String::new(),
            args_template: Vec::new(),
            output_format: StoredOutputFormat::RawText,
            model_allowlist: Vec::new(),
            working_dir: None,
            timeout_seconds: 0,
            env_vars: BTreeMap::new(),
            updated_at_utc: None,
        }
    }
}

impl CliBridgeConfigDoc {
    /// Map this stored doc into the core `CliBridgeConfig` the
    /// `CliBridgeCloudRuntimeBuilder` consumes. `Err` only if the doc is not
    /// actually configured (the caller should gate on `configured` first; this
    /// is defence-in-depth so a malformed doc never silently builds a lane).
    pub fn to_cli_bridge_config(&self) -> Result<CliBridgeConfig, String> {
        if !self.configured {
            return Err("CLI bridge config is not configured".to_string());
        }
        if self.executable_path.trim().is_empty() {
            return Err("CLI bridge executable_path is empty".to_string());
        }
        if !self.args_template.iter().any(|a| a.contains("{prompt}")) {
            return Err("CLI bridge args_template is missing the {prompt} placeholder".to_string());
        }
        if self.timeout_seconds == 0 {
            return Err("CLI bridge timeout_seconds must be > 0".to_string());
        }
        Ok(CliBridgeConfig {
            cli_kind: self.cli_kind.to_core(),
            executable_path: PathBuf::from(self.executable_path.clone()),
            args_template: self.args_template.clone(),
            output_format: self.output_format.to_core(),
            env_vars: self
                .env_vars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            working_dir: self
                .working_dir
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from),
            timeout_seconds: self.timeout_seconds,
        })
    }
}

/// A JSON-file-backed store for the CLI-bridge config. Disk-agnostic: the path is
/// derived from the caller-supplied `app_data_root`, never hardcoded.
#[derive(Clone, Debug)]
pub struct CliBridgeConfigStore {
    path: PathBuf,
}

impl CliBridgeConfigStore {
    /// Bind the store to `<app_data_root>/cli_bridge_config.json`.
    pub fn new(app_data_root: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_root.as_ref().join(CLI_BRIDGE_CONFIG_FILE),
        }
    }

    /// Bind the store to an explicit file path (tests / alternate wirings).
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the persisted config. A missing file is NOT an error — it yields the
    /// default (unconfigured) document (first run). A present-but-corrupt file
    /// returns an error so the caller can surface it rather than silently
    /// dropping the operator's config.
    pub fn load(&self) -> Result<CliBridgeConfigDoc, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice::<CliBridgeConfigDoc>(&bytes).map_err(|error| {
                format!(
                    "cli bridge config store at {} is corrupt: {error}",
                    self.path.display()
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(CliBridgeConfigDoc::default())
            }
            Err(error) => Err(format!(
                "failed to read cli bridge config store at {}: {error}",
                self.path.display()
            )),
        }
    }

    /// Persist the document atomically (write a temp file in the same directory,
    /// then rename over the target) so a crash mid-write cannot corrupt the live
    /// config. Creates the parent directory if needed.
    pub fn save(&self, doc: &CliBridgeConfigDoc) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create cli bridge config store dir {}: {error}",
                    parent.display()
                )
            })?;
        }
        let json = serde_json::to_vec_pretty(doc)
            .map_err(|error| format!("failed to serialize cli bridge config: {error}"))?;
        // Unique temp name in the same dir so the final rename is atomic on the
        // same filesystem.
        let tmp = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, &json).map_err(|error| {
            format!(
                "failed to write temp cli bridge config store {}: {error}",
                tmp.display()
            )
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|error| {
            // Best-effort cleanup of the temp file on rename failure.
            let _ = std::fs::remove_file(&tmp);
            format!(
                "failed to commit cli bridge config store {}: {error}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured_doc() -> CliBridgeConfigDoc {
        CliBridgeConfigDoc {
            schema_version: CLI_BRIDGE_CONFIG_SCHEMA_VERSION,
            configured: true,
            cli_kind: StoredCliKind::ClaudeCode,
            executable_path: "claude".to_string(),
            args_template: vec![
                "-p".to_string(),
                "{prompt}".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "--output-format".to_string(),
                "text".to_string(),
            ],
            output_format: StoredOutputFormat::RawText,
            model_allowlist: vec!["sonnet".to_string(), "opus".to_string()],
            working_dir: None,
            timeout_seconds: 120,
            env_vars: BTreeMap::new(),
            updated_at_utc: Some("2026-05-31T00:00:00Z".to_string()),
        }
    }

    #[test]
    fn missing_file_yields_unconfigured_default() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = CliBridgeConfigStore::new(tmp.path());
        let loaded = store.load().expect("load default");
        assert!(!loaded.configured);
        assert_eq!(loaded.schema_version, CLI_BRIDGE_CONFIG_SCHEMA_VERSION);
        assert!(loaded.args_template.is_empty());
    }

    #[test]
    fn store_round_trips_through_a_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = CliBridgeConfigStore::new(tmp.path());
        let doc = configured_doc();
        store.save(&doc).expect("save");
        assert!(store.path().exists(), "real file written to disk");
        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded, doc);
        assert!(reloaded.configured);
        assert_eq!(reloaded.executable_path, "claude");
    }

    #[test]
    fn corrupt_file_surfaces_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = CliBridgeConfigStore::new(tmp.path());
        std::fs::write(store.path(), b"{not valid json").expect("write garbage");
        let err = store.load().expect_err("corrupt must error");
        assert!(err.contains("corrupt"), "{err}");
    }

    #[test]
    fn to_cli_bridge_config_maps_kind_and_format() {
        let doc = configured_doc();
        let cfg = doc.to_cli_bridge_config().expect("convert");
        assert_eq!(cfg.cli_kind, CliKind::ClaudeCode);
        assert_eq!(cfg.output_format, CliOutputFormat::RawText);
        assert_eq!(cfg.executable_path, PathBuf::from("claude"));
        assert_eq!(cfg.timeout_seconds, 120);
        assert!(cfg.args_template.iter().any(|a| a.contains("{prompt}")));
    }

    #[test]
    fn to_cli_bridge_config_rejects_unconfigured_doc() {
        let doc = CliBridgeConfigDoc::default();
        let err = doc.to_cli_bridge_config().expect_err("unconfigured");
        assert!(err.contains("not configured"), "{err}");
    }

    #[test]
    fn stored_cli_kind_round_trips_core() {
        for kind in [
            CliKind::ClaudeCode,
            CliKind::CodexCli,
            CliKind::GeminiCli,
            CliKind::Other,
        ] {
            assert_eq!(StoredCliKind::from_core(kind).to_core(), kind);
        }
    }

    #[test]
    fn stored_output_format_round_trips_core() {
        for fmt in [
            CliOutputFormat::Json,
            CliOutputFormat::RawText,
            CliOutputFormat::JsonStream,
        ] {
            assert_eq!(StoredOutputFormat::from_core(fmt).to_core(), fmt);
        }
    }

    #[test]
    fn doc_serialises_camel_case() {
        let doc = configured_doc();
        let val = serde_json::to_value(&doc).expect("ser");
        assert!(val.get("schemaVersion").is_some());
        assert!(val.get("cliKind").is_some());
        assert!(val.get("executablePath").is_some());
        assert!(val.get("argsTemplate").is_some());
        assert!(val.get("modelAllowlist").is_some());
        assert!(val.get("timeoutSeconds").is_some());
        assert!(val.get("schema_version").is_none());
    }

    #[test]
    fn stored_cli_kind_serialises_snake_case() {
        let val = serde_json::to_value(StoredCliKind::ClaudeCode).expect("ser");
        assert_eq!(val, serde_json::json!("claude_code"));
        let val = serde_json::to_value(StoredCliKind::CodexCli).expect("ser");
        assert_eq!(val, serde_json::json!("codex_cli"));
    }

    #[test]
    fn working_dir_blank_string_maps_to_none() {
        let mut doc = configured_doc();
        doc.working_dir = Some("   ".to_string());
        let cfg = doc.to_cli_bridge_config().expect("convert");
        assert!(cfg.working_dir.is_none(), "blank working_dir -> None");
    }
}
