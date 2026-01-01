use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Canonical capability identifiers from Master Spec §11.1 (Capabilities & Consent Model).
const CANONICAL_CAPABILITY_IDS: &[&str] = &[
    "CALENDAR_READ_BASIC",
    "CALENDAR_READ_DETAILS",
    "CALENDAR_READ_ANALYTICS",
    "CALENDAR_WRITE_LOCAL",
    "CALENDAR_WRITE_EXTERNAL",
    "CALENDAR_DELETE_LOCAL",
    "CALENDAR_DELETE_EXTERNAL",
    "CALENDAR_MOVE_EVENT",
    "CALENDAR_RESOLVE_CONFLICT",
    "CALENDAR_ACTIVITY_SUMMARY",
    "CALENDAR_COMPARE_ACTIVITY_WINDOWS",
    "terminal.attach_human",
    "export.debug_bundle",
    "fr.read",
    "diagnostics.read",
    "jobs.read",
    "export.include_payloads",
];

/// Registry error type for capability SSoT violations.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum RegistryError {
    #[error("Unknown capability: {0} (HSK-4001)")]
    UnknownCapability(String),
    #[error("Unknown capability profile: {0}")]
    UnknownProfile(String),
    #[error("Capability not granted: {0}")]
    AccessDenied(String),
}

/// A named capability profile (e.g., "Analyst", "Coder") defining a whitelist of allowed capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub id: String,
    pub allowed: Vec<String>,
}

