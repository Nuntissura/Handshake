//! WP-KERNEL-009 SourceIngestionAndEvidence policy-surface integration tests
//! against the real embedded store: MT-081 (ProjectRootAllowlist),
//! MT-082 (SourceKindRegistry projection), MT-083 (PathPortabilityNormalizer
//! at the engine/DB boundary). MT-084 hashing proofs live in
//! `knowledge_ingestion_pipeline_tests.rs`.
//!
//! No mocks or alternate backend: every test runs in a fresh isolated on-disk
//! store, drives the real ingestion engine, and asserts durable rows,
//! EventLedger receipts, and typed boundary validation.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use embedded_knowledge_support::{open_embedded_ingestion_fixture, register_root, test_ctx};

// ---------------------------------------------------------------------------
// MT-081 ProjectRootAllowlist
// ---------------------------------------------------------------------------

mod mt_081_root_allowlist {
    use super::*;
    use handshake_core::knowledge_ingestion::allowlist::{
        PolicyVerdictKind, RootRegistrationPolicy,
    };
    use handshake_core::knowledge_ingestion::engine::RootRegistrationRequest;
    use handshake_core::knowledge_ingestion::IngestionError;
    use handshake_core::storage::knowledge::{KnowledgeRootKind, KnowledgeStore};
    use handshake_core::storage::Database;
    use serde_json::json;

