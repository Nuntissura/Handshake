//! Deterministic provider registry (env-configured).
//!
//! The initial registry is intentionally simple and deterministic:
//! - Configuration comes from environment variables (startup-time).
//! - No network probing is performed during resolution.
//! - base_url inputs are treated as untrusted (SSRF guard for Cloud tier).

use super::{LlmError, ModelTier};
use crate::model_runtime::RuntimeBinding;
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;

const DEFAULT_LOCAL_EMBEDDING_DIMENSION: usize = 768;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// MT-003 (WP-1) Ollama-kill: the default provider. Local inference resolves
    /// through the embedded ModelRuntime (Candle default / llama.cpp opt-in), NOT
    /// an auto-detected third-party daemon. Replaces the removed `Ollama` variant.
    LocalRuntime,
    OpenAiCompat,
}

/// Net-new (MT-003) local-model configuration read from the environment for the
/// [`ProviderKind::LocalRuntime`] default provider. This is the config the boot
/// path turns into a [`crate::model_runtime::ModelRegistration`] + `LoadSpec`
/// before it is loaded into the embedded ModelRuntime.
///
/// `ModelRegistry::register` requires a non-empty `artifact_path` and a non-zero
/// `sha256`; both invariants are enforced when this config is decoded from env.
#[derive(Debug, Clone)]
pub struct LocalModelConfig {
    /// Filesystem path to the local model artifact (GGUF for llama.cpp,
    /// safetensors for Candle). Never empty (validated at decode time).
    pub artifact_path: PathBuf,
    /// Expected SHA-256 of the artifact. Never all-zeroes (validated at decode).
    pub sha256: [u8; 32],
    /// Which embedded runtime binding the artifact loads under.
    pub runtime_binding: RuntimeBinding,
    /// Operator-facing display name / base-model tag for the registration.
    pub display_name: String,
    /// Declared embedding dimensionality for a dedicated embedding model.
    /// `None` means this config is a chat/completion model, not an embedding
    /// registration.
    pub embedding_dimension: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeRole {
    Frontend,
    Orchestrator,
    Worker,
    Validator,
}

#[derive(Debug, Clone)]
pub struct ProviderRecord {
    pub provider_id: String,
    pub kind: ProviderKind,
    pub tier: ModelTier,
    pub base_url: String,
    pub default_model_id: String,
    pub api_key_env: Option<String>,
    /// Present only for [`ProviderKind::LocalRuntime`] when a local model is
    /// configured. `None` means "no local model configured" and the boot path
    /// fails closed to `DisabledLlmClient` (no daemon fallback).
    pub local_model: Option<LocalModelConfig>,
    /// Optional second local model dedicated to embeddings. It is registered in
    /// the same boot registry/catalog as `local_model`, but it does not replace
    /// the chat/completion `profile().model_id`.
    pub local_embedding_model: Option<LocalModelConfig>,
}

#[derive(Debug, Clone)]
pub struct RoleAssignment {
    pub role: RuntimeRole,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedProvider {
    pub provider_id: String,
    pub kind: ProviderKind,
    pub tier: ModelTier,
    pub base_url: String,
    pub model_id: String,
    pub api_key_env: Option<String>,
    /// See [`ProviderRecord::local_model`].
    pub local_model: Option<LocalModelConfig>,
    /// See [`ProviderRecord::local_embedding_model`].
    pub local_embedding_model: Option<LocalModelConfig>,
}

#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    pub providers: BTreeMap<String, ProviderRecord>,
    pub assignments: BTreeMap<RuntimeRole, RoleAssignment>,
}

impl ProviderRegistry {
    /// Loads a deterministic registry from env vars.
    ///
    /// MT-003 (WP-1) Ollama-kill config:
    /// - `HANDSHAKE_LLM_PROVIDER` in {`local_runtime`, `openai_compat`}
    ///   (default: `local_runtime`). The removed `ollama` daemon default and its
    ///   `/api/tags` auto-detect no longer exist.
    ///
    /// Local runtime (default; resolves through the embedded ModelRuntime):
    /// - `HANDSHAKE_LOCAL_MODEL_PATH` (optional; when unset the boot path fails
    ///   closed to `DisabledLlmClient` with NO daemon fallback)
    /// - `HANDSHAKE_LOCAL_MODEL_SHA256` (required when the path is set; 64 hex
    ///   chars, non-zero)
    /// - `HANDSHAKE_LOCAL_MODEL_BINDING` in {`candle`, `llama_cpp`}
    ///   (default: `candle`, the compiled-in CPU baseline engine)
    /// - `HANDSHAKE_LOCAL_MODEL_NAME` (optional display name / base-model tag)
    ///
    /// OpenAI-compatible (retained non-authoritative external_compat compat lane):
    /// - `OPENAI_COMPAT_BASE_URL` (required)
    /// - `OPENAI_COMPAT_MODEL` (required)
    /// - `OPENAI_COMPAT_TIER` in {`local`, `cloud`} (default: `cloud`)
    /// - `OPENAI_COMPAT_API_KEY_ENV` (optional; name of env var containing API key)
    pub fn from_env() -> Result<Self, LlmError> {
        let provider = std::env::var("HANDSHAKE_LLM_PROVIDER")
            .ok()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "local_runtime".to_string());

