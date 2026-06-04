use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub const PROCESS_LEDGER_TABLE_NAME: &str = "kernel_process_lifecycle";
pub const PROCESS_LEDGER_RING_CAPACITY: usize = 10_000;
pub const PROCESS_LEDGER_BATCH_SIZE: usize = 100;
pub const PROCESS_LEDGER_FLUSH_INTERVAL_MS: u64 = 250;
pub const PROCESS_LEDGER_METADATA_CAP_BYTES: usize = 16 * 1024;
pub const PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY: usize = PROCESS_LEDGER_RING_CAPACITY;
pub const PROCESS_LEDGER_DEFAULT_BATCH_SIZE: usize = PROCESS_LEDGER_BATCH_SIZE;
pub const PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS: u64 = PROCESS_LEDGER_FLUSH_INTERVAL_MS;
pub const PROCESS_LEDGER_MIGRATION_SQL: &str =
    include_str!("../../migrations/0021_kernel_process_lifecycle.sql");

pub const PROCESS_START_INSERT_SQL: &str = r#"
INSERT INTO kernel_process_lifecycle (
    process_uuid,
    os_pid,
    parent_session_id,
    parent_process_id,
    sandbox_adapter_id,
    sandbox_internal_id,
    engine_kind,
    started_at,
    model_artifact_sha256,
    work_profile_id,
    owner_role,
    owner_wp,
    role_id,
    wp_id,
    mt_id,
    sandbox_capabilities_snapshot,
    metadata_jsonb
) VALUES (
    $1::uuid,
    $2,
    $3,
    $4::uuid,
    $5,
    $6,
    $7,
    $8,
    $9,
    $10,
    $11,
    $12,
    $13,
    $14,
    $15,
    $16::jsonb,
    $17::jsonb
)
ON CONFLICT (process_uuid) DO UPDATE SET
    os_pid = COALESCE(EXCLUDED.os_pid, kernel_process_lifecycle.os_pid),
    parent_session_id = COALESCE(EXCLUDED.parent_session_id, kernel_process_lifecycle.parent_session_id),
    parent_process_id = COALESCE(EXCLUDED.parent_process_id, kernel_process_lifecycle.parent_process_id),
    sandbox_adapter_id = COALESCE(EXCLUDED.sandbox_adapter_id, kernel_process_lifecycle.sandbox_adapter_id),
    sandbox_internal_id = COALESCE(EXCLUDED.sandbox_internal_id, kernel_process_lifecycle.sandbox_internal_id),
    engine_kind = EXCLUDED.engine_kind,
    started_at = LEAST(kernel_process_lifecycle.started_at, EXCLUDED.started_at),
    model_artifact_sha256 = COALESCE(EXCLUDED.model_artifact_sha256, kernel_process_lifecycle.model_artifact_sha256),
    work_profile_id = COALESCE(EXCLUDED.work_profile_id, kernel_process_lifecycle.work_profile_id),
    owner_role = EXCLUDED.owner_role,
    owner_wp = COALESCE(EXCLUDED.owner_wp, kernel_process_lifecycle.owner_wp),
    role_id = COALESCE(EXCLUDED.role_id, kernel_process_lifecycle.role_id),
    wp_id = COALESCE(EXCLUDED.wp_id, kernel_process_lifecycle.wp_id),
    mt_id = COALESCE(EXCLUDED.mt_id, kernel_process_lifecycle.mt_id),
    sandbox_capabilities_snapshot = EXCLUDED.sandbox_capabilities_snapshot,
    metadata_jsonb = EXCLUDED.metadata_jsonb
"#;

