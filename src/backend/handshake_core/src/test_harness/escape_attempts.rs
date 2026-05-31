//! MT-058 — escape-attempt negative-test catalog + harness.
//!
//! Sandbox abstractions only have safety value if their promises are
//! load-bearing under attack. This module defines the canonical
//! escape-attempt catalog (10 entries from the MT-058 contract) plus
//! a `SandboxEscapeHarness` that runs each attempt against every
//! registered adapter, classifies the outcome (Green / Red / Yellow /
//! Skipped), and produces a `SandboxEscapeReport` durable to JSON
//! under `../Handshake_Artifacts/` for Integration Validator review.
//!
//! Outcome legend:
//!   - Green   adapter denied the escape (the intended behavior)
//!   - Red     adapter allowed the escape — blocks WP integration
//!   - Yellow  adapter behavior matches a documented weaker-enforcement
//!             marker for that adapter (e.g., Docker without
//!             `--userns=keep-id` shows UID 0; recorded per RW-1
//!             scoring rather than treated as a test failure)
//!   - Skipped adapter is unavailable in this environment, the attempt
//!             is not OS-applicable, or fixture setup failed; the
//!             validator reads `skipped_adapters` to spot coverage gaps

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::sandbox::adapter::SandboxAdapter;
use crate::sandbox::adapter::TrustClass;
use crate::sandbox::types::{
    AdapterId, BindMode, BindSpec, ImageRef, NetPolicy, ProcessSpec, ProcessStatus, ResourceLimits,
    Signal,
};
use std::collections::{BTreeMap as Map, BTreeSet as Set};

/// Canonical escape-attempt id, kebab-case for stability in JSON
/// reports + governance citation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EscapeAttemptId {
    EscFsReadOutOfBind,
    EscFsWriteOutOfBind,
    EscFsSymlinkTraversal,
    EscNetDenyAllDns,
    EscNetDenyAllTcp,
    EscNetLoopbackOnlyExternal,
    EscPrivEscalateUid,
    EscNamespacePid,
    EscDeviceAccess,
    EscWin32ForegroundInject,
}

impl EscapeAttemptId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EscFsReadOutOfBind => "ESC-FS-READ-OUT-OF-BIND",
            Self::EscFsWriteOutOfBind => "ESC-FS-WRITE-OUT-OF-BIND",
            Self::EscFsSymlinkTraversal => "ESC-FS-SYMLINK-TRAVERSAL",
            Self::EscNetDenyAllDns => "ESC-NET-DENY-ALL-DNS",
            Self::EscNetDenyAllTcp => "ESC-NET-DENY-ALL-TCP",
            Self::EscNetLoopbackOnlyExternal => "ESC-NET-LOOPBACK-ONLY-EXTERNAL",
            Self::EscPrivEscalateUid => "ESC-PRIV-ESCALATE-UID",
            Self::EscNamespacePid => "ESC-NAMESPACE-PID",
            Self::EscDeviceAccess => "ESC-DEVICE-ACCESS",
            Self::EscWin32ForegroundInject => "ESC-WIN32-FOREGROUND-INJECT",
        }
    }

    pub fn all() -> [EscapeAttemptId; 10] {
        [
            Self::EscFsReadOutOfBind,
            Self::EscFsWriteOutOfBind,
            Self::EscFsSymlinkTraversal,
            Self::EscNetDenyAllDns,
            Self::EscNetDenyAllTcp,
            Self::EscNetLoopbackOnlyExternal,
            Self::EscPrivEscalateUid,
            Self::EscNamespacePid,
            Self::EscDeviceAccess,
            Self::EscWin32ForegroundInject,
        ]
    }
}

/// One escape attempt: spawned ProcessSpec + how to interpret the exit.
#[derive(Clone, Debug)]
pub struct EscapeAttempt {
    pub id: EscapeAttemptId,
    pub description: &'static str,
    pub spec: ProcessSpec,
    /// `Some(true)` requires `exit_code == 0` for Green (e.g.,
    /// "id -u; assert NOT 0" — the script itself returns 0 only when
    /// the privilege check passed). `Some(false)` requires `exit_code != 0`
    /// for Green (e.g., DNS lookup must fail). `None` means: any
    /// exit_code is acceptable as long as the side-effect was denied
    /// (the spec's script encodes the denial itself).
    pub green_when_exit_is_zero: Option<bool>,
    /// When the adapter is known to provide weaker enforcement on
    /// this axis (e.g., Docker without `--userns=keep-id` for
    /// ESC-PRIV-ESCALATE-UID), the harness records Yellow instead
    /// of Red for that (adapter, attempt) pair.
    pub documented_weaker_enforcement_adapters: Vec<&'static str>,
    /// `Some(target_os)` restricts the attempt to a specific OS:
    /// "windows" for Win32-only attempts, "linux" for POSIX-only.
    /// `None` runs on any OS.
    pub os_restriction: Option<&'static str>,
}

