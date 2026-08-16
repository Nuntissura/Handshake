use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, ScalarValue, SurrealStorage, SurrealStorageConfig,
    SurrealTestInspectorError, EXPECTED_SCHEMA_INFO_SHA256,
};
use handshake_core::storage::{NewDocument, NewWorkspace, WriteContext};

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

    let context = WriteContext::system(Some("surreal-test-inspector-contract".to_owned()));
    let workspace = storage
        .create_workspace(
            &context,
            NewWorkspace {
                name: "Inspector workspace".to_owned(),
            },
        )
        .await
        .expect("create representative workspace through product API");
    let document = storage
        .create_document(
            &context,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "Inspector document".to_owned(),
            },
        )
        .await
        .expect("create representative document through product API");

    assert_eq!(
        inspector
            .row_count(&workspaces, RowFilter::All)
            .await
            .expect("count workspaces"),
        1
    );
    assert!(inspector
        .exists(
            &workspaces,
            RowFilter::FieldEquals {
                field: workspace_name.clone(),
                value: ScalarValue::from("Inspector workspace"),
            },
        )
        .await
        .expect("observe workspace by bound field value"));
    assert!(inspector
        .exists(&documents, RowFilter::IdEquals(document.id.clone()))
        .await
        .expect("observe document by bound record id"));

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
        .expect("project validated document fields");
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].record_id.table, "documents");
    assert_eq!(
        projected[0].record_id.key_string(),
        Some(document.id.as_str())
    );
    assert_eq!(
        projected[0].values.get("title"),
        Some(&serde_json::Value::String("Inspector document".to_owned()))
    );

    let referenced = inspector
        .referenced_ids(&document_reference, RowFilter::All)
        .await
        .expect("observe document workspace references");
    assert_eq!(referenced.len(), 1);
    assert_eq!(referenced[0].table, "workspaces");
    assert_eq!(referenced[0].key_string(), Some(workspace.id.as_str()));

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