pub const PROCESS_STOP_UPSERT_SQL: &str = r#"
INSERT INTO kernel_process_lifecycle (
    process_uuid,
    os_pid,
    parent_session_id,
    parent_process_id,
    sandbox_adapter_id,
    sandbox_internal_id,
    engine_kind,
    started_at,
    stopped_at,
    exit_code,
    stop_reason,
    model_artifact_sha256,
    work_profile_id,
    owner_role,
    owner_wp,
    role_id,
    wp_id,
    mt_id,
    sandbox_capabilities_snapshot,
    metadata_jsonb
) VALUES (
    $1::uuid,
    $2,
    $3,
    $4::uuid,
    $5,
    $6,
    $7,
    $8,
    $9,
    $10,
    $11,
    $12,
    $13,
    $14,
    $15,
    $16,
    $17,
    $18,
    $19::jsonb,
    $20::jsonb
)
ON CONFLICT (process_uuid) DO UPDATE SET
    os_pid = COALESCE(EXCLUDED.os_pid, kernel_process_lifecycle.os_pid),
    parent_process_id = COALESCE(EXCLUDED.parent_process_id, kernel_process_lifecycle.parent_process_id),
    stopped_at = EXCLUDED.stopped_at,
    exit_code = EXCLUDED.exit_code,
    stop_reason = EXCLUDED.stop_reason,
    sandbox_internal_id = COALESCE(EXCLUDED.sandbox_internal_id, kernel_process_lifecycle.sandbox_internal_id),
    model_artifact_sha256 = COALESCE(EXCLUDED.model_artifact_sha256, kernel_process_lifecycle.model_artifact_sha256),
    work_profile_id = COALESCE(EXCLUDED.work_profile_id, kernel_process_lifecycle.work_profile_id),
    owner_role = EXCLUDED.owner_role,
    owner_wp = COALESCE(EXCLUDED.owner_wp, kernel_process_lifecycle.owner_wp),
    role_id = COALESCE(EXCLUDED.role_id, kernel_process_lifecycle.role_id),
    wp_id = COALESCE(EXCLUDED.wp_id, kernel_process_lifecycle.wp_id),
    mt_id = COALESCE(EXCLUDED.mt_id, kernel_process_lifecycle.mt_id),
    sandbox_capabilities_snapshot = EXCLUDED.sandbox_capabilities_snapshot,
    metadata_jsonb = EXCLUDED.metadata_jsonb
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessEngineKind {
    LlamaCpp,
    Candle,
    AbliterationTool,
    SandboxContainer,
    MechanicalJob,
    AsrWorker,
    ComfyUiWorker,
    PluginProcess,
    HelperSubprocess,
    ExternalCompat,
    Webview2Cdp,
    /// MT-127: cloud-lane Official-CLI bridge subprocess (Claude Code,
    /// Codex CLI, gemini-cli, ...). Spawned via `std::process::Command`
    /// by `LiveCliSpawner`; attributable + reclaimable but NOT a regular
    /// local model runtime engine (no LoRA/KV/steering through a CLI).
    OfficialCliBridge,
}

impl ProcessEngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llamacpp",
            Self::Candle => "candle",
            Self::AbliterationTool => "abliteration_tool",
            Self::SandboxContainer => "sandbox_container",
            Self::MechanicalJob => "mechanical_job",
            Self::AsrWorker => "asr_worker",
            Self::ComfyUiWorker => "comfyui_worker",
            Self::PluginProcess => "plugin_process",
            Self::HelperSubprocess => "helper_subprocess",
            Self::ExternalCompat => "external_compat",
            Self::Webview2Cdp => "webview2_cdp",
            Self::OfficialCliBridge => "official_cli_bridge",
        }
    }

    pub fn is_regular_model_runtime_engine(self) -> bool {
        matches!(self, Self::LlamaCpp | Self::Candle)
    }
}

