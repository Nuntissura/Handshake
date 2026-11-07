mod revision166_upgrade {
    use super::*;
    use std::sync::Arc;

    #[derive(Deserialize)]
    struct Source {
        name: String,
        sql: String,
        sha256: String,
    }
    #[derive(Deserialize)]
    struct Seed {
        sql: String,
        sha256: String,
    }
    #[derive(Deserialize)]
    struct Receipt {
        module: String,
        schema_version: String,
        schema_revision: i64,
        sql: String,
        sha256: String,
    }
    #[derive(Deserialize)]
    struct Installer {
        production_begin: String,
        finalize: String,
        model_lane_backfill: String,
        model_lane_empty_sessions: String,
    }
    #[derive(Deserialize)]
    struct Fixture {
        source_commit: String,
        sources: Vec<Source>,
        registry_seed: Seed,
        receipts: Vec<Receipt>,
        installer: Installer,
    }
    #[derive(Debug, PartialEq, SurrealValue)]
    struct ModuleWitness {
        schema_version: String,
        schema_revision: i64,
        apply_state: String,
    }
    #[derive(SurrealValue)]
    struct ModuleBindings {
        schema_version: String,
        schema_revision: i64,
    }

    fn fixture() -> Arc<Fixture> {
        let json = include_str!("../../../tests/fixtures/schema_revision_166_first_pin.json");
        assert_eq!(
            sha256_hex(json.as_bytes()),
            "73a69a3c1ce9b636839f0ece83edaeb33d1bfd41dfb9a348994517b5f9c96039"
        );
        let fixture: Fixture =
            serde_json::from_str(json).expect("exact historical first-pin fixture");
        assert_eq!(
            fixture.source_commit,
            "fb5624af0cfaaf0e54aad0c2ca2f80a5164f9607"
        );
        assert_eq!(fixture.sources.len(), revision166::SOURCE_PINS.len());
        for (source, (name, hash)) in fixture.sources.iter().zip(revision166::SOURCE_PINS) {
            assert_eq!(source.name, *name);
            assert_eq!(source.sha256, *hash);
            assert_eq!(sha256_hex(source.sql.as_bytes()), *hash);
        }
        assert_eq!(
            sha256_hex(fixture.registry_seed.sql.as_bytes()),
            fixture.registry_seed.sha256
        );
        for receipt in &fixture.receipts {
            assert_eq!(sha256_hex(receipt.sql.as_bytes()), receipt.sha256);
        }
        assert_eq!(fixture.receipts.len(), 5);
        let sources = fixture
            .sources
            .iter()
            .map(|s| (s.name.as_str(), s.sql.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            sha256_hex(serde_json::to_vec(&sources).unwrap().as_slice()),
            revision166::PRODUCTION_GENERATED
        );
        let manifest = fixture
            .sources
            .iter()
            .map(|s| (s.name.as_str(), s.sha256.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            sha256_hex(
                serde_json::to_vec(&(
                    SCHEMA_VERSION,
                    PRE_ROUTING_REVISION,
                    SCHEMA_LINEAGE_SHA256,
                    manifest
                ))
                .unwrap()
                .as_slice()
            ),
            revision166::PRODUCTION_SOURCE
        );
        Arc::new(fixture)
    }

    fn declarations(source: &str) -> BTreeMap<String, String> {
        let mut declarations = BTreeMap::new();
        let source = source.replace(";DEFINE", ";\nDEFINE");
        for (offset, _) in source.match_indices("DEFINE ") {
            if offset != 0 && source.as_bytes()[offset - 1] != b'\n' {
                continue;
            }
            let end = surql_statement_end(&source, offset).expect("complete declaration");
            let statement = &source[offset..end];
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            let kind = tokens[1];
            let if_absent = tokens[2..].starts_with(&["IF", "NOT", "EXISTS"]);
            let index = if if_absent {
                5
            } else {
                assert_eq!(tokens[2], "OVERWRITE");
                3
            };
            let name = tokens[index].trim_end_matches(';');
            let identity = match kind {
                "FIELD" | "INDEX" | "EVENT" => {
                    assert_eq!(tokens[index + 1], "ON");
                    let table = tokens[index + if tokens[index + 2] == "TABLE" { 3 } else { 2 }]
                        .trim_end_matches(';');
                    format!("{}:{table}:{name}", kind.to_ascii_lowercase())
                }
                "FUNCTION" => format!("function:{}", declarative_function_name(name).unwrap()),
                _ => format!("{}:{name}", kind.to_ascii_lowercase()),
            };
            if !if_absent || !declarations.contains_key(&identity) {
                declarations.insert(identity, statement.to_owned());
            }
        }
        declarations
    }

    fn catalog(sources: &[(&str, &str)]) -> String {
        let mut entries = BTreeSet::new();
        for (_, source) in sources {
            for (identity, statement) in declarations(source) {
                if let Some(table) = identity.strip_prefix("table:") {
                    if statement.contains(" TYPE NORMAL AS") {
                        entries.insert(format!("view:{table}"));
                    }
                }
                entries.insert(identity);
            }
        }
        compute_catalog_hash(&entries.into_iter().collect::<Vec<_>>())
    }

    #[test]
    fn exact_first_pin_tuple_and_complete_declaration_delta_are_preserved() {
        let fixture = fixture();
        let old = fixture
            .sources
            .iter()
            .map(|s| (s.name.as_str(), s.sql.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(catalog(&old[..1]), revision166::BASE_CATALOG);
        assert_eq!(catalog(&old), revision166::PRODUCTION_CATALOG);
        assert_eq!(
            revision166::predecessor_catalog_sha256(false).unwrap(),
            revision166::BASE_CATALOG
        );
        assert_eq!(
            revision166::predecessor_catalog_sha256(true).unwrap(),
            revision166::PRODUCTION_CATALOG
        );
        let current = production_schema_sources();
        assert_eq!(old.len(), current.len());
        let emitted = format!(
            "{}\n{}\n{}",
            revision166::upgrade_statements(true),
            include_str!("model_lane_schema.surql"),
            revision166::cloud_upgrade_statements(true)
        );
        let emitted = declarations(&emitted);
        let base_emitted = declarations(&revision166::upgrade_statements(false));
        let mut changed_sources = Vec::new();
        for ((name, previous), (current_name, next)) in old.iter().zip(&current) {
            assert_eq!(name, current_name);
            if previous == next {
                continue;
            }
            changed_sources.push(*name);
            let previous = declarations(previous);
            let next = declarations(next);
            assert!(
                previous.keys().all(|key| next.contains_key(key)),
                "removed historical declaration needs explicit migration: {name}"
            );
            if *name == "shared" {
                for (key, declaration) in next
                    .iter()
                    .filter(|(key, value)| previous.get(*key) != Some(*value))
                {
                    assert_eq!(
                        base_emitted.get(key),
                        Some(declaration),
                        "missing BASE declaration: {key}"
                    );
                }
            }
        }
        assert_eq!(
            changed_sources,
            ["shared", "model_registry", "model_lane", "cloud_model_lane"]
        );
        let previous = declarations(
            &old.iter()
                .map(|(_, sql)| *sql)
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let next = declarations(
            &current
                .iter()
                .map(|(_, sql)| *sql)
                .collect::<Vec<_>>()
                .join("\n"),
        );
        for (key, declaration) in next
            .iter()
            .filter(|(key, value)| previous.get(*key) != Some(*value))
        {
            assert_eq!(
                emitted.get(key),
                Some(declaration),
                "missing final production declaration: {key}"
            );
        }
        assert_ne!(revision166::BASE_GENERATED, PRE_ROUTING_BASE_GENERATED);
        assert_ne!(
            revision166::PRODUCTION_GENERATED,
            PRE_ROUTING_PRODUCTION_GENERATED
        );
        assert_ne!(
            revision166::PRODUCTION_SOURCE,
            PRE_ROUTING_PRODUCTION_SOURCE
        );
        let config = SurrealStorageConfig::for_data_dir(std::env::temp_dir()).unwrap();
        for production in [false, true] {
            let state = SchemaState {
                version: SCHEMA_VERSION.to_owned(),
                revision: 166,
                target_revision: 166,
                namespace: config.namespace().to_owned(),
                database: config.database().to_owned(),
                source_manifest_sha256: if production {
                    revision166::PRODUCTION_SOURCE
                } else {
                    SCHEMA_LINEAGE_SHA256
                }
                .to_owned(),
                generated_surql_sha256: if production {
                    revision166::PRODUCTION_GENERATED
                } else {
                    revision166::BASE_GENERATED
                }
                .to_owned(),
                info_fingerprint_sha256: if production {
                    EXPECTED_PRODUCTION_SCHEMA_PIN.unwrap().0
                } else {
                    EXPECTED_SCHEMA_INFO_SHA256
                }
                .to_owned(),
                apply_state: "complete".to_owned(),
            };
            assert!(state.is_exact_pre_routing(production, &config));
            for field in 0..9 {
                let mut wrong = state.clone();
                match field {
                    0 => wrong.version.push_str("-wrong"),
                    1 => wrong.revision = 165,
                    2 => wrong.target_revision = 167,
                    3 => wrong.namespace.push_str("-wrong"),
                    4 => wrong.database.push_str("-wrong"),
                    5 => wrong.source_manifest_sha256 = PRE_ROUTING_PRODUCTION_SOURCE.to_owned(),
                    6 => {
                        wrong.generated_surql_sha256 = if production {
                            PRE_ROUTING_PRODUCTION_GENERATED
                        } else {
                            PRE_ROUTING_BASE_GENERATED
                        }
                        .to_owned()
                    }
                    7 => wrong.info_fingerprint_sha256 = PENDING_SCHEMA_INFO_SHA256.to_owned(),
                    8 => wrong.apply_state = "schema_applied".to_owned(),
                    _ => unreachable!(),
                }
                assert!(
                    !wrong.is_exact_pre_routing(production, &config),
                    "wrong tuple field {field}"
                );
            }
            assert!(!state.is_exact_pre_routing(!production, &config));
        }
    }

    fn base_bindings() -> BootstrapBindings {
        BootstrapBindings {
            schema_version: SCHEMA_VERSION.to_owned(),
            schema_revision: 166,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            database: DEFAULT_DATABASE.to_owned(),
            source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
            generated_surql_sha256: revision166::BASE_GENERATED.to_owned(),
        }
    }

    async fn observe_historical(
        database: &SurrealAdminContext<'_>,
        production: bool,
    ) -> Result<ObservedSchema, SurrealStorageError> {
        let observed = read_schema_catalog(database).await?;
        let (info, tables, fields, indexes) = if production {
            let pin = EXPECTED_PRODUCTION_SCHEMA_PIN.expect("retained historical INFO");
            (pin.0, 337, pin.1, pin.2)
        } else {
            (EXPECTED_SCHEMA_INFO_SHA256, 311, 3986, 859)
        };
        assert_eq!(
            (
                observed.info_fingerprint_sha256.as_str(),
                observed.tables_defined,
                observed.fields_defined,
                observed.indexes_defined
            ),
            (info, tables, fields, indexes)
        );
        Ok(observed)
    }

    async fn install(
        storage: &SurrealStorage,
        fixture: Arc<Fixture>,
        production: bool,
    ) -> Result<(), SurrealStorageError> {
        storage.with_admin_operation(move |database| Box::pin(async move {
            let script = fresh_bootstrap_script(&fixture.sources[0].sql).expect("split exact first-pin installer");
            database.query_bound(script.definitions, base_bindings()).await?;
            database.query_bound(script.indexes_and_rest, base_bindings()).await?;
            let expected = expected_knowledge_schema_registry_metadata().unwrap();
            let registry = read_knowledge_schema_registry_metadata(&database).await?;
            if registry != expected {
                assert!(registry.is_empty(), "historical installer must not replace divergent registry");
                database.query(fixture.registry_seed.sql.clone()).await?;
            }
            assert_eq!(read_knowledge_schema_registry_metadata(&database).await?, expected);
            let observed = observe_historical(&database, false).await?;
            let pending = read_context_and_state(&database).await?.expect("historical BASE receipt");
            assert_eq!(pending.apply_state, "schema_applied");
            assert_eq!(pending.info_fingerprint_sha256, PENDING_SCHEMA_INFO_SHA256);
            database.query_bound(fixture.installer.finalize.clone(), FinalizeBindings {
                schema_version: SCHEMA_VERSION.to_owned(), schema_revision: 166,
                namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(), generated_surql_sha256: revision166::BASE_GENERATED.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(), info_fingerprint_sha256: observed.info_fingerprint_sha256,
            }).await?;
            assert!(read_context_and_state(&database).await?.unwrap().is_exact_pre_routing(false, &database.config));
            if production {
                database.query_bound(fixture.installer.production_begin.clone(), ProductionBeginBindings {
                    schema_version: SCHEMA_VERSION.to_owned(), schema_revision: 166,
                    namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
                    base_source: SCHEMA_LINEAGE_SHA256.to_owned(), base_generated: revision166::BASE_GENERATED.to_owned(), base_info: EXPECTED_SCHEMA_INFO_SHA256.to_owned(),
                    source: revision166::PRODUCTION_SOURCE.to_owned(), catalog: revision166::PRODUCTION_GENERATED.to_owned(), pending: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                }).await?;
                let mut receipt_index = 0;
                for source in fixture.sources.iter().skip(1) {
                    database.query(source.sql.clone()).await?;
                    if source.name == "model_lane" {
                        database.query(fixture.installer.model_lane_backfill.clone()).await?;
                        let mut response = database.query(fixture.installer.model_lane_empty_sessions.clone()).await?;
                        let sessions: Vec<SurrealValueData> = response.take(0)?;
                        assert!(sessions.is_empty());
                    }
                    if fixture.receipts.get(receipt_index).is_some_and(|receipt| receipt.module == source.name) {
                        let receipt = &fixture.receipts[receipt_index];
                        let id = receipt.sql.split_whitespace().nth(1).unwrap();
                        let mut response = database.query(format!("SELECT * FROM ONLY {id};")).await?;
                        assert!(response.take::<Option<ModuleWitness>>(0)?.is_none());
                        database.query_bound(receipt.sql.clone(), ModuleBindings { schema_version: receipt.schema_version.clone(), schema_revision: receipt.schema_revision }).await?;
                        let mut response = database.query(format!("SELECT * FROM ONLY {id};")).await?;
                        assert_eq!(response.take::<Option<ModuleWitness>>(0)?, Some(ModuleWitness { schema_version: receipt.schema_version.clone(), schema_revision: receipt.schema_revision, apply_state: "complete".to_owned() }));
                        receipt_index += 1;
                    }
                }
                assert_eq!(receipt_index, 5);
                let observed = observe_historical(&database, true).await?;
                database.query_bound(fixture.installer.finalize.clone(), FinalizeBindings {
                    schema_version: SCHEMA_VERSION.to_owned(), schema_revision: 166,
                    namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: revision166::PRODUCTION_SOURCE.to_owned(), generated_surql_sha256: revision166::PRODUCTION_GENERATED.to_owned(),
                    pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(), info_fingerprint_sha256: observed.info_fingerprint_sha256,
                }).await?;
            }
            assert!(read_context_and_state(&database).await?.unwrap().is_exact_pre_routing(production, &database.config));
            database.query("CREATE workspaces:revision166_a CONTENT { name: 'first-pin sentinel A' }; CREATE workspaces:revision166_b CONTENT { name: 'first-pin sentinel B' };").await?;
            Ok(())
        })).await
    }

    async fn snapshot(
        storage: &SurrealStorage,
        fixture: Arc<Fixture>,
        production: bool,
    ) -> Result<Vec<SurrealValueData>, SurrealStorageError> {
        storage.with_admin_operation(move |database| Box::pin(async move {
            let mut sql = "SELECT * FROM ONLY handshake_schema_state:primary; SELECT * FROM knowledge_schema_registry ORDER BY id; SELECT * FROM workspaces ORDER BY id;".to_owned();
            if production { for receipt in &fixture.receipts {
                sql.push_str(&format!("SELECT * FROM ONLY {};", receipt.sql.split_whitespace().nth(1).unwrap()));
            }}
            let mut response = database.query(sql).await?;
            let mut rows = Vec::new();
            for index in 0..if production { 8 } else { 3 } { rows.push(response.take::<SurrealValueData>(index)?); }
            Ok(rows)
        })).await
    }

    #[derive(Clone, Copy)]
    enum Negative {
        None,
        MixedCatalog,
        WrongTuple,
    }

    async fn disk_case(production: bool, negative: Negative) {
        let fixture = fixture();
        let mut directory = tempfile::Builder::new()
            .prefix("mt172-first-pin-")
            .tempdir()
            .unwrap();
        directory.disable_cleanup(true);
        let mut stores = Vec::new();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let storage = open_test_storage(&directory).await?;
            stores.push(storage.clone());
            install(&storage, fixture.clone(), production).await?;
            let before = snapshot(&storage, fixture.clone(), production).await?;
            storage.shutdown().await?;
            drop(storage);
            let reopened = open_test_storage(&directory).await?;
            stores.push(reopened.clone());
            assert_eq!(snapshot(&reopened, fixture.clone(), production).await?, before, "durable166 reopen preserves all native rows");
            reopened.with_admin_operation(move |database| Box::pin(async move {
                let state = read_context_and_state(&database).await?.unwrap();
                assert!(state.is_exact_pre_routing(production, &database.config));
                observe_historical(&database, production).await?;
                Ok(())
            })).await?;
            if !matches!(negative, Negative::None) {
                reopened.with_admin_operation(move |database| Box::pin(async move {
                    match negative {
                        Negative::MixedCatalog => { database.query(schema_function_definition("wp012_model_lane_receipt")).await?; },
                        Negative::WrongTuple => { database.query_bound(
                            "UPDATE ONLY handshake_schema_state:primary SET generated_surql_sha256 = $wrong;",
                            ("wrong", PRE_ROUTING_PRODUCTION_GENERATED.to_owned()),
                        ).await?; },
                        Negative::None => unreachable!(),
                    }
                    Ok(())
                })).await?;
                let mixed_rows = snapshot(&reopened, fixture.clone(), production).await?;
                let mixed_info = reopened.with_admin_operation(|database| Box::pin(async move { Ok(read_schema_catalog(&database).await?.info_fingerprint_sha256) })).await?;
                let expected_error = match negative {
                    Negative::MixedCatalog => {
                        assert_ne!(mixed_info, EXPECTED_PRODUCTION_SCHEMA_PIN.unwrap().0);
                        "HANDSHAKE_SURREAL_PRE_ROUTING_CATALOG_MISMATCH"
                    },
                    Negative::WrongTuple => {
                        assert_eq!(mixed_info, EXPECTED_PRODUCTION_SCHEMA_PIN.unwrap().0);
                        "HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"
                    },
                    Negative::None => unreachable!(),
                };
                let error = super::super::super::bootstrap_production_schemas(&reopened).await.expect_err("historical INFO cannot authorize a mixed source or catalog");
                assert!(error.to_string().contains(expected_error), "{error}");
                assert_eq!(snapshot(&reopened, fixture.clone(), production).await?, mixed_rows);
                assert_eq!(reopened.with_admin_operation(|database| Box::pin(async move { Ok(read_schema_catalog(&database).await?.info_fingerprint_sha256) })).await?, mixed_info);
            } else {
                let report = if production { super::super::super::bootstrap_production_schemas(&reopened).await? } else { bootstrap_schema(&reopened).await? };
                assert_eq!(report.outcome, SchemaBootstrapOutcome::UpgradedSupportedPredecessor);
                let expected_info = if production { ROUTING_SUCCESSOR_PRODUCTION_INFO.unwrap().0 } else { ROUTING_SUCCESSOR_BASE_INFO.unwrap() };
                assert_eq!(report.info_fingerprint_sha256, expected_info);
                let after = snapshot(&reopened, fixture.clone(), production).await?;
                assert_eq!(after[1..3], before[1..3], "registry and sentinel data stay byte-for-native-value equal");
                if production {
                    assert_eq!(after[4..], before[4..], "four unrelated module receipts stay unchanged");
                    let SurrealValueData::Object(mut expected) = before[3].clone() else { panic!("native model receipt"); };
                    assert_eq!(expected.get("schema_version"), Some(&SurrealValue::into_value("mt003-model-lane-authority-v9".to_owned())));
                    assert_eq!(expected.get("schema_revision"), Some(&SurrealValue::into_value(9_i64)));
                    expected.insert("schema_version".to_owned(), SurrealValue::into_value("mt003-model-lane-authority-v11".to_owned()));
                    expected.insert("schema_revision".to_owned(), SurrealValue::into_value(11_i64));
                    assert_eq!(after[3], SurrealValueData::Object(expected), "only approved model receipt version changes");
                }
                reopened.with_admin_operation(move |database| Box::pin(async move {
                    let state = read_context_and_state(&database).await?.unwrap();
                    assert_eq!((state.revision, state.target_revision, state.apply_state.as_str()), (167, 167, "complete"));
                    assert_eq!(state.source_manifest_sha256, if production { production_source_sha256() } else { SCHEMA_LINEAGE_SHA256.to_owned() });
                    assert_eq!(state.generated_surql_sha256, if production { production_generated_sha256() } else { GENERATED_SURREALQL_SHA256.to_owned() });
                    assert_eq!(state.info_fingerprint_sha256, expected_info);
                    Ok(())
                })).await?;
                reopened.shutdown().await?;
                drop(reopened);
                let second = open_test_storage(&directory).await?;
                stores.push(second.clone());
                let repeated = if production { super::super::super::bootstrap_production_schemas(&second).await? } else { bootstrap_schema(&second).await? };
                assert!(repeated.reused_existing_schema);
                assert_eq!(repeated.info_fingerprint_sha256, expected_info);
                assert_eq!(snapshot(&second, fixture.clone(), production).await?, after, "durable167 reopen preserves complete native state");
            }
            Ok::<(), SurrealStorageError>(())
        })).await;
        let mut shutdown_failures = Vec::new();
        for storage in &stores {
            let result = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(
                storage.shutdown_with_report(),
            ))
            .await;
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => shutdown_failures.push(error.to_string()),
                Err(_) => shutdown_failures.push("shutdown panicked".to_owned()),
            }
        }
        drop(stores);
        if !shutdown_failures.is_empty() {
            eprintln!(
                "MT172_FIRST_PIN_STORE_PRESERVED {}",
                directory.path().display()
            );
            panic!(
                "first-pin proof shutdown failures: {shutdown_failures:?}; body_failed={}",
                !matches!(body, Ok(Ok(())))
            );
        }
        match body {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                eprintln!(
                    "MT172_FIRST_PIN_STORE_PRESERVED {}",
                    directory.path().display()
                );
                panic!("first-pin proof: {error}");
            }
            Err(payload) => {
                eprintln!(
                    "MT172_FIRST_PIN_STORE_PRESERVED {}",
                    directory.path().display()
                );
                std::panic::resume_unwind(payload);
            }
        }
        let path = directory.path().to_path_buf();
        directory
            .close()
            .expect("remove closed first-pin proof store");
        assert!(!path.exists());
        eprintln!("MT172_FIRST_PIN_STORE_REMOVED {}", path.display());
    }

    #[tokio::test]
    async fn revision_166_first_pin_upgrade_preserves_data_and_restarts_current() {
        for production in [false, true] {
            disk_case(production, Negative::None).await;
        }
    }

    #[tokio::test]
    async fn revision_166_first_pin_mixed_catalog_is_rejected_without_mutation() {
        disk_case(true, Negative::MixedCatalog).await;
    }

    #[tokio::test]
    async fn revision_166_first_pin_mixed_source_tuple_is_rejected_without_mutation() {
        disk_case(true, Negative::WrongTuple).await;
    }
}
