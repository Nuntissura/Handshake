//! WP-KERNEL-009 PostgresEventLedgerCore integration tests against REAL
//! Handshake-managed PostgreSQL: MT-058 (WikiProjectionTables), MT-059
//! (RichDocumentTables + EditorCodeNode), MT-060 (ContextBundleTables +
//! RetrievalTrace).

mod knowledge_pg_support;

use handshake_core::storage::knowledge::KnowledgeStore;
use knowledge_pg_support::knowledge_pg;
use serde_json::json;

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// MT-058 WikiProjectionTables
// ---------------------------------------------------------------------------

mod mt_058_projections {
    use super::*;
    use handshake_core::storage::knowledge::{
        KnowledgeAuthorityClass, KnowledgeProjectionKind, KnowledgeRebuildStatus,
        NewKnowledgeWikiProjection,
    };

    fn projection(workspace_id: &str, title: &str) -> NewKnowledgeWikiProjection {
        NewKnowledgeWikiProjection {
            workspace_id: workspace_id.to_string(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: title.to_string(),
            source_records: json!([
                {"record_family": "KnowledgeClaim", "record_id": "KCL-demo"},
            ]),
            rendered_content: "# Generated page\n\nprojection only".to_string(),
            staleness_hash: super::sha256_hex(b"render-input-v1"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn projection_lifecycle_rebuild_stale_delete_without_authority_mutation() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP projection_lifecycle...: no PostgreSQL");
            return;
        };
        let workspace_id = pg.create_workspace().await;

        let created = pg
            .db
            .upsert_knowledge_wiki_projection(projection(&workspace_id, "Kernel Events"))
            .await
            .expect("create projection");
        assert!(created.projection_id.starts_with("KWP-"));
        assert_eq!(created.rebuild_status, KnowledgeRebuildStatus::Stale);

        // Rebuild workers can mark the projection rebuilding, then failed,
        // without touching the rendered content.
        let rebuilding = pg
            .db
            .set_knowledge_projection_rebuild_status(
                &created.projection_id,
                KnowledgeRebuildStatus::Rebuilding,
            )
            .await
            .expect("mark rebuilding");
        assert_eq!(
            rebuilding.rebuild_status,
            KnowledgeRebuildStatus::Rebuilding
        );
        let failed = pg
            .db
            .set_knowledge_projection_rebuild_status(
                &created.projection_id,
                KnowledgeRebuildStatus::Failed,
            )
            .await
            .expect("mark failed");
        assert_eq!(failed.rebuild_status, KnowledgeRebuildStatus::Failed);
        assert_eq!(failed.rendered_content, created.rendered_content);

        // Rebuild completes: fresh + rebuild timestamp.
        let fresh = pg
            .db
            .mark_knowledge_projection_rebuilt(
                &created.projection_id,
                &super::sha256_hex(b"render-input-v2"),
                "# Generated page v2",
                None,
            )
            .await
            .expect("mark rebuilt");
        assert_eq!(fresh.rebuild_status, KnowledgeRebuildStatus::Fresh);
        assert!(fresh.last_rebuilt_at.is_some());

        // Upserting the same (workspace, kind, title) updates in place.
        let again = pg
            .db
            .upsert_knowledge_wiki_projection(projection(&workspace_id, "Kernel Events"))
            .await
            .expect("re-upsert projection");
        assert_eq!(again.projection_id, created.projection_id);
        assert_eq!(
            again.rebuild_status,
            KnowledgeRebuildStatus::Stale,
            "re-upsert marks the projection stale for rebuild"
        );

        // Registry classifies the table as projection, never authority.
        let registry = pg
            .db
            .list_knowledge_schema_registry()
            .await
            .expect("registry");
        let row = registry
            .iter()
            .find(|row| row.table_name == "knowledge_wiki_projections")
            .expect("projection table registered");
        assert_eq!(row.authority_class, KnowledgeAuthorityClass::Projection);

        // Deleting the projection mutates NO authority records: count an
        // authority surface before and after the delete.
        let before: i64 = {
            let mut conn = pg.raw_connection().await;
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_schema_registry")
                .fetch_one(&mut conn)
                .await
                .expect("count registry")
        };
        pg.db
            .delete_knowledge_wiki_projection(&created.projection_id)
            .await
            .expect("delete projection");
        let gone = pg
            .db
            .get_knowledge_wiki_projection(&created.projection_id)
            .await
            .expect("get after delete");
        assert!(gone.is_none(), "projection row is gone");
        let after: i64 = {
            let mut conn = pg.raw_connection().await;
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_schema_registry")
                .fetch_one(&mut conn)
                .await
                .expect("count registry after")
        };
        assert_eq!(
            before, after,
            "deleting a projection must not touch authority rows"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_authority_table_references_the_projection_table() {
        let Some(pg) = knowledge_pg().await else {
            eprintln!("SKIP no_authority_table_references_the_projection_table: no PostgreSQL");
            return;
        };
        // Structural boundary proof straight from the catalog: zero foreign
        // keys anywhere point INTO knowledge_wiki_projections.
        let mut conn = pg.raw_connection().await;
        let inbound: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.table_constraints tc
            JOIN information_schema.constraint_column_usage ccu
              ON tc.constraint_name = ccu.constraint_name
             AND tc.constraint_schema = ccu.constraint_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
              AND tc.constraint_schema = current_schema()
              AND ccu.table_name = 'knowledge_wiki_projections'
            "#,
        )
        .fetch_one(&mut conn)
        .await
        .expect("catalog query");
        assert_eq!(
            inbound, 0,
            "spec 2.3.13.11: no FK may point from authority into projection content"
        );
    }
}