/// Environment variable the integration-test driver sets to the absolute
/// path of the built `handshake-foreground-inject-probe` binary.
///
/// Cargo exposes that path to the test crate as
/// `CARGO_BIN_EXE_handshake-foreground-inject-probe` (only available inside
/// integration tests/examples/benches, not in the library crate where this
/// catalog lives). The driver forwards it through this env var so the catalog
/// can embed an ABSOLUTE path into `cmd[0]`. The `WindowsNativeJailAdapter`'s
/// `resolve_executable` takes the `is_absolute()` fast-path for an absolute
/// command, so the probe resolves deterministically to the cargo-built artifact
/// instead of relying on a fragile `which::which` PATH lookup.
pub const FOREGROUND_INJECT_PROBE_ENV: &str = "HANDSHAKE_FOREGROUND_INJECT_PROBE";

/// Default bare command name when the env override is absent. Kept so the
/// catalog is well-formed on hosts where the probe was not built; on such
/// hosts the Win32 attempt is OS-restricted and Skipped before spawn anyway.
const FOREGROUND_INJECT_PROBE_DEFAULT: &str = "handshake-foreground-inject-probe.exe";

/// Resolve the foreground-inject probe command. Prefers the absolute path the
/// test driver injects via [`FOREGROUND_INJECT_PROBE_ENV`]; falls back to the
/// bare exe name otherwise.
fn foreground_inject_probe_cmd() -> String {
    std::env::var(FOREGROUND_INJECT_PROBE_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| FOREGROUND_INJECT_PROBE_DEFAULT.to_string())
}

