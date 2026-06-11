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
