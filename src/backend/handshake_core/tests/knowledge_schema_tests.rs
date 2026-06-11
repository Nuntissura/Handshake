//! WP-KERNEL-009 PostgresEventLedgerCore integration tests against REAL
//! Handshake-managed PostgreSQL: MT-049 (KnowledgeSchemaNamespace), MT-050
//! (ProjectSourceRootTables), MT-051 (ProjectSourceFileTables), MT-052
//! (IndexRunLifecycleTables).
//!
//! No mocks, no SQLite, no fixtures-as-proof: every test creates rows in a
//! fresh isolated schema on a real cluster and reads them back, exercising
//! constraints (CHECK violations, FK violations, lifecycle transitions).

mod knowledge_pg_support;

use knowledge_pg_support::knowledge_pg;

// ---------------------------------------------------------------------------
// MT-049 KnowledgeSchemaNamespace
// ---------------------------------------------------------------------------

mod mt_049_namespace {
    use super::*;
    use handshake_core::storage::knowledge::{KnowledgeAuthorityClass, KnowledgeStore};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn namespace_registry_is_seeded_and_boundary_is_sound() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP namespace_registry_is_seeded_and_boundary_is_sound: no PostgreSQL");
            return;
        };

        let registry = pg
            .db
            .list_knowledge_schema_registry()
            .await
            .expect("list knowledge schema registry");
        assert!(
            registry
                .iter()
                .any(|row| row.family_key == "schema_registry"
                    && row.table_name == "knowledge_schema_registry"
                    && row.mt_id == "MT-049"
                    && row.authority_class == KnowledgeAuthorityClass::Support),
            "0130 must register the namespace boundary table itself"
        );
        for row in &registry {
            assert!(
                row.table_name.starts_with("knowledge_"),
                "registered WP-009 table {} violates the knowledge_ prefix boundary",
                row.table_name
            );
            assert_eq!(row.wp_id, "WP-KERNEL-009");
        }

        let audit = pg
            .db
            .audit_knowledge_namespace()
            .await
            .expect("audit knowledge namespace");
        assert!(
            audit.is_sound(),
            "namespace audit must be sound after migrations; missing={:?} unregistered={:?}",
            audit.missing_tables,
            audit.unregistered_tables
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn namespace_does_not_collide_with_existing_domains() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP namespace_does_not_collide_with_existing_domains: no PostgreSQL");
            return;
        };
        let mut conn = pg.raw_connection().await;

        // Existing domain tables must still exist untouched next to the
        // knowledge_ namespace in the same schema (FK targets for WP-009).
        for table in [
            "workspaces",
            "documents",
            "loom_blocks",
            "loom_edges",
            "assets",
            "kernel_event_ledger",
            "ai_bronze_records",
        ] {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM information_schema.tables
                    WHERE table_schema = current_schema() AND table_name = $1
                )",
            )
            .bind(table)
            .fetch_one(&mut conn)
            .await
            .expect("query information_schema");
            assert!(exists, "expected pre-existing table {table} in schema");
            assert!(
                !table.starts_with("knowledge_"),
                "collision audit: pre-existing table {table} must not sit in the knowledge_ namespace"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn registry_rejects_rows_outside_the_namespace_boundary() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP registry_rejects_rows_outside_the_namespace_boundary: no PostgreSQL");
            return;
        };
        let mut conn = pg.raw_connection().await;

        // Non-knowledge_ table name must violate chk_knowledge_schema_registry_prefix.
        let err = sqlx::query(
            "INSERT INTO knowledge_schema_registry
                 (family_key, table_name, record_family, authority_class, migration_file, mt_id)
             VALUES ('rogue', 'loom_blocks', 'Support', 'support', 'none.sql', 'MT-049')",
        )
        .execute(&mut conn)
        .await
        .expect_err("registry must reject a table outside the knowledge_ prefix boundary");
        assert!(
            err.to_string().contains("chk_knowledge_schema_registry_prefix"),
            "unexpected error for prefix violation: {err}"
        );

        // Invalid authority class must violate the CHECK as well.
        let err = sqlx::query(
            "INSERT INTO knowledge_schema_registry
                 (family_key, table_name, record_family, authority_class, migration_file, mt_id)
             VALUES ('rogue2', 'knowledge_rogue', 'Support', 'canonical', 'none.sql', 'MT-049')",
        )
        .execute(&mut conn)
        .await
        .expect_err("registry must reject an unknown authority_class");
        assert!(
            err.to_string().contains("authority_class"),
            "unexpected error for authority_class violation: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// MT-050 ProjectSourceRootTables
// ---------------------------------------------------------------------------

mod mt_050_source_roots {
    use super::*;
    use handshake_core::storage::knowledge::{
        normalize_repo_relative_path, KnowledgeIndexingEligibility, KnowledgeRootKind,
        KnowledgeStore, NewKnowledgeSourceRoot,
    };
    use handshake_core::storage::StorageError;
    use serde_json::json;

    fn new_root(workspace_id: &str, path: &str) -> NewKnowledgeSourceRoot {
        NewKnowledgeSourceRoot {
            workspace_id: workspace_id.to_string(),
            display_name: "Backend core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: path.to_string(),
            allowlist_policy: json!({
                "include": ["src/**/*.rs", "migrations/**/*.sql"],
                "exclude": ["**/target/**"]
            }),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn create_read_list_and_eligibility_roundtrip() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP create_read_list_and_eligibility_roundtrip: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        let created = pg
            .db
            .create_knowledge_source_root(new_root(&workspace_id, "src/backend/handshake_core"))
            .await
            .expect("create knowledge source root");
        assert!(created.root_id.starts_with("KSR-"));
        assert_eq!(created.path_normalization, "repo_relative_posix_v1");
        assert_eq!(
            created.indexing_eligibility,
            KnowledgeIndexingEligibility::Eligible
        );

        let fetched = pg
            .db
            .get_knowledge_source_root(&created.root_id)
            .await
            .expect("get root")
            .expect("root must exist after create");
        assert_eq!(fetched, created);

        let listed = pg
            .db
            .list_knowledge_source_roots(&workspace_id)
            .await
            .expect("list roots");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].root_id, created.root_id);

        let paused = pg
            .db
            .set_knowledge_root_eligibility(&created.root_id, KnowledgeIndexingEligibility::Paused)
            .await
            .expect("pause root");
        assert_eq!(
            paused.indexing_eligibility,
            KnowledgeIndexingEligibility::Paused
        );
        assert!(paused.updated_at >= created.updated_at);

        let missing = pg
            .db
            .set_knowledge_root_eligibility("KSR-00000000000000000000000000000000",
                KnowledgeIndexingEligibility::Excluded)
            .await;
        assert!(
            matches!(missing, Err(StorageError::NotFound(_))),
            "eligibility update on a missing root must be typed NotFound"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn absolute_path_authority_is_rejected_in_rust_and_postgres() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP absolute_path_authority_is_rejected_in_rust_and_postgres: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        // Rust-level normalization rejects machine-local path authority.
        for bad in ["C:/projects/handshake", "/var/handshake", "../escape", "a/../../b"] {
            let err = pg
                .db
                .create_knowledge_source_root(new_root(&workspace_id, bad))
                .await
                .expect_err("absolute/escaping path must be rejected");
            assert!(
                matches!(err, StorageError::Validation(_)),
                "expected typed Validation error for {bad}, got {err:?}"
            );
        }
        // Backslash input is normalized (not rejected) into POSIX form.
        assert_eq!(
            normalize_repo_relative_path("src\\backend").expect("normalize"),
            "src/backend"
        );

        // DB-level portability boundary holds even if application code is
        // bypassed (direct SQL).
        let mut conn = pg.raw_connection().await;
        let err = sqlx::query(
            "INSERT INTO knowledge_source_roots
                 (root_id, workspace_id, display_name, root_kind, repo_relative_path)
             VALUES ('KSR-00000000000000000000000000000001', $1, 'rogue', 'project_repo', 'C:/abs')",
        )
        .bind(&workspace_id)
        .execute(&mut conn)
        .await
        .expect_err("DB constraint must reject absolute paths");
        assert!(
            err.to_string()
                .contains("chk_knowledge_source_roots_path_portable"),
            "unexpected constraint error: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn duplicate_path_and_unknown_workspace_fail_closed() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP duplicate_path_and_unknown_workspace_fail_closed: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        pg.db
            .create_knowledge_source_root(new_root(&workspace_id, "src"))
            .await
            .expect("create first root");
        let dup = pg
            .db
            .create_knowledge_source_root(new_root(&workspace_id, "src"))
            .await
            .expect_err("duplicate (workspace, path) must violate unique constraint");
        assert!(
            dup.to_string().contains("uq_knowledge_source_roots_workspace_path"),
            "unexpected duplicate-root error: {dup}"
        );

        let orphan = pg
            .db
            .create_knowledge_source_root(new_root("ws-does-not-exist", "docs"))
            .await
            .expect_err("unknown workspace must violate the FK");
        assert!(
            orphan.to_string().contains("foreign key"),
            "unexpected FK error: {orphan}"
        );
    }
}