        match provider.as_str() {
            "local_runtime" | "local" => {
                let local_model = local_model_config_from_env()?;
                let local_embedding_model = local_embedding_model_config_from_env()?;
                // The role-level `model_id` is the display name when a local
                // model is configured; otherwise a stable placeholder used only
                // for the DisabledLlmClient identity when the boot path fails
                // closed.
                let model_id = local_model
                    .as_ref()
                    .map(|cfg| cfg.display_name.clone())
                    .unwrap_or_else(|| "embedded-local-unconfigured".to_string());

                let record = ProviderRecord {
                    provider_id: "local_runtime".to_string(),
                    kind: ProviderKind::LocalRuntime,
                    tier: ModelTier::Local,
                    base_url: String::new(),
                    default_model_id: model_id.clone(),
                    api_key_env: None,
                    local_model,
                    local_embedding_model,
                };

                let mut providers = BTreeMap::new();
                providers.insert(record.provider_id.clone(), record);

                let mut assignments = BTreeMap::new();
                for role in [
                    RuntimeRole::Frontend,
                    RuntimeRole::Orchestrator,
                    RuntimeRole::Worker,
                    RuntimeRole::Validator,
                ] {
                    assignments.insert(
                        role,
                        RoleAssignment {
                            role,
                            provider_id: "local_runtime".to_string(),
                            model_id: model_id.clone(),
                        },
                    );
                }

                Ok(Self {
                    providers,
                    assignments,
                })
            }
            "openai_compat" => {
                let base_url = std::env::var("OPENAI_COMPAT_BASE_URL").map_err(|_| {
                    LlmError::InvalidBaseUrl("OPENAI_COMPAT_BASE_URL missing".to_string())
                })?;
                let model_id = std::env::var("OPENAI_COMPAT_MODEL").map_err(|_| {
                    LlmError::ProviderError(
                        "HSK-400-INVALID-CONFIG: OPENAI_COMPAT_MODEL missing".to_string(),
                    )
                })?;

                let tier = std::env::var("OPENAI_COMPAT_TIER")
                    .ok()
                    .map(|v| v.trim().to_lowercase())
                    .as_deref()
                    .and_then(|v| match v {
                        "local" => Some(ModelTier::Local),
                        "cloud" => Some(ModelTier::Cloud),
                        _ => None,
                    })
                    .unwrap_or(ModelTier::Cloud);

                let validated_base_url = validate_base_url_for_tier(&base_url, tier)?;

                let api_key_env = std::env::var("OPENAI_COMPAT_API_KEY_ENV")
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());

                let record = ProviderRecord {
                    provider_id: "openai_compat".to_string(),
                    kind: ProviderKind::OpenAiCompat,
                    tier,
                    base_url: validated_base_url,
                    default_model_id: model_id.clone(),
                    api_key_env,
                    local_model: None,
                    local_embedding_model: None,
                };

                let mut providers = BTreeMap::new();
                providers.insert(record.provider_id.clone(), record);

                let mut assignments = BTreeMap::new();
                for role in [
                    RuntimeRole::Frontend,
                    RuntimeRole::Orchestrator,
                    RuntimeRole::Worker,
                    RuntimeRole::Validator,
                ] {
                    assignments.insert(
                        role,
                        RoleAssignment {
                            role,
                            provider_id: "openai_compat".to_string(),
                            model_id: model_id.clone(),
                        },
                    );
                }

                Ok(Self {
                    providers,
                    assignments,
                })
            }
            other => Err(LlmError::ProviderError(format!(
                "HSK-400-INVALID-CONFIG: unknown HANDSHAKE_LLM_PROVIDER={other}"
            ))),
        }
    }

    pub fn resolve(&self, role: RuntimeRole) -> Result<ResolvedProvider, LlmError> {
        let assignment = self.assignments.get(&role).ok_or_else(|| {
            LlmError::ProviderError("HSK-400-INVALID-CONFIG: missing role assignment".to_string())
        })?;
        let record = self.providers.get(&assignment.provider_id).ok_or_else(|| {
            LlmError::ProviderError("HSK-400-INVALID-CONFIG: missing provider record".to_string())
        })?;
        Ok(ResolvedProvider {
            provider_id: record.provider_id.clone(),
            kind: record.kind,
            tier: record.tier,
            base_url: record.base_url.clone(),
            model_id: assignment.model_id.clone(),
            api_key_env: record.api_key_env.clone(),
            local_model: record.local_model.clone(),
            local_embedding_model: record.local_embedding_model.clone(),
        })
    }
}

