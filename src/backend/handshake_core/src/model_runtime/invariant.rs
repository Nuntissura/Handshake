use std::{
    fmt, fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use reqwest::Url;

use crate::process_ledger::ProcessEngineKind;

use super::{LoadSpec, ModelRuntime, ModelRuntimeError, ProviderKind, RuntimeKind};

const DEFAULT_SIGNATURE_MAX_AGE_DAYS: i64 = 30;
pub const LLAMA_CPP_LOCAL_ENGINE_ORIGIN: &str = "handshake://model-runtime/llama_cpp";
pub const CANDLE_LOCAL_ENGINE_ORIGIN: &str = "handshake://model-runtime/candle";

const ABLITERATION_FORBIDDEN_HOT_PATH_REFERENCES: &[&str] = &[
    "distillation::abliterate",
    "abliterate::",
    "run_abliteration_offline",
    "orthogonalise_weight",
    "AbliterationConfig",
    "AbliterationProvenance",
    "AbliterationTool",
];

const ABLITERATION_REQUIRED_GENERATE_PATHS: &[&str] = &[
    "src/model_runtime/llama_cpp/generate.rs",
    "src/model_runtime/candle/generate.rs",
];

const ABLITERATION_FORBIDDEN_RUNTIME_EXPORT_REFERENCES: &[&str] = &[
    "mod abliterate",
    "pub mod abliterate",
    "pub(crate) mod abliterate",
    "pub use crate::distillation::abliterate",
    "pub use super::distillation::abliterate",
    "pub use abliterate",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalEngineImportRecord {
    pub endpoint_url: Url,
    pub openai_compatible: bool,
    pub operator_signed_at_utc: DateTime<Utc>,
    pub operator_signature: String,
}

impl ExternalEngineImportRecord {
    pub fn new(
        endpoint_url: impl AsRef<str>,
        openai_compatible: bool,
        operator_signed_at_utc: DateTime<Utc>,
        operator_signature: impl Into<String>,
    ) -> Result<Self, ModelRuntimeError> {
        let operator_signature = operator_signature.into().trim().to_string();
        if operator_signature.is_empty() {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "signed ExternalEngineImportRecord".to_string(),
                got: "empty operator signature".to_string(),
            });
        }
        let endpoint_url = parse_local_http_endpoint(endpoint_url.as_ref())?;
        Ok(Self {
            endpoint_url,
            openai_compatible,
            operator_signed_at_utc,
            operator_signature,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalModelAdapterInvariant {
    pub provider: ProviderKind,
    pub owned_process_engine_kind: Option<ProcessEngineKind>,
}

impl LocalModelAdapterInvariant {
    pub fn validate(spec: &LoadSpec) -> Result<Self, ModelRuntimeError> {
        EngineOriginValidator::default().validate_load_spec(spec)
    }

    pub fn assert_model_runtime_engine_kind(
        engine_kind: ProcessEngineKind,
    ) -> Result<(), ModelRuntimeError> {
        match engine_kind {
            ProcessEngineKind::LlamaCpp | ProcessEngineKind::Candle => Ok(()),
            ProcessEngineKind::AbliterationTool => Err(ModelRuntimeError::AdapterMismatch {
                expected:
                    "offline-only abliteration artifact re-registered as a regular local model runtime"
                        .to_string(),
                got: "engine_kind=AbliterationTool".to_string(),
            }),
            other => Err(ModelRuntimeError::AdapterMismatch {
                expected: "regular local model runtime engine kind (LlamaCpp or Candle)"
                    .to_string(),
                got: format!("engine_kind={}", other.as_str()),
            }),
        }
    }

    pub fn assert_reviewed_abliteration_reregistration_engine_kind(
        reviewed_registration: &crate::distillation::abliterate_review::AbliteratedSkillBankModelRegistration,
        engine_kind: ProcessEngineKind,
    ) -> Result<(), ModelRuntimeError> {
        if reviewed_registration.action
            != crate::distillation::abliterate_review::SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION
        {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "MT-107 reviewed abliteration Skill Bank registration action".to_string(),
                got: reviewed_registration.action.clone(),
            });
        }
        if reviewed_registration.artifact_path.as_os_str().is_empty() {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "reviewed abliterated model artifact path".to_string(),
                got: "empty artifact_path".to_string(),
            });
        }

        match engine_kind {
            ProcessEngineKind::LlamaCpp | ProcessEngineKind::Candle => Ok(()),
            other => Err(ModelRuntimeError::AdapterMismatch {
                expected:
                    "reviewed abliterated artifact must re-register as LlamaCpp or Candle ModelRuntime"
                        .to_string(),
                got: format!("engine_kind={}", other.as_str()),
            }),
        }
    }
}

