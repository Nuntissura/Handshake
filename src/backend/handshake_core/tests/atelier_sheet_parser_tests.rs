//! WP-KERNEL-005 MT-008 typed sheet template parser proof.
//!
//! This test uses a live PostgreSQL `DATABASE_URL` and the canonical kernel
//! EventLedger. It proves sheet text is no longer stored as opaque raw text
//! only: parsing a sheet version persists a typed AST snapshot and emits a
//! leak-safe EventLedger row that later MTs can build selective editing,
//! block-list storage, and unmapped-text preservation on.

mod atelier_pg_support;

use handshake_core::atelier::{
    event_family, AtelierError, AtelierStore, NewCharacter, NewSheetVersion, ParsedSheetFieldType,
    SheetFieldEdit, SheetFieldEditRequest, SheetFieldSelector, SheetVersionRevertRequest,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

fn temp_search_path_url(url: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}options=-csearch_path%3Dpg_temp")
}

async fn assert_parse_event_has_canonical_projection_link(
    store: &AtelierStore,
    version_id: Uuid,
    template_id: &str,
) {
    let database = PostgresDatabase::new(store.pool().clone());
    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &version_id.to_string())
        .await
        .expect("list kernel events for parsed sheet version");
    let parse_event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.source_component == "atelier"
                && event.payload["event_family"] == event_family::SHEET_TEMPLATE_PARSED
                && event.payload["atelier_payload"]["template_id"] == template_id
        })
        .expect("typed sheet parse appends a canonical kernel EventLedger row");

    let projection: (Option<String>, Option<i64>) = sqlx::query_as(
        r#"SELECT kernel_event_id, kernel_event_sequence
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = $2
             AND aggregate_id = $3
             AND payload->>'template_id' = $4
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(event_family::SHEET_TEMPLATE_PARSED)
    .bind("atelier_sheet_version")
    .bind(version_id.to_string())
    .bind(template_id)
    .fetch_one(store.pool())
    .await
    .expect("read atelier projection linkage for parsed sheet version");

    assert_eq!(
        projection.0.as_deref(),
        Some(parse_event.event_id.as_str()),
        "atelier_event projection links to the canonical kernel event id"
    );
    assert_eq!(
        projection.1,
        Some(parse_event.event_sequence),
        "atelier_event projection links to the canonical kernel event sequence"
    );
}

#[tokio::test]
async fn atelier_sheet_template_parser_persists_typed_ast_and_eventledger_evidence() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_template_parser_persists_typed_ast_and_eventledger_evidence: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-parser-character-{}", Uuid::new_v4()),
            display_name: "Typed Sheet Parser Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