/// Decodes the [`LocalModelConfig`] from the `HANDSHAKE_LOCAL_MODEL_*` env vars.
///
/// Returns `Ok(None)` when `HANDSHAKE_LOCAL_MODEL_PATH` is unset/empty (no local
/// model configured -> boot path fails closed). When the path IS set, the sha256
/// is required and validated so the downstream `ModelRegistry::register`
/// invariants (non-empty artifact_path, non-zero sha256) cannot be violated.
fn local_model_config_from_env() -> Result<Option<LocalModelConfig>, LlmError> {
    local_model_config_from_env_prefix("HANDSHAKE_LOCAL_MODEL", None)
}

/// Decodes the optional dedicated embedding model config from
/// `HANDSHAKE_LOCAL_EMBEDDING_MODEL_*` env vars. When a path is supplied, the
/// dimensionality defaults to LoomSearchV2's 768-vector contract and may be
/// overridden by `HANDSHAKE_LOCAL_EMBEDDING_MODEL_DIMENSION`.
fn local_embedding_model_config_from_env() -> Result<Option<LocalModelConfig>, LlmError> {
    local_model_config_from_env_prefix(
        "HANDSHAKE_LOCAL_EMBEDDING_MODEL",
        Some(DEFAULT_LOCAL_EMBEDDING_DIMENSION),
    )
}

