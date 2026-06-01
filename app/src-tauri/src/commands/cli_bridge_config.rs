//! Tauri IPC surface for the Official-CLI bridge operator config (WP-KERNEL-004
//! follow-up).
//!
//! Gives the operator a settings-menu config surface to point the official-CLI
//! swarm lane at a real CLI (Claude Code / Codex CLI / a Generic any-CLI) so a
//! CLI-backed swarm session actually runs in the app and its stdout streams into
//! the in-app terminal panel. The CLI bridge runtime is already complete
//! (`cli_bridge_runtime.rs` / `official_cli_bridge.rs` / `production_factory.rs`,
//! commit b9ae06f3); the only missing piece was the operator config that flips
//! `official_cli` from `None` to `Some(...)`. This module is that config surface.
//!
//! Mirrors `cloud_lane.rs` shape: channel consts, thin `#[tauri::command]`
//! wrappers delegating to pure `*_impl` fns (testable without a Tauri runtime),
//! a typed `thiserror` error mapped to a `String` for the IPC boundary, and a
//! single Tauri-managed state holding the disk-backed store + an in-memory doc
//! guard.
//!
//! CAPABILITY/SECRET POSTURE: NO API key is stored. The CLI authenticates via the
//! operator's own CLI login. The config is executable path + args template +
//! model allowlist + timeout + (optional) non-secret env vars. The
//! [`test_config`] command runs a REAL `<executable> --version` preflight so a
//! missing/broken CLI is surfaced honestly rather than mocked.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, RwLock};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use thiserror::Error;

use super::cli_bridge_store::{
    CliBridgeConfigDoc, CliBridgeConfigStore, StoredCliKind, StoredOutputFormat,
};

pub const KERNEL_CLI_BRIDGE_GET_CONFIG_IPC_CHANNEL: &str = "kernel_cli_bridge_get_config";
pub const KERNEL_CLI_BRIDGE_SET_CONFIG_IPC_CHANNEL: &str = "kernel_cli_bridge_set_config";
pub const KERNEL_CLI_BRIDGE_CLEAR_CONFIG_IPC_CHANNEL: &str = "kernel_cli_bridge_clear_config";
pub const KERNEL_CLI_BRIDGE_LIST_PRESETS_IPC_CHANNEL: &str = "kernel_cli_bridge_list_presets";
pub const KERNEL_CLI_BRIDGE_TEST_CONFIG_IPC_CHANNEL: &str = "kernel_cli_bridge_test_config";

/// Hard timeout for the `test_config` preflight so a hung CLI cannot block the
/// IPC thread.
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(3);

// ---------------------------------------------------------------------------
// IPC projections / payloads (camelCase wire shape, mirrors cloud_lane.rs).
// ---------------------------------------------------------------------------

/// Outbound projection of the stored config. NEVER carries a secret (the bridge
/// stores none). A faithful projection of the persisted doc so the frontend can
/// render the current state + a "configured / not configured" status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliBridgeConfigSummary {
    pub configured: bool,
    pub cli_kind: StoredCliKind,
    pub executable_path: String,
    pub args_template: Vec<String>,
    pub output_format: StoredOutputFormat,
    pub model_allowlist: Vec<String>,
    pub working_dir: Option<String>,
    pub timeout_seconds: u64,
    pub env_var_names: Vec<String>,
    pub updated_at_utc: Option<String>,
}

impl CliBridgeConfigSummary {
    fn from_doc(doc: &CliBridgeConfigDoc) -> Self {
        Self {
            configured: doc.configured,
            cli_kind: doc.cli_kind,
            executable_path: doc.executable_path.clone(),
            args_template: doc.args_template.clone(),
            output_format: doc.output_format,
            model_allowlist: doc.model_allowlist.clone(),
            working_dir: doc.working_dir.clone(),
            timeout_seconds: doc.timeout_seconds,
            env_var_names: doc.env_vars.keys().cloned().collect(),
            updated_at_utc: doc.updated_at_utc.clone(),
        }
    }
}

