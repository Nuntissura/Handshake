//! Deterministic provider registry (env-configured).
//!
//! The initial registry is intentionally simple and deterministic:
//! - Configuration comes from environment variables (startup-time).
//! - No network probing is performed during resolution.
//! - base_url inputs are treated as untrusted (SSRF guard for Cloud tier).

use super::{LlmError, ModelTier};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Ollama,
    OpenAiCompat,
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
}

#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    pub providers: BTreeMap<String, ProviderRecord>,
    pub assignments: BTreeMap<RuntimeRole, RoleAssignment>,
}

impl ProviderRegistry {
    /// Loads a deterministic registry from env vars.
    ///
    /// v1 config:
    /// - `HANDSHAKE_LLM_PROVIDER` in {`ollama`, `openai_compat`} (default: `ollama`)
    ///
    /// Ollama:
    /// - `OLLAMA_URL` (default: http://localhost:11434)
    /// - `OLLAMA_MODEL` (default: llama3)
    ///
    /// OpenAI-compatible:
    /// - `OPENAI_COMPAT_BASE_URL` (required)
    /// - `OPENAI_COMPAT_MODEL` (required)
    /// - `OPENAI_COMPAT_TIER` in {`local`, `cloud`} (default: `cloud`)
    /// - `OPENAI_COMPAT_API_KEY_ENV` (optional; name of env var containing API key)
    pub fn from_env() -> Result<Self, LlmError> {
        let provider = std::env::var("HANDSHAKE_LLM_PROVIDER")
            .ok()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "ollama".to_string());

        match provider.as_str() {
            "ollama" => {
                let base_url = std::env::var("OLLAMA_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                let base_url = base_url.trim_end_matches('/').to_string();
                let model_id =
                    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_string());

                let record = ProviderRecord {
                    provider_id: "ollama".to_string(),
                    kind: ProviderKind::Ollama,
                    tier: ModelTier::Local,
                    base_url,
                    default_model_id: model_id.clone(),
                    api_key_env: None,
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
                            provider_id: "ollama".to_string(),
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
                let base_url = std::env::var("OPENAI_COMPAT_BASE_URL")
                    .map_err(|_| LlmError::InvalidBaseUrl("OPENAI_COMPAT_BASE_URL missing".to_string()))?;
                let model_id = std::env::var("OPENAI_COMPAT_MODEL")
                    .map_err(|_| LlmError::ProviderError("HSK-400-INVALID-CONFIG: OPENAI_COMPAT_MODEL missing".to_string()))?;

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
        })
    }
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