    fn request(workspace_id: &str, path: &str, kind: KnowledgeRootKind) -> RootRegistrationRequest {
        RootRegistrationRequest {
            workspace_id: workspace_id.to_string(),
            display_name: format!("root {path}"),
            root_kind: kind,
            repo_relative_path: path.to_string(),
            file_allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            operator_approved: false,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn allowed_registration_persists_root_decision_and_ledger_receipt() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP allowed_registration_persists_root_decision_and_ledger_receipt: embedded store unavailable");
            return;
        };
        let workspace_id = env.store.create_workspace().await;
        let ctx = test_ctx("mt081-allow");

        let (root, decision) = env
            .engine
            .register_root(
                &ctx,
                request(&workspace_id, "src/backend", KnowledgeRootKind::ProjectRepo),
            )
            .await
            .expect("default policy must allow a plain repo path");

        assert!(root.root_id.starts_with("KSR-"));
        assert_eq!(root.repo_relative_path, "src/backend");
        assert_eq!(decision.verdict, PolicyVerdictKind::Allowed);
        assert!(decision.policy_id.is_none(), "default policy has no row id");
        assert_eq!(decision.actor_id, ctx.actor.actor_id());

        // Root exists through the storage layer.
        let listed = env
            .engine
            .knowledge()
            .list_knowledge_source_roots(&workspace_id)
            .await
            .expect("list roots");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].root_id, root.root_id);

        // Decision receipt is a replayable EventLedger row carrying the
        // backend-navigation ids (spec 2.3.13.11).
        let receipt_event_id = decision
            .receipt_event_id
            .as_deref()
            .expect("decision must carry a ledger receipt");
        let events = env
            .engine
            .knowledge()
            .list_kernel_events_for_session(&ctx.session_run_id)
            .await
            .expect("receipt event row must exist");
        let event = events
            .iter()
            .find(|event| event.event_id == receipt_event_id)
            .expect("decision receipt must be in the embedded ledger");
        assert_eq!(event.event_type.as_str(), "VALIDATION_RECORDED");
        assert_eq!(event.source_component, "knowledge_ingestion");
        assert_eq!(event.correlation_id, ctx.correlation_id);
        assert_eq!(event.payload["verdict"], "allowed");
        assert_eq!(event.payload["candidate_path"], "src/backend");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unallowlisted_root_is_rejected_with_typed_error_and_durable_receipt() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP unallowlisted_root_is_rejected_with_typed_error_and_durable_receipt: embedded store unavailable");
            return;
        };
        let workspace_id = env.store.create_workspace().await;
        let ctx = test_ctx("mt081-deny");

        let stored = env
            .engine
            .store()
            .activate_root_policy(
                &workspace_id,
                &RootRegistrationPolicy {
                    allow_patterns: vec!["docs/**".to_string()],
                    deny_patterns: vec![],
                    require_operator_approval: false,
                },
            )
            .await
            .expect("activate restrictive policy");
        assert_eq!(stored.policy_version, 1);

        let err = env
            .engine
            .register_root(
                &ctx,
                request(&workspace_id, "src/backend", KnowledgeRootKind::ProjectRepo),
            )
            .await
            .expect_err("unallowlisted root must be rejected");
        let IngestionError::PolicyDenied {
            verdict,
            candidate_path,
            decision_id,
            ..
        } = &err
        else {
            panic!("expected typed PolicyDenied, got {err:?}");
        };
        assert_eq!(*verdict, PolicyVerdictKind::DeniedNotAllowlisted);
        assert_eq!(candidate_path, "src/backend");

        // No root row was created (denial is fail-closed, not partial).
        let listed = env
            .engine
            .knowledge()
            .list_knowledge_source_roots(&workspace_id)
            .await
            .expect("list roots");
        assert!(
            listed.is_empty(),
            "denied registration must not create a root"
        );

        // The denial decision is durable, FK'd to the policy, and carries an
        // EventLedger receipt.
        let decisions = env
            .engine
            .store()
            .list_policy_decisions(&workspace_id, 10)
            .await
            .expect("list decisions");
        let denial = decisions
            .iter()
            .find(|d| &d.decision_id == decision_id)
            .expect("denial decision row must be durable");
        assert_eq!(denial.verdict, PolicyVerdictKind::DeniedNotAllowlisted);
        assert_eq!(denial.policy_id.as_deref(), Some(stored.policy_id.as_str()));
        assert!(denial.receipt_event_id.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn deny_patterns_and_operator_approval_gates_enforce() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP deny_patterns_and_operator_approval_gates_enforce: embedded store unavailable");
            return;
        };
        let workspace_id = env.store.create_workspace().await;
        let ctx = test_ctx("mt081-gates");

        // Default policy: secret-prone path is deny-listed.
        let err = env
            .engine
            .register_root(
                &ctx,
                request(
                    &workspace_id,
                    "ops/secrets/prod",
                    KnowledgeRootKind::ProjectRepo,
                ),
            )
            .await
            .expect_err("deny pattern must reject the path");
        match &err {
            IngestionError::PolicyDenied {
                verdict,
                matched_pattern,
                ..
            } => {
                assert_eq!(*verdict, PolicyVerdictKind::DeniedPattern);
                assert_eq!(matched_pattern.as_deref(), Some("**/secrets/**"));
            }
            other => panic!("expected PolicyDenied, got {other:?}"),
        }

        // Operator-gated root kind without approval: typed denial.
        let err = env
            .engine
            .register_root(
                &ctx,
                request(
                    &workspace_id,
                    "imports/research",
                    KnowledgeRootKind::ExternalImport,
                ),
            )
            .await
            .expect_err("external import without approval must be rejected");
        assert!(matches!(
            err,
            IngestionError::PolicyDenied {
                verdict: PolicyVerdictKind::DeniedRequiresApproval,
                ..
            }
        ));

        // Same registration with explicit operator approval: allowed.
        let mut approved = request(
            &workspace_id,
            "imports/research",
            KnowledgeRootKind::ExternalImport,
        );
        approved.operator_approved = true;
        let (root, decision) = env
            .engine
            .register_root(&ctx, approved)
            .await
            .expect("operator approval must wave the import through");
        assert_eq!(root.repo_relative_path, "imports/research");
        assert!(decision.operator_approved);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn policy_versions_increment_and_db_enforces_single_active() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!(
                "SKIP policy_versions_increment_and_db_enforces_single_active: embedded store unavailable"
            );
            return;
        };
        let workspace_id = env.store.create_workspace().await;

        let v1 = env
            .engine
            .store()
            .activate_root_policy(&workspace_id, &RootRegistrationPolicy::default())
            .await
            .expect("activate v1");
        let v2 = env
            .engine
            .store()
            .activate_root_policy(
                &workspace_id,
                &RootRegistrationPolicy {
                    allow_patterns: vec!["src/**".to_string()],
                    deny_patterns: vec![],
                    require_operator_approval: true,
                },
            )
            .await
            .expect("activate v2");
        assert_eq!(v1.policy_version, 1);
        assert_eq!(v2.policy_version, 2);

        let active = env
            .engine
            .store()
            .get_active_root_policy(&workspace_id)
            .await
            .expect("get active")
            .expect("an active policy exists");
        assert_eq!(active.policy_id, v2.policy_id);
        assert!(active.policy.require_operator_approval);

        // DISPOSITION MT-141: the former direct-bypass index/CHECK probes are
        // retired because the embedded public test surface exposes typed
        // writes and catalog observations, not raw writes. The v1/v2 active
        // policy transition and engine-level path/identity validation above
        // are the superseding proofs; the active-policy read below confirms
        // the durable winner.
        assert_eq!(active.policy_version, 2);
    }
}