IDENTITY

   
CHAR-ID-002 — Name: <string>
CHAR-ID-003 — Alias-Name: <string>
CHAR-BODY-001 — Body Type: <slim|curvy|other:<descriptor>|optional>
CHAR-SCORE-001 — Visual Score: <score_10|unset>
CHAR-ALT-001 — Alternate Metric: <string|number|none>
CHAR-AGE-001 — Age: <integer|adult>
CHAR-UNK-001 — Mystery: <unknown>
CHAR-RULE-001 — Rule Text: <rule> (Preserve inline rule prose)
CHAR-BLOCK-001 — Profile Block: <Face_Profile_Block | optional>
Hustle_Block
CHAR-HUS-001 — Hustle Label: <descriptor>
HUSTLES
CHAR-HUS-ROOT-Hustles: <list of Hustle_Block|optional>
Freeform note without field
";
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-008-red-test".to_string()),
        })
        .await
        .expect("append sheet version");

    let parsed = store
        .parse_sheet_template_version(
            sheet.version_id,
            "wp-kernel-005-mt-008",
            Some("test://wp-kernel-005/mt-008"),
        )
        .await
        .expect("parse sheet template version");

    assert_eq!(parsed.version_id, sheet.version_id);
    assert_eq!(parsed.template_id, "wp-kernel-005-mt-008");
    assert_eq!(parsed.ast.template_version.as_deref(), Some("2.00"));
    assert_eq!(parsed.ast.template_hash.len(), 64, "sha256 hex hash");
    assert!(parsed
        .ast
        .sections
        .iter()
        .any(|section| section.name == "IDENTITY"));
    assert!(parsed
        .ast
        .block_schemas
        .iter()
        .any(|schema| schema.name == "Hustle_Block"));

    let name_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ID-002")
        .expect("name field parsed");
    assert!(matches!(
        &name_field.field_type,
        ParsedSheetFieldType::String
    ));
    assert_eq!(name_field.section.as_deref(), Some("IDENTITY"));
    assert_eq!(name_field.line_number, 5);
    assert!(
        name_field.byte_start < name_field.byte_end,
        "mapped field carries source byte span"
    );
    assert_eq!(
        std::str::from_utf8(&raw_sheet.as_bytes()[name_field.byte_start..name_field.byte_end])
            .expect("mapped field source bytes are valid UTF-8"),
        "CHAR-ID-002 — Name: <string>"
    );
    assert_eq!(name_field.raw, "CHAR-ID-002 — Name: <string>");
    let alias_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ID-003")
        .expect("plain-hyphen field parsed");
    assert!(matches!(
        &alias_field.field_type,
        ParsedSheetFieldType::String
    ));
    assert_eq!(alias_field.label, "Alias-Name");

    let body_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-BODY-001")
        .expect("enum-with-other field parsed");
    let ParsedSheetFieldType::Enum {
        values,
        allow_other_type,
        allowed_special_values,
    } = &body_field.field_type
    else {
        panic!("body field must be an enum-with-other field");
    };
    assert_eq!(values, &vec!["slim".to_string(), "curvy".to_string()]);
    assert_eq!(allow_other_type.as_deref(), Some("descriptor"));
    assert_eq!(allowed_special_values, &vec!["optional".to_string()]);
    assert!(body_field.optional);

    let score_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-SCORE-001")
        .expect("score field parsed");
    let ParsedSheetFieldType::Score10 {
        allowed_special_values,
    } = &score_field.field_type
    else {
        panic!("score field must be score_10");
    };
    assert_eq!(allowed_special_values, &vec!["unset".to_string()]);

    let union_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ALT-001")
        .expect("primary-type union field parsed");
    let ParsedSheetFieldType::Union {
        variants,
        allowed_special_values,
    } = &union_field.field_type
    else {
        panic!("primary-type union field must be emitted as Union");
    };
    assert!(matches!(variants[0], ParsedSheetFieldType::String));
    assert!(matches!(variants[1], ParsedSheetFieldType::Number));
    assert_eq!(allowed_special_values, &vec!["none".to_string()]);

    let age_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-AGE-001")
        .expect("integer sentinel field parsed");
    assert!(matches!(
        &age_field.field_type,
        ParsedSheetFieldType::Integer
    ));
    assert_eq!(age_field.allowed_special_values, vec!["adult".to_string()]);

    let unknown_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-UNK-001")
        .expect("unknown-only field parsed");
    assert!(matches!(
        &unknown_field.field_type,
        ParsedSheetFieldType::String
    ));
    assert_eq!(
        unknown_field.allowed_special_values,
        vec!["unknown".to_string()]
    );

    let rule_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-RULE-001")
        .expect("inline rule field parsed");
    assert!(matches!(&rule_field.field_type, ParsedSheetFieldType::Rule));

    let block_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-BLOCK-001")
        .expect("bare block field parsed");
    assert!(
        matches!(
            &block_field.field_type,
            ParsedSheetFieldType::Block { block_schema_name } if block_schema_name == "Face_Profile_Block"
        ),
        "bare Foo_Block descriptor is typed as a block field"
    );

    let block_list_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-HUS-ROOT")
        .expect("block-list field parsed");
    assert!(
        matches!(
            &block_list_field.field_type,
            ParsedSheetFieldType::BlockList { block_schema_name } if block_schema_name == "Hustle_Block"
        ),
        "list of Hustle_Block is typed as a block-list field"
    );

    let unmapped = parsed
        .ast
        .unmapped_lines
        .iter()
        .find(|line| line.raw == "Freeform note without field")
        .expect("unmapped freeform line preserved");
    assert_eq!(unmapped.line_number, 18);
    assert!(
        unmapped.byte_start < unmapped.byte_end,
        "unmapped line carries source byte span"
    );
    assert_eq!(
        std::str::from_utf8(&raw_sheet.as_bytes()[unmapped.byte_start..unmapped.byte_end])
            .expect("unmapped source bytes are valid UTF-8"),
        "Freeform note without field"
    );
    let blank_unmapped = parsed
        .ast
        .unmapped_lines
        .iter()
        .find(|line| line.line_number == 3 && line.raw.is_empty())
        .expect("blank unmapped line preserved with source byte span");
    assert_eq!(
        std::str::from_utf8(
            &raw_sheet.as_bytes()[blank_unmapped.byte_start..blank_unmapped.byte_end]
        )
        .expect("blank source bytes are valid UTF-8"),
        ""
    );
    let whitespace_unmapped = parsed
        .ast
        .unmapped_lines
        .iter()
        .find(|line| line.line_number == 4 && line.raw == "   ")
        .expect("whitespace-only unmapped line preserved with source byte span");
    assert_eq!(
        std::str::from_utf8(
            &raw_sheet.as_bytes()[whitespace_unmapped.byte_start..whitespace_unmapped.byte_end]
        )
        .expect("whitespace source bytes are valid UTF-8"),
        "   "
    );

    let ast_json: serde_json::Value = sqlx::query_scalar(
        "SELECT ast FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(sheet.version_id)
    .bind("wp-kernel-005-mt-008")
    .fetch_one(store.pool())
    .await
    .expect("persisted parse AST row");
    assert_eq!(ast_json["fields"][0]["id"], "CHAR-ID-002");
    assert_eq!(ast_json["fields"][0]["raw"], "CHAR-ID-002 — Name: <string>");
    assert_eq!(ast_json["fields"][0]["byte_start"], name_field.byte_start);
    assert_eq!(ast_json["fields"][0]["byte_end"], name_field.byte_end);
    assert_eq!(ast_json["fields"][4]["field_type"]["kind"], "union");
    assert!(ast_json["unmapped_lines"]
        .as_array()
        .expect("unmapped_lines is array")
        .iter()
        .any(|line| line["raw"] == ""));
    assert!(ast_json["unmapped_lines"]
        .as_array()
        .expect("unmapped_lines is array")
        .iter()
        .any(|line| line["raw"] == "   "));
    assert!(ast_json["unmapped_lines"]
        .as_array()
        .expect("unmapped_lines is array")
        .iter()
        .any(|line| line["raw"] == "Freeform note without field"));

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &sheet.version_id.to_string())
        .await
        .expect("list kernel events for parsed sheet version");
    let parse_event = kernel_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_TEMPLATE_PARSED)
        .expect("typed sheet parse appends a kernel EventLedger row");
    assert_eq!(
        parse_event.payload["atelier_payload"]["version_id"],
        sheet.version_id.to_string()
    );
    assert_eq!(
        parse_event.payload["atelier_payload"]["template_id"],
        "wp-kernel-005-mt-008"
    );
    assert_eq!(parse_event.payload["atelier_payload"]["field_count"], 11);
    assert_eq!(parse_event.payload["atelier_payload"]["unmapped_count"], 3);
    assert!(
        parse_event.payload["atelier_payload"]
            .get("raw_text")
            .is_none(),
        "EventLedger payload must not mirror raw sheet text"
    );
    assert_parse_event_has_canonical_projection_link(
        &store,
        sheet.version_id,
        "wp-kernel-005-mt-008",
    )
    .await;

    let invalid_score = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: sheet.version_id,
            template_id: "wp-kernel-005-mt-008".to_string(),
            source_path: Some("test://wp-kernel-005/mt-008".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-SCORE-001".to_string(),
                replacement_text: "<eleven>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-008-invalid-score".to_string()),
        })
        .await
        .expect_err("invalid score_10 replacement must be rejected");
    match invalid_score {
        AtelierError::Validation(message) => assert!(
            message.contains("invalid_score_10") && message.contains("CHAR-SCORE-001"),
            "invalid score rejection is structured and field-scoped: {message}"
        ),
        other => panic!("expected invalid-score validation denial, got {other:?}"),
    }

    let score_applied = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: sheet.version_id,
            template_id: "wp-kernel-005-mt-008".to_string(),
            source_path: Some("test://wp-kernel-005/mt-008".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-SCORE-001".to_string(),
                replacement_text: "7.5".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-008-normalize-score".to_string()),
        })
        .await
        .expect("valid score_10 replacement is normalized and applied");
    assert!(score_applied
        .version
        .raw_text
        .contains("CHAR-SCORE-001 — Visual Score: <7.5/10>"));
}

#[tokio::test]
async fn atelier_sheet_template_parser_uses_store_pool_not_private_ledger_handle() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_template_parser_uses_store_pool_not_private_ledger_handle: PostgreSQL unavailable"
        );
        return;
    };
    let projection_store = AtelierStore::connect(&url)
        .await
        .expect("connect projection store");
    projection_store
        .ensure_schema()
        .await
        .expect("ensure atelier schema");
    let character = projection_store
        .create_character(&NewCharacter {
            public_id: format!("sheet-parser-broken-ledger-{}", Uuid::new_v4()),
            display_name: "Broken Ledger Parser Character".to_string(),
        })
        .await
        .expect("create character");
    let sheet = projection_store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "CHARACTER TEMPLATE (v2.00)\nIDENTITY\nCHAR-ID-002 - Name: <string>\n"
                .to_string(),
            author: "operator".to_string(),
            tool: Some("mt-008-broken-ledger-test".to_string()),
        })
        .await
        .expect("append sheet version");

    let template_id = format!("wp-kernel-005-mt-008-broken-ledger-{}", Uuid::new_v4());
    let private_ledger_url = temp_search_path_url(&url);
    let private_ledger_pool = match PgPoolOptions::new()
        .max_connections(2)
        .connect(&private_ledger_url)
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!(
                "SKIP atelier_sheet_template_parser_uses_store_pool_not_private_ledger_handle: cannot connect sibling postgres database: {err}"
            );
            return;
        }
    };
    let private_ledger = PostgresDatabase::new(private_ledger_pool).into_arc();
    let parsing_store =
        AtelierStore::with_event_ledger(projection_store.pool().clone(), private_ledger);

    let parsed = parsing_store
        .parse_sheet_template_version(sheet.version_id, &template_id, None)
        .await
        .expect("parse uses the store pool canonical EventLedger, not the private handle");
    assert_eq!(parsed.version_id, sheet.version_id);
    assert_parse_event_has_canonical_projection_link(
        &parsing_store,
        sheet.version_id,
        &template_id,
    )
    .await;
}