/// The catalog. Stable across MT-058 — additions belong to a future
/// MT, not to silently extend this one.
pub fn escape_catalog() -> Vec<EscapeAttempt> {
    let alpine = ImageRef::new("alpine:3.20");
    let bind_workspace = vec![BindSpec {
        host_path: std::path::PathBuf::from("/tmp"),
        guest_path: std::path::PathBuf::from("/workspace/allowed"),
        mode: BindMode::ReadWrite,
    }];
    let empty_resources = ResourceLimits::default();

    vec![
        EscapeAttempt {
            id: EscapeAttemptId::EscFsReadOutOfBind,
            description: "Read /etc/shadow from inside sandbox; expect ACCESS_DENIED or absence.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "cat /etc/shadow && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscFsWriteOutOfBind,
            description: "Write to /workspace/forbidden_dir/x (outside the allowed bind); expect EROFS or ENOENT.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "touch /workspace/forbidden_dir/x && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscFsSymlinkTraversal,
            description: "Create a symlink /workspace/allowed/leak -> /etc/passwd and read through it; expect the symlink to resolve to nothing escaping the bind.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "ln -sf /etc/passwd /workspace/allowed/leak && cat /workspace/allowed/leak 2>/dev/null | grep -q root && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec!["docker", "wsl2_podman"],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscNetDenyAllDns,
            description: "DNS lookup under NetPolicy::DenyAll; non-zero exit means denial.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "getent hosts cnn.com && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscNetDenyAllTcp,
            description: "Direct TCP /dev/tcp to 8.8.8.8:53 under DenyAll; non-zero exit means denial.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "(exec 3<>/dev/tcp/8.8.8.8/53; echo > &3) && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscNetLoopbackOnlyExternal,
            description: "Outbound to 8.8.8.8:53 under LoopbackOnly; loopback allowed, external denied.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "(exec 3<>/dev/tcp/8.8.8.8/53; echo > &3) && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::LoopbackOnly,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscPrivEscalateUid,
            description: "Assert running UID is NOT 0 AND chown 0:0 fails. Green when both checks pass.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "u=$(id -u); [ \"$u\" != \"0\" ] && touch /workspace/allowed/x && (chown 0:0 /workspace/allowed/x 2>/dev/null && exit 1 || exit 0) || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(true),
            documented_weaker_enforcement_adapters: vec!["docker"],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscNamespacePid,
            description: "Read /proc/1/status — visible PID 1 must be container init, NOT a host operator process.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "cat /proc/1/status | grep -E '^Name:[[:space:]]+(pause|tini|catatonit|sh|init|conmon|sleep|true|app)' && exit 0 || exit 1".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(true),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscDeviceAccess,
            description: "List /dev — assert host block devices (sda, nvme0n1) are NOT visible.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: alpine.clone(),
                cmd: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "ls /dev | grep -E '^(sda|nvme0n1|vda|xvda)' && exit 1 || exit 0".to_string(),
                ],
                env: Map::new(),
                cwd: None,
                binds: bind_workspace.clone(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources.clone(),
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(true),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("linux"),
        },
        EscapeAttempt {
            id: EscapeAttemptId::EscWin32ForegroundInject,
            description: "HBR-QUIET-001 acid test — SetForegroundWindow from inside WindowsNativeJailAdapter must fail (ERROR_ACCESS_DENIED) and produce no foreground transition in the focus-audit ledger. Restricted to windows + win-native-integration feature.",
            spec: ProcessSpec {
                id: AdapterId::new("placeholder"),
                image_or_root: ImageRef::new("windows:nativejail"),
                cmd: vec![foreground_inject_probe_cmd()],
                env: Map::new(),
                cwd: None,
                binds: vec![],
                net_policy: NetPolicy::DenyAll,
                resource_limits: empty_resources,
                required_capabilities: Set::new(),
                trust_class: TrustClass::default(),
                metadata: Map::new(),
            },
            green_when_exit_is_zero: Some(false),
            documented_weaker_enforcement_adapters: vec![],
            os_restriction: Some("windows"),
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscapeVerdict {
    Green,
    Red,
    Yellow { weaker_enforcement_note: String },
    Skipped { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct EscapeAttemptResult {
    pub attempt_id: String,
    pub adapter_id: String,
    pub verdict: EscapeVerdict,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SandboxEscapeReport {
    pub schema_id: String,
    pub run_id: String,
    pub generated_at_utc: String,
    pub rows: Vec<EscapeAttemptResult>,
    pub skipped_adapters: Vec<String>,
}

impl SandboxEscapeReport {
    pub fn red_attempts(&self) -> impl Iterator<Item = &EscapeAttemptResult> {
        self.rows
            .iter()
            .filter(|row| matches!(row.verdict, EscapeVerdict::Red))
    }

    pub fn has_any_red(&self) -> bool {
        self.red_attempts().next().is_some()
    }

    pub fn green_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row.verdict, EscapeVerdict::Green))
            .count()
    }

    pub fn yellow_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row.verdict, EscapeVerdict::Yellow { .. }))
            .count()
    }

    pub fn skipped_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row.verdict, EscapeVerdict::Skipped { .. }))
            .count()
    }

    /// Write the report as JSON under the operator's artifact root.
    /// Returns the absolute path so the validator can cite it.
    pub fn persist_to_artifacts(
        &self,
        artifact_root: &std::path::Path,
    ) -> std::io::Result<std::path::PathBuf> {
        std::fs::create_dir_all(artifact_root)?;
        let file_name = format!("sandbox-escape-results-{}.json", self.run_id);
        let path = artifact_root.join(file_name);
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;
        std::fs::write(&path, json)?;
        Ok(path)
    }
}

pub enum EscapeAdapterSlot {
    Available(Arc<dyn SandboxAdapter>),
    Unavailable { reason: String },
}

pub struct SandboxEscapeHarness {
    adapters: BTreeMap<AdapterId, EscapeAdapterSlot>,
    target_os: &'static str,
}

impl SandboxEscapeHarness {
    pub fn new(target_os: &'static str) -> Self {
        Self {
            adapters: BTreeMap::new(),
            target_os,
        }
    }

    pub fn with_adapter(mut self, id: AdapterId, slot: EscapeAdapterSlot) -> Self {
        self.adapters.insert(id, slot);
        self
    }