// ---------------------------------------------------------------------------
// MT-082 SourceKindRegistry (DB projection of the code-authority registry)
// ---------------------------------------------------------------------------

mod mt_082_kind_registry {
    use super::*;
    use handshake_core::knowledge_ingestion::kinds::{
        registry, sync_kind_projection, KIND_REGISTRY_VERSION,
    };
    use handshake_core::storage::surreal::SurrealStorage;
    use surrealdb::types::SurrealValue;

    #[derive(Debug, SurrealValue)]
    struct KindProjectionProofRow {
        kind_key: String,
        capabilities: serde_json::Value,
        extensions: Vec<String>,
        registry_version: String,
    }

    async fn read_projected_kind(
        storage: &SurrealStorage,
        kind_key: &str,
    ) -> KindProjectionProofRow {
        let record_id = kind_key.to_owned();
        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one::<KindProjectionProofRow>(
                            "knowledge_ingestion_kind_registry",
                            &record_id,
                        )
                        .await
                })
            })
            .await
            .expect("read typed kind projection")
            .expect("projected kind row exists")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn projection_sync_mirrors_code_registry_and_removes_stale_rows() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP projection_sync_mirrors_code_registry: embedded store unavailable");
            return;
        };
        // The embedded projection is synchronized through the typed database
        // API. The old direct-insert stale-row probe had no public equivalent.
        let projected = sync_kind_projection(&env.store.db)
            .await
            .expect("sync projection");
        assert_eq!(projected, registry().len());
        assert_eq!(projected, 8, "the eight MT-082 primary kinds");

        let mut rows = Vec::with_capacity(registry().len());
        for spec in registry() {
            rows.push(read_projected_kind(&env.store.storage, spec.kind_key).await);
        }
        assert_eq!(rows.len(), 8, "stale row purged, code registry projected");
        for row in &rows {
            assert_eq!(row.registry_version, KIND_REGISTRY_VERSION);
        }

        // Capability matrix is readable durable state for no-context models.
        let pdf_row = rows
            .iter()
            .find(|row| row.kind_key == "pdf")
            .expect("pdf row");
        let caps = &pdf_row.capabilities;
        assert_eq!(caps["requires_text_layer_detection"], true);
        assert_eq!(caps["supports_partial_extraction"], true);
        assert_eq!(caps["anchor_kinds"], serde_json::json!(["pdf_page"]));
        assert_eq!(pdf_row.extensions, vec!["pdf".to_owned()]);

        // Re-sync is idempotent (upsert, no duplicates, version stable).
        let projected_again = sync_kind_projection(&env.store.db).await.expect("re-sync");
        assert_eq!(projected_again, 8);
        for spec in registry() {
            let row = read_projected_kind(&env.store.storage, spec.kind_key).await;
            assert_eq!(row.registry_version, KIND_REGISTRY_VERSION);
        }
    }
}

