// WP-1 dual-substrate import adaptation: the reference worktree's version of
// this suite also proves write-backed observation (`create_workspace` /
// `create_document` through the product API, then inspector row observation).
// Those product APIs live in the workspaces/documents outer-module ports,
// which are intentionally NOT part of this storage-engine-seam import, and the
// inspector facade deliberately exposes no write path of its own. This
// adaptation keeps the full catalog/selector/reference/rejection contract and
// replaces the write-backed observations with empty-store observations; the
// write-backed assertions return with the outer-module port MTs.
use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, ScalarValue, SurrealStorage, SurrealStorageConfig,
    SurrealTestInspectorError, EXPECTED_SCHEMA_INFO_SHA256,
};

#[tokio::test]
async fn inspector_observes_catalog_rows_and_references_through_closed_selectors() {
    let temp = tempfile::tempdir().expect("create inspector data root");
    let config =
        SurrealStorageConfig::for_data_dir(temp.path()).expect("configure inspector store");
    let storage = SurrealStorage::open(config)
        .await
        .expect("open inspector store");
    let bootstrap = bootstrap_schema(&storage)
        .await
        .expect("bootstrap inspector schema");
    let inspector = storage.test_inspector();

    let catalog = inspector
        .schema_catalog()
        .await
        .expect("inspect schema catalog");
    assert_eq!(catalog.schema_version, bootstrap.schema_version);
    assert_eq!(catalog.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
    assert_eq!(catalog.tables_defined, catalog.tables.len());
    assert_eq!(
        catalog.fields_defined,
        catalog
            .tables
            .iter()
            .map(|table| table.fields.len())
            .sum::<usize>()
    );
    assert_eq!(
        catalog.indexes_defined,
        catalog
            .tables
            .iter()
            .map(|table| table.indexes.len())
            .sum::<usize>()
    );

    let table_names = inspector.table_names().await.expect("inspect table names");
    for expected in [
        "workspaces",
        "documents",
        "loom_blocks",
        "kernel_event_ledger",
    ] {
        assert!(table_names.iter().any(|table| table == expected));
    }

    let workspaces = inspector
        .table_selector("workspaces")
        .await
        .expect("select workspaces catalog capability");
    let documents = inspector
        .table_selector("documents")
        .await
        .expect("select documents catalog capability");
    let workspace_name = workspaces.field("name").expect("select workspace name");
    let document_title = documents.field("title").expect("select document title");
    let document_workspace = documents
        .field("workspace_id")
        .expect("select document workspace reference");

    let references = inspector
        .references_to(&workspaces)
        .await
        .expect("inspect references to workspaces");
    let document_reference = references
        .iter()
        .find(|reference| {
            reference.source_table() == "documents"
                && reference.source_field() == "workspace_id"
                && reference.target_table() == "workspaces"
        })
        .expect("documents.workspace_id references workspaces")
        .clone();
    assert_eq!(document_reference.on_delete(), "CASCADE");

    // Empty-store observations: no write path exists in this import slice, so
    // every row observation must report the bootstrapped-but-empty state.
    assert_eq!(
        inspector
            .row_count(&workspaces, RowFilter::All)
            .await
            .expect("count workspaces"),
        0
    );
    assert!(!inspector
        .exists(
            &workspaces,
            RowFilter::FieldEquals {
                field: workspace_name.clone(),
                value: ScalarValue::from("Inspector workspace"),
            },
        )
        .await
        .expect("observe workspace absence by bound field value"));

    let projected = inspector
        .project(
            &documents,
            &[document_title.clone(), document_workspace],
            RowFilter::FieldEquals {
                field: document_title,
                value: ScalarValue::from("Inspector document"),
            },
        )
        .await
        .expect("project validated document fields over an empty table");
    assert!(projected.is_empty());

    let referenced = inspector
        .referenced_ids(&document_reference, RowFilter::All)
        .await
        .expect("observe document workspace references over an empty table");
    assert!(referenced.is_empty());

    let event_table = inspector
        .table_selector("kernel_event_ledger")
        .await
        .expect("select representative event table");
    assert_eq!(
        inspector
            .row_count(&event_table, RowFilter::All)
            .await
            .expect("count representative event rows"),
        0
    );

    storage.shutdown().await.expect("close inspector store");
}

#[tokio::test]
async fn inspector_rejects_unstructured_identifiers_and_cross_table_fields() {
    let temp = tempfile::tempdir().expect("create inspector rejection data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path())
        .expect("configure inspector rejection store");
    let storage = SurrealStorage::open(config)
        .await
        .expect("open inspector rejection store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap inspector rejection schema");
    let inspector = storage.test_inspector();

    assert!(matches!(
        inspector
            .table_selector("workspaces; DELETE workspaces")
            .await,
        Err(SurrealTestInspectorError::UnsafeIdentifier(_))
    ));
    let workspaces = inspector
        .table_selector("workspaces")
        .await
        .expect("select workspaces capability");
    assert!(matches!(
        workspaces.field("name.*"),
        Err(SurrealTestInspectorError::UnsafeIdentifier(_))
    ));
    let documents = inspector
        .table_selector("documents")
        .await
        .expect("select documents capability");
    let document_title = documents.field("title").expect("select document title");
    assert!(matches!(
        inspector
            .project(&workspaces, &[document_title], RowFilter::All)
            .await,
        Err(SurrealTestInspectorError::SelectorTableMismatch { .. })
    ));

    storage
        .shutdown()
        .await
        .expect("close inspector rejection store");
}