/// Inbound payload to set the full config (everything but schema_version, which
/// the store owns). `env_vars` is a flat list of key/value pairs so the wire
/// shape stays simple for the frontend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCliBridgeConfigRequest {
    pub cli_kind: StoredCliKind,
    pub executable_path: String,
    pub args_template: Vec<String>,
    pub output_format: StoredOutputFormat,
    pub model_allowlist: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub env_vars: Vec<EnvVarPair>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarPair {
    pub key: String,
    pub value: String,
}

/// A static preset prefilling the operator config for a known CLI. The operator
/// can edit every field after selecting it (prefill-and-edit), so a CLI changing
/// its flags is recoverable without a code change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliBridgePreset {
    pub id: String,
    pub label: String,
    pub cli_kind: StoredCliKind,
    pub executable_hint: String,
    pub args_template: Vec<String>,
    pub output_format: StoredOutputFormat,
    pub model_allowlist: Vec<String>,
    pub default_timeout_seconds: u64,
    /// The `--version`-style preflight argument the `test_config` command uses.
    pub version_arg: String,
    /// OPT-IN structured capture variant. When the operator selects "structured
    /// capture", the UI swaps `args_template` + `output_format` to these so the
    /// CLI bridge parses the JSON event stream into typed agent-activity rows
    /// (tool calls, thinking, text) in the per-session transcript. The HONEST
    /// default above stays `RawText` (raw stdout, byte-faithful) — structured
    /// capture is opt-in. `None` => this CLI has no known structured mode (the
    /// operator can still set one manually).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_args_template: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output_format: Option<StoredOutputFormat>,
}

/// Inbound payload for the real preflight test.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCliBridgeConfigRequest {
    pub executable_path: String,
    /// Override the version arg (defaults to `--version`).
    #[serde(default)]
    pub version_arg: Option<String>,
}

/// Receipt from the real preflight. `ok=true` only when the executable spawned
/// and exited 0; otherwise the real OS error / exit code is in `detail`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliBridgeTestReceipt {
    pub ok: bool,
    pub version_line: Option<String>,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum CliBridgeConfigError {
    #[error("executable_path must not be empty")]
    EmptyExecutable,
    #[error("args_template must contain a {{prompt}} placeholder for prompt substitution")]
    MissingPromptPlaceholder,
    #[error("timeout_seconds must be > 0")]
    InvalidTimeout,
    #[error("model_allowlist must not be empty")]
    EmptyAllowlist,
    #[error("cli bridge config store error: {0}")]
    Store(String),
    #[error("cli bridge config internal lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Tauri-managed state: the disk-backed store + the in-memory doc (seeded from
/// the store at construction). Mirrors `CloudLaneIpcState` lock discipline.
pub struct CliBridgeConfigState {
    store: CliBridgeConfigStore,
    doc: RwLock<CliBridgeConfigDoc>,
}

impl CliBridgeConfigState {
    /// Construct from an `app_data_root`, loading any persisted config. A corrupt
    /// file is surfaced (the in-memory doc falls back to default so the app does
    /// not crash, and `get_config` re-reads the store to re-surface the error).
    pub fn new(app_data_root: impl AsRef<std::path::Path>) -> Self {
        let store = CliBridgeConfigStore::new(app_data_root);
        let doc = store.load().unwrap_or_default();
        Self {
            store,
            doc: RwLock::new(doc),
        }
    }

    /// Bind to an explicit store path (tests / alternate wirings).
    pub fn with_store(store: CliBridgeConfigStore) -> Self {
        let doc = store.load().unwrap_or_default();
        Self {
            store,
            doc: RwLock::new(doc),
        }
    }

    fn doc_read(
        &self,
    ) -> Result<std::sync::RwLockReadGuard<'_, CliBridgeConfigDoc>, CliBridgeConfigError> {
        self.doc
            .read()
            .map_err(|e| CliBridgeConfigError::LockPoisoned(e.to_string()))
    }

    fn doc_write(
        &self,
    ) -> Result<std::sync::RwLockWriteGuard<'_, CliBridgeConfigDoc>, CliBridgeConfigError> {
        self.doc
            .write()
            .map_err(|e| CliBridgeConfigError::LockPoisoned(e.to_string()))
    }
}

fn map_err(err: CliBridgeConfigError) -> String {
    err.to_string()
}

// ---------------------------------------------------------------------------
// Presets.
// ---------------------------------------------------------------------------

