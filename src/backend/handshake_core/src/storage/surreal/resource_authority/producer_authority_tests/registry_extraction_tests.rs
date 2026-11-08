use super::*;
use crate::storage::artifacts::{
    ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use std::path::PathBuf;

pub(super) struct ArtifactCaseState {
    previous_root: Option<OsString>,
    root_installed: bool,
    root: PathBuf,
    publications: Vec<Arc<WorkspaceArtifactPublicationAuthorization>>,
}

impl ArtifactCaseState {
    fn install(root: PathBuf) -> TestResult<Self> {
        std::fs::create_dir_all(&root)?;
        let root = root.canonicalize()?;
        let previous_root =
            std::env::var_os(crate::storage::artifacts::HANDSHAKE_WORKSPACE_ROOT_ENV);
        std::env::set_var(
            crate::storage::artifacts::HANDSHAKE_WORKSPACE_ROOT_ENV,
            &root,
        );
        Ok(Self {
            previous_root,
            root_installed: true,
            root,
            publications: Vec::new(),
        })
    }

    pub(super) async fn reconcile(&mut self, storage: &SurrealStorage) -> TestResult {
        for authority in &self.publications {
            if storage.transaction_settlement_pending(authority.operation_id()) {
                let retention: Arc<dyn std::any::Any + Send + Sync> = authority.clone();
                let outcome: Result<
                    (Value, Arc<WorkspaceArtifactPublicationAuthorization>),
                    ResourceAuthorityError,
                > = storage
                    .reconcile_workspace_artifact_publication(
                        authority.operation_context.clone(),
                        retention,
                        authority.publication_resources().clone(),
                    )
                    .await;
                if storage.transaction_settlement_pending(authority.operation_id()) {
                    return Err(format!(
                        "original artifact publication remains unsettled: {:?}",
                        outcome.err()
                    )
                    .into());
                }
            }
        }
        self.publications.clear();
        if self.root_installed {
            match self.previous_root.take() {
                Some(root) => std::env::set_var(
                    crate::storage::artifacts::HANDSHAKE_WORKSPACE_ROOT_ENV,
                    root,
                ),
                None => {
                    std::env::remove_var(crate::storage::artifacts::HANDSHAKE_WORKSPACE_ROOT_ENV)
                }
            }
            self.root_installed = false;
        }
        Ok(())
    }
}

async fn publish_and_attempt_reads(
    fixture: &mut Fixture,
    workspace: &str,
    inputs: Vec<Uuid>,
) -> TestResult<Value> {
    let storage = fixture.bootstrap.surreal.clone();
    let authority = storage
        .capture_workspace_artifact_publication_authorization(
            Uuid::now_v7(),
            &fixture.bootstrap.capability_registry,
            fixture.credentials(),
            workspace,
        )
        .await?;
    let state = fixture.artifact_case.as_mut().unwrap();
    assert_eq!(authority.managed_root(), state.root);
    state.publications.push(authority.clone());
    let worker = storage.clone();
    let value: Value = storage
        .with_workspace_artifact_publication(&authority, move |permit| {
            Box::pin(async move {
                let mut outcomes = Vec::new();
                for input in inputs {
                    let denied = match worker
                        .read_registry_extraction_artifact(&permit, input)
                        .await
                    {
                        Err(ResourceAuthorityError::Denied { .. }) => true,
                        Err(error) => return Err(error),
                        Ok(_) => false,
                    };
                    outcomes.push(json!({"artifact_id": input, "denied": denied}));
                }
                let payload =
                    serde_json::to_vec(&json!({"component_fixture": true, "reads": outcomes}))
                        .map_err(|_| {
                            ResourceAuthorityError::InvalidInput(
                                "test payload serialization failed",
                            )
                        })?;
                let manifest = ArtifactManifest {
                    artifact_id: Uuid::now_v7(),
                    layer: ArtifactLayer::L3,
                    kind: ArtifactPayloadKind::Report,
                    mime: "application/json".into(),
                    filename_hint: Some("registry-extraction-component.json".into()),
                    created_at: Utc::now(),
                    created_by_job_id: None,
                    source_entity_refs: Vec::new(),
                    source_artifact_refs: Vec::new(),
                    content_hash: sha256_hex(&payload),
                    size_bytes: payload.len() as u64,
                    classification: ArtifactClassification::High,
                    exportable: false,
                    retention_ttl_days: Some(1),
                    pinned: None,
                    hash_basis: None,
                    hash_exclude_paths: Vec::new(),
                };
                let persisted = permit
                    .persist_file_artifact_owned(&manifest, &payload)
                    .map_err(|_| {
                        ResourceAuthorityError::InvalidInput("test owned publication failed")
                    })?;
                permit.register_persisted_artifact(&persisted).await?;
                Ok::<_, ResourceAuthorityError>(
                    json!({"artifact_id": manifest.artifact_id, "reads": outcomes}),
                )
            })
        })
        .await?;
    assert!(!storage.transaction_settlement_pending(authority.operation_id()));
    let artifact = storage
        .canonical_resource_locator(ResourceKind::Artifact, field(&value, "artifact_id")?)
        .await?
        .ok_or("published Artifact is absent from canonical resources")?;
    let parent = storage
        .canonical_resource_locator(ResourceKind::Workspace, workspace)
        .await?
        .ok_or("original Workspace is absent from canonical resources")?;
    assert_eq!(artifact.parent_resource_id.as_ref(), Some(&parent.id));
    assert_eq!(artifact.owner_account_id, parent.owner_account_id);
    assert_eq!(artifact.access_space_id, parent.access_space_id);
    assert_eq!(artifact.lifecycle_state, "active");
    Ok(value)
}

#[cfg(windows)]
#[test]
fn registry_extraction_artifact_read_denies_workspace_grants_and_unregistered_or_foreign_ids(
) -> TestResult {
    run_case(|fixture| {
        Box::pin(async move {
            fixture.artifact_case = Some(ArtifactCaseState::install(
                fixture
                    .backend
                    .as_ref()
                    .unwrap()
                    .data_dir
                    .join("registry-extraction-managed-root"),
            )?);
            let workspace = fixture.workspace.as_ref().unwrap().clone();
            let foreign_workspace = fixture.create_workspace().await?;
            let source = publish_and_attempt_reads(fixture, &workspace, Vec::new()).await?;
            let foreign =
                publish_and_attempt_reads(fixture, &foreign_workspace, Vec::new()).await?;
            for request in [
                fixture.request(&workspace, "fs.read", ResourceAction::Read),
                fixture.request(&workspace, "fs.write", ResourceAction::Create),
            ] {
                fixture
                    .bootstrap
                    .surreal
                    .authorize_protected_resource(request)
                    .await?;
            }
            let source_id = Uuid::parse_str(field(&source, "artifact_id")?)?;
            let foreign_id = Uuid::parse_str(field(&foreign, "artifact_id")?)?;
            let absent_id = Uuid::now_v7();
            assert!(fixture
                .bootstrap
                .surreal
                .canonical_resource_locator(ResourceKind::Artifact, &absent_id.to_string())
                .await?
                .is_none());
            let result = publish_and_attempt_reads(
                fixture,
                &workspace,
                vec![source_id, foreign_id, absent_id],
            )
            .await?;
            let outcomes = result["reads"].as_array().ok_or("read outcomes missing")?;
            assert_eq!(outcomes.len(), 3);
            for (outcome, expected_id) in outcomes.iter().zip([source_id, foreign_id, absent_id]) {
                assert_eq!(outcome["artifact_id"], expected_id.to_string());
                assert_eq!(
                    outcome["denied"], true,
                    "Workspace access must not authorize Artifact payloads"
                );
            }
            Ok(())
        })
    })
}
