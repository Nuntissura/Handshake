use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct CapabilityProfile {
    pub id: String,
    pub capabilities: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("HSK-4001: UnknownCapability {0}")]
    UnknownCapability(String),
    #[error("Capability profile not found: {0}")]
    UnknownProfile(String),
    #[error("Profile {profile_id} missing capability {capability}")]
    MissingCapability {
        profile_id: String,
        capability: String,
    },
    #[error("Unknown job kind for capability mapping: {0}")]
    UnknownJobKind(String),
}

#[derive(Clone)]
pub struct CapabilityRegistry {
    valid_axes: HashSet<String>,
    valid_full_ids: HashSet<String>,
    profiles: HashMap<String, CapabilityProfile>,
    job_requirements: HashMap<String, String>,
    job_profiles: HashMap<String, String>,
}

impl CapabilityRegistry {
    pub fn new(
        valid_axes: HashSet<String>,
        valid_full_ids: HashSet<String>,
        profiles: Vec<CapabilityProfile>,
        job_requirements: HashMap<String, String>,
        job_profiles: HashMap<String, String>,
    ) -> Result<Self, RegistryError> {
        let mut profile_map = HashMap::new();
        for profile in profiles {
            // Validate every capability reference up-front to prevent drift.
            for cap in &profile.capabilities {
                if !Self::is_valid_id(&valid_axes, &valid_full_ids, cap) {
                    return Err(RegistryError::UnknownCapability(cap.clone()));
                }
            }
            profile_map.insert(profile.id.clone(), profile);
        }

        // Validate job requirement targets exist.
        for required in job_requirements.values() {
            if !Self::is_valid_id(&valid_axes, &valid_full_ids, required) {
                return Err(RegistryError::UnknownCapability(required.clone()));
            }
        }

        // Validate job profile mapping targets exist.
        for profile_id in job_profiles.values() {
            if !profile_map.contains_key(profile_id) {
                return Err(RegistryError::UnknownProfile(profile_id.clone()));
            }
        }

        Ok(Self {
            valid_axes,
            valid_full_ids,
            profiles: profile_map,
            job_requirements,
            job_profiles,
        })
    }

    fn is_valid_id(
        valid_axes: &HashSet<String>,
        valid_full_ids: &HashSet<String>,
        id: &str,
    ) -> bool {
        if valid_full_ids.contains(id) {
            return true;
        }
        if valid_axes.contains(id) {
            return true;
        }
        if let Some((axis, _scope)) = id.split_once(':') {
            return valid_axes.contains(axis);
        }
        false
    }

    pub fn is_valid(&self, capability_id: &str) -> bool {
        Self::is_valid_id(&self.valid_axes, &self.valid_full_ids, capability_id)
    }

    pub fn validate(&self, capability_id: &str) -> Result<(), RegistryError> {
        if self.is_valid(capability_id) {
            Ok(())
        } else {
            Err(RegistryError::UnknownCapability(capability_id.to_string()))
        }
    }

    pub fn can_perform(&self, requested: &str, granted: &[String]) -> Result<(), RegistryError> {
        self.validate(requested)?;

        for grant in granted {
            if !self.is_valid(grant) {
                return Err(RegistryError::UnknownCapability(grant.clone()));
            }
            if grant == requested {
                return Ok(());
            }

            if let Some((req_axis, _req_scope)) = requested.split_once(':') {
                // Axis inheritance: fs.read grants fs.read:logs
                if grant == req_axis {
                    return Ok(());
                }
            }
        }

        Err(RegistryError::MissingCapability {
            profile_id: String::new(),
            capability: requested.to_string(),
        })
    }

    pub fn profile(&self, profile_id: &str) -> Result<&CapabilityProfile, RegistryError> {
        self.profiles
            .get(profile_id)
            .ok_or_else(|| RegistryError::UnknownProfile(profile_id.to_string()))
    }

    pub fn profile_can(&self, profile_id: &str, requested: &str) -> Result<(), RegistryError> {
        let profile = self.profile(profile_id)?;
        let result = self.can_perform(requested, &profile.capabilities);

        match result {
            Err(RegistryError::MissingCapability { .. }) => Err(RegistryError::MissingCapability {
                profile_id: profile_id.to_string(),
                capability: requested.to_string(),
            }),
            other => other,
        }
    }

    pub fn required_capability_for_job(&self, job_kind: &str) -> Result<String, RegistryError> {
        self.job_requirements
            .get(job_kind)
            .cloned()
            .ok_or_else(|| RegistryError::UnknownJobKind(job_kind.to_string()))
    }

    pub fn profile_for_job_kind(&self, job_kind: &str) -> Result<String, RegistryError> {
        self.job_profiles
            .get(job_kind)
            .cloned()
            .ok_or_else(|| RegistryError::UnknownJobKind(job_kind.to_string()))
    }

    pub fn new_default() -> Self {
        let valid_axes = HashSet::from_iter(
            [
                "fs.read",
                "fs.write",
                "proc.exec",
                "net.http",
                "device",
                "secrets.use",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        let valid_full_ids = HashSet::from_iter(
            ["doc.read", "doc.summarize", "term.exec"]
                .iter()
                .map(|s| s.to_string()),
        );

        let profiles = vec![
            CapabilityProfile {
                id: "default".to_string(),
                capabilities: vec!["doc.read".into(), "doc.summarize".into()],
            },
            CapabilityProfile {
                id: "terminal".to_string(),
                capabilities: vec!["term.exec".into()],
            },
        ];

        let job_requirements = HashMap::from([
            ("doc_summarize".to_string(), "doc.summarize".to_string()),
            ("doc_test".to_string(), "doc.summarize".to_string()),
            ("term_exec".to_string(), "term.exec".to_string()),
            ("terminal_exec".to_string(), "term.exec".to_string()),
        ]);

        let job_profiles = HashMap::from([
            ("doc_summarize".to_string(), "default".to_string()),
            ("doc_test".to_string(), "default".to_string()),
            ("term_exec".to_string(), "terminal".to_string()),
            ("terminal_exec".to_string(), "terminal".to_string()),
        ]);

        match Self::new(
            valid_axes,
            valid_full_ids,
            profiles,
            job_requirements,
            job_profiles,
        ) {
            Ok(registry) => registry,
            Err(err) => panic!("default capability registry failed to initialize: {err}"),
        }
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        CapabilityRegistry::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_full_and_axis_ids() {
        let registry = CapabilityRegistry::default();
        assert!(registry.is_valid("doc.summarize"));
        assert!(registry.is_valid("fs.read"));
        assert!(registry.is_valid("fs.read:logs"));
        assert!(!registry.is_valid("unknown.cap"));
    }

    #[test]
    fn axis_inheritance_allows_scoped_requests() {
        let registry = CapabilityRegistry::default();
        let granted = vec!["fs.read".to_string()];
        assert!(registry.can_perform("fs.read:logs", &granted).is_ok());
    }

    #[test]
    fn rejects_unknown_capabilities() {
        let registry = CapabilityRegistry::default();
        let err = registry
            .can_perform("unknown.cap", &[])
            .expect_err("expected unknown capability");
        match err {
            RegistryError::UnknownCapability(id) => assert_eq!(id, "unknown.cap"),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn profile_can_maps_missing_capability() {
        let registry = CapabilityRegistry::default();
        let err = registry
            .profile_can("default", "term.exec")
            .expect_err("expected missing capability");
        match err {
            RegistryError::MissingCapability {
                profile_id,
                capability,
            } => {
                assert_eq!(profile_id, "default");
                assert_eq!(capability, "term.exec");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