pub fn assert_abliteration_offline_invariant() -> Result<(), Vec<String>> {
    assert_abliteration_offline_invariant_at(Path::new(env!("CARGO_MANIFEST_DIR")))
}

pub fn assert_abliteration_offline_invariant_at(
    core_root: impl AsRef<Path>,
) -> Result<(), Vec<String>> {
    let core_root = core_root.as_ref();
    let mut violations = Vec::new();
    let hot_path_files = abliteration_hot_path_files(core_root);

    for relative in ABLITERATION_REQUIRED_GENERATE_PATHS {
        let path = core_root.join(relative);
        if !path.is_file() {
            violations.push(format!(
                "missing required runtime hot-path file {}; abliteration offline-only invariant cannot prove the generate path is clean",
                display_relative(core_root, &path)
            ));
        }
    }

    if hot_path_files.is_empty() {
        violations.push(format!(
            "no runtime generate or technique files were scanned under {}; abliteration offline-only invariant would be vacuous",
            core_root.display()
        ));
    }

    for path in hot_path_files {
        scan_file_for_forbidden_references(
            core_root,
            &path,
            ABLITERATION_FORBIDDEN_HOT_PATH_REFERENCES,
            "hot-path",
            &mut violations,
        );
    }

    for path in model_runtime_mod_files(core_root, &mut violations) {
        scan_file_for_forbidden_references(
            core_root,
            &path,
            ABLITERATION_FORBIDDEN_RUNTIME_EXPORT_REFERENCES,
            "model_runtime export",
            &mut violations,
        );
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[derive(Clone, Debug)]
pub struct EngineOriginValidator {
    deny_patterns: Vec<String>,
    signature_max_age: Duration,
    now: DateTime<Utc>,
}

impl Default for EngineOriginValidator {
    fn default() -> Self {
        Self {
            deny_patterns: vec![
                "ollama".to_string(),
                "lm_studio".to_string(),
                "lm-studio".to_string(),
                "lm studio".to_string(),
                "lmstudio".to_string(),
                "localhost:11434".to_string(),
                "127.0.0.1:11434".to_string(),
                "[::1]:11434".to_string(),
                "0.0.0.0:11434".to_string(),
                "localhost:1234".to_string(),
                "127.0.0.1:1234".to_string(),
                "[::1]:1234".to_string(),
                "0.0.0.0:1234".to_string(),
            ],
            signature_max_age: Duration::days(DEFAULT_SIGNATURE_MAX_AGE_DAYS),
            now: Utc::now(),
        }
    }
}

impl EngineOriginValidator {
    pub fn with_now(now: DateTime<Utc>) -> Self {
        Self {
            now,
            ..Self::default()
        }
    }

    pub fn with_deny_patterns(mut self, deny_patterns: Vec<String>) -> Self {
        self.deny_patterns = deny_patterns
            .into_iter()
            .map(|pattern| pattern.to_ascii_lowercase())
            .collect();
        self
    }

    pub fn validate_load_spec(
        &self,
        spec: &LoadSpec,
    ) -> Result<LocalModelAdapterInvariant, ModelRuntimeError> {
        match spec.provider {
            ProviderKind::Local => self.validate_local_spec(spec),
            ProviderKind::ExternalCompat => self.validate_external_compat_spec(spec),
            ProviderKind::ByokCloud | ProviderKind::OfficialCli => {
                Err(ModelRuntimeError::AdapterMismatch {
                    expected: "local model runtime provider".to_string(),
                    got: format!("{:?}", spec.provider),
                })
            }
        }
    }

    fn validate_local_spec(
        &self,
        spec: &LoadSpec,
    ) -> Result<LocalModelAdapterInvariant, ModelRuntimeError> {
        if spec.external_engine_import.is_some() {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "no ExternalEngineImportRecord for ProviderKind::Local".to_string(),
                got: "external import record present".to_string(),
            });
        }
        self.assert_regular_artifact_file(spec)?;
        let engine_origin = self.required_engine_origin(spec.engine_origin.as_deref())?;
        self.assert_origin_not_denied(engine_origin)?;
        let owned_process_engine_kind =
            self.assert_local_origin_matches_runtime(engine_origin, spec.runtime_kind)?;
        LocalModelAdapterInvariant::assert_model_runtime_engine_kind(owned_process_engine_kind)?;
        Ok(LocalModelAdapterInvariant {
            provider: ProviderKind::Local,
            owned_process_engine_kind: Some(owned_process_engine_kind),
        })
    }

    fn validate_external_compat_spec(
        &self,
        spec: &LoadSpec,
    ) -> Result<LocalModelAdapterInvariant, ModelRuntimeError> {
        let record = spec.external_engine_import.as_ref().ok_or_else(|| {
            ModelRuntimeError::AdapterMismatch {
                expected: "signed ExternalEngineImportRecord for ProviderKind::ExternalCompat"
                    .to_string(),
                got: "missing external import record".to_string(),
            }
        })?;
        let engine_origin = self.required_engine_origin(spec.engine_origin.as_deref())?;
        let engine_origin_url = parse_local_http_endpoint(engine_origin)?;
        if engine_origin_url != record.endpoint_url {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "ExternalEngineImportRecord endpoint bound to LoadSpec.engine_origin"
                    .to_string(),
                got: format!(
                    "engine_origin={} record={}",
                    engine_origin_url, record.endpoint_url
                ),
            });
        }
        self.assert_external_import_record(record)?;
        Ok(LocalModelAdapterInvariant {
            provider: ProviderKind::ExternalCompat,
            owned_process_engine_kind: None,
        })
    }

    fn assert_regular_artifact_file(&self, spec: &LoadSpec) -> Result<(), ModelRuntimeError> {
        let path_text = spec.artifact_path.to_string_lossy().to_ascii_lowercase();
        if path_text.starts_with("http://") || path_text.starts_with("https://") {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "regular local model artifact file".to_string(),
                got: spec.artifact_path.display().to_string(),
            });
        }
        if !spec.artifact_path.is_file() {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "regular local model artifact file".to_string(),
                got: spec.artifact_path.display().to_string(),
            });
        }
        Ok(())
    }

    fn required_engine_origin<'a>(
        &self,
        origin: Option<&'a str>,
    ) -> Result<&'a str, ModelRuntimeError> {
        let origin = origin
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .ok_or_else(|| ModelRuntimeError::AdapterMismatch {
                expected: "explicit model engine origin".to_string(),
                got: "missing engine_origin".to_string(),
            })?;
        Ok(origin)
    }

    fn assert_origin_not_denied(&self, origin: &str) -> Result<(), ModelRuntimeError> {
        let normalized = origin.to_ascii_lowercase();
        let compact = compact_origin(&normalized);
        if let Some(pattern) = self.deny_patterns.iter().find(|pattern| {
            normalized.contains(pattern.as_str()) || compact.contains(&compact_origin(pattern))
        }) {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "Handshake-owned local runtime adapter".to_string(),
                got: format!("forbidden third-party origin matched `{pattern}`: {origin}"),
            });
        }
        Ok(())
    }

    fn assert_local_origin_matches_runtime(
        &self,
        origin: &str,
        runtime_kind: RuntimeKind,
    ) -> Result<ProcessEngineKind, ModelRuntimeError> {
        let (expected_origin, engine_kind) = match runtime_kind {
            RuntimeKind::LlamaCpp => (LLAMA_CPP_LOCAL_ENGINE_ORIGIN, ProcessEngineKind::LlamaCpp),
            RuntimeKind::Candle => (CANDLE_LOCAL_ENGINE_ORIGIN, ProcessEngineKind::Candle),
        };
        if origin != expected_origin {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: format!("Handshake-owned local origin `{expected_origin}`"),
                got: origin.to_string(),
            });
        }
        Ok(engine_kind)
    }

    fn assert_external_import_record(
        &self,
        record: &ExternalEngineImportRecord,
    ) -> Result<(), ModelRuntimeError> {
        if !record.openai_compatible {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "OpenAI-compatible external engine endpoint".to_string(),
                got: "openai_compatible=false".to_string(),
            });
        }
        if record.operator_signed_at_utc > self.now {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "non-future operator signature timestamp".to_string(),
                got: record.operator_signed_at_utc.to_rfc3339(),
            });
        }
        let age = self
            .now
            .signed_duration_since(record.operator_signed_at_utc);
        if age > self.signature_max_age {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: format!(
                    "operator signature no older than {} days",
                    self.signature_max_age.num_days()
                ),
                got: format!("signature age {} days", age.num_days()),
            });
        }
        Ok(())
    }
}

