use serde::{Deserialize, Serialize};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

use crate::storage::artifacts::{ArtifactManifest, PersistedFileArtifact};
use super::{CanonicalResourceLocator, ResourceAuthorityError, WorkspaceArtifactPublicationPermit};

#[derive(SurrealValue)]
struct PublicationKey {
    artifact_id: String,
}

#[derive(SurrealValue)]
struct PublicationInsert {
    artifact_id: String,
    publication: ArtifactPublicationRow,
}

#[derive(SurrealValue)]
struct ArtifactRegistryBinding {
    artifact_resource: surrealdb::types::RecordId,
    artifact_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(crate) struct ArtifactPublicationLocator {
    pub(crate) artifact_id: String,
    pub(crate) artifact_resource_id: String,
    pub(crate) workspace_external_id: String,
    pub(crate) workspace_resource_id: String,
    pub(crate) manifest_sha256: String,
    pub(crate) producer_operation_id: String,
    pub(crate) registration_version: i64,
}

#[derive(SurrealValue)]
struct PublicationSelection {
    artifact_id: String,
    workspace_external_id: String,
}

pub(crate) struct RegisteredArtifactWitness {
    publication: ArtifactPublicationRow,
    manifest: ArtifactManifest,
}

impl RegisteredArtifactWitness {
    pub(crate) fn artifact_id(&self) -> &str { &self.publication.artifact_id }
    pub(crate) fn payload_sha256(&self) -> &str { &self.manifest.content_hash }
    pub(crate) fn manifest(&self) -> &ArtifactManifest { &self.manifest }
}

pub(super) async fn register_artifact_publication(
    permit: &WorkspaceArtifactPublicationPermit<'_>,
    artifact_resource: &CanonicalResourceLocator,
    persisted: &PersistedFileArtifact,
) -> Result<RegisteredArtifactWitness, ResourceAuthorityError> {
    let publication = ArtifactPublicationRow::from_persisted(permit, artifact_resource, persisted)?;
    let transaction = permit.transaction();
    let current: Option<CanonicalResourceLocator> = transaction.query_first(
        "SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version FROM $artifact_resource WHERE resource_kind = 'artifact' AND external_resource_id = $artifact_id;",
        ArtifactRegistryBinding { artifact_resource: artifact_resource.id.clone(), artifact_id: publication.artifact_id.clone() },
    ).await?;
    if current.as_ref() != Some(artifact_resource) {
        return Err(ResourceAuthorityError::InvalidInput("publication Artifact registry changed"));
    }
    let inserted: Option<ArtifactPublicationRow> = transaction.query_first(
        "CREATE type::record('artifact_publication', $artifact_id) CONTENT $publication RETURN artifact_id, artifact_resource_id, workspace_external_id, workspace_resource_id, manifest_json, manifest_sha256, producer_operation_id, registration_version;",
        PublicationInsert { artifact_id: publication.artifact_id.clone(), publication: publication.clone() },
    ).await?;
    if inserted.as_ref() != Some(&publication) {
        return Err(ResourceAuthorityError::InvalidInput("publication insert is unavailable"));
    }
    let observed = read_association(transaction, &publication.artifact_id).await?
        .ok_or(ResourceAuthorityError::InvalidInput("publication readback unavailable"))?;
    if observed != publication {
        return Err(ResourceAuthorityError::InvalidInput("publication immutable binding differs"));
    }
    let manifest = observed.validated_manifest()?;
    Ok(RegisteredArtifactWitness { publication: observed, manifest })
}

async fn read_association(
    transaction: &super::super::SurrealTransactionContext<'_>,
    artifact_id: &str,
) -> Result<Option<ArtifactPublicationRow>, ResourceAuthorityError> {
    Ok(transaction.query_first(
        "SELECT artifact_id, artifact_resource_id, workspace_external_id, workspace_resource_id, manifest_json, manifest_sha256, producer_operation_id, registration_version FROM type::record('artifact_publication', $artifact_id);",
        PublicationKey { artifact_id: artifact_id.to_owned() },
    ).await?)
}

pub(crate) async fn read_artifact_publication_locator(
    transaction: &super::super::SurrealTransactionContext<'_>,
    artifact_id: &str,
    workspace_external_id: &str,
) -> Result<Option<ArtifactPublicationLocator>, ResourceAuthorityError> {
    Ok(transaction.query_first(
        "SELECT artifact_id, artifact_resource_id, workspace_external_id, workspace_resource_id, manifest_sha256, producer_operation_id, registration_version FROM type::record('artifact_publication', $artifact_id) WHERE artifact_id = $artifact_id AND workspace_external_id = $workspace_external_id AND registration_version = 1;",
        PublicationSelection { artifact_id: artifact_id.to_owned(), workspace_external_id: workspace_external_id.to_owned() },
    ).await?)
}

// Only the resource-authority adapter may call this after minting its original
// NAV transaction permit; locators alone never authorize the full-manifest read.
pub(super) async fn read_artifact_publication(
    transaction: &super::super::SurrealTransactionContext<'_>,
    artifact_resource: &CanonicalResourceLocator,
    workspace: &CanonicalResourceLocator,
    expected: &ArtifactPublicationLocator,
) -> Result<Option<ArtifactPublicationRow>, ResourceAuthorityError> {
    if artifact_resource.parent_resource_id.as_ref() != Some(&workspace.id)
        || artifact_resource.owner_account_id != workspace.owner_account_id
        || artifact_resource.access_space_id != workspace.access_space_id
        || artifact_resource.lifecycle_state != "active" || workspace.lifecycle_state != "active"
    {
        return Err(ResourceAuthorityError::InvalidInput("publication read lineage differs"));
    }
    for (resource, kind) in [(artifact_resource, "artifact"), (workspace, "workspace")] {
        #[derive(SurrealValue)]
        struct RegistryRead { resource: surrealdb::types::RecordId, kind: String }
        let current: Option<CanonicalResourceLocator> = transaction.query_first(
            "SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version FROM $resource WHERE resource_kind = $kind;",
            RegistryRead { resource: resource.id.clone(), kind: kind.to_owned() },
        ).await?;
        if current.as_ref() != Some(resource) {
            return Err(ResourceAuthorityError::InvalidInput("publication read registry changed"));
        }
    }
    let Some(publication) = read_association(transaction, &artifact_resource.external_resource_id).await? else {
        return Ok(None);
    };
    if publication.artifact_resource_id != super::record_key(artifact_resource.id.clone())?
        || publication.workspace_resource_id != super::record_key(workspace.id.clone())?
        || publication.workspace_external_id != workspace.external_resource_id
        || publication.artifact_id != artifact_resource.external_resource_id
        || publication.artifact_id != expected.artifact_id
        || publication.artifact_resource_id != expected.artifact_resource_id
        || publication.workspace_resource_id != expected.workspace_resource_id
        || publication.workspace_external_id != expected.workspace_external_id
        || publication.manifest_sha256 != expected.manifest_sha256
        || publication.producer_operation_id != expected.producer_operation_id
        || publication.registration_version != expected.registration_version
    {
        return Err(ResourceAuthorityError::InvalidInput("publication read association differs"));
    }
    publication.validated_manifest()?;
    Ok(Some(publication))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SurrealValue)]
