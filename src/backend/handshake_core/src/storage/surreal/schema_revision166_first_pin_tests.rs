#[tokio::test]
async fn revision_166_first_pin_info_matches_historical_catalog() {
    const FIRST_PIN_REVISION: i64 = 166;
    const FIRST_PIN_BASE_GENERATED: &str = "86d70b003a031c84163076b8c99bc8ca957696302907ce88b4723c617a5a944f";
    const FIRST_PIN_PRODUCTION_GENERATED: &str = "29d1e0d7442159f01f69e3f7a2cc406baaa96475c3e302728e1f8c0f92b2689b";
    const FIRST_PIN_PRODUCTION_SOURCE: &str = "40a646d1e860cc0bd59e319b0a2b973cd9f589d4d080dd0243ea2713505da900";
    const HISTORICAL_COMMIT: &str = "fb5624af0cfaaf0e54aad0c2ca2f80a5164f9607";
    assert_eq!(SCHEMA_VERSION, "wp-kernel-012-surreal-v1");
    assert_eq!(SCHEMA_LINEAGE_SHA256, "225ed19c0259ef121867ca5da1995813db0c48ee0cbfaded2d871e47b50f7fc1");
    assert_eq!(sha256_hex(include_str!("../../../Cargo.lock").as_bytes()), "3f0943e207d51cb3ad585b840f0fce9169d29bc39e33148e8ef02d80c96cd4f6");
    #[derive(serde::Deserialize)]
    struct FrozenSource { name: String, sql: String, sha256: String, origin: String }
    #[derive(serde::Deserialize)]
    struct FrozenSeed { sql: String, sha256: String }
    #[derive(serde::Deserialize)]
    struct FrozenReceipt { module: String, schema_version: String, schema_revision: i64, sql: String, sha256: String }
    #[derive(serde::Deserialize)]
    struct FrozenInstaller { production_begin: String, finalize: String, model_lane_backfill: String, model_lane_empty_sessions: String }
    #[derive(serde::Deserialize)]
    struct Frozen166 { source_commit: String, sources: Vec<FrozenSource>, registry_seed: FrozenSeed, receipts: Vec<FrozenReceipt>, installer: FrozenInstaller }
    const FIRST_PIN_SOURCES: &[(&str, &str)] = &[
    ("shared", "86d70b003a031c84163076b8c99bc8ca957696302907ce88b4723c617a5a944f"),
    ("model_registry", "e68d2abce933d715798a458a7aa70e0d09752707d3226178bafb2a39005955d6"),
    ("loom_search", "55838a2b94e7eca696de04aadf4ce5b70b9a29562f6ee01f8b0e0f1f46d8bce8"),
    ("model_lane_state", "529bfea18e6f6c3ecedcfe8406594fac95867e9300ce978faf508869e0358450"),
    ("model_lane", "f486549c1344b93649b615ac2235fc7370c12be53a6a302be184fefdad6f6341"),
    ("cloud_model_lane_state", "d55b974bf7a46de6b27a43b38f31b8bc99f5b2f882189203b440d3359cc18ce7"),
    ("cloud_model_lane", "ec417603c3520794a829b2f95e03b570f371db8b2b743ddbb7c61146dfb69b77"),
    ("process_ledger", "88fcf556e38da0e1b281ea6eb54a0a07bf22886296a67776c9a2c0e4d272a1a1"),
    ("palmistry_state", "4ecef5ccae3e59e24161610114b995462388f3a0dfdb4f01f9b138a36f563b6f"),
    ("palmistry", "55fbeb9c36c1bc9bdaf7b2f9d152bca37860f8b143d895202fee4ab494928f57"),
    ("swarm_outbox_state", "06569d29f38fce05c16c439d1a20d8ceea4c0bcb627be306f8cab388e5cd4b51"),
    ("swarm_outbox", "1436a9b91a2ccb6907d4f8c8742090d227708392aa969212f840ff9986f1042f"),
    ("user_manual_knowledge", "abd49a97afbda36eec818a04fc08ffd5f51f88f8ff8e42ea19fcb7e937985805"),
    ("workspace_settings_state", "16f1ad96ee279f6b18b8fab145f457b2485669da6f5c8c562aedd789b11ccc5e"),
    ("workspace_settings", "e04efa94b38a2749903a4dd39f1aacd54f2a86b8834ffb27ad5937cf8bcb0d0c"),
    ];
    let fixture_json = include_str!("../../../tests/fixtures/schema_revision_166_first_pin.json");
    assert_eq!(sha256_hex(fixture_json.as_bytes()), "73a69a3c1ce9b636839f0ece83edaeb33d1bfd41dfb9a348994517b5f9c96039");
    let fixture: Frozen166 = serde_json::from_str(fixture_json)
        .expect("decode historical first-pin diagnostic fixture");
    assert_eq!(fixture.source_commit, "fb5624af0cfaaf0e54aad0c2ca2f80a5164f9607");
    assert_eq!(fixture.sources.len(), FIRST_PIN_SOURCES.len());
    for (source, (name, expected_sha)) in fixture.sources.iter().zip(FIRST_PIN_SOURCES) {
        assert_eq!(source.name, *name);
        assert_eq!(source.sha256, *expected_sha);
        assert_eq!(sha256_hex(source.sql.as_bytes()), *expected_sha);
    }
    assert_eq!(sha256_hex(fixture.registry_seed.sql.as_bytes()), "64d0711c5273c6eb103c3d574b2f7ee98d9d0ebfd46e9c25ad65908b46573b75");
    assert_eq!(sha256_hex(fixture.sources[0].sql.as_bytes()), FIRST_PIN_BASE_GENERATED);
    let source_pairs = fixture.sources.iter().map(|s| (s.name.as_str(), s.sql.as_str())).collect::<Vec<_>>();
    assert_eq!(sha256_hex(serde_json::to_string(&source_pairs).unwrap().as_bytes()), FIRST_PIN_PRODUCTION_GENERATED);
    let manifest = fixture.sources.iter().map(|s| (s.name.as_str(), sha256_hex(s.sql.as_bytes()))).collect::<Vec<_>>();
    assert_eq!(sha256_hex(serde_json::to_string(&(SCHEMA_VERSION, FIRST_PIN_REVISION, SCHEMA_LINEAGE_SHA256, manifest)).unwrap().as_bytes()), FIRST_PIN_PRODUCTION_SOURCE);

    assert_eq!(sha256_hex(fixture.installer.production_begin.as_bytes()), "813abd98bfbb9834d6cd1a0cfb45bab15e5ce30af28f4af5bd69f7722e1f7aa8");
    assert_eq!(sha256_hex(fixture.installer.finalize.as_bytes()), "8058dab573fee5f4bc128dc371926ad073b5a5669e494ec2044f93b6e623ed0b");
    assert_eq!(sha256_hex(fixture.installer.model_lane_backfill.as_bytes()), "78bbfb97a6eee0b36ee0747d943bbb970bd52a83c568c3aea478c07e27919abb");
    assert_eq!(sha256_hex(fixture.installer.model_lane_empty_sessions.as_bytes()), "0c8ce52ebff0e67350fca4ddf499fbc282a6ca665fbe451c4dd8a7b6f93b993e");
    assert_eq!((&fixture.receipts[0].module, &fixture.receipts[0].schema_version, fixture.receipts[0].schema_revision), (&"model_lane".to_owned(), &"mt003-model-lane-authority-v9".to_owned(), 9));
    assert_eq!(sha256_hex(fixture.receipts[0].sql.as_bytes()), "5ecbaed8931e03da0c5dd7cc6753dfb27bfc18f5a4990d94406678133ebacc3d");
    assert_eq!((&fixture.receipts[1].module, &fixture.receipts[1].schema_version, fixture.receipts[1].schema_revision), (&"cloud_model_lane".to_owned(), &"mt006-cloud-authority-v1".to_owned(), 1));
    assert_eq!(sha256_hex(fixture.receipts[1].sql.as_bytes()), "8c8868439782a7fd7421faa3b72b67c283afdfdec6553abd1e0f2cbf6e9cebce");
    assert_eq!((&fixture.receipts[2].module, &fixture.receipts[2].schema_version, fixture.receipts[2].schema_revision), (&"palmistry".to_owned(), &"mt003-palmistry-authority-v1".to_owned(), 1));
    assert_eq!(sha256_hex(fixture.receipts[2].sql.as_bytes()), "bcd776971e853fd075db173209b3140189af5c80e057aed19c32212ea7b011bc");
    assert_eq!((&fixture.receipts[3].module, &fixture.receipts[3].schema_version, fixture.receipts[3].schema_revision), (&"swarm_outbox".to_owned(), &"mt003-swarm-terminal-outbox-v1".to_owned(), 1));
    assert_eq!(sha256_hex(fixture.receipts[3].sql.as_bytes()), "38d5c12c198bb35a4a368029cf6de69309ab2b7f7c072882deef7b6cea982c6f");
    assert_eq!((&fixture.receipts[4].module, &fixture.receipts[4].schema_version, fixture.receipts[4].schema_revision), (&"workspace_settings".to_owned(), &"mt021-workspace-settings-authority-v1".to_owned(), 1));
    assert_eq!(sha256_hex(fixture.receipts[4].sql.as_bytes()), "bd1d138e04c757efb09187cc48a6d6ba13a3d4b962c87c7e8af7992592eb4cdd");
    assert_eq!(fixture.receipts.len(), 5);
    assert_eq!(fixture.registry_seed.sha256, "64d0711c5273c6eb103c3d574b2f7ee98d9d0ebfd46e9c25ad65908b46573b75");
    assert_eq!(fixture.receipts[0].sha256, "5ecbaed8931e03da0c5dd7cc6753dfb27bfc18f5a4990d94406678133ebacc3d");
    assert_eq!(fixture.receipts[1].sha256, "8c8868439782a7fd7421faa3b72b67c283afdfdec6553abd1e0f2cbf6e9cebce");
    assert_eq!(fixture.receipts[2].sha256, "bcd776971e853fd075db173209b3140189af5c80e057aed19c32212ea7b011bc");
    assert_eq!(fixture.receipts[3].sha256, "38d5c12c198bb35a4a368029cf6de69309ab2b7f7c072882deef7b6cea982c6f");
    assert_eq!(fixture.receipts[4].sha256, "bd1d138e04c757efb09187cc48a6d6ba13a3d4b962c87c7e8af7992592eb4cdd");

    async fn observe_catalog(database: &SurrealAdminContext<'_>, stage: &'static str) -> Result<ObservedSchema, SurrealStorageError> {
        let observed = read_schema_catalog(database).await?;
        let mut db_response = database.query("INFO FOR DB STRUCTURE;").await?;
        let db_info: SurrealValueData = db_response.take(0)?;
        eprintln!("MT139_SCHEMA_RAW_INFO {}", serde_json::json!({
            "schema": "handshake.schema_info_measurement_raw@1", "stage": stage,
            "scope": "database", "engine_value": &db_info
        }));
        let query = observed.table_names.iter()
            .map(|table| format!("INFO FOR TABLE `{table}` STRUCTURE;"))
            .collect::<String>();
        let mut table_responses = database.query(query).await?;
        let mut tables = BTreeMap::new();
        for (index, table) in observed.table_names.iter().enumerate() {
            let info: SurrealValueData = table_responses.take(index)?;
            eprintln!("MT139_SCHEMA_RAW_INFO {}", serde_json::json!({
                "schema": "handshake.schema_info_measurement_raw@1", "stage": stage,
                "scope": "table", "table": table, "engine_value": &info
            }));
            tables.insert(table.clone(), info);
        }
        let raw_fingerprint = canonical_catalog_fingerprint(db_info, tables);
        if raw_fingerprint != observed.info_fingerprint_sha256 {
            return Err(SurrealStorageError::TransactionWorker(format!(
                "MT139_MEASUREMENT_CATALOG_CHANGED: stage={stage}; observed={}; raw={raw_fingerprint}",
                observed.info_fingerprint_sha256)));
        }
        eprintln!("MT139_SCHEMA_OBSERVATION {}", serde_json::json!({
            "schema": "handshake.schema_info_measurement@1", "stage": stage,
            "schema_version": SCHEMA_VERSION, "schema_revision": FIRST_PIN_REVISION,
            "info_sha256": observed.info_fingerprint_sha256,
            "tables": observed.tables_defined, "fields": observed.fields_defined,
            "indexes": observed.indexes_defined,
            "base_generated_sha256": FIRST_PIN_BASE_GENERATED,
            "production_source_sha256": FIRST_PIN_PRODUCTION_SOURCE,
            "production_generated_sha256": FIRST_PIN_PRODUCTION_GENERATED,
            "qualification": "unqualified_engine_observation",
            "diagnostic_scope": "historical_first_pin_catalog_only",
            "source_commit": HISTORICAL_COMMIT,
            "current_lock_sha256": "3f0943e207d51cb3ad585b840f0fce9169d29bc39e33148e8ef02d80c96cd4f6",
            "historical_lock_sha256": "a507b3a253ac53072b9e54ed4ea44810eda66b32918966ec51e1193ef657919f"
        }));
        Ok(observed)
    }

    #[derive(Debug, PartialEq, SurrealValue)]
    struct ModuleWitness { schema_version: String, schema_revision: i64, apply_state: String }
    #[derive(SurrealValue)]
    struct ModuleBindings { schema_version: String, schema_revision: i64 }
    let client = Surreal::new::<Mem>(()).await.expect("open first-pin diagnostic memory store");
    client.use_ns(DEFAULT_NAMESPACE).use_db(DEFAULT_DATABASE).await.expect("select diagnostic memory context");
    let database = SurrealAdminContext {
        client: &client,
        config: SurrealStorageConfig::for_data_dir(std::env::temp_dir()).expect("default memory catalog context"),
    };
    let script = fresh_bootstrap_script(&fixture.sources[0].sql).expect("split exact historical first-pin SQL");
    for statement in [script.definitions, script.indexes_and_rest] {
        database.query_bound(statement, BootstrapBindings {
            schema_version: SCHEMA_VERSION.to_owned(), schema_revision: FIRST_PIN_REVISION,
            namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
            source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(), generated_surql_sha256: FIRST_PIN_BASE_GENERATED.to_owned(),
        }).await.expect("apply historical BASE DDL without substitutions");
    }
    let expected_registry = expected_knowledge_schema_registry_metadata().expect("unchanged historical registry seed");
    let existing_registry = read_knowledge_schema_registry_metadata(&database).await.expect("read historical registry");
    if existing_registry != expected_registry {
        assert!(existing_registry.is_empty(), "do not overwrite divergent registry metadata");
        database.query(fixture.registry_seed.sql.clone()).await.expect("apply exact historical registry seed");
    }
    assert_eq!(read_knowledge_schema_registry_metadata(&database).await.expect("read seeded registry"), expected_registry);
    let base = observe_catalog(&database, "BASE").await.expect("retain actual historical BASE INFO");
    let pending = read_context_and_state(&database).await.expect("read pending diagnostic receipt").expect("historical BASE receipt");
    assert_eq!(pending.revision, FIRST_PIN_REVISION);
    assert_eq!(pending.apply_state, "schema_applied");
    assert_eq!(pending.info_fingerprint_sha256, PENDING_SCHEMA_INFO_SHA256);
    let mut receipt_index = 0;
    for source in fixture.sources.iter().skip(1) {
        database.query(source.sql.clone()).await.expect("apply exact ordered historical module DDL");
        if source.name == "model_lane" {
            database.query(fixture.installer.model_lane_backfill.clone()).await.expect("apply historical model-lane backfill");
            let mut response = database.query(fixture.installer.model_lane_empty_sessions.clone()).await.expect("inspect historical empty sessions");
            let sessions: Vec<SurrealValueData> = response.take(0).expect("decode historical sessions");
            assert!(sessions.is_empty(), "diagnostic memory store must contain no authenticated sessions");
        }
        if fixture.receipts.get(receipt_index).is_some_and(|receipt| receipt.module == source.name) {
            let receipt = &fixture.receipts[receipt_index];
            let id = receipt.sql.split_whitespace().nth(1).expect("historical receipt identity");
            let mut response = database.query(format!("SELECT * FROM ONLY {id};")).await.expect("inspect module receipt before DDL completion");
            let before: Option<ModuleWitness> = response.take(0).expect("decode absent module receipt");
            assert!(before.is_none(), "historical module receipt must be absent before installation");
            database.query_bound(receipt.sql.clone(), ModuleBindings {
                schema_version: receipt.schema_version.clone(), schema_revision: receipt.schema_revision,
            }).await.expect("apply exact historical module receipt after successful DDL");
            let mut response = database.query(format!("SELECT * FROM ONLY {id};")).await.expect("read installed module receipt");
            let after: Option<ModuleWitness> = response.take(0).expect("decode installed module receipt");
            assert_eq!(after, Some(ModuleWitness { schema_version: receipt.schema_version.clone(), schema_revision: receipt.schema_revision, apply_state: "complete".to_owned() }));
            receipt_index += 1;
        }
    }
    assert_eq!(receipt_index, 5);
    let final_catalog = observe_catalog(&database, "FINAL").await.expect("retain actual historical full production INFO");
    let retained = read_context_and_state(&database).await.expect("read retained pending receipt").expect("pending diagnostic receipt");
    assert_eq!(retained.apply_state, "schema_applied");
    assert_eq!(retained.info_fingerprint_sha256, PENDING_SCHEMA_INFO_SHA256);
    let expected_production = EXPECTED_PRODUCTION_SCHEMA_PIN.expect("unchanged retained historical production INFO");
    assert_eq!(EXPECTED_SCHEMA_INFO_SHA256, "6f3dfff2786829ca4c54d54680b763289cb17deaf82867ab1fe1d87cdaac4116");
    assert_eq!(expected_production, ("0115112c2aaa1e277d2add3da50a0f4dc7e2aebfe8b878fd81fd481a4f2d8a19", 4557, 935));
    assert_eq!(
        (base.info_fingerprint_sha256.as_str(), final_catalog.info_fingerprint_sha256.as_str(), final_catalog.fields_defined, final_catalog.indexes_defined),
        (EXPECTED_SCHEMA_INFO_SHA256, expected_production.0, expected_production.1, expected_production.2),
        "historical first-pin diagnostic must retain both raw catalogs and fail on either unchanged pin mismatch",
    );
}