pub struct LocalModelAdapter {
    runtime: Box<dyn ModelRuntime>,
    provider: ProviderKind,
    owned_process_engine_kind: ProcessEngineKind,
}

impl LocalModelAdapter {
    pub fn new(runtime: Box<dyn ModelRuntime>, spec: LoadSpec) -> Result<Self, ModelRuntimeError> {
        let decision = LocalModelAdapterInvariant::validate(&spec)?;
        if decision.provider != ProviderKind::Local {
            return Err(ModelRuntimeError::AdapterMismatch {
                expected: "ProviderKind::Local for Handshake-owned LocalModelAdapter".to_string(),
                got: format!("{:?}", decision.provider),
            });
        }
        let owned_process_engine_kind = decision.owned_process_engine_kind.ok_or_else(|| {
            ModelRuntimeError::AdapterMismatch {
                expected: "owned process ledger engine kind for LocalModelAdapter".to_string(),
                got: "none".to_string(),
            }
        })?;
        Ok(Self {
            runtime,
            provider: decision.provider,
            owned_process_engine_kind,
        })
    }

    pub fn provider(&self) -> ProviderKind {
        self.provider
    }

    pub fn runtime(&self) -> &dyn ModelRuntime {
        self.runtime.as_ref()
    }

    pub fn owned_process_engine_kind(&self) -> ProcessEngineKind {
        self.owned_process_engine_kind
    }
}