#[tokio::test]
async fn atelier_sheet_template_parser_connect_store_records_canonical_eventledger() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_template_parser_connect_store_records_canonical_eventledger: PostgreSQL unavailable"
        );
        return;
    };
    let store = AtelierStore::connect(&url)
        .await
        .expect("connect projection-only store");
    store.ensure_schema().await.expect("ensure atelier schema");
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-parser-ledgerless-{}", Uuid::new_v4()),
            display_name: "Ledgerless Parser Character".to_string(),
        })
        .await
        .expect("create character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "CHARACTER TEMPLATE (v2.00)\nIDENTITY\nCHAR-ID-002 - Name: <string>\n"
                .to_string(),
            author: "operator".to_string(),
            tool: Some("mt-008-ledgerless-test".to_string()),
        })
        .await
        .expect("append sheet version");

    let parsed = store
        .parse_sheet_template_version(sheet.version_id, "wp-kernel-005-mt-008-ledgerless", None)
        .await
        .expect("connect store parses through canonical EventLedger");
    assert_eq!(parsed.version_id, sheet.version_id);
    assert_parse_event_has_canonical_projection_link(
        &store,
        sheet.version_id,
        "wp-kernel-005-mt-008-ledgerless",
    )
    .await;

    let snapshots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(sheet.version_id)
    .bind("wp-kernel-005-mt-008-ledgerless")
    .fetch_one(store.pool())
    .await
    .expect("count parse snapshots for connect store");
    assert_eq!(
        snapshots, 1,
        "connect store parse creates exactly one typed snapshot row"
    );
}

#[tokio::test]
async fn atelier_sheet_template_parser_missing_version_is_not_found() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_template_parser_missing_version_is_not_found: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

    let missing_version_id = Uuid::new_v4();
    let err = store
        .parse_sheet_template_version(missing_version_id, "wp-kernel-005-mt-008", None)
        .await
        .expect_err("missing sheet version must error");

    match err {
        AtelierError::NotFound(message) => assert!(
            message.contains(&missing_version_id.to_string()),
            "not-found error names the missing sheet version"
        ),
        other => panic!("expected NotFound for missing sheet version, got {other:?}"),
    }

    let leaked_snapshots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_sheet_parse_snapshot WHERE version_id = $1",
    )
    .bind(missing_version_id)
    .fetch_one(store.pool())
    .await
    .expect("count leaked snapshots for missing version");
    assert_eq!(
        leaked_snapshots, 0,
        "missing version parse must not create a snapshot row"
    );
}

#[tokio::test]
async fn atelier_sheet_template_parser_rejects_malformed_descriptors_without_snapshot() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_template_parser_rejects_malformed_descriptors_without_snapshot: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-parser-malformed-character-{}", Uuid::new_v4()),
            display_name: "Malformed Sheet Parser Character".to_string(),
        })
        .await
        .expect("create character");
    let malformed_sheet = "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-BAD-001 — Broken: <string
";
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: malformed_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-008-malformed".to_string()),
        })
        .await
        .expect("append malformed sheet version");

    let err = store
        .parse_sheet_template_version(
            sheet.version_id,
            "wp-kernel-005-mt-008-malformed",
            Some("test://wp-kernel-005/mt-008-malformed"),
        )
        .await
        .expect_err("malformed descriptor must produce a structured parser error");
    match err {
        AtelierError::Validation(message) => assert!(
            message.contains("structured_parse_error")
                && message.contains("field_id=CHAR-BAD-001")
                && message.contains("reason=unclosed_descriptor"),
            "malformed parser error names line, field, and reason: {message}"
        ),
        other => panic!("expected structured parser validation error, got {other:?}"),
    }

    let snapshots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(sheet.version_id)
    .bind("wp-kernel-005-mt-008-malformed")
    .fetch_one(store.pool())
    .await
    .expect("count malformed parse snapshots");
    assert_eq!(
        snapshots, 0,
        "malformed template parse must not create a snapshot row"
    );

    let parse_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &sheet.version_id.to_string())
        .await
        .expect("list malformed parse EventLedger rows");
    assert!(
        parse_events
            .iter()
            .all(|event| event.payload["event_family"] != event_family::SHEET_TEMPLATE_PARSED),
        "malformed template parse must not emit a parsed-template event"
    );

    let block_malformed_sheet = "\
CHARACTER TEMPLATE (v2.00)
Trait_Block
CHAR-TRAIT-001 — Trait Label: <string>
IDENTITY
CHAR-TRAITS-001 — Traits: <list of Trait_Block|optional>
CHAR-TRAITS-001[0].CHAR-TRAIT-001 — Trait Label: <bad
";
    let block_sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: block_malformed_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-008-malformed-block".to_string()),
        })
        .await
        .expect("append malformed block-instance sheet version");
    let block_err = store
        .parse_sheet_template_version(
            block_sheet.version_id,
            "wp-kernel-005-mt-008-malformed-block",
            Some("test://wp-kernel-005/mt-008-malformed-block"),
        )
        .await
        .expect_err("malformed block instance descriptor must produce a structured parser error");
    match block_err {
        AtelierError::Validation(message) => assert!(
            message.contains("structured_parse_error")
                && message.contains("field_id=CHAR-TRAIT-001")
                && message.contains("reason=unclosed_descriptor"),
            "malformed block-instance parser error names field and reason: {message}"
        ),
        other => panic!("expected structured parser validation error, got {other:?}"),
    }

    let block_snapshots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(block_sheet.version_id)
    .bind("wp-kernel-005-mt-008-malformed-block")
    .fetch_one(store.pool())
    .await
    .expect("count malformed block-instance parse snapshots");
    assert_eq!(
        block_snapshots, 0,
        "malformed block-instance parse must not create a snapshot row"
    );
}

#[tokio::test]
async fn atelier_sheet_selective_apply_and_revert_preserve_append_only_history() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_selective_apply_and_revert_preserve_append_only_history: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-selective-edit-character-{}", Uuid::new_v4()),
            display_name: "Selective Sheet Edit Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-ID-002 — Name: <string>