fn local_model_config_from_env_prefix(
    prefix: &str,
    default_embedding_dimension: Option<usize>,
) -> Result<Option<LocalModelConfig>, LlmError> {
    let path_var = format!("{prefix}_PATH");
    let sha_var = format!("{prefix}_SHA256");
    let binding_var = format!("{prefix}_BINDING");
    let name_var = format!("{prefix}_NAME");
    let dimension_var = format!("{prefix}_DIMENSION");

    let Some(path) = std::env::var(&path_var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    else {
        return Ok(None);
    };

    let sha_hex = std::env::var(&sha_var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            LlmError::ProviderError(format!(
                "HSK-400-INVALID-CONFIG: {sha_var} is required when {path_var} is set"
            ))
        })?;
    let sha256 = decode_sha256(&sha_hex, &sha_var)?;

    let runtime_binding = match std::env::var(&binding_var)
        .ok()
        .map(|v| v.trim().to_lowercase())
        .as_deref()
    {
        Some("llama_cpp") | Some("llamacpp") | Some("llama-cpp") => RuntimeBinding::LlamaCpp,
        Some("candle") | None => RuntimeBinding::Candle,
        Some(other) => {
            return Err(LlmError::ProviderError(format!(
                "HSK-400-INVALID-CONFIG: unknown {binding_var}={other} \
                 (expected candle|llama_cpp)"
            )));
        }
    };

    let display_name = std::env::var(&name_var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default_local_display_name(&path));

    let embedding_dimension = match default_embedding_dimension {
        Some(default_dim) => Some(
            std::env::var(&dimension_var)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .map(|value| {
                    value.parse::<usize>().map_err(|err| {
                        LlmError::ProviderError(format!(
                            "HSK-400-INVALID-CONFIG: {dimension_var} must be a positive integer: {err}"
                        ))
                    })
                })
                .transpose()?
                .unwrap_or(default_dim),
        ),
        None => None,
    };
    if matches!(embedding_dimension, Some(0)) {
        return Err(LlmError::ProviderError(format!(
            "HSK-400-INVALID-CONFIG: {dimension_var} must be greater than zero"
        )));
    }

    Ok(Some(LocalModelConfig {
        artifact_path: PathBuf::from(path),
        sha256,
        runtime_binding,
        display_name,
        embedding_dimension,
    }))
}

/// Parses a 64-char hex SHA-256 into a non-zero `[u8; 32]`.
fn decode_sha256(hex_str: &str, var_name: &str) -> Result<[u8; 32], LlmError> {
    let bytes = hex::decode(hex_str.trim()).map_err(|err| {
        LlmError::ProviderError(format!(
            "HSK-400-INVALID-CONFIG: {var_name} is not valid hex: {err}"
        ))
    })?;
    let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        LlmError::ProviderError(format!(
            "HSK-400-INVALID-CONFIG: {var_name} must be 32 bytes \
             (64 hex chars), got {} bytes",
            bytes.len()
        ))
    })?;
    if arr == [0u8; 32] {
        return Err(LlmError::ProviderError(format!(
            "HSK-400-INVALID-CONFIG: {var_name} must not be all zeroes"
        )));
    }
    Ok(arr)
}

/// Derives a display name from the artifact path file stem.
fn default_local_display_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "embedded-local".to_string())
}

/// Validates base_url deterministically (no DNS resolution).
///
/// Cloud tier rules (default-deny SSRF):
/// - must be https
/// - must not be localhost/loopback/private/link-local IPs
/// - must not contain embedded credentials
pub fn validate_base_url_for_tier(raw: &str, tier: ModelTier) -> Result<String, LlmError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LlmError::InvalidBaseUrl("empty".to_string()));
    }

    let url = reqwest::Url::parse(trimmed)
        .map_err(|e| LlmError::InvalidBaseUrl(format!("parse error: {e}")))?;

    let scheme = url.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(LlmError::InvalidBaseUrl(format!(
            "unsupported scheme: {}",
            url.scheme()
        )));
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(LlmError::InvalidBaseUrl(
            "must not include credentials".to_string(),
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| LlmError::InvalidBaseUrl("missing host".to_string()))?;

    // Normalize trailing slash at the string layer (keep any path prefix).
    let normalized = trimmed.trim_end_matches('/').to_string();

    if tier == ModelTier::Local {
        return Ok(normalized);
    }

    // Cloud tier: enforce https.
    if scheme != "https" {
        return Err(LlmError::SsrBlocked(
            "cloud tier requires https".to_string(),
        ));
    }

    // Cloud tier SSRF guard: block obvious internal targets.
    if is_localhost_name(host) {
        return Err(LlmError::SsrBlocked("localhost host".to_string()));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_disallowed_cloud_ip(&ip) {
            return Err(LlmError::SsrBlocked("disallowed IP range".to_string()));
        }
    }

    Ok(normalized)
}

fn is_localhost_name(host: &str) -> bool {
    let h = host.trim().to_lowercase();
    h == "localhost" || h.ends_with(".localhost")
}

fn is_disallowed_cloud_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_disallowed_cloud_ipv4(v4),
        IpAddr::V6(v6) => is_disallowed_cloud_ipv6(v6),
    }
}