impl TryFrom<&str> for ProcessEngineKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "llamacpp" | "llama_cpp" => Ok(Self::LlamaCpp),
            "candle" => Ok(Self::Candle),
            "abliteration_tool" => Ok(Self::AbliterationTool),
            "sandbox_container" => Ok(Self::SandboxContainer),
            "mechanical_job" => Ok(Self::MechanicalJob),
            "asr_worker" => Ok(Self::AsrWorker),
            "comfyui_worker" => Ok(Self::ComfyUiWorker),
            "plugin_process" => Ok(Self::PluginProcess),
            "helper_subprocess" => Ok(Self::HelperSubprocess),
            "external_compat" => Ok(Self::ExternalCompat),
            "webview2_cdp" => Ok(Self::Webview2Cdp),
            "official_cli_bridge" | "officialclibridge" => Ok(Self::OfficialCliBridge),
            other => Err(format!("unknown process engine kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LedgerEventKind {
    Start,
    Stop,
}

impl LedgerEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "START",
            Self::Stop => "STOP",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStart {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub parent_session_id: Option<String>,
    pub parent_process_id: Option<Uuid>,
    pub sandbox_adapter_id: Option<String>,
    pub sandbox_internal_id: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub started_at: DateTime<Utc>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub sandbox_capabilities_snapshot: Value,
    pub metadata_jsonb: Value,
}

impl ProcessStart {
    pub fn new(
        engine_kind: ProcessEngineKind,
        owner_role: impl Into<String>,
        owner_wp: Option<String>,
    ) -> Self {
        Self {
            process_uuid: Uuid::now_v7(),
            os_pid: None,
            parent_session_id: None,
            parent_process_id: None,
            sandbox_adapter_id: None,
            sandbox_internal_id: None,
            engine_kind,
            started_at: Utc::now(),
            model_artifact_sha256: None,
            work_profile_id: None,
            owner_role: owner_role.into(),
            owner_wp: owner_wp.clone(),
            role_id: None,
            wp_id: owner_wp,
            mt_id: None,
            sandbox_capabilities_snapshot: json!({}),
            metadata_jsonb: json!({}),
        }
    }

    pub fn with_process_uuid(mut self, process_uuid: Uuid) -> Self {
        self.process_uuid = process_uuid;
        self
    }

    pub fn with_parent_session_id(mut self, parent_session_id: impl Into<String>) -> Self {
        self.parent_session_id = Some(parent_session_id.into());
        self
    }

    pub fn with_parent_process_id(mut self, parent_process_id: Uuid) -> Self {
        self.parent_process_id = Some(parent_process_id);
        self
    }

    pub fn with_sandbox_adapter_id(mut self, sandbox_adapter_id: impl Into<String>) -> Self {
        self.sandbox_adapter_id = Some(sandbox_adapter_id.into());
        self
    }

    pub fn with_sandbox_internal_id(mut self, sandbox_internal_id: impl Into<String>) -> Self {
        self.sandbox_internal_id = Some(sandbox_internal_id.into());
        self
    }

    pub fn with_model_artifact_sha256(mut self, model_artifact_sha256: impl Into<String>) -> Self {
        self.model_artifact_sha256 = Some(model_artifact_sha256.into());
        self
    }

    pub fn with_work_profile_id(mut self, work_profile_id: impl Into<String>) -> Self {
        self.work_profile_id = Some(work_profile_id.into());
        self
    }

    pub fn with_role_id(mut self, role_id: impl Into<String>) -> Self {
        let role_id = role_id.into();
        self.owner_role = role_id.clone();
        self.role_id = Some(role_id);
        self
    }

    pub fn with_wp_id(mut self, wp_id: impl Into<String>) -> Self {
        let wp_id = wp_id.into();
        self.owner_wp = Some(wp_id.clone());
        self.wp_id = Some(wp_id);
        self
    }

    pub fn with_mt_id(mut self, mt_id: impl Into<String>) -> Self {
        self.mt_id = Some(mt_id.into());
        self
    }

    pub fn with_sandbox_capabilities_snapshot(mut self, snapshot: Value) -> Self {
        self.sandbox_capabilities_snapshot = snapshot;
        self
    }

    pub fn with_metadata_jsonb(mut self, metadata_jsonb: Value) -> Self {
        self.metadata_jsonb = metadata_jsonb;
        self
    }

    pub fn with_os_pid(mut self, os_pid: u32) -> Self {
        self.os_pid = Some(os_pid);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStop {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub parent_session_id: Option<String>,
    pub parent_process_id: Option<Uuid>,
    pub sandbox_adapter_id: Option<String>,
    pub sandbox_internal_id: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub started_at: DateTime<Utc>,
    pub stopped_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub stop_reason: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub sandbox_capabilities_snapshot: Value,
    pub metadata_jsonb: Value,
}

impl ProcessStop {
    pub fn from_start(start: &ProcessStart, exit_code: Option<i32>) -> Self {
        Self {
            process_uuid: start.process_uuid,
            os_pid: start.os_pid,
            parent_session_id: start.parent_session_id.clone(),
            parent_process_id: start.parent_process_id,
            sandbox_adapter_id: start.sandbox_adapter_id.clone(),
            sandbox_internal_id: start.sandbox_internal_id.clone(),
            engine_kind: start.engine_kind,
            started_at: start.started_at,
            stopped_at: Utc::now(),
            exit_code,
            stop_reason: None,
            model_artifact_sha256: start.model_artifact_sha256.clone(),
            work_profile_id: start.work_profile_id.clone(),
            owner_role: start.owner_role.clone(),
            owner_wp: start.owner_wp.clone(),
            role_id: start.role_id.clone(),
            wp_id: start.wp_id.clone(),
            mt_id: start.mt_id.clone(),
            sandbox_capabilities_snapshot: start.sandbox_capabilities_snapshot.clone(),
            metadata_jsonb: start.metadata_jsonb.clone(),
        }
    }

    pub fn with_stop_reason(mut self, stop_reason: impl Into<String>) -> Self {
        self.stop_reason = Some(stop_reason.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_kind", content = "event")]
pub enum LedgerEvent {
    Start(ProcessStart),
    Stop(ProcessStop),
}

impl LedgerEvent {
    pub fn kind(&self) -> LedgerEventKind {
        match self {
            Self::Start(_) => LedgerEventKind::Start,
            Self::Stop(_) => LedgerEventKind::Stop,
        }
    }

    pub fn process_uuid(&self) -> Uuid {
        match self {
            Self::Start(event) => event.process_uuid,
            Self::Stop(event) => event.process_uuid,
        }
    }

    pub fn parent_session_id(&self) -> Option<&str> {
        match self {
            Self::Start(event) => event.parent_session_id.as_deref(),
            Self::Stop(event) => event.parent_session_id.as_deref(),
        }
    }

    pub fn sampled_payload(&self) -> Value {
        match self {
            Self::Start(event) => json!({
                "event_kind": LedgerEventKind::Start.as_str(),
                "process_uuid": event.process_uuid.to_string(),
                "os_pid": event.os_pid,
                "parent_session_id": event.parent_session_id,
                "parent_process_id": event.parent_process_id.map(|id| id.to_string()),
                "sandbox_adapter_id": event.sandbox_adapter_id,
                "sandbox_internal_id": event.sandbox_internal_id,
                "engine_kind": event.engine_kind.as_str(),
                "started_at": event.started_at,
                "model_artifact_sha256": event.model_artifact_sha256,
                "work_profile_id": event.work_profile_id,
                "owner_role": event.owner_role,
                "owner_wp": event.owner_wp,
                "role_id": event.role_id,
                "wp_id": event.wp_id,
                "mt_id": event.mt_id,
                "sandbox_capabilities_snapshot": event.sandbox_capabilities_snapshot,
                "metadata_jsonb": event.metadata_jsonb,
            }),
            Self::Stop(event) => json!({
                "event_kind": LedgerEventKind::Stop.as_str(),
                "process_uuid": event.process_uuid.to_string(),
                "os_pid": event.os_pid,
                "parent_session_id": event.parent_session_id,
                "parent_process_id": event.parent_process_id.map(|id| id.to_string()),
                "sandbox_adapter_id": event.sandbox_adapter_id,
                "sandbox_internal_id": event.sandbox_internal_id,
                "engine_kind": event.engine_kind.as_str(),
                "started_at": event.started_at,
                "stopped_at": event.stopped_at,
                "exit_code": event.exit_code,
                "stop_reason": event.stop_reason,
                "model_artifact_sha256": event.model_artifact_sha256,
                "work_profile_id": event.work_profile_id,
                "owner_role": event.owner_role,
                "owner_wp": event.owner_wp,
                "role_id": event.role_id,
                "wp_id": event.wp_id,
                "mt_id": event.mt_id,
                "sandbox_capabilities_snapshot": event.sandbox_capabilities_snapshot,
                "metadata_jsonb": event.metadata_jsonb,
            }),
        }
    }
}