CHAR-ID-003 — Alias: <string>
CHAR-AGE-001 — Age: <integer|adult>
CHAR-BODY-001 — Body Type: <slim|curvy|other:<descriptor>|optional>
CHAR-MOOD-001 — Mood: <happy|sad|optional>
CHAR-METRIC-001 — Metric: <integer|number|none>
CHAR-RULE-001 — Rule Text: <rule> (Preserve inline rule prose)
Freeform note that must stay byte-preserved
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-013-original".to_string()),
        })
        .await
        .expect("append original sheet version");

    let parsed = store
        .parse_sheet_template_version(
            original.version_id,
            "wp-kernel-005-mt-013",
            Some("test://wp-kernel-005/mt-013"),
        )
        .await
        .expect("parse selective-edit source sheet");

    for (field_id, replacement_text, expected_reason) in [
        ("CHAR-AGE-001", "not an integer", "invalid_integer"),
        ("CHAR-MOOD-001", "giant", "invalid_enum"),
        ("CHAR-METRIC-001", "not numeric", "invalid_union"),
    ] {
        let denial = store
            .apply_sheet_field_edits(&SheetFieldEditRequest {
                version_id: original.version_id,
                template_id: "wp-kernel-005-mt-013".to_string(),
                source_path: Some("test://wp-kernel-005/mt-013".to_string()),
                expected_template_hash: Some(parsed.ast.template_hash.clone()),
                actor_role: "operator".to_string(),
                edits: vec![SheetFieldEdit {
                    block_instance_id: None,
                    field_id: field_id.to_string(),
                    replacement_text: replacement_text.to_string(),
                }],
                author: "operator".to_string(),
                tool: Some("mt-013-invalid-typed-value".to_string()),
            })
            .await
            .expect_err("invalid typed field replacement must be denied");
        match denial {
            AtelierError::Validation(message) => assert!(
                message.contains(expected_reason) && message.contains(field_id),
                "typed denial names reason and field id: {message}"
            ),
            other => panic!("expected typed validation denial, got {other:?}"),
        }
    }

    let applied = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-013".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-ID-003".to_string(),
                    replacement_text: "<descriptor>".to_string(),
                },
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-AGE-001".to_string(),
                    replacement_text: "adult".to_string(),
                },
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-BODY-001".to_string(),
                    replacement_text: "athletic".to_string(),
                },
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-RULE-001".to_string(),
                    replacement_text: "<strict rule>".to_string(),
                },
            ],
            author: "operator".to_string(),
            tool: Some("mt-013-selective-apply".to_string()),
        })
        .await
        .expect("selectively apply a field edit");

    assert_eq!(applied.version.seq, original.seq + 1);
    assert_eq!(applied.version.parent_version_id, Some(original.version_id));
    assert_eq!(
        applied.applied_field_ids,
        vec![
            "CHAR-ID-003".to_string(),
            "CHAR-AGE-001".to_string(),
            "CHAR-BODY-001".to_string(),
            "CHAR-RULE-001".to_string()
        ]
    );
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-ID-003 — Alias: <descriptor>"));
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-AGE-001 — Age: <adult>"));
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-BODY-001 — Body Type: <athletic>"));
    assert!(
        !applied.version.raw_text.contains("<athletic>|optional"),
        "nested descriptor replacement must consume the full balanced descriptor span"
    );
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-RULE-001 — Rule Text: <strict rule> (Preserve inline rule prose)"));
    assert!(applied
        .version
        .raw_text
        .contains("Freeform note that must stay byte-preserved"));
    assert!(original.raw_text.contains("CHAR-ID-003 — Alias: <string>"));

    let applied_for_revert = store
        .parse_sheet_template_version(
            applied.version.version_id,
            "wp-kernel-005-mt-013",
            Some("test://wp-kernel-005/mt-013/current-head"),
        )
        .await
        .expect("parse current head before full revert");

    let reverted = store
        .revert_sheet_version_as_new(&SheetVersionRevertRequest {
            character_internal_id: character.internal_id,
            target_version_id: original.version_id,
            template_id: "wp-kernel-005-mt-013".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013/current-head".to_string()),
            expected_template_hash: Some(applied_for_revert.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            field_selectors: Vec::new(),
            author: "operator".to_string(),
            tool: Some("mt-013-revert".to_string()),
        })
        .await
        .expect("revert prior sheet version as a new append-only version");

    assert_eq!(reverted.version.seq, applied.version.seq + 1);
    assert_eq!(
        reverted.version.parent_version_id,
        Some(applied.version.version_id),
        "revert is appended as a new child of the current head"
    );
    assert_eq!(reverted.reverted_to_version_id, original.version_id);
    assert_eq!(reverted.version.raw_text, original.raw_text);

    let history = store
        .sheet_version_history(character.internal_id)
        .await
        .expect("load append-only history after selective apply/revert");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].raw_text, original.raw_text);
    assert_eq!(history[1].raw_text, applied.version.raw_text);
    assert_eq!(history[2].raw_text, original.raw_text);

    let apply_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &applied.version.version_id.to_string(),
        )
        .await
        .expect("list apply EventLedger rows");
    let apply_event = apply_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_FIELD_EDITS_APPLIED)
        .expect("selective apply emits EventLedger evidence");
    assert_eq!(
        apply_event.payload["atelier_payload"]["source_version_id"],
        original.version_id.to_string()
    );
    assert_eq!(
        apply_event.payload["atelier_payload"]["applied_field_ids"][0],
        "CHAR-ID-003"
    );
    assert!(
        apply_event.payload["atelier_payload"]
            .get("raw_text")
            .is_none(),
        "selective apply event must not leak raw sheet text"
    );

    let revert_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &reverted.version.version_id.to_string(),
        )
        .await
        .expect("list revert EventLedger rows");
    let revert_event = revert_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_VERSION_REVERTED)
        .expect("revert emits EventLedger evidence");
    assert_eq!(
        revert_event.payload["atelier_payload"]["reverted_to_version_id"],
        original.version_id.to_string()
    );
    assert!(
        revert_event.payload["atelier_payload"]
            .get("raw_text")
            .is_none(),
        "revert event must not leak raw sheet text"
    );

    let head_edit = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: reverted.version.version_id,
            template_id: "wp-kernel-005-mt-013".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-ID-002".to_string(),
                    replacement_text: "New Head Name".to_string(),
                },
                SheetFieldEdit {
                    block_instance_id: None,
                    field_id: "CHAR-ID-003".to_string(),
                    replacement_text: "New Head Alias".to_string(),
                },
            ],
            author: "operator".to_string(),
            tool: Some("mt-013-head-edit-before-selective-revert".to_string()),
        })
        .await
        .expect("append current-head edits before selected revert");

    let head_edit_for_revert = store
        .parse_sheet_template_version(
            head_edit.version.version_id,
            "wp-kernel-005-mt-013",
            Some("test://wp-kernel-005/mt-013/current-head-selected"),
        )
        .await
        .expect("parse current head before selected revert");

    let selected_revert = store
        .revert_sheet_version_as_new(&SheetVersionRevertRequest {
            character_internal_id: character.internal_id,
            target_version_id: original.version_id,
            template_id: "wp-kernel-005-mt-013".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013/current-head-selected".to_string()),
            expected_template_hash: Some(head_edit_for_revert.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            field_selectors: vec![SheetFieldSelector {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-013-selected-field-revert".to_string()),
        })
        .await
        .expect("selectively revert one field from prior sheet version");

    assert_eq!(
        selected_revert.version.parent_version_id,
        Some(head_edit.version.version_id),
        "selected revert is appended from the current head"
    );
    assert!(
        selected_revert
            .version
            .raw_text
            .contains("CHAR-ID-002 — Name: <New Head Name>"),
        "selected revert must preserve unrelated current-head field edits"
    );
    assert!(
        selected_revert
            .version
            .raw_text
            .contains("CHAR-ID-003 — Alias: <string>"),
        "selected revert must restore only the selected target field"
    );

    let selected_revert_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &selected_revert.version.version_id.to_string(),
        )
        .await
        .expect("list selected revert EventLedger rows");
    let selected_revert_event = selected_revert_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_VERSION_REVERTED)
        .expect("selected revert emits EventLedger evidence");
    assert_eq!(
        selected_revert_event.payload["atelier_payload"]["reverted_field_ids"][0],
        "CHAR-ID-003"
    );
}