impl fmt::Debug for LocalModelAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalModelAdapter")
            .field("provider", &self.provider)
            .field("owned_process_engine_kind", &self.owned_process_engine_kind)
            .field("runtime", &"<dyn ModelRuntime>")
            .finish()
    }
}

fn parse_local_http_endpoint(endpoint_url: &str) -> Result<Url, ModelRuntimeError> {
    let endpoint_url =
        Url::parse(endpoint_url).map_err(|error| ModelRuntimeError::AdapterMismatch {
            expected: "valid external engine endpoint URL".to_string(),
            got: error.to_string(),
        })?;
    if !matches!(endpoint_url.scheme(), "http" | "https") {
        return Err(ModelRuntimeError::AdapterMismatch {
            expected: "local HTTP(S) external engine endpoint".to_string(),
            got: endpoint_url.to_string(),
        });
    }
    let host = endpoint_url
        .host_str()
        .ok_or_else(|| ModelRuntimeError::AdapterMismatch {
            expected: "local external engine endpoint host".to_string(),
            got: endpoint_url.to_string(),
        })?;
    let is_localhost = host.eq_ignore_ascii_case("localhost");
    let is_loopback = host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if !is_localhost && !is_loopback {
        return Err(ModelRuntimeError::AdapterMismatch {
            expected: "local OpenAI-compatible external engine endpoint".to_string(),
            got: endpoint_url.to_string(),
        });
    }
    Ok(endpoint_url)
}

fn compact_origin(origin: &str) -> String {
    origin
        .chars()
        .filter(|character| !matches!(character, '_' | '-' | ' ' | '.'))
        .collect()
}

fn abliteration_hot_path_files(core_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative in ABLITERATION_REQUIRED_GENERATE_PATHS {
        let path = core_root.join(relative);
        if path.is_file() {
            files.push(path);
        }
    }

    let techniques_dir = core_root.join("src/model_runtime/techniques");
    if let Ok(entries) = fs::read_dir(techniques_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

fn model_runtime_mod_files(core_root: &Path, violations: &mut Vec<String>) -> Vec<PathBuf> {
    let root = core_root.join("src/model_runtime");
    let mut files = Vec::new();
    collect_mod_files(&root, &mut files, violations);
    files.sort();
    files.dedup();
    files
}

fn collect_mod_files(dir: &Path, files: &mut Vec<PathBuf>, violations: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if dir.exists() {
                violations.push(format!("read {} failed: {error}", dir.display()));
            }
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                violations.push(format!(
                    "read entry under {} failed: {error}",
                    dir.display()
                ));
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            collect_mod_files(&path, files, violations);
        } else if path.file_name().and_then(|value| value.to_str()) == Some("mod.rs") {
            files.push(path);
        }
    }
}

fn scan_file_for_forbidden_references(
    core_root: &Path,
    path: &Path,
    forbidden_references: &[&str],
    scan_kind: &str,
    violations: &mut Vec<String>,
) {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            violations.push(format!("read {} failed: {error}", path.display()));
            return;
        }
    };
    let normalized_source = source.to_ascii_lowercase();
    for forbidden in forbidden_references {
        let normalized_forbidden = forbidden.to_ascii_lowercase();
        if normalized_source.contains(&normalized_forbidden) {
            violations.push(format!(
                "{} contains forbidden abliteration {scan_kind} reference `{forbidden}`",
                display_relative(core_root, path)
            ));
        }
    }
}

fn display_relative(core_root: &Path, path: &Path) -> String {
    path.strip_prefix(core_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