    pub async fn run(&self, attempts: &[EscapeAttempt]) -> SandboxEscapeReport {
        let run_id = uuid::Uuid::now_v7().to_string();
        let mut report = SandboxEscapeReport {
            schema_id: "hsk.sandbox_escape_report@1".to_string(),
            run_id,
            generated_at_utc: chrono::Utc::now().to_rfc3339(),
            rows: Vec::new(),
            skipped_adapters: Vec::new(),
        };

        let mut skipped_adapters: BTreeSet<AdapterId> = BTreeSet::new();
        for (adapter_id, slot) in &self.adapters {
            let adapter = match slot {
                EscapeAdapterSlot::Available(adapter) => adapter.clone(),
                EscapeAdapterSlot::Unavailable { reason } => {
                    skipped_adapters.insert(adapter_id.clone());
                    for attempt in attempts {
                        report.rows.push(EscapeAttemptResult {
                            attempt_id: attempt.id.as_str().to_string(),
                            adapter_id: adapter_id.as_str().to_string(),
                            verdict: EscapeVerdict::Skipped {
                                reason: format!(
                                    "adapter {} unavailable: {}",
                                    adapter_id.as_str(),
                                    reason
                                ),
                            },
                            exit_code: None,
                        });
                    }
                    continue;
                }
            };

            for attempt in attempts {
                if let Some(os) = attempt.os_restriction {
                    if os != self.target_os {
                        report.rows.push(EscapeAttemptResult {
                            attempt_id: attempt.id.as_str().to_string(),
                            adapter_id: adapter_id.as_str().to_string(),
                            verdict: EscapeVerdict::Skipped {
                                reason: format!(
                                    "attempt restricted to os={os}; harness target_os={}",
                                    self.target_os
                                ),
                            },
                            exit_code: None,
                        });
                        continue;
                    }
                }

                let outcome = run_single(adapter.clone(), adapter_id.clone(), attempt).await;
                report.rows.push(outcome);
            }
        }
        report.skipped_adapters = skipped_adapters
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect();
        report
    }
}

async fn run_single(
    adapter: Arc<dyn SandboxAdapter>,
    adapter_id: AdapterId,
    attempt: &EscapeAttempt,
) -> EscapeAttemptResult {
    let mut spec = attempt.spec.clone();
    spec.id = adapter_id.clone();

    let handle = match adapter.spawn(spec).await {
        Ok(handle) => handle,
        Err(error) => {
            return EscapeAttemptResult {
                attempt_id: attempt.id.as_str().to_string(),
                adapter_id: adapter_id.as_str().to_string(),
                verdict: EscapeVerdict::Skipped {
                    reason: format!("spawn failed: {error}"),
                },
                exit_code: None,
            };
        }
    };

    let natural_timeout = Duration::from_millis(30_000);
    let started = std::time::Instant::now();
    let _terminal_status = loop {
        match adapter.status(&handle).await {
            Ok(status @ (ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. })) => {
                break status;
            }
            Ok(_) if started.elapsed() < natural_timeout => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok(_) => {
                // Force-kill to free the sandbox; record skip.
                let _ = adapter.kill(&handle, Signal::Kill).await;
                return EscapeAttemptResult {
                    attempt_id: attempt.id.as_str().to_string(),
                    adapter_id: adapter_id.as_str().to_string(),
                    verdict: EscapeVerdict::Skipped {
                        reason: "process did not terminate within 30s; forcibly killed".to_string(),
                    },
                    exit_code: None,
                };
            }
            Err(error) => {
                return EscapeAttemptResult {
                    attempt_id: attempt.id.as_str().to_string(),
                    adapter_id: adapter_id.as_str().to_string(),
                    verdict: EscapeVerdict::Skipped {
                        reason: format!("status poll failed: {error}"),
                    },
                    exit_code: None,
                };
            }
        }
    };

    let exit_code = match adapter.exit_code(&handle).await {
        Ok(code) => code,
        Err(error) => {
            return EscapeAttemptResult {
                attempt_id: attempt.id.as_str().to_string(),
                adapter_id: adapter_id.as_str().to_string(),
                verdict: EscapeVerdict::Skipped {
                    reason: format!("exit_code() failed: {error}"),
                },
                exit_code: None,
            };
        }
    };

    let verdict = classify_outcome(attempt, &adapter_id, exit_code);
    EscapeAttemptResult {
        attempt_id: attempt.id.as_str().to_string(),
        adapter_id: adapter_id.as_str().to_string(),
        verdict,
        exit_code,
    }
}

fn classify_outcome(
    attempt: &EscapeAttempt,
    adapter_id: &AdapterId,
    exit_code: Option<i32>,
) -> EscapeVerdict {
    let is_green = match (attempt.green_when_exit_is_zero, exit_code) {
        (Some(true), Some(0)) => true,
        (Some(true), Some(_)) => false,
        (Some(false), Some(0)) => false,
        (Some(false), Some(_)) => true,
        (None, _) => true,
        (_, None) => false,
    };
    if is_green {
        return EscapeVerdict::Green;
    }
    if attempt
        .documented_weaker_enforcement_adapters
        .iter()
        .any(|note_adapter| *note_adapter == adapter_id.as_str())
    {
        return EscapeVerdict::Yellow {
            weaker_enforcement_note: format!(
                "{} documented weaker enforcement on {}",
                adapter_id.as_str(),
                attempt.id.as_str()
            ),
        };
    }
    EscapeVerdict::Red
}