#[tokio::test]
async fn atelier_sheet_selective_apply_rejects_stale_non_head_versions() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_selective_apply_rejects_stale_non_head_versions: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-stale-head-character-{}", Uuid::new_v4()),
            display_name: "Sheet Stale Head Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-ID-002 — Name: <string>
CHAR-ID-003 — Alias: <string>
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-013-stale-head-original".to_string()),
        })
        .await
        .expect("append original sheet version");
    let parsed = store
        .parse_sheet_template_version(
            original.version_id,
            "wp-kernel-005-mt-013-stale-head",
            Some("test://wp-kernel-005/mt-013-stale-head"),
        )
        .await
        .expect("parse original sheet");

    let head_edit = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-013-stale-head".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013-stale-head".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-002".to_string(),
                replacement_text: "Current Head Name".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-013-current-head-edit".to_string()),
        })
        .await
        .expect("append current head edit");

    let stale_apply = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-013-stale-head".to_string(),
            source_path: Some("test://wp-kernel-005/mt-013-stale-head".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text: "Stale Alias".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-013-stale-non-head-apply".to_string()),
        })
        .await
        .expect_err("valid hash for an older non-head version must still be stale");
    match stale_apply {
        AtelierError::Validation(message) => assert!(
            message.contains("stale_selection") && message.contains("current head"),
            "stale non-head denial should name current-head protection: {message}"
        ),
        other => panic!("expected stale non-head validation denial, got {other:?}"),
    }

    let history = store
        .sheet_version_history(character.internal_id)
        .await
        .expect("load sheet history after stale non-head apply");
    assert_eq!(
        history.len(),
        2,
        "stale non-head apply must not append a third version"
    );
    assert_eq!(
        history.last().expect("history has current head").version_id,
        head_edit.version.version_id,
        "current head remains the valid head edit"
    );
    assert!(history
        .last()
        .expect("history has current head")
        .raw_text
        .contains("CHAR-ID-002 — Name: <Current Head Name>"));
}

#[tokio::test]
async fn append_sheet_version_participates_in_character_sequence_lock() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP append_sheet_version_participates_in_character_sequence_lock: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("wp-kernel-005-mt-013-lock-{}", Uuid::new_v4()),
            display_name: "WP Kernel 005 MT-013 Append Lock".to_string(),
        })
        .await
        .expect("create character for append lock proof");

    let mut lock_tx = store.pool().begin().await.expect("begin lock transaction");
    let seq_lock_key = format!("atelier_sheet_version_seq:{}", character.internal_id);
    sqlx::query("SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)")
        .bind(&seq_lock_key)
        .execute(&mut *lock_tx)
        .await
        .expect("hold character sheet sequence advisory lock");

    let append_store = store.clone();
    let character_internal_id = character.internal_id;
    let append_handle = tokio::spawn(async move {
        append_store
            .append_sheet_version(&NewSheetVersion {
                character_internal_id,
                raw_text: "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-ID-002 — Name: <string>
"
                .to_string(),
                author: "operator".to_string(),
                tool: Some("mt-013-append-lock-proof".to_string()),
            })
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(
        !append_handle.is_finished(),
        "append_sheet_version must wait on the same character sequence advisory lock as selective apply"
    );

    lock_tx.rollback().await.expect("release advisory lock");
    let appended = append_handle
        .await
        .expect("append task should join after lock release")
        .expect("append should complete after lock release");
    assert_eq!(appended.seq, 1);
    assert_eq!(appended.parent_version_id, None);
}

#[tokio::test]
async fn atelier_sheet_blocklist_unmapped_and_protected_edit_guards_are_enforced() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_blocklist_unmapped_and_protected_edit_guards_are_enforced: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-guard-character-{}", Uuid::new_v4()),
            display_name: "Sheet Guard Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
Trait_Block
CHAR-TRAIT-001 — Trait Label: <string>
CHAR-TRAIT-002 — Trait Note: <paragraph|editable:sheet-curator>
IDENTITY
CHAR-ID-002 — Name: <string|protected>
CHAR-ID-003 — Alias: <string|editable:sheet-curator>
CHAR-TRAITS-001 — Traits: <list of Trait_Block|optional>
  Freeform note that must survive guarded edits  
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-009-010-011-original".to_string()),
        })
        .await
        .expect("append original sheet version");

    let parsed = store
        .parse_sheet_template_version(
            original.version_id,
            "wp-kernel-005-mt-009-010-011",
            Some("test://wp-kernel-005/mt-009-010-011"),
        )
        .await
        .expect("parse guarded block-list sheet");
    let trait_schema = parsed
        .ast
        .block_schemas
        .iter()
        .find(|schema| schema.name == "Trait_Block")
        .expect("Trait_Block schema parsed");
    assert_eq!(
        trait_schema.fields,
        vec!["CHAR-TRAIT-001".to_string(), "CHAR-TRAIT-002".to_string()],
        "block schema records recursive member field ids"
    );
    let traits_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-TRAITS-001")
        .expect("block-list root parsed");
    assert!(
        matches!(
            &traits_field.field_type,
            ParsedSheetFieldType::BlockList { block_schema_name } if block_schema_name == "Trait_Block"
        ),
        "list of Trait_Block remains typed as a block-list field"
    );
    let name_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ID-002")
        .expect("protected name field parsed");
    assert!(
        name_field.protected,
        "protected descriptor is preserved in AST"
    );
    let alias_field = parsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ID-003")
        .expect("role-scoped alias field parsed");
    assert_eq!(
        alias_field.editable_roles,
        vec!["sheet-curator".to_string()],
        "editable role scope is preserved in AST"
    );
    let unmapped = parsed
        .ast
        .unmapped_lines
        .iter()
        .find(|line| line.raw == "  Freeform note that must survive guarded edits  ")
        .expect("unmapped freeform line preserved");
    assert_eq!(
        std::str::from_utf8(&raw_sheet.as_bytes()[unmapped.byte_start..unmapped.byte_end])
            .expect("unmapped source bytes are valid UTF-8"),
        "  Freeform note that must survive guarded edits  "
    );

    let protected_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-002".to_string(),
                replacement_text: "<descriptor>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-protected-denial".to_string()),
        })
        .await
        .expect_err("protected field edit must be denied");
    match protected_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("protected_field") && message.contains("CHAR-ID-002"),
            "protected-field denial names reason and field id: {message}"
        ),
        other => panic!("expected protected-field validation denial, got {other:?}"),
    }

    let role_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "guest".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text: "<descriptor>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-role-denial".to_string()),
        })
        .await
        .expect_err("unauthorized role edit must be denied");
    match role_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("role_scope_denied")
                && message.contains("CHAR-ID-003")
                && message.contains("guest"),
            "role-scope denial names reason, field id, and actor role: {message}"
        ),
        other => panic!("expected role-scope validation denial, got {other:?}"),
    }

    let stale_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: Some("stale-template-hash".to_string()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text: "<descriptor>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-stale-denial".to_string()),
        })
        .await
        .expect_err("stale source hash must be denied before mutation");
    match stale_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("stale_selection"),
            "stale source denial names stale_selection: {message}"
        ),
        other => panic!("expected stale-selection validation denial, got {other:?}"),
    }

    let missing_hash_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: None,
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text: "<descriptor>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-missing-hash-denial".to_string()),
        })
        .await
        .expect_err("missing source hash must be denied before mutation");
    match missing_hash_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("stale_selection") && message.contains("expected_template_hash"),
            "missing-hash denial is treated as stale-selection protection: {message}"
        ),
        other => panic!("expected missing-hash validation denial, got {other:?}"),
    }

    let schema_member_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-TRAIT-002".to_string(),
                replacement_text: "<paragraph>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-009-unscoped-schema-member-denial".to_string()),
        })
        .await
        .expect_err("block schema member edits must require a block instance selector");
    match schema_member_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("block_instance_required") && message.contains("CHAR-TRAIT-002"),
            "unscoped schema-member denial names block_instance_required and field id: {message}"
        ),
        other => panic!("expected block-instance-required validation denial, got {other:?}"),
    }

    let history_after_denials = store
        .sheet_version_history(character.internal_id)
        .await
        .expect("history after denied guarded edits");
    assert_eq!(
        history_after_denials.len(),
        1,
        "denied guarded edits must not append sheet versions"
    );

    let applied = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text:
                    "<descriptor|editable:guest|editable=guest|role:guest|roles:guest>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-allowed-apply".to_string()),
        })
        .await
        .expect("authorized role-scoped edit succeeds");

    assert_eq!(applied.version.seq, original.seq + 1);
    assert_eq!(applied.version.parent_version_id, Some(original.version_id));
    assert_eq!(
        applied.applied_field_ids,
        vec!["CHAR-ID-003".to_string()],
        "authorized apply reports the parser-bounded fields it rewrote"
    );
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-ID-002 — Name: <string|protected>"));
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-ID-003 — Alias: <descriptor|editable:sheet-curator>"));
    assert!(
        !applied.version.raw_text.contains("guest"),
        "authorized replacement must not broaden editable role tokens"
    );
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-TRAIT-002 — Trait Note: <paragraph|editable:sheet-curator>"));
    assert!(applied
        .version
        .raw_text
        .contains("Freeform note that must survive guarded edits"));
    assert_eq!(
        applied.preserved_unmapped_lines[0].raw,
        "  Freeform note that must survive guarded edits  "
    );

    let rejection_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &original.version_id.to_string())
        .await
        .expect("list source-version rejection events");
    let rejection_reasons: Vec<String> = rejection_events
        .iter()
        .filter(|event| event.payload["event_family"] == event_family::SHEET_FIELD_EDIT_REJECTED)
        .filter_map(|event| {
            event.payload["atelier_payload"]["reason_code"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    assert!(
        rejection_reasons.contains(&"protected_field".to_string())
            && rejection_reasons.contains(&"role_scope_denied".to_string())
            && rejection_reasons.contains(&"stale_selection".to_string()),
        "denied guarded edits emit rejection evidence: {rejection_reasons:?}"
    );

    let reparsed = store
        .parse_sheet_template_version(
            applied.version.version_id,
            "wp-kernel-005-mt-009-010-011",
            Some("test://wp-kernel-005/mt-009-010-011/reparsed"),
        )
        .await
        .expect("reparse edited sheet with preserved role metadata");
    let edited_alias = reparsed
        .ast
        .fields
        .iter()
        .find(|field| field.id == "CHAR-ID-003")
        .expect("edited alias field is still parsed");
    assert_eq!(
        edited_alias.editable_roles,
        vec!["sheet-curator".to_string()],
        "authorized edit must preserve role-scope metadata for future edits"
    );

    let followup_guest_denial = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: applied.version.version_id,
            template_id: "wp-kernel-005-mt-009-010-011".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009-010-011/reparsed".to_string()),
            expected_template_hash: Some(reparsed.ast.template_hash.clone()),
            actor_role: "guest".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
                replacement_text: "<guest bypass>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-followup-role-denial".to_string()),
        })
        .await
        .expect_err("role-scope guard must survive authorized edits");
    match followup_guest_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("role_scope_denied") && message.contains("guest"),
            "follow-up guest edit remains denied after role metadata preservation: {message}"
        ),
        other => panic!("expected follow-up role-scope validation denial, got {other:?}"),
    }

    let apply_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &applied.version.version_id.to_string(),
        )
        .await
        .expect("list allowed apply events");
    let apply_event = apply_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_FIELD_EDITS_APPLIED)
        .expect("authorized guarded apply emits EventLedger evidence");
    assert_eq!(
        apply_event.payload["atelier_payload"]["actor_role"],
        "sheet-curator"
    );
    assert!(
        apply_event.payload["atelier_payload"]
            .get("raw_text")
            .is_none(),
        "guarded apply EventLedger payload must stay leak-safe"
    );
}