/// The static preset list. Each prefills exe hint + args_template (with a real
/// `{prompt}`) + output_format + allowlist + timeout. Flags confirmed against
/// the official CLI docs (current as of 2026-05-31); the operator can override
/// every field after selecting so a flag change is recoverable without a build.
pub fn cli_bridge_presets() -> Vec<CliBridgePreset> {
    vec![
        CliBridgePreset {
            id: "claude_code".to_string(),
            label: "Claude Code".to_string(),
            cli_kind: StoredCliKind::ClaudeCode,
            executable_hint: "claude".to_string(),
            // claude -p "<prompt>" --model <model> --output-format text
            args_template: vec![
                "-p".to_string(),
                "{prompt}".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "--output-format".to_string(),
                "text".to_string(),
            ],
            output_format: StoredOutputFormat::RawText,
            model_allowlist: vec![
                "sonnet".to_string(),
                "opus".to_string(),
                "claude-sonnet-4-6".to_string(),
            ],
            default_timeout_seconds: 120,
            version_arg: "--version".to_string(),
            // Opt-in structured capture: claude -p "<prompt>" --model <model>
            //   --output-format stream-json --verbose
            // (stream-json requires --verbose in headless/-p mode).
            structured_args_template: Some(vec![
                "-p".to_string(),
                "{prompt}".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
            ]),
            structured_output_format: Some(StoredOutputFormat::JsonStream),
        },
        CliBridgePreset {
            id: "codex_cli".to_string(),
            label: "Codex CLI".to_string(),
            cli_kind: StoredCliKind::CodexCli,
            executable_hint: "codex".to_string(),
            // codex exec --skip-git-repo-check --model <model> "<prompt>"
            args_template: vec![
                "exec".to_string(),
                "--skip-git-repo-check".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ],
            output_format: StoredOutputFormat::RawText,
            model_allowlist: vec!["gpt-5.4".to_string(), "gpt-5.3-codex".to_string()],
            default_timeout_seconds: 120,
            version_arg: "--version".to_string(),
            // Opt-in structured capture: codex exec --json … emits JSONL events.
            structured_args_template: Some(vec![
                "exec".to_string(),
                "--json".to_string(),
                "--skip-git-repo-check".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
                "{prompt}".to_string(),
            ]),
            structured_output_format: Some(StoredOutputFormat::JsonStream),
        },
        CliBridgePreset {
            id: "generic".to_string(),
            label: "Generic (any CLI)".to_string(),
            cli_kind: StoredCliKind::Other,
            executable_hint: String::new(),
            // Operator supplies executable + extends the args. {prompt} is the
            // one required element; {model} is optional.
            args_template: vec!["{prompt}".to_string()],
            output_format: StoredOutputFormat::RawText,
            // Empty: the operator supplies the allowlist. set_config requires it
            // to be non-empty, so a Generic config cannot be saved without one.
            model_allowlist: Vec::new(),
            default_timeout_seconds: 120,
            version_arg: "--version".to_string(),
            // Generic CLI has no known structured mode; operator supplies one.
            structured_args_template: None,
            structured_output_format: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// #[tauri::command] thin wrappers.
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn kernel_cli_bridge_get_config(
    state: State<'_, CliBridgeConfigState>,
) -> Result<CliBridgeConfigSummary, String> {
    let _ = KERNEL_CLI_BRIDGE_GET_CONFIG_IPC_CHANNEL;
    get_config_impl(state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn kernel_cli_bridge_set_config(
    request: SetCliBridgeConfigRequest,
    state: State<'_, CliBridgeConfigState>,
) -> Result<CliBridgeConfigSummary, String> {
    let _ = KERNEL_CLI_BRIDGE_SET_CONFIG_IPC_CHANNEL;
    set_config_impl(request, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn kernel_cli_bridge_clear_config(
    operator_signature: String,
    state: State<'_, CliBridgeConfigState>,
) -> Result<CliBridgeConfigSummary, String> {
    let _ = KERNEL_CLI_BRIDGE_CLEAR_CONFIG_IPC_CHANNEL;
    clear_config_impl(&operator_signature, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn kernel_cli_bridge_list_presets() -> Result<Vec<CliBridgePreset>, String> {
    let _ = KERNEL_CLI_BRIDGE_LIST_PRESETS_IPC_CHANNEL;
    Ok(cli_bridge_presets())
}

#[tauri::command]
pub async fn kernel_cli_bridge_test_config(
    request: TestCliBridgeConfigRequest,
) -> Result<CliBridgeTestReceipt, String> {
    let _ = KERNEL_CLI_BRIDGE_TEST_CONFIG_IPC_CHANNEL;
    Ok(test_config_impl(request))
}

// ---------------------------------------------------------------------------
// Pure-function implementations (testable without a Tauri runtime).
// ---------------------------------------------------------------------------

pub fn get_config_impl(
    state: &CliBridgeConfigState,
) -> Result<CliBridgeConfigSummary, CliBridgeConfigError> {
    // Re-read the store so a corrupt on-disk file surfaces honestly (the
    // in-memory doc may be the default fallback from construction).
    let doc = state
        .store
        .load()
        .map_err(CliBridgeConfigError::Store)?;
    // Keep the in-memory guard in sync with the freshly-read doc.
    {
        let mut guard = state.doc_write()?;
        *guard = doc.clone();
    }
    Ok(CliBridgeConfigSummary::from_doc(&doc))
}

pub fn set_config_impl(
    request: SetCliBridgeConfigRequest,
    state: &CliBridgeConfigState,
) -> Result<CliBridgeConfigSummary, CliBridgeConfigError> {
    let executable_path = request.executable_path.trim().to_string();
    if executable_path.is_empty() {
        return Err(CliBridgeConfigError::EmptyExecutable);
    }
    // Mirror register_bridge validation: args_template MUST contain {prompt}.
    if !request
        .args_template
        .iter()
        .any(|arg| arg.contains("{prompt}"))
    {
        return Err(CliBridgeConfigError::MissingPromptPlaceholder);
    }
    if request.timeout_seconds == 0 {
        return Err(CliBridgeConfigError::InvalidTimeout);
    }
    let model_allowlist: Vec<String> = request
        .model_allowlist
        .into_iter()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    if model_allowlist.is_empty() {
        return Err(CliBridgeConfigError::EmptyAllowlist);
    }
    let env_vars = request
        .env_vars
        .into_iter()
        .filter(|p| !p.key.trim().is_empty())
        .map(|p| (p.key.trim().to_string(), p.value))
        .collect();
    let now = Utc::now().to_rfc3339();
    let doc = CliBridgeConfigDoc {
        schema_version: super::cli_bridge_store::CLI_BRIDGE_CONFIG_SCHEMA_VERSION,
        configured: true,
        cli_kind: request.cli_kind,
        executable_path,
        args_template: request.args_template,
        output_format: request.output_format,
        model_allowlist,
        working_dir: request
            .working_dir
            .filter(|s| !s.trim().is_empty()),
        timeout_seconds: request.timeout_seconds,
        env_vars,
        updated_at_utc: Some(now),
    };
    state
        .store
        .save(&doc)
        .map_err(CliBridgeConfigError::Store)?;
    {
        let mut guard = state.doc_write()?;
        *guard = doc.clone();
    }
    Ok(CliBridgeConfigSummary::from_doc(&doc))
}

pub fn clear_config_impl(
    _operator_signature: &str,
    state: &CliBridgeConfigState,
) -> Result<CliBridgeConfigSummary, CliBridgeConfigError> {
    // Clear == revert to the honest unconfigured default. The official_cli swarm
    // lane reverts to None on the next app start.
    let doc = CliBridgeConfigDoc::default();
    state
        .store
        .save(&doc)
        .map_err(CliBridgeConfigError::Store)?;
    {
        let mut guard = state.doc_write()?;
        *guard = doc.clone();
    }
    Ok(CliBridgeConfigSummary::from_doc(&doc))
}

/// Run a REAL `<executable> <version_arg>` with a hard timeout. This is a genuine
/// resource touch (Spec-Realism Sub-rule 2), not a mock: a missing/broken exe
/// surfaces the real OS error, a working exe surfaces its real version line.
pub fn test_config_impl(request: TestCliBridgeConfigRequest) -> CliBridgeTestReceipt {
    let exe = request.executable_path.trim();
    if exe.is_empty() {
        return CliBridgeTestReceipt {
            ok: false,
            version_line: None,
            detail: "executable_path is empty".to_string(),
        };
    }
    let version_arg = request
        .version_arg
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("--version")
        .to_string();

    let mut cmd = Command::new(PathBuf::from(exe));
    cmd.arg(&version_arg);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    // HBR-QUIET: no console window pops on Windows for the backgrounded preflight.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return CliBridgeTestReceipt {
                ok: false,
                version_line: None,
                detail: format!("failed to spawn '{exe}': {err}"),
            };
        }
    };

    // Bounded wait so a hung CLI cannot block the IPC thread (mirrors
    // wait_with_output_bounded in official_cli_bridge.rs).
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(PREFLIGHT_TIMEOUT) {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version_line = stdout
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(str::to_string);
            if output.status.success() {
                CliBridgeTestReceipt {
                    ok: true,
                    version_line: version_line.clone(),
                    detail: version_line
                        .unwrap_or_else(|| "exit 0 (no version line on stdout)".to_string()),
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string());
                CliBridgeTestReceipt {
                    ok: false,
                    version_line,
                    detail: format!(
                        "'{exe} {version_arg}' exited {code}: {}",
                        stderr.trim()
                    ),
                }
            }
        }
        Ok(Err(err)) => CliBridgeTestReceipt {
            ok: false,
            version_line: None,
            detail: format!("failed to wait on '{exe}': {err}"),
        },
        Err(_) => CliBridgeTestReceipt {
            ok: false,
            version_line: None,
            detail: format!(
                "'{exe} {version_arg}' timed out after {}s",
                PREFLIGHT_TIMEOUT.as_secs()
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> CliBridgeConfigState {
        let tmp = tempfile::tempdir().expect("tempdir");
        // Leak the tempdir so the path stays valid for the state's lifetime in
        // the test (the OS reclaims it at process exit).
        let path = tmp.path().join("cli_bridge_config.json");
        std::mem::forget(tmp);
        CliBridgeConfigState::with_store(CliBridgeConfigStore::with_path(path))
    }

    fn good_request() -> SetCliBridgeConfigRequest {
        SetCliBridgeConfigRequest {
            cli_kind: StoredCliKind::ClaudeCode,
            executable_path: "claude".to_string(),
            args_template: vec![
                "-p".to_string(),
                "{prompt}".to_string(),
                "--model".to_string(),
                "{model}".to_string(),
            ],
            output_format: StoredOutputFormat::RawText,
            model_allowlist: vec!["sonnet".to_string()],
            working_dir: None,
            timeout_seconds: 120,
            env_vars: Vec::new(),
        }
    }

    #[test]
    fn fresh_state_is_unconfigured() {
        let state = make_state();
        let summary = get_config_impl(&state).expect("get");
        assert!(!summary.configured);
        assert!(summary.args_template.is_empty());
    }

    #[test]
    fn set_then_get_round_trips_through_store() {
        let state = make_state();
        let summary = set_config_impl(good_request(), &state).expect("set");
        assert!(summary.configured);
        assert_eq!(summary.executable_path, "claude");
        assert!(summary.updated_at_utc.is_some());

        let reloaded = get_config_impl(&state).expect("get");
        assert!(reloaded.configured);
        assert_eq!(reloaded.model_allowlist, vec!["sonnet".to_string()]);
    }

    #[test]
    fn set_rejects_missing_prompt_placeholder() {
        let state = make_state();
        let mut req = good_request();
        req.args_template = vec!["--model".to_string(), "{model}".to_string()];
        let err = set_config_impl(req, &state).expect_err("missing prompt");
        assert!(matches!(
            err,
            CliBridgeConfigError::MissingPromptPlaceholder
        ));
    }

    #[test]
    fn set_rejects_empty_executable() {
        let state = make_state();
        let mut req = good_request();
        req.executable_path = "   ".to_string();
        let err = set_config_impl(req, &state).expect_err("empty exe");
        assert!(matches!(err, CliBridgeConfigError::EmptyExecutable));
    }

    #[test]
    fn set_rejects_zero_timeout() {
        let state = make_state();
        let mut req = good_request();
        req.timeout_seconds = 0;
        let err = set_config_impl(req, &state).expect_err("zero timeout");
        assert!(matches!(err, CliBridgeConfigError::InvalidTimeout));
    }

    #[test]
    fn set_rejects_empty_allowlist() {
        let state = make_state();
        let mut req = good_request();
        req.model_allowlist = vec!["   ".to_string()];
        let err = set_config_impl(req, &state).expect_err("empty allowlist");
        assert!(matches!(err, CliBridgeConfigError::EmptyAllowlist));
    }

    #[test]
    fn clear_reverts_to_unconfigured() {
        let state = make_state();
        set_config_impl(good_request(), &state).expect("set");
        let cleared = clear_config_impl("ilja", &state).expect("clear");
        assert!(!cleared.configured);
        let reloaded = get_config_impl(&state).expect("get");
        assert!(!reloaded.configured);
    }

    #[test]
    fn presets_list_has_three_with_prompt_placeholders() {
        let presets = cli_bridge_presets();
        assert_eq!(presets.len(), 3);
        for preset in &presets {
            assert!(
                preset.args_template.iter().any(|a| a.contains("{prompt}")),
                "preset {} must carry {{prompt}}",
                preset.id
            );
        }
        let claude = presets.iter().find(|p| p.id == "claude_code").expect("claude");
        assert_eq!(claude.cli_kind, StoredCliKind::ClaudeCode);
        assert!(!claude.model_allowlist.is_empty());
        let codex = presets.iter().find(|p| p.id == "codex_cli").expect("codex");
        assert!(codex.args_template.iter().any(|a| a == "exec"));
        let generic = presets.iter().find(|p| p.id == "generic").expect("generic");
        assert!(generic.model_allowlist.is_empty(), "generic allowlist operator-supplied");
    }

    #[test]
    fn test_config_surfaces_missing_executable_honestly() {
        let receipt = test_config_impl(TestCliBridgeConfigRequest {
            executable_path: "definitely-not-a-real-cli-xyz-12345".to_string(),
            version_arg: None,
        });
        assert!(!receipt.ok, "bogus exe must fail");
        assert!(
            receipt.detail.to_lowercase().contains("spawn")
                || receipt.detail.to_lowercase().contains("not")
                || receipt.detail.to_lowercase().contains("cannot")
                || receipt.detail.to_lowercase().contains("no such")
                || receipt.detail.to_lowercase().contains("found"),
            "detail should carry the real OS error: {}",
            receipt.detail
        );
    }

    #[test]
    fn test_config_against_a_real_executable_spawns() {
        // Use a real OS executable + a lone flag that the binary accepts, so the
        // preflight genuinely spawns. On Windows `where /?` prints usage to
        // stdout and exits cleanly; elsewhere `env --version` spawns + exits 0.
        // The point is the REAL spawn path executes (not a mock).
        #[cfg(windows)]
        let (exe, arg) = ("where", "/?");
        #[cfg(not(windows))]
        let (exe, arg) = ("env", "--version");

        let receipt = test_config_impl(TestCliBridgeConfigRequest {
            executable_path: exe.to_string(),
            version_arg: Some(arg.to_string()),
        });
        // The spawn itself must succeed (detail must not be a spawn failure).
        assert!(
            !receipt.detail.contains("failed to spawn"),
            "real exe must spawn, got: {}",
            receipt.detail
        );
    }

    #[test]
    fn summary_serialises_camel_case() {
        let state = make_state();
        let summary = set_config_impl(good_request(), &state).expect("set");
        let val = serde_json::to_value(&summary).expect("ser");
        assert!(val.get("cliKind").is_some());
        assert!(val.get("executablePath").is_some());
        assert!(val.get("argsTemplate").is_some());
        assert!(val.get("modelAllowlist").is_some());
        assert!(val.get("timeoutSeconds").is_some());
        assert!(val.get("envVarNames").is_some());
        assert!(val.get("cli_kind").is_none());
    }

    #[test]
    fn preset_serialises_camel_case() {
        let presets = cli_bridge_presets();
        let val = serde_json::to_value(&presets[0]).expect("ser");
        assert!(val.get("executableHint").is_some());
        assert!(val.get("argsTemplate").is_some());
        assert!(val.get("defaultTimeoutSeconds").is_some());
        assert!(val.get("versionArg").is_some());
    }
}
