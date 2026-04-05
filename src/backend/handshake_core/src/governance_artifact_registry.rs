use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::workflows::locus::{
    GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1, GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1,
};
use crate::storage::StorageError;

pub const GOVERNANCE_ARTIFACT_REGISTRY_SORT_KEY_V1: &str = "registry_id";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceArtifactKind {
    Codex,
    Protocols,
    Rubrics,
    Checks,
    Templates,
    Schemas,
}

impl GovernanceArtifactKind {
    pub const fn all() -> &'static [Self] {
        const ALL: [GovernanceArtifactKind; 6] = [
            GovernanceArtifactKind::Codex,
            GovernanceArtifactKind::Protocols,
            GovernanceArtifactKind::Rubrics,
            GovernanceArtifactKind::Checks,
            GovernanceArtifactKind::Templates,
            GovernanceArtifactKind::Schemas,
        ];
        &ALL
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Protocols => "protocols",
            Self::Rubrics => "rubrics",
            Self::Checks => "checks",
            Self::Templates => "templates",
            Self::Schemas => "schemas",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceArtifactProvenance {
    pub source_artifact: String,
    pub snapshot_version: String,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceArtifactRegistryEntry {
    pub artifact_id: Uuid,
    pub kind: GovernanceArtifactKind,
    pub provenance: GovernanceArtifactProvenance,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceArtifactRegistryManifest {
    #[serde(default = "default_governance_artifact_registry_schema_id")]
    pub schema_id: String,
    #[serde(default = "default_governance_artifact_registry_schema_version")]
    pub schema_version: String,
    pub registry_id: Uuid,
    pub entries: Vec<GovernanceArtifactRegistryEntry>,
}

fn default_governance_artifact_registry_schema_id() -> String {
    GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1.to_string()
}

fn default_governance_artifact_registry_schema_version() -> String {
    GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1.to_string()
}

#[async_trait]
pub trait GovernanceArtifactRegistryStore: Send + Sync {
    async fn load_manifest(
        &self,
        registry_id: &Uuid,
    ) -> Result<Option<GovernanceArtifactRegistryManifest>, StorageError>;

    async fn save_manifest(
        &self,
        manifest: GovernanceArtifactRegistryManifest,
    ) -> Result<(), StorageError>;

    async fn list_manifests(
        &self,
    ) -> Result<Vec<GovernanceArtifactRegistryManifest>, StorageError>;
}

#[derive(Default)]
struct InMemoryGovernanceArtifactRegistryStore {
    manifests: Arc<Mutex<HashMap<Uuid, GovernanceArtifactRegistryManifest>>>,
}

#[async_trait]
impl GovernanceArtifactRegistryStore for InMemoryGovernanceArtifactRegistryStore {
    async fn load_manifest(
        &self,
        registry_id: &Uuid,
    ) -> Result<Option<GovernanceArtifactRegistryManifest>, StorageError> {
        let manifests = self.manifests.lock().await;
        Ok(manifests.get(registry_id).cloned())
    }

    async fn save_manifest(
        &self,
        manifest: GovernanceArtifactRegistryManifest,
    ) -> Result<(), StorageError> {
        if manifest.entries.is_empty() {
            return Err(StorageError::Validation("governance artifact registry manifest cannot be empty"));
        }

        let mut manifests = self.manifests.lock().await;
        manifests.insert(manifest.registry_id, manifest);
        Ok(())
    }

    async fn list_manifests(
        &self,
    ) -> Result<Vec<GovernanceArtifactRegistryManifest>, StorageError> {
        let manifests = self.manifests.lock().await;
        let mut values = manifests.values().cloned().collect::<Vec<_>>();
        values.sort_by_key(|manifest| manifest.registry_id);
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::str::FromStr;
    use uuid::Uuid;

    fn sample_provenance() -> GovernanceArtifactProvenance {
        GovernanceArtifactProvenance {
            source_artifact: "hsk://source/governance/v1".to_string(),
            snapshot_version: "1.0.0".to_string(),
            imported_at: DateTime::from_str("2026-04-05T19:00:00Z").unwrap(),
        }
    }

    fn sample_entry() -> GovernanceArtifactRegistryEntry {
        GovernanceArtifactRegistryEntry {
            artifact_id: Uuid::new_v4(),
            kind: GovernanceArtifactKind::Codex,
            provenance: sample_provenance(),
            content_hash: "sha256:0123456789abcdef".to_string(),
        }
    }

    fn sample_manifest() -> GovernanceArtifactRegistryManifest {
        GovernanceArtifactRegistryManifest {
            schema_id: GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1.to_string(),
            schema_version: GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1.to_string(),
            registry_id: Uuid::new_v4(),
            entries: vec![sample_entry()],
        }
    }

    #[tokio::test]
    async fn governance_artifact_registry_entry_round_trip() {
        let manifest = sample_manifest();

        let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
        let decoded: GovernanceArtifactRegistryManifest =
            serde_json::from_str(&encoded).expect("deserialize manifest");

        assert_eq!(manifest, decoded);
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].artifact_id, manifest.entries[0].artifact_id);
        assert_eq!(decoded.entries[0].kind, manifest.entries[0].kind);
        assert_eq!(decoded.entries[0].content_hash, manifest.entries[0].content_hash);
        assert_eq!(decoded.entries[0].provenance, manifest.entries[0].provenance);
    }

    #[test]
    fn governance_artifact_kind_is_exhaustive() {
        let kinds = GovernanceArtifactKind::all();
        assert_eq!(kinds.len(), 6);

        for kind in kinds {
            let raw = serde_json::to_value(kind).expect("serialize artifact kind");
            let recovered: GovernanceArtifactKind =
                serde_json::from_value(raw).expect("deserialize artifact kind");
            assert_eq!(*kind, recovered);
            let raw = serde_json::to_value(kind).expect("serialize artifact kind");
            assert_eq!(raw.as_str(), Some(kind.as_str()));
        }
    }

    #[tokio::test]
    async fn governance_artifact_registry_store_trait_contract() {
        let store = InMemoryGovernanceArtifactRegistryStore::default();
        let manifest = sample_manifest();
        let registry_id = manifest.registry_id;

        store
            .save_manifest(manifest.clone())
            .await
            .expect("save manifest");

        let reloaded = store
            .load_manifest(&registry_id)
            .await
            .expect("load manifest")
            .expect("loaded manifest");
        assert_eq!(manifest, reloaded);

        let all = store.list_manifests().await.expect("list manifests");
        assert_eq!(all, vec![manifest]);
    }

    #[test]
    fn governance_artifact_registry_extensions_present_in_serde_value() {
        let manifest = sample_manifest();
        let value: Value = serde_json::to_value(&manifest).expect("convert to value");
        assert_eq!(
            value
                .get("schema_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1
        );
        assert_eq!(
            value
                .get("schema_version")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1
        );
    }
}