#[tokio::test]
async fn atelier_sheet_revert_enforces_protected_and_role_scope_guards() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_revert_enforces_protected_and_role_scope_guards: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-revert-guard-character-{}", Uuid::new_v4()),
            display_name: "Sheet Revert Guard Character".to_string(),
        })
        .await
        .expect("create revert guard character");
    let original_raw = "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-ID-002 — Name: <string|protected>
CHAR-ID-003 — Alias: <string|editable:sheet-curator>
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: original_raw.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-011-revert-guard-original".to_string()),
        })
        .await
        .expect("append original guarded sheet");

    let protected_changed_raw = original_raw.replace(
        "CHAR-ID-002 — Name: <string|protected>",
        "CHAR-ID-002 — Name: <rewritten protected|protected>",
    );
    let protected_changed = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: protected_changed_raw,
            author: "operator".to_string(),
            tool: Some("mt-011-revert-protected-setup".to_string()),
        })
        .await
        .expect("append protected-field drift fixture");
    let protected_head = store
        .parse_sheet_template_version(
            protected_changed.version_id,
            "wp-kernel-005-mt-011-revert",
            Some("test://wp-kernel-005/mt-011-revert/protected"),
        )
        .await
        .expect("parse protected-field drift head");

    let protected_denial = store
        .revert_sheet_version_as_new(&SheetVersionRevertRequest {
            character_internal_id: character.internal_id,
            target_version_id: original.version_id,
            template_id: "wp-kernel-005-mt-011-revert".to_string(),
            source_path: Some("test://wp-kernel-005/mt-011-revert/protected".to_string()),
            expected_template_hash: Some(protected_head.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            field_selectors: vec![SheetFieldSelector {
                block_instance_id: None,
                field_id: "CHAR-ID-002".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-revert-protected-denial".to_string()),
        })
        .await
        .expect_err("selected revert must not rewrite protected fields");
    match protected_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("protected_field") && message.contains("CHAR-ID-002"),
            "protected revert denial names reason and field id: {message}"
        ),
        other => panic!("expected protected-field revert denial, got {other:?}"),
    }
    assert_eq!(
        store
            .sheet_version_history(character.internal_id)
            .await
            .expect("history after protected revert denial")
            .len(),
        2,
        "denied protected revert must not append a sheet version"
    );

    let alias_changed_raw = original_raw.replace(
        "CHAR-ID-003 — Alias: <string|editable:sheet-curator>",
        "CHAR-ID-003 — Alias: <current alias|editable:sheet-curator>",
    );
    let alias_changed = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: alias_changed_raw,
            author: "operator".to_string(),
            tool: Some("mt-011-revert-role-setup".to_string()),
        })
        .await
        .expect("append role-scoped drift fixture");
    let alias_head = store
        .parse_sheet_template_version(
            alias_changed.version_id,
            "wp-kernel-005-mt-011-revert",
            Some("test://wp-kernel-005/mt-011-revert/role"),
        )
        .await
        .expect("parse role-scoped drift head");

    let role_denial = store
        .revert_sheet_version_as_new(&SheetVersionRevertRequest {
            character_internal_id: character.internal_id,
            target_version_id: original.version_id,
            template_id: "wp-kernel-005-mt-011-revert".to_string(),
            source_path: Some("test://wp-kernel-005/mt-011-revert/role".to_string()),
            expected_template_hash: Some(alias_head.ast.template_hash.clone()),
            actor_role: "guest".to_string(),
            field_selectors: vec![SheetFieldSelector {
                block_instance_id: None,
                field_id: "CHAR-ID-003".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-011-revert-role-denial".to_string()),
        })
        .await
        .expect_err("selected revert must enforce role-scoped field permissions");
    match role_denial {
        AtelierError::Validation(message) => assert!(
            message.contains("role_scope_denied")
                && message.contains("CHAR-ID-003")
                && message.contains("guest"),
            "role-scoped revert denial names reason, field id, and role: {message}"
        ),
        other => panic!("expected role-scope revert denial, got {other:?}"),
    }
    assert_eq!(
        store
            .sheet_version_history(character.internal_id)
            .await
            .expect("history after role-scoped revert denial")
            .len(),
        3,
        "denied role-scoped revert must not append a sheet version"
    );

    let protected_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &protected_changed.version_id.to_string(),
        )
        .await
        .expect("list protected revert rejection events");
    assert!(protected_events.iter().any(|event| {
        event.payload["event_family"] == event_family::SHEET_FIELD_EDIT_REJECTED
            && event.payload["atelier_payload"]["operation"] == "sheet_revert"
            && event.payload["atelier_payload"]["reason_code"] == "protected_field"
    }));
    let role_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &alias_changed.version_id.to_string(),
        )
        .await
        .expect("list role revert rejection events");
    assert!(role_events.iter().any(|event| {
        event.payload["event_family"] == event_family::SHEET_FIELD_EDIT_REJECTED
            && event.payload["atelier_payload"]["operation"] == "sheet_revert"
            && event.payload["atelier_payload"]["reason_code"] == "role_scope_denied"
    }));
}