fn is_disallowed_cloud_ipv4(ip: &Ipv4Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_link_local() {
        return true;
    }
    if ip.is_private() {
        return true;
    }

    // 100.64.0.0/10 (CGNAT)
    if ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]) {
        return true;
    }

    // 198.18.0.0/15 (benchmarking)
    if ip.octets()[0] == 198 && (18..=19).contains(&ip.octets()[1]) {
        return true;
    }

    // Multicast 224.0.0.0/4
    if (224..=239).contains(&ip.octets()[0]) {
        return true;
    }

    false
}

fn is_disallowed_cloud_ipv6(ip: &Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() {
        return true;
    }
    // Link-local fe80::/10
    if (ip.segments()[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // Unique local fc00::/7
    if (ip.segments()[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(previous) => std::env::set_var(self.key, previous),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn validate_base_url_local_allows_http_and_localhost() {
        let base_url = match validate_base_url_for_tier("http://localhost:1234/", ModelTier::Local)
        {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "expected Ok, got Err: {err}");
                return;
            }
        };
        assert_eq!(base_url, "http://localhost:1234");
    }

    #[test]
    fn validate_base_url_cloud_requires_https() {
        let err = match validate_base_url_for_tier("http://example.com", ModelTier::Cloud) {
            Ok(_) => {
                assert!(false, "expected Err(SsrBlocked)");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::SsrBlocked(_)));
    }

    #[test]
    fn validate_base_url_cloud_blocks_localhost_name() {
        let err = match validate_base_url_for_tier("https://localhost:1234", ModelTier::Cloud) {
            Ok(_) => {
                assert!(false, "expected Err(SsrBlocked)");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::SsrBlocked(_)));
    }

    #[test]
    fn validate_base_url_cloud_blocks_private_ip() {
        let err = match validate_base_url_for_tier("https://127.0.0.1:443", ModelTier::Cloud) {
            Ok(_) => {
                assert!(false, "expected Err(SsrBlocked)");
                return;
            }
            Err(err) => err,
        };

        assert!(matches!(err, LlmError::SsrBlocked(_)));
    }

    #[test]
    fn validate_base_url_blocks_embedded_credentials() {
        let err =
            match validate_base_url_for_tier("https://user:pass@example.com", ModelTier::Local) {
                Ok(_) => {
                    assert!(false, "expected Err(InvalidBaseUrl)");
                    return;
                }
                Err(err) => err,
            };

        assert!(matches!(err, LlmError::InvalidBaseUrl(_)));
    }

    #[test]
    fn provider_registry_from_env_defaults_to_local_runtime_without_daemon() {
        let _lock = match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // MT-003 Ollama-kill: with nothing configured, the default provider is
        // the embedded LocalRuntime, NOT a daemon, and no local model is
        // configured so the boot path will fail closed to DisabledLlmClient.
        let _provider = EnvGuard::remove("HANDSHAKE_LLM_PROVIDER");
        let _path = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_PATH");
        let _sha = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_SHA256");
        let _binding = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_BINDING");
        let _name = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_NAME");

        let registry = match ProviderRegistry::from_env() {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "expected Ok registry, got Err: {err}");
                return;
            }
        };

        for role in [
            RuntimeRole::Frontend,
            RuntimeRole::Orchestrator,
            RuntimeRole::Worker,
            RuntimeRole::Validator,
        ] {
            let resolved = match registry.resolve(role) {
                Ok(value) => value,
                Err(err) => {
                    assert!(false, "resolve({role:?}) returned Err: {err}");
                    return;
                }
            };
            assert_eq!(resolved.provider_id, "local_runtime");
            assert!(matches!(resolved.kind, ProviderKind::LocalRuntime));
            assert!(matches!(resolved.tier, ModelTier::Local));
            assert!(resolved.api_key_env.is_none());
            assert!(
                resolved.local_model.is_none(),
                "no local model must be configured by default"
            );
        }
    }

    #[test]
    fn provider_registry_from_env_local_runtime_reads_local_model_config() {
        let _lock = match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let _provider = EnvGuard::set("HANDSHAKE_LLM_PROVIDER", "local_runtime");
        let _path = EnvGuard::set("HANDSHAKE_LOCAL_MODEL_PATH", "/models/qwen.gguf");
        // A valid 64-char (32-byte) non-zero hex string; the registry only
        // decodes/validates the hex, it does not hash the artifact here.
        let _sha = EnvGuard::set(
            "HANDSHAKE_LOCAL_MODEL_SHA256",
            "9b871512327c09ce91dd649b3f96a63b7408ef267c8cc5710114e629730cb61f",
        );
        let _binding = EnvGuard::set("HANDSHAKE_LOCAL_MODEL_BINDING", "candle");
        let _name = EnvGuard::set("HANDSHAKE_LOCAL_MODEL_NAME", "qwen-local");

        let registry = match ProviderRegistry::from_env() {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "expected Ok registry, got Err: {err}");
                return;
            }
        };

        let resolved = match registry.resolve(RuntimeRole::Orchestrator) {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "resolve returned Err: {err}");
                return;
            }
        };

        assert!(matches!(resolved.kind, ProviderKind::LocalRuntime));
        let local = match resolved.local_model {
            Some(local) => local,
            None => {
                assert!(false, "expected a configured local model");
                return;
            }
        };
        assert_eq!(
            local.artifact_path,
            std::path::PathBuf::from("/models/qwen.gguf")
        );
        assert_eq!(local.runtime_binding, RuntimeBinding::Candle);
        assert_eq!(local.display_name, "qwen-local");
        assert_ne!(local.sha256, [0u8; 32]);
    }

    #[test]
    fn provider_registry_from_env_local_runtime_rejects_bad_sha256() {
        let _lock = match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let _provider = EnvGuard::set("HANDSHAKE_LLM_PROVIDER", "local_runtime");
        let _path = EnvGuard::set("HANDSHAKE_LOCAL_MODEL_PATH", "/models/qwen.gguf");
        let _sha = EnvGuard::set("HANDSHAKE_LOCAL_MODEL_SHA256", "not-hex");
        let _binding = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_BINDING");
        let _name = EnvGuard::remove("HANDSHAKE_LOCAL_MODEL_NAME");

        let err = match ProviderRegistry::from_env() {
            Ok(_) => {
                assert!(false, "expected Err for invalid sha256");
                return;
            }
            Err(err) => err,
        };
        assert!(matches!(err, LlmError::ProviderError(_)), "{err}");
    }

    #[test]
    fn provider_registry_from_env_openai_compat_propagates_api_key_env() {
        let _lock = match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let _provider = EnvGuard::set("HANDSHAKE_LLM_PROVIDER", "openai_compat");
        let _base = EnvGuard::set("OPENAI_COMPAT_BASE_URL", "http://127.0.0.1:1234/");
        let _model = EnvGuard::set("OPENAI_COMPAT_MODEL", "test-model");
        let _tier = EnvGuard::set("OPENAI_COMPAT_TIER", "local");
        let _api_key_env = EnvGuard::set("OPENAI_COMPAT_API_KEY_ENV", "TEST_OPENAI_KEY");
        let _ollama_url = EnvGuard::remove("OLLAMA_URL");
        let _ollama_model = EnvGuard::remove("OLLAMA_MODEL");

        let registry = match ProviderRegistry::from_env() {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "expected Ok registry, got Err: {err}");
                return;
            }
        };

        let resolved = match registry.resolve(RuntimeRole::Orchestrator) {
            Ok(value) => value,
            Err(err) => {
                assert!(false, "resolve returned Err: {err}");
                return;
            }
        };

        assert_eq!(resolved.provider_id, "openai_compat");
        assert!(matches!(resolved.kind, ProviderKind::OpenAiCompat));
        assert_eq!(resolved.base_url, "http://127.0.0.1:1234");
        assert_eq!(resolved.model_id, "test-model");
        assert!(matches!(resolved.tier, ModelTier::Local));
        assert_eq!(resolved.api_key_env, Some("TEST_OPENAI_KEY".to_string()));
    }
}