/// Centralized Single Source of Truth for capabilities, profiles, and job mappings.
/// Enforces Spec §11.1 requirements:
/// - Validates axes/IDs against hardcoded whitelists.
/// - Resolves axis inheritance (axis implies axis:scope).
/// - Rejects unknown IDs with HSK-4001.
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// Valid capability axes (e.g., "fs.read", "net.http")
    valid_axes: HashSet<String>,
    /// Valid full capability IDs (e.g., "doc.summarize")
    valid_full_ids: HashSet<String>,
    /// Pre-defined profiles (e.g., "Analyst")
    profiles: HashMap<String, CapabilityProfile>,
    /// Mapping of JobKind -> CapabilityProfile ID
    job_profile_map: HashMap<String, String>,
    /// Mapping of JobKind -> Required Capability IDs
    job_requirements: HashMap<String, Vec<String>>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        // Spec §11.1.3.1 - Mandatory Axes
        let mut valid_axes = HashSet::new();
        valid_axes.insert("fs.read".to_string());
        valid_axes.insert("fs.write".to_string());
        valid_axes.insert("proc.exec".to_string());
        valid_axes.insert("net.http".to_string());
        valid_axes.insert("device".to_string());
        valid_axes.insert("secrets.use".to_string());

        // Additional axes from §11.1 (implied/extended)
        valid_axes.insert("creative".to_string()); // for creative.export

        // Spec §11.1 - Full IDs
        let mut valid_full_ids = HashSet::new();
        valid_full_ids.insert("doc.summarize".to_string());
        valid_full_ids.insert("terminal.exec".to_string()); // Historically used, though proc.exec is axis
        valid_full_ids.insert("export.debug_bundle".to_string());
        valid_full_ids.insert("export.include_payloads".to_string());
        for id in CANONICAL_CAPABILITY_IDS {
            valid_full_ids.insert((*id).to_string());
        }

        // Spec §11.1 - Profiles
        let mut profiles = HashMap::new();

        // "Analyst" profile (Read-only heavy)
        profiles.insert(
            "Analyst".to_string(),
            CapabilityProfile {
                id: "Analyst".to_string(),
                allowed: vec![
                    "fs.read".to_string(),
                    "net.http".to_string(),
                    "doc.summarize".to_string(),
                    "export.debug_bundle".to_string(),
                    "fr.read".to_string(),
                    "diagnostics.read".to_string(),
                    "jobs.read".to_string(),
                    "export.include_payloads".to_string(),
                ],
            },
        );

        // "Coder" profile (Read/Write/Exec)
        profiles.insert(
            "Coder".to_string(),
            CapabilityProfile {
                id: "Coder".to_string(),
                allowed: vec![
                    "fs.read".to_string(),
                    "fs.write".to_string(),
                    "proc.exec".to_string(),
                    "net.http".to_string(),
                    "doc.summarize".to_string(),
                    "terminal.exec".to_string(),
                ],
            },
        );

        // Job Kind -> Profile Mapping
        let mut job_profile_map = HashMap::new();
        // Primary job kinds (matches JobKind::as_str())
        job_profile_map.insert("doc_edit".to_string(), "Coder".to_string());
        job_profile_map.insert("sheet_transform".to_string(), "Analyst".to_string());
        job_profile_map.insert("canvas_cluster".to_string(), "Analyst".to_string());
        job_profile_map.insert("asr_transcribe".to_string(), "Analyst".to_string());
        job_profile_map.insert("workflow_run".to_string(), "Analyst".to_string());
        job_profile_map.insert("term_exec".to_string(), "Coder".to_string());
        job_profile_map.insert("terminal_exec".to_string(), "Coder".to_string());
        job_profile_map.insert("doc_summarize".to_string(), "Analyst".to_string());
        job_profile_map.insert("doc_test".to_string(), "Analyst".to_string());
        job_profile_map.insert("debug_bundle_export".to_string(), "Analyst".to_string());
        // Backward-compatible aliases
        job_profile_map.insert("Research".to_string(), "Analyst".to_string());
        job_profile_map.insert("Development".to_string(), "Coder".to_string());

        // Job Kind -> Required Capability Mapping
        let mut job_requirements = HashMap::new();
        job_requirements.insert("doc_edit".to_string(), vec!["doc.summarize".to_string()]);
        job_requirements.insert(
            "sheet_transform".to_string(),
            vec!["doc.summarize".to_string()],
        );
        job_requirements.insert(
            "canvas_cluster".to_string(),
            vec!["doc.summarize".to_string()],
        );
        job_requirements.insert(
            "asr_transcribe".to_string(),
            vec!["doc.summarize".to_string()],
        );
        job_requirements.insert(
            "workflow_run".to_string(),
            vec!["doc.summarize".to_string()],
        );
        job_requirements.insert(
            "doc_summarize".to_string(),
            vec!["doc.summarize".to_string()],
        );
        job_requirements.insert("doc_test".to_string(), vec!["doc.summarize".to_string()]);
        job_requirements.insert("term_exec".to_string(), vec!["terminal.exec".to_string()]);
        job_requirements.insert(
            "terminal_exec".to_string(),
            vec!["terminal.exec".to_string()],
        );
        job_requirements.insert(
            "debug_bundle_export".to_string(),
            vec![
                "export.debug_bundle".to_string(),
                "fr.read".to_string(),
                "diagnostics.read".to_string(),
                "jobs.read".to_string(),
            ],
        );

        Self {
            valid_axes,
            valid_full_ids,
            profiles,
            job_profile_map,
            job_requirements,
        }
    }

    /// Validates if a capability ID is known to the system.
    /// Returns true if valid, false otherwise.
    pub fn is_valid(&self, capability_id: &str) -> bool {
        if self.valid_full_ids.contains(capability_id) {
            return true;
        }
        // Check axis format: "axis:scope"
        if let Some((axis, _scope)) = capability_id.split_once(':') {
            return self.valid_axes.contains(axis);
        }
        // If not split, check if it's an axis grant itself (unscoped axis)
        self.valid_axes.contains(capability_id)
    }

    /// Resolves if a requested capability is granted by a list of held capabilities.
    /// Returns Ok(true) if allowed, Ok(false) if denied but valid, Err(UnknownCapability) if invalid.
    pub fn can_perform(&self, requested: &str, granted: &[String]) -> Result<bool, RegistryError> {
        // 1. Sanity check: requested must be valid [HSK-4001]
        if !self.is_valid(requested) {
            return Err(RegistryError::UnknownCapability(requested.to_string()));
        }

        // 2. Check against granted list
        for grant in granted {
            // Exact match covers full IDs and exact scoped matches
            if grant == requested {
                return Ok(true);
            }

            // Axis inheritance: If grant is "fs.read", it covers "fs.read:*"
            // If requested is "fs.read:logs", and grant is "fs.read", that's a match.
            // Note: grant must be the parent axis (no colon)
            if let Some((req_axis, _req_scope)) = requested.split_once(':') {
                if grant == req_axis {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Resolves if a profile allows a requested capability.
    pub fn profile_can(&self, profile_id: &str, requested: &str) -> Result<bool, RegistryError> {
        let profile = self
            .profiles
            .get(profile_id)
            .ok_or_else(|| RegistryError::UnknownProfile(profile_id.to_string()))?;

        self.can_perform(requested, &profile.allowed)
    }

    /// Returns the CapabilityProfile associated with a specific Job Kind.
    pub fn profile_for_job(&self, job_kind: &str) -> Result<&CapabilityProfile, RegistryError> {
        let profile_id = self.job_profile_map.get(job_kind).ok_or_else(|| {
            RegistryError::UnknownProfile(format!("No profile mapped for job kind: {}", job_kind))
        })?;

        self.profiles
            .get(profile_id)
            .ok_or_else(|| RegistryError::UnknownProfile(profile_id.to_string()))
    }

    /// Returns the required capabilities to run a given job kind.
    pub fn required_capabilities_for_job(
        &self,
        job_kind: &str,
    ) -> Result<Vec<String>, RegistryError> {
        self.job_requirements.get(job_kind).cloned().ok_or_else(|| {
            RegistryError::UnknownProfile(format!(
                "No capability requirement defined for job kind: {}",
                job_kind
            ))
        })
    }

    // Read-only views for inspection
    pub fn axes(&self) -> &HashSet<String> {
        &self.valid_axes
    }

    pub fn ids(&self) -> &HashSet<String> {
        &self.valid_full_ids
    }
}

// Global static registry (Thread-safe singleton if needed, though mostly used via AppState)
pub static GLOBAL_REGISTRY: Lazy<CapabilityRegistry> = Lazy::new(CapabilityRegistry::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_validation() {
        let registry = CapabilityRegistry::new();

        assert!(registry.is_valid("fs.read"));
        assert!(registry.is_valid("fs.read:logs")); // Valid axis + arbitrary scope
        assert!(registry.is_valid("doc.summarize")); // Valid full ID

        assert!(!registry.is_valid("magic.wand")); // Invalid axis
        assert!(!registry.is_valid("unknown_id")); // Invalid ID
    }

    #[test]
    fn test_hsk_4001_unknown_capability() {
        let registry = CapabilityRegistry::new();
        let granted = vec!["fs.read".to_string()];

        let result = registry.can_perform("magic.wand", &granted);
        assert!(matches!(
            result,
            Err(RegistryError::UnknownCapability(ref c)) if c == "magic.wand"
        ));
    }

    #[test]
    fn test_profile_mapping_covers_job_kinds() {
        let registry = CapabilityRegistry::new();
        let kinds = [
            "doc_edit",
            "sheet_transform",
            "canvas_cluster",
            "asr_transcribe",
            "workflow_run",
            "term_exec",
            "terminal_exec",
            "doc_summarize",
            "doc_test",
        ];

        for kind in kinds {
            let profile = match registry.profile_for_job(kind) {
                Ok(profile) => profile,
                Err(err) => {
                    unreachable!("expected profile for {kind}, got error: {err}");
                }
            };
            // ensure profile has at least one capability
            assert!(
                !profile.allowed.is_empty(),
                "profile {} for kind {} must allow something",
                profile.id,
                kind
            );
        }

        assert!(matches!(
            registry.profile_for_job("unknown_kind"),
            Err(RegistryError::UnknownProfile(_))
        ));
    }

    #[test]
    fn test_axis_inheritance() {
        let registry = CapabilityRegistry::new();
        let granted = vec!["fs.read".to_string()];

        // Grant "fs.read" should allow "fs.read:logs"
        assert!(matches!(
            registry.can_perform("fs.read:logs", &granted),
            Ok(true)
        ));

        // Grant "fs.read" should allow "fs.read"
        assert!(matches!(
            registry.can_perform("fs.read", &granted),
            Ok(true)
        ));

        // Grant "fs.read" should NOT allow "fs.write"
        assert!(matches!(
            registry.can_perform("fs.write", &granted),
            Ok(false)
        ));
    }

    #[test]
    fn test_profile_resolution() {
        let registry = CapabilityRegistry::new();

        // Analyst has fs.read
        assert!(matches!(
            registry.profile_can("Analyst", "fs.read"),
            Ok(true)
        ));
        assert!(matches!(
            registry.profile_can("Analyst", "fs.read:report"),
            Ok(true)
        ));

        // Analyst does NOT have fs.write
        assert!(matches!(
            registry.profile_can("Analyst", "fs.write"),
            Ok(false)
        ));

        // Unknown profile error
        assert!(matches!(
            registry.profile_can("SuperUser", "fs.read"),
            Err(RegistryError::UnknownProfile(_))
        ));
    }
}