pub(crate) struct ArtifactPublicationRow {
    pub(crate) artifact_id: String,
    pub(crate) artifact_resource_id: String,
    pub(crate) workspace_external_id: String,
    pub(crate) workspace_resource_id: String,
    pub(crate) manifest_json: String,
    pub(crate) manifest_sha256: String,
    pub(crate) producer_operation_id: String,
    pub(crate) registration_version: i64,
}

impl ArtifactPublicationRow {
    pub(super) fn from_persisted(
        permit: &WorkspaceArtifactPublicationPermit<'_>,
        artifact_resource: &CanonicalResourceLocator,
        persisted: &PersistedFileArtifact,
    ) -> Result<Self, ResourceAuthorityError> {
        let workspace = permit.workspace_parent();
        if artifact_resource.parent_resource_id.as_ref() != Some(&workspace.id)
            || artifact_resource.owner_account_id != workspace.owner_account_id
            || artifact_resource.access_space_id != workspace.access_space_id
            || artifact_resource.lifecycle_state != "active"
            || artifact_resource.external_resource_id != persisted.manifest().artifact_id.to_string()
        {
            return Err(ResourceAuthorityError::InvalidInput("publication Artifact lineage differs"));
        }
        let manifest_json = serde_json::to_string(persisted.manifest())
            .map_err(|_| ResourceAuthorityError::InvalidInput("publication manifest encoding failed"))?;
        let row = Self {
            artifact_id: persisted.manifest().artifact_id.to_string(),
            artifact_resource_id: super::record_key(artifact_resource.id.clone())?,
            workspace_external_id: workspace.external_resource_id.clone(),
            workspace_resource_id: super::record_key(workspace.id.clone())?,
            manifest_sha256: super::sha256_hex(manifest_json.as_bytes()),
            manifest_json,
            producer_operation_id: permit.operation_id().to_string(),
            registration_version: 1,
        };
        row.validated_manifest()?;
        Ok(row)
    }

    pub(crate) fn validated_manifest(&self) -> Result<ArtifactManifest, ResourceAuthorityError> {
        for value in [&self.artifact_id, &self.artifact_resource_id,
            &self.workspace_resource_id, &self.producer_operation_id]
        {
            let parsed = Uuid::parse_str(value)
                .map_err(|_| ResourceAuthorityError::InvalidInput("invalid publication UUID"))?;
            if parsed.to_string() != *value {
                return Err(ResourceAuthorityError::InvalidInput("noncanonical publication UUID"));
            }
        }
        if self.registration_version != 1 || self.workspace_external_id.is_empty()
            || super::sha256_hex(self.manifest_json.as_bytes()) != self.manifest_sha256
        {
            return Err(ResourceAuthorityError::InvalidInput("publication manifest binding differs"));
        }
        let manifest: ArtifactManifest = serde_json::from_str(&self.manifest_json)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid publication manifest"))?;
        if manifest.artifact_id.to_string() != self.artifact_id
            || manifest.content_hash.len() != 64
            || !manifest.content_hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ResourceAuthorityError::InvalidInput("publication payload binding differs"));
        }
        Ok(manifest)
    }
}