#[tokio::test]
async fn atelier_sheet_blocklist_instances_are_stored_and_apply_is_instance_scoped() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_blocklist_instances_are_stored_and_apply_is_instance_scoped: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-block-instance-character-{}", Uuid::new_v4()),
            display_name: "Sheet Block Instance Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
Trait_Block
CHAR-TRAIT-001 — Trait Label: <string>
CHAR-TRAIT-002 — Trait Note: <paragraph|editable:sheet-curator>
IDENTITY
CHAR-TRAITS-001 — Traits: <list of Trait_Block|optional>
CHAR-TRAITS-001[0].CHAR-TRAIT-001 — Trait Label: <first label>
CHAR-TRAITS-001[0].CHAR-TRAIT-002 — Trait Note: <first note>
CHAR-TRAITS-001[1].CHAR-TRAIT-001 — Trait Label: <second label>
CHAR-TRAITS-001[1].CHAR-TRAIT-002 — Trait Note: <second note>
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-009-original".to_string()),
        })
        .await
        .expect("append original sheet version");

    let parsed = store
        .parse_sheet_template_version(
            original.version_id,
            "wp-kernel-005-mt-009",
            Some("test://wp-kernel-005/mt-009"),
        )
        .await
        .expect("parse repeated block-list instances");
    assert_eq!(
        parsed.ast.block_instances.len(),
        2,
        "two repeated Trait_Block instances are parsed as durable block instances"
    );
    assert_eq!(
        parsed.ast.block_instances[0].instance_id,
        "CHAR-TRAITS-001[0]"
    );
    assert_eq!(
        parsed.ast.block_instances[1].instance_id,
        "CHAR-TRAITS-001[1]"
    );
    assert_eq!(
        parsed.ast.block_instances[1].fields[1].field_id,
        "CHAR-TRAIT-002"
    );
    assert_eq!(
        parsed.ast.block_instances[1].fields[1].raw,
        "CHAR-TRAITS-001[1].CHAR-TRAIT-002 — Trait Note: <second note>"
    );

    let ast_json: serde_json::Value = sqlx::query_scalar(
        "SELECT ast FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(original.version_id)
    .bind("wp-kernel-005-mt-009")
    .fetch_one(store.pool())
    .await
    .expect("persisted block-instance parse AST row");
    assert_eq!(
        ast_json["block_instances"][0]["instance_id"],
        "CHAR-TRAITS-001[0]"
    );
    assert_eq!(
        ast_json["block_instances"][1]["fields"][1]["field_id"],
        "CHAR-TRAIT-002"
    );

    let parse_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &original.version_id.to_string())
        .await
        .expect("list parse events for block-instance sheet");
    let parse_event = parse_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_TEMPLATE_PARSED)
        .expect("block-instance parse emits EventLedger evidence");
    assert_eq!(
        parse_event.payload["atelier_payload"]["block_instance_count"],
        2
    );

    let wrong_instance = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: Some("CHAR-TRAITS-001[9]".to_string()),
                field_id: "CHAR-TRAIT-002".to_string(),
                replacement_text: "<wrong instance>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-009-wrong-instance".to_string()),
        })
        .await
        .expect_err("unknown block instance must be denied before mutation");
    match wrong_instance {
        AtelierError::Validation(message) => assert!(
            message.contains("unknown parsed block field_id=CHAR-TRAITS-001[9].CHAR-TRAIT-002"),
            "wrong-instance denial names the canonical instance field id: {message}"
        ),
        other => panic!("expected wrong-instance validation denial, got {other:?}"),
    }

    let duplicate_instance_edit = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![
                SheetFieldEdit {
                    block_instance_id: Some("CHAR-TRAITS-001[1]".to_string()),
                    field_id: "CHAR-TRAIT-002".to_string(),
                    replacement_text: "<duplicate a>".to_string(),
                },
                SheetFieldEdit {
                    block_instance_id: Some("CHAR-TRAITS-001[1]".to_string()),
                    field_id: "CHAR-TRAIT-002".to_string(),
                    replacement_text: "<duplicate b>".to_string(),
                },
            ],
            author: "operator".to_string(),
            tool: Some("mt-009-duplicate-instance".to_string()),
        })
        .await
        .expect_err("duplicate instance-scoped edit must be denied before mutation");
    match duplicate_instance_edit {
        AtelierError::Validation(message) => assert!(
            message.contains("duplicate sheet field edit")
                && message.contains("CHAR-TRAITS-001[1].CHAR-TRAIT-002"),
            "duplicate denial names canonical instance field id: {message}"
        ),
        other => panic!("expected duplicate-instance validation denial, got {other:?}"),
    }

    let history_after_denials = store
        .sheet_version_history(character.internal_id)
        .await
        .expect("history after denied instance edits");
    assert_eq!(
        history_after_denials.len(),
        1,
        "denied instance edits must not append sheet versions"
    );
    let instance_rejections = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &original.version_id.to_string())
        .await
        .expect("list instance rejection events");
    let bounded_rejection_count = instance_rejections
        .iter()
        .filter(|event| event.payload["event_family"] == event_family::SHEET_FIELD_EDIT_REJECTED)
        .filter(|event| event.payload["atelier_payload"]["reason_code"] == "bounded_apply_denied")
        .count();
    assert!(
        bounded_rejection_count >= 2,
        "wrong and duplicate instance denials emit rejection evidence"
    );
    assert!(
        instance_rejections.iter().any(|event| {
            event.payload["event_family"] == event_family::SHEET_FIELD_EDIT_REJECTED
                && event.payload["atelier_payload"]["attempted_field_ids"]
                    .as_array()
                    .is_some_and(|fields| {
                        fields.iter().any(|field| {
                            field.as_str() == Some("CHAR-TRAITS-001[9].CHAR-TRAIT-002")
                        })
                    })
        }),
        "wrong-instance rejection evidence must preserve the full selected instance key"
    );

    let applied = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: Some("CHAR-TRAITS-001[1]".to_string()),
                field_id: "CHAR-TRAIT-002".to_string(),
                replacement_text: "<updated second note>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-009-instance-apply".to_string()),
        })
        .await
        .expect("apply to one repeated block instance");

    assert!(applied
        .version
        .raw_text
        .contains("CHAR-TRAITS-001[0].CHAR-TRAIT-002 — Trait Note: <first note>"));
    assert!(applied
        .version
        .raw_text
        .contains("CHAR-TRAITS-001[1].CHAR-TRAIT-002 — Trait Note: <updated second note>"));
    assert_eq!(
        applied.applied_field_ids,
        vec!["CHAR-TRAITS-001[1].CHAR-TRAIT-002".to_string()]
    );

    let apply_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &applied.version.version_id.to_string(),
        )
        .await
        .expect("list instance-scoped apply events");
    let apply_event = apply_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_FIELD_EDITS_APPLIED)
        .expect("instance-scoped apply emits EventLedger evidence");
    assert_eq!(
        apply_event.payload["atelier_payload"]["applied_field_ids"][0],
        "CHAR-TRAITS-001[1].CHAR-TRAIT-002"
    );
    assert!(
        apply_event.payload["atelier_payload"]
            .get("raw_text")
            .is_none(),
        "instance-scoped apply EventLedger payload must stay leak-safe"
    );
}