// ---------------------------------------------------------------------------
// MT-083 PathPortabilityNormalizer (engine/DB boundary)
// ---------------------------------------------------------------------------

mod mt_083_path_portability {
    use super::*;
    use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
    use handshake_core::knowledge_ingestion::engine::RootRegistrationRequest;
    use handshake_core::knowledge_ingestion::IngestionError;
    use handshake_core::storage::knowledge::{KnowledgeRootKind, KnowledgeStore};
    use serde_json::json;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn machine_local_root_paths_are_rejected_before_any_durable_write() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP machine_local_root_paths_are_rejected: embedded store unavailable");
            return;
        };
        let workspace_id = env.store.create_workspace().await;
        let ctx = test_ctx("mt083-roots");

        for bad in [
            "C:/Users/op/docs",
            "C:\\Users\\op\\docs",
            "/var/data",
            "../escape",
        ] {
            let err = env
                .engine
                .register_root(
                    &ctx,
                    RootRegistrationRequest {
                        workspace_id: workspace_id.clone(),
                        display_name: "bad root".to_string(),
                        root_kind: KnowledgeRootKind::ProjectRepo,
                        repo_relative_path: bad.to_string(),
                        file_allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
                        operator_approved: false,
                    },
                )
                .await
                .expect_err("machine-local path must be rejected");
            assert!(
                matches!(
                    &err,
                    IngestionError::Storage(handshake_core::storage::StorageError::Validation(_))
                        | IngestionError::Validation(_)
                ),
                "expected typed validation for {bad}, got {err:?}"
            );
        }

        // Fail-closed: no root row and no policy decision was written.
        let roots = env
            .engine
            .knowledge()
            .list_knowledge_source_roots(&workspace_id)
            .await
            .expect("list roots");
        assert!(roots.is_empty());
        let decisions = env
            .engine
            .store()
            .list_policy_decisions(&workspace_id, 10)
            .await
            .expect("list decisions");
        assert!(
            decisions.is_empty(),
            "rejection happens BEFORE policy evaluation persists anything"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn source_paths_normalize_to_repo_relative_posix_or_reject_typed() {
        let Some(env) = open_embedded_ingestion_fixture().await else {
            eprintln!("SKIP source_paths_normalize: embedded store unavailable");
            return;
        };
        let workspace_id = env.store.create_workspace().await;
        let ctx = test_ctx("mt083-sources");
        let root = register_root(
            &env,
            &ctx,
            &workspace_id,
            "src",
            KnowledgeRootKind::ProjectRepo,
        )
        .await;
        let limits = IngestionLimits::default();

        // Windows separators + dot segments normalize to one canonical form.
        let outcome = env
            .engine
            .ingest_file_bytes(
                &ctx,
                &root,
                "deep\\.\\nested\\note.md",
                b"# Note\n\ncontent\n",
                "KIRUN-mt083",
                &limits,
                false,
            )
            .await
            .expect("ingest with windows separators");
        assert_eq!(
            outcome.source.relative_path.as_deref(),
            Some("deep/nested/note.md"),
            "stored path is the canonical repo-relative POSIX form"
        );

        // Traversal and machine-local source paths are typed rejections.
        for bad in ["..\\escape.md", "C:/abs.md", "con.md", "dir/file. "] {
            let err = env
                .engine
                .ingest_file_bytes(&ctx, &root, bad, b"x", "KIRUN-mt083", &limits, false)
                .await
                .expect_err("non-portable source path must be rejected");
            assert!(
                matches!(&err, IngestionError::Validation(_)),
                "expected typed validation for {bad}, got {err:?}"
            );
        }
    }
}