#[tokio::test]
async fn atelier_sheet_blocklist_instances_support_recursive_storage_and_apply() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_sheet_blocklist_instances_support_recursive_storage_and_apply: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("sheet-recursive-block-character-{}", Uuid::new_v4()),
            display_name: "Recursive Sheet Block Character".to_string(),
        })
        .await
        .expect("create character");
    let raw_sheet = "\
CHARACTER TEMPLATE (v2.00)
Trait_Block
CHAR-TRAIT-001 — Trait Label: <string>
CHAR-TRAIT-CHILDREN — Child Traits: <list of Subtrait_Block|optional>
Subtrait_Block
CHAR-SUBTRAIT-001 — Subtrait Label: <string>
CHAR-SUBTRAIT-002 — Subtrait Note: <paragraph|editable:sheet-curator>
IDENTITY
CHAR-TRAITS-001 — Traits: <list of Trait_Block|optional>
CHAR-TRAITS-001[0].CHAR-TRAIT-001 — Trait Label: <first parent>
CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN — Child Traits: <list of Subtrait_Block|optional>
CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0].CHAR-SUBTRAIT-001 — Subtrait Label: <nested label>
CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0].CHAR-SUBTRAIT-002 — Subtrait Note: <nested note>
";
    let original = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: raw_sheet.to_string(),
            author: "operator".to_string(),
            tool: Some("mt-009-recursive-original".to_string()),
        })
        .await
        .expect("append original recursive sheet version");

    let parsed = store
        .parse_sheet_template_version(
            original.version_id,
            "wp-kernel-005-mt-009-recursive",
            Some("test://wp-kernel-005/mt-009/recursive"),
        )
        .await
        .expect("parse recursive block-list instances");
    assert_eq!(
        parsed.ast.block_instances.len(),
        2,
        "parent and nested block-list instances are both stored"
    );
    let nested_instance = parsed
        .ast
        .block_instances
        .iter()
        .find(|instance| instance.instance_id == "CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0]")
        .expect("nested recursive block instance stored");
    assert_eq!(
        nested_instance.root_field_id,
        "CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN"
    );
    assert_eq!(nested_instance.block_schema_name, "Subtrait_Block");
    assert_eq!(nested_instance.ordinal, 0);
    assert_eq!(nested_instance.fields[1].field_id, "CHAR-SUBTRAIT-002");
    assert_eq!(
        nested_instance.fields[1].raw,
        "CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0].CHAR-SUBTRAIT-002 — Subtrait Note: <nested note>"
    );

    let ast_json: serde_json::Value = sqlx::query_scalar(
        "SELECT ast FROM atelier_sheet_parse_snapshot WHERE version_id = $1 AND template_id = $2",
    )
    .bind(original.version_id)
    .bind("wp-kernel-005-mt-009-recursive")
    .fetch_one(store.pool())
    .await
    .expect("persisted recursive block-instance AST row");
    assert!(
        ast_json["block_instances"]
            .as_array()
            .expect("block_instances is an array")
            .iter()
            .any(|instance| {
                instance["instance_id"] == "CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0]"
            }),
        "persisted AST names the nested instance path"
    );

    let parse_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &original.version_id.to_string())
        .await
        .expect("list parse events for recursive block sheet");
    let parse_event = parse_events
        .iter()
        .find(|event| event.payload["event_family"] == event_family::SHEET_TEMPLATE_PARSED)
        .expect("recursive block parse emits EventLedger evidence");
    assert_eq!(
        parse_event.payload["atelier_payload"]["block_instance_count"],
        2
    );

    let wrong_nested_instance = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-recursive".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009/recursive".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: Some("CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[9]".to_string()),
                field_id: "CHAR-SUBTRAIT-002".to_string(),
                replacement_text: "<wrong nested instance>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-009-recursive-wrong-instance".to_string()),
        })
        .await
        .expect_err("unknown nested block instance must be denied before mutation");
    match wrong_nested_instance {
        AtelierError::Validation(message) => assert!(
            message.contains(
                "unknown parsed block field_id=CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[9].CHAR-SUBTRAIT-002"
            ),
            "wrong nested-instance denial names the canonical recursive instance field id: {message}"
        ),
        other => panic!("expected wrong nested-instance validation denial, got {other:?}"),
    }

    let applied = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: original.version_id,
            template_id: "wp-kernel-005-mt-009-recursive".to_string(),
            source_path: Some("test://wp-kernel-005/mt-009/recursive".to_string()),
            expected_template_hash: Some(parsed.ast.template_hash.clone()),
            actor_role: "sheet-curator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: Some("CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0]".to_string()),
                field_id: "CHAR-SUBTRAIT-002".to_string(),
                replacement_text: "<updated nested note>".to_string(),
            }],
            author: "operator".to_string(),
            tool: Some("mt-009-recursive-apply".to_string()),
        })
        .await
        .expect("apply to nested recursive block instance");
    assert!(
        applied.version.raw_text.contains(
            "CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0].CHAR-SUBTRAIT-002 — Subtrait Note: <updated nested note>"
        )
    );
    assert_eq!(
        applied.applied_field_ids,
        vec!["CHAR-TRAITS-001[0].CHAR-TRAIT-CHILDREN[0].CHAR-SUBTRAIT-002".to_string()]
    );
}
