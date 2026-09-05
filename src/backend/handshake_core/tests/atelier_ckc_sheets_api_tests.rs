//! WP-CKC-posekit-overhaul (SurrealDB port) MT-059: the CKC `sheets` lane over HTTP.
//!
//! Ports the reference `atelier_stealth_window_tests.rs` sheet/template/story/moodboard API proofs
//! onto the embedded SurrealDB harness and the production app router (`handshake_core::api::routes`,
//! which merges `api::atelier_ckc_sheets`). PostgreSQL catalog/`sqlx` assertions become
//! `AtelierStore` reads (`count_events_for_aggregate`) and typed row counts on the harness.
//!
//! Guarded mutations follow the reference model-operation lease guard: either
//! `x-hsk-actor-id: operator` + `x-hsk-actor-kind: operator`, or a live
//! `atelier_model_coordination_lease` bound to the route's coordination thread
//! (`x-hsk-model-lease-id` + `x-hsk-session-id`). Both paths are exercised here.

mod atelier_surreal_support;

use std::sync::Arc;

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::api::atelier_ckc_sheets::{
    ckc_character_create_model_operation_thread_id,
    ckc_sheet_artifacts_model_operation_thread_id,
};
use handshake_core::atelier::model_lease::NewModelLeaseClaim;
use handshake_core::atelier::refs::character_ref;
use handshake_core::atelier::sheet::sheet_event_family;
use handshake_core::atelier::sheet_artifacts::sheet_artifact_event_family;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::role_mailbox_claim_lease::{
    RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use reqwest::StatusCode;
use uuid::Uuid;

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn app_state(harness: &AtelierSurrealHarness) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: harness.database.clone(),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt059-ckc-sheets-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// Serve the PRODUCTION app router so the proof covers the lane router being mounted next to
/// `api::atelier`, not a test-only composition.
async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, handshake_core::api::routes(state))
            .await
            .expect("Handshake API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

/// The operator declaration that exempts a guarded mutation from the lease requirement.
fn with_operator(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
}

fn with_lease(
    request: reqwest::RequestBuilder,
    actor: &str,
    claim_id: Uuid,
    session_id: &str,
) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", actor)
        .header("x-hsk-model-lease-id", claim_id.to_string())
        .header("x-hsk-session-id", session_id)
}

async fn claim_lease(
    harness: &AtelierSurrealHarness,
    thread_id: &str,
    actor: &str,
    session_id: &str,
) -> Uuid {
    harness
        .atelier
        .claim_model_lease(&NewModelLeaseClaim {
            thread_id: thread_id.to_owned(),
            executor_kind: RoleMailboxExecutorKind::LocalLargeModel,
            actor_id: actor.to_owned(),
            session_id: session_id.to_owned(),
            claim_mode: RoleMailboxClaimMode::ExclusiveLease,
            ttl_seconds: 900,
            linked_work_packet_id: "WP-CKC-posekit-overhaul".to_owned(),
            linked_micro_task_id: "MT-059".to_owned(),
        })
        .await
        .expect("claim model-operation lease")
        .claim_id
}

async fn json_body(response: reqwest::Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let text = response.text().await.expect("read body");
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .unwrap_or_else(|err| panic!("body is not JSON ({err}): status={status} body={text}"));
    (status, value)
}

fn str_field<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` must be a string in {value}"))
}

#[tokio::test]
async fn atelier_character_sheet_api_round_trips_refs_and_conflicts() {
    let harness = AtelierSurrealHarness::create().await;
    let (base_url, client, server) = serve(app_state(&harness)).await;
    let public_id = format!("mt009-char-{}", Uuid::now_v7());

    let (status, character) = json_body(
        with_operator(client.post(format!("{base_url}/atelier/characters")))
            .json(&serde_json::json!({
                "public_id": public_id,
                "display_name": "MT-009 Character Sheet API",
            }))
            .send()
            .await
            .expect("create character"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{character}");
    let character_internal_id = str_field(&character, "internal_id").to_owned();
    let character_uuid = Uuid::parse_str(&character_internal_id).expect("internal_id is a uuid");
    assert_eq!(character_uuid.get_version_num(), 7, "character internal_id must be UUID v7");
    let expected_character_ref = format!("atelier://character/{character_internal_id}");
    assert_eq!(str_field(&character, "character_ref"), expected_character_ref);
    assert_eq!(str_field(&character, "public_id"), public_id);

    let (status, duplicate_body) = json_body(
        with_operator(client.post(format!("{base_url}/atelier/characters")))
            .json(&serde_json::json!({
                "public_id": public_id,
                "display_name": "Duplicate MT-009 Character",
            }))
            .send()
            .await
            .expect("duplicate create"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "duplicate public_id must be a typed 409, not an infra 500: {duplicate_body}"
    );
    assert_eq!(duplicate_body["error"].as_str(), Some("conflict"));

    let listed: Vec<serde_json::Value> = client
        .get(format!("{base_url}/atelier/characters"))
        .send()
        .await
        .expect("list characters")
        .json()
        .await
        .expect("list json");
    assert!(
        listed
            .iter()
            .any(|row| row["internal_id"].as_str() == Some(character_internal_id.as_str())),
        "created character must be in the list"
    );
    let (status, fetched) = json_body(
        client
            .get(format!("{base_url}/atelier/characters/{character_internal_id}"))
            .send()
            .await
            .expect("get character"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(str_field(&fetched, "character_ref"), expected_character_ref);

    let sheet_versions_url = format!("{base_url}/atelier/characters/{character_internal_id}/sheet-versions");

    let missing_owner = with_operator(client.post(&sheet_versions_url))
        .json(&serde_json::json!({
            "raw_text": "name: MT-009\nrole: route proof",
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await
        .expect("append without owner");
    assert_eq!(
        missing_owner.status(),
        StatusCode::BAD_REQUEST,
        "full CKC sheet append must include CHAR-ID-001 so ownership is deterministic"
    );

    let duplicate_owner = with_operator(client.post(&sheet_versions_url))
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-001 — Character_ID: wrong-character-id"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await
        .expect("append with duplicate owner");
    assert_eq!(
        duplicate_owner.status(),
        StatusCode::BAD_REQUEST,
        "CKC sheet append must reject ambiguous duplicate CHAR-ID-001 ownership lines"
    );

    let (status, first) = json_body(
        with_operator(client.post(&sheet_versions_url))
            .json(&serde_json::json!({
                "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-006 — Primary_Role: route proof"),
                "expected_parent_version_id": null,
                "tool": "argus",
            }))
            .send()
            .await
            .expect("first append"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let first_version_id = str_field(&first, "version_id").to_owned();
    assert_eq!(
        Uuid::parse_str(&first_version_id).expect("uuid").get_version_num(),
        7,
        "version_id must be UUID v7"
    );
    let expected_first_sheet_ref =
        format!("atelier://sheet/{character_internal_id}/{first_version_id}");
    assert_eq!(first["seq"], 1);
    assert_eq!(first["author"].as_str(), Some("operator"));
    assert_eq!(str_field(&first, "sheet_version_ref"), expected_first_sheet_ref);
    assert_eq!(
        harness
            .row_count_by_field("atelier_sheet_field_value_projection", "value", "route proof")
            .await,
        1,
        "append-only CKC sheet writes must populate the field-value projection in the same write"
    );

    let (status, second) = json_body(
        with_operator(client.post(&sheet_versions_url))
            .json(&serde_json::json!({
                "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-006 — Primary_Role: updated route proof"),
                "expected_parent_version_id": first_version_id,
                "tool": "argus",
            }))
            .send()
            .await
            .expect("second append"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    assert_eq!(second["parent_version_id"].as_str(), Some(first_version_id.as_str()));
    assert_eq!(second["seq"], 2);
    let second_version_id = str_field(&second, "version_id").to_owned();
    let expected_second_sheet_ref =
        format!("atelier://sheet/{character_internal_id}/{second_version_id}");
    assert_eq!(str_field(&second, "sheet_version_ref"), expected_second_sheet_ref);

    let (status, stale_body) = json_body(
        with_operator(client.post(&sheet_versions_url))
            .json(&serde_json::json!({
                "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: stale write"),
                "expected_parent_version_id": first_version_id,
                "tool": "argus",
            }))
            .send()
            .await
            .expect("stale append"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "stale expected_parent_version_id must not append over a newer head: {stale_body}"
    );
    assert_eq!(stale_body["error"].as_str(), Some("stale_sheet_version"));
    assert_eq!(str_field(&stale_body, "character_ref"), expected_character_ref);
    assert_eq!(
        stale_body["expected_parent_version_id"].as_str(),
        Some(first_version_id.as_str())
    );
    assert_eq!(
        str_field(&stale_body, "expected_parent_sheet_version_ref"),
        expected_first_sheet_ref
    );
    assert_eq!(
        str_field(&stale_body, "expected_sheet_version_ref"),
        expected_first_sheet_ref
    );
    assert_eq!(
        stale_body["current_head_version_id"].as_str(),
        Some(second_version_id.as_str())
    );
    assert_eq!(
        str_field(&stale_body, "current_head_sheet_version_ref"),
        expected_second_sheet_ref
    );
    assert_eq!(
        stale_body["current_parent_version_id"].as_str(),
        Some(second_version_id.as_str())
    );
    assert_eq!(
        str_field(&stale_body, "current_sheet_version_ref"),
        expected_second_sheet_ref
    );

    let conflict_event_count = harness
        .atelier
        .count_events_for_aggregate(
            sheet_event_family::SHEET_VERSION_CONFLICT,
            "atelier_sheet_version",
            &format!("{}:conflict", character_ref(character_uuid)),
        )
        .await
        .expect("count conflict events");
    assert_eq!(
        conflict_event_count, 1,
        "stale sheet writes must leave a durable conflict event"
    );

    let history: Vec<serde_json::Value> = client
        .get(&sheet_versions_url)
        .send()
        .await
        .expect("history")
        .json()
        .await
        .expect("history json");
    assert_eq!(history.len(), 2, "the stale write must not have appended");

    let (status, by_id) = json_body(
        client
            .get(format!("{base_url}/atelier/sheet-versions/{second_version_id}"))
            .send()
            .await
            .expect("get sheet version"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(str_field(&by_id, "sheet_version_ref"), expected_second_sheet_ref);

    let missing_character_id = Uuid::now_v7();
    let missing_history = client
        .get(format!(
            "{base_url}/atelier/characters/{missing_character_id}/sheet-versions"
        ))
        .send()
        .await
        .expect("missing history");
    assert_eq!(
        missing_history.status(),
        StatusCode::NOT_FOUND,
        "unknown character history must not look like a valid empty sheet"
    );

    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_bundled_template_import_export_and_field_suggestions() {
    let harness = AtelierSurrealHarness::create().await;
    let (base_url, client, server) = serve(app_state(&harness)).await;
    let public_id = format!("mt009-template-char-{}", Uuid::now_v7());
    let display_name = "MT-009 Template Proof";

    let (status, template) = json_body(
        client
            .get(format!("{base_url}/atelier/sheet-templates/default"))
            .send()
            .await
            .expect("template"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "CKC must expose the built-in v2.00 character sheet template"
    );
    assert_eq!(template["template_version"].as_str(), Some("v2.00"));
    assert_eq!(
        template["file_name"].as_str(),
        Some("CHARACTER_SHEET__v2.00.txt")
    );
    assert!(template["field_count"].as_i64().unwrap_or_default() > 100);
    let raw_template = str_field(&template, "raw_text");
    assert!(raw_template.contains("CHARACTER SHEET"));
    assert!(raw_template.contains("CHAR-ID-001 — Character_ID: <string>"));
    assert!(raw_template.contains("CHAR-ID-002 — Name: <string>"));

    let (status, safe_subset) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-templates/default/safe-subset"
            ))
            .send()
            .await
            .expect("safe subset"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "CKC must expose the original LLM-safe short field subset"
    );
    assert_eq!(safe_subset["template_version"].as_str(), Some("v2.00"));
    assert_eq!(
        safe_subset["file_name"].as_str(),
        Some("LLM_SAFE_SUBSET__v2.00.json")
    );
    assert!(safe_subset["field_ids"]
        .as_array()
        .expect("safe subset field ids")
        .iter()
        .any(|value| value.as_str() == Some("CHAR-ID-006")));

    let (status, character) = json_body(
        with_operator(client.post(format!("{base_url}/atelier/characters")))
            .json(&serde_json::json!({
                "public_id": public_id,
                "display_name": display_name,
                "create_default_sheet": true,
            }))
            .send()
            .await
            .expect("create character with default sheet"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{character}");
    let character_internal_id = str_field(&character, "internal_id").to_owned();
    let sheet_versions_url = format!("{base_url}/atelier/characters/{character_internal_id}/sheet-versions");
    let import_url = format!("{sheet_versions_url}/import");

    let history: Vec<serde_json::Value> = client
        .get(&sheet_versions_url)
        .send()
        .await
        .expect("history")
        .json()
        .await
        .expect("history json");
    assert_eq!(
        history.len(),
        1,
        "create_default_sheet=true must create the first v2.00 sheet version"
    );
    let first_version_id = str_field(&history[0], "version_id").to_owned();
    let first_raw = str_field(&history[0], "raw_text").to_owned();
    assert!(first_raw.contains(&format!("CHAR-ID-001 — Character_ID: {public_id}")));
    assert!(first_raw.contains(&format!("CHAR-ID-002 — Name: {display_name}")));

    let (status, txt_export) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-versions/{first_version_id}/export?format=txt"
            ))
            .send()
            .await
            .expect("txt export"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(txt_export["format"].as_str(), Some("txt"));
    assert_eq!(txt_export["content"].as_str(), Some(first_raw.as_str()));
    assert!(str_field(&txt_export, "file_name").ends_with(".txt"));
    assert_eq!(str_field(&txt_export, "content_hash").len(), 64);

    let unsupported = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{first_version_id}/export?format=docx"
        ))
        .send()
        .await
        .expect("unsupported export");
    assert_eq!(unsupported.status(), StatusCode::BAD_REQUEST);

    let imported_raw = first_raw.replace(
        "CHAR-ID-006 — Primary_Role: <string>",
        "CHAR-ID-006 — Primary_Role: proof-primary-role",
    );
    let (status, imported) = json_body(
        with_operator(client.post(&import_url))
            .json(&serde_json::json!({
                "raw_text": imported_raw,
                "expected_parent_version_id": first_version_id,
                "tool": "ckc-template-import-test",
            }))
            .send()
            .await
            .expect("import"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "CKC import must append a guarded sheet version, not mutate the current one: {imported}"
    );
    let imported_version_id = str_field(&imported, "version_id").to_owned();
    assert_eq!(
        imported["parent_version_id"].as_str(),
        Some(first_version_id.as_str())
    );
    assert_eq!(imported["seq"].as_i64(), Some(2));

    let (status, json_export) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-versions/{imported_version_id}/export?format=json"
            ))
            .send()
            .await
            .expect("json export"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_export["format"].as_str(), Some("json"));
    let json_export_content = str_field(&json_export, "content").to_owned();
    assert!(json_export_content.contains("proof-primary-role"));

    let (status, round_trip) = json_body(
        with_operator(client.post(&import_url))
            .json(&serde_json::json!({
                "raw_text": json_export_content,
                "expected_parent_version_id": imported_version_id,
                "tool": "ckc-template-json-round-trip-test",
            }))
            .send()
            .await
            .expect("json round trip import"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "CKC JSON export content must be importable as the next sheet version: {round_trip}"
    );
    let round_trip_version_id = str_field(&round_trip, "version_id").to_owned();
    assert_eq!(
        round_trip["parent_version_id"].as_str(),
        Some(imported_version_id.as_str())
    );
    assert_eq!(
        round_trip["raw_text"].as_str(),
        Some(imported_raw.as_str()),
        "JSON export import must restore the exact raw sheet text"
    );

    let mismatched_raw = first_raw.replace(
        &format!("CHAR-ID-001 — Character_ID: {public_id}"),
        "CHAR-ID-001 — Character_ID: wrong-character-id",
    );
    let mismatched = with_operator(client.post(&import_url))
        .json(&serde_json::json!({
            "raw_text": mismatched_raw,
            "expected_parent_version_id": round_trip_version_id,
            "tool": "ckc-template-mismatch-test",
        }))
        .send()
        .await
        .expect("mismatched import");
    assert_eq!(
        mismatched.status(),
        StatusCode::BAD_REQUEST,
        "CKC import must reject a sheet whose CHAR-ID-001 belongs to another character"
    );

    let duplicate_owner_raw =
        format!("{first_raw}\nCHAR-ID-001 — Character_ID: wrong-character-id\n");
    let duplicate_owner = with_operator(client.post(&import_url))
        .json(&serde_json::json!({
            "raw_text": duplicate_owner_raw,
            "expected_parent_version_id": round_trip_version_id,
            "tool": "ckc-template-duplicate-owner-test",
        }))
        .send()
        .await
        .expect("duplicate owner import");
    assert_eq!(
        duplicate_owner.status(),
        StatusCode::BAD_REQUEST,
        "CKC import must reject sheets with duplicate CHAR-ID-001 ownership lines"
    );

    let (status, safe_export) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-versions/{round_trip_version_id}/export?format=safe-txt"
            ))
            .send()
            .await
            .expect("safe export"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(safe_export["format"].as_str(), Some("safe-txt"));
    let safe_content = str_field(&safe_export, "content");
    assert!(safe_content.contains("CHAR-ID-001 — Character_ID"));
    assert!(
        !safe_content.contains("CHAR-SEX-001"),
        "short/SFW-safe export must remove fields outside the LLM-safe subset"
    );

    let unsafe_variant_raw =
        first_raw.replace("CHAR-SEX-001 — Sex_Model:", "CHAR-SEX-001—Sex_Model:");
    assert!(
        unsafe_variant_raw.contains("CHAR-SEX-001—Sex_Model:"),
        "fixture must exercise no-space CKC field separator parsing"
    );
    let (status, unsafe_variant) = json_body(
        with_operator(client.post(&import_url))
            .json(&serde_json::json!({
                "raw_text": unsafe_variant_raw,
                "expected_parent_version_id": round_trip_version_id,
                "tool": "ckc-template-safe-export-variant-test",
            }))
            .send()
            .await
            .expect("unsafe variant import"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{unsafe_variant}");
    let unsafe_variant_version_id = str_field(&unsafe_variant, "version_id").to_owned();

    let (status, safe_json_export) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-versions/{unsafe_variant_version_id}/export?format=safe-json"
            ))
            .send()
            .await
            .expect("safe json export"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(safe_json_export["format"].as_str(), Some("safe-json"));
    let safe_json_content = str_field(&safe_json_export, "content");
    assert!(
        !safe_json_content.contains("CHAR-SEX-001"),
        "safe-json export must remove unsafe fields even when the separator has no spaces"
    );

    let (status, suggestions) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/sheet-field-suggestions?field_id=CHAR-ID-006&limit=5"
            ))
            .send()
            .await
            .expect("suggestions"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let suggestions = suggestions.as_array().expect("suggestions array");
    let primary_role = suggestions
        .iter()
        .find(|row| {
            row["field_id"].as_str() == Some("CHAR-ID-006")
                && row["value"].as_str() == Some("proof-primary-role")
        })
        .expect("CKC should remember prior input values per field for future sheet suggestions");
    assert!(
        primary_role["occurrences"].as_i64().unwrap_or_default() >= 3,
        "the value was written by the import, the JSON round trip and the unsafe variant: {primary_role}"
    );
    assert_eq!(
        primary_role["latest_character_internal_id"].as_str(),
        Some(character_internal_id.as_str())
    );
    assert!(
        !suggestions
            .iter()
            .any(|row| row["value"].as_str() == Some("<string>")),
        "template placeholder descriptors must never become suggestions"
    );

    let empty_field = client
        .get(format!(
            "{base_url}/atelier/sheet-field-suggestions?field_id=%20&limit=5"
        ))
        .send()
        .await
        .expect("empty field suggestions");
    assert_eq!(empty_field.status(), StatusCode::BAD_REQUEST);

    let normalized_public_id = format!("mt009-normalized-{}", Uuid::now_v7());
    let (status, normalized_character) = json_body(
        with_operator(client.post(format!("{base_url}/atelier/characters")))
            .json(&serde_json::json!({
                "public_id": format!("  {normalized_public_id}\n"),
                "display_name": "MT-009 Normalized Public ID",
                "create_default_sheet": true,
            }))
            .send()
            .await
            .expect("normalized create"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{normalized_character}");
    assert_eq!(
        normalized_character["public_id"].as_str(),
        Some(normalized_public_id.as_str()),
        "CKC public_id must normalize before storage and default-sheet creation"
    );
    let normalized_character_internal_id =
        str_field(&normalized_character, "internal_id").to_owned();
    let normalized_history: Vec<serde_json::Value> = client
        .get(format!(
            "{base_url}/atelier/characters/{normalized_character_internal_id}/sheet-versions"
        ))
        .send()
        .await
        .expect("normalized history")
        .json()
        .await
        .expect("normalized history json");
    let normalized_raw = str_field(&normalized_history[0], "raw_text");
    assert!(normalized_raw.contains(&format!(
        "CHAR-ID-001 — Character_ID: {normalized_public_id}"
    )));
    assert!(!normalized_raw.contains(&format!("  {normalized_public_id}")));

    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_story_and_moodboard_api_links_native_documents() {
    let harness = AtelierSurrealHarness::create().await;
    let (base_url, client, server) = serve(app_state(&harness)).await;
    let public_id = format!("mt012-story-char-{}", Uuid::now_v7());

    let (status, character) = json_body(
        with_operator(client.post(format!("{base_url}/atelier/characters")))
            .json(&serde_json::json!({
                "public_id": public_id,
                "display_name": "MT-012 Story Character",
            }))
            .send()
            .await
            .expect("create character"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{character}");
    let character_internal_id = str_field(&character, "internal_id").to_owned();
    let expected_character_ref = format!("atelier://character/{character_internal_id}");
    let documents_url = format!("{base_url}/atelier/characters/{character_internal_id}/documents");

    let (status, story) = json_body(
        with_operator(client.post(&documents_url))
            .json(&serde_json::json!({
                "doc_type": "story",
                "title": "Origin scenes",
                "body_raw_text": "Scene one: CKC story text stays separate from sheet notes.",
                "tags": [" story ", "origin", "story"],
            }))
            .send()
            .await
            .expect("create story"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{story}");
    let story_document_id = str_field(&story, "document_id").to_owned();
    let expected_story_ref = format!("atelier://document/{story_document_id}");
    assert_eq!(story["doc_type"].as_str(), Some("story"));
    assert_eq!(str_field(&story, "document_ref"), expected_story_ref);
    assert_eq!(str_field(&story, "character_ref"), expected_character_ref);
    assert_eq!(story["tags"], serde_json::json!(["story", "origin"]));
    assert_eq!(
        story["current_version"]["body_raw_text"].as_str(),
        Some("Scene one: CKC story text stays separate from sheet notes.")
    );
    let first_story_version_id = str_field(&story, "current_version_id").to_owned();
    let expected_first_story_version_ref =
        format!("atelier://document/{story_document_id}/version/{first_story_version_id}");

    let (status, fetched_story) = json_body(
        client
            .get(format!("{base_url}/atelier/character-documents/{story_document_id}"))
            .send()
            .await
            .expect("get document"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(str_field(&fetched_story, "document_ref"), expected_story_ref);

    let versions_url = format!("{base_url}/atelier/character-documents/{story_document_id}/versions");
    let (status, story_append) = json_body(
        with_operator(client.post(&versions_url))
            .json(&serde_json::json!({
                "title": "Origin scenes",
                "body_raw_text": "Scene two: guarded append from the active story version.",
                "tags": ["story", "origin", "append"],
                "expected_parent_version_id": first_story_version_id,
            }))
            .send()
            .await
            .expect("append story version"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{story_append}");
    let second_story_version_id = str_field(&story_append, "current_version_id").to_owned();
    let expected_second_story_version_ref =
        format!("atelier://document/{story_document_id}/version/{second_story_version_id}");
    assert_eq!(
        story_append["current_version"]["parent_version_id"].as_str(),
        Some(first_story_version_id.as_str()),
        "CKC story document append must link to the caller's expected parent"
    );
    assert_eq!(story_append["current_version_seq"], 2);
    assert_eq!(
        story_append["tags"],
        serde_json::json!(["story", "origin", "append"]),
        "the document row returned with the append must already carry the new metadata"
    );

    let (status, stale_story_append) = json_body(
        with_operator(client.post(&versions_url))
            .json(&serde_json::json!({
                "title": "Origin scenes",
                "body_raw_text": "This stale write must not become the story head.",
                "tags": ["story", "stale"],
                "expected_parent_version_id": first_story_version_id,
            }))
            .send()
            .await
            .expect("stale story append"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "stale CKC story document append must not advance the current document head: {stale_story_append}"
    );
    assert_eq!(
        stale_story_append["error"].as_str(),
        Some("stale_character_document_version")
    );
    assert_eq!(str_field(&stale_story_append, "document_ref"), expected_story_ref);
    assert_eq!(
        stale_story_append["expected_parent_version_id"].as_str(),
        Some(first_story_version_id.as_str())
    );
    assert_eq!(
        str_field(&stale_story_append, "expected_parent_document_version_ref"),
        expected_first_story_version_ref
    );
    assert_eq!(
        str_field(&stale_story_append, "expected_document_version_ref"),
        expected_first_story_version_ref
    );
    assert_eq!(
        stale_story_append["current_head_version_id"].as_str(),
        Some(second_story_version_id.as_str())
    );
    assert_eq!(
        str_field(&stale_story_append, "current_head_document_version_ref"),
        expected_second_story_version_ref
    );

    let story_history: Vec<serde_json::Value> = client
        .get(&versions_url)
        .send()
        .await
        .expect("story history")
        .json()
        .await
        .expect("story history json");
    assert_eq!(story_history.len(), 2, "the stale write must not have appended");

    let (status, moodboard_doc) = json_body(
        with_operator(client.post(&documents_url))
            .json(&serde_json::json!({
                "doc_type": "moodboard",
                "title": "Visual continuity board",
                "body_raw_text": "Moodboard document links the native canvas-style board.",
                "tags": ["moodboard", "visual"],
            }))
            .send()
            .await
            .expect("create moodboard document"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{moodboard_doc}");
    let moodboard_document_id = str_field(&moodboard_doc, "document_id").to_owned();
    let first_moodboard_version_id = str_field(&moodboard_doc, "current_version_id").to_owned();
    let expected_moodboard_document_ref = format!("atelier://document/{moodboard_document_id}");
    assert_eq!(moodboard_doc["doc_type"].as_str(), Some("moodboard"));
    assert_eq!(
        str_field(&moodboard_doc, "document_ref"),
        expected_moodboard_document_ref
    );

    let story_list: Vec<serde_json::Value> = client
        .get(format!("{documents_url}?doc_type=story"))
        .send()
        .await
        .expect("story list")
        .json()
        .await
        .expect("story list json");
    assert_eq!(story_list.len(), 1, "doc_type filter keeps story separate");
    assert_eq!(str_field(&story_list[0], "document_ref"), expected_story_ref);
    let all_docs: Vec<serde_json::Value> = client
        .get(&documents_url)
        .send()
        .await
        .expect("all docs")
        .json()
        .await
        .expect("all docs json");
    assert_eq!(all_docs.len(), 2);
    let bad_filter = client
        .get(format!("{documents_url}?doc_type=poster"))
        .send()
        .await
        .expect("bad filter");
    assert_eq!(bad_filter.status(), StatusCode::BAD_REQUEST);

    let cards_url = format!("{base_url}/atelier/character-documents/{story_document_id}/story-cards");
    let (status, card) = json_body(
        with_operator(client.post(&cards_url))
            .json(&serde_json::json!({
                "title": "Meet-cute beat card",
                "body_raw_text": "First reusable scene card.",
                "tags": ["scene", "setup", "scene"],
            }))
            .send()
            .await
            .expect("add card"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{card}");
    let card_id = str_field(&card, "card_id").to_owned();
    assert_eq!(str_field(&card, "story_document_ref"), expected_story_ref);
    assert_eq!(str_field(&card, "card_ref"), format!("atelier://story-card/{card_id}"));
    assert_eq!(card["tags"], serde_json::json!(["scene", "setup"]));
    let cards: Vec<serde_json::Value> = client
        .get(&cards_url)
        .send()
        .await
        .expect("list cards")
        .json()
        .await
        .expect("cards json");
    assert_eq!(cards.len(), 1);

    let beats_url = format!("{base_url}/atelier/character-documents/{story_document_id}/story-beats");
    let (status, beat) = json_body(
        with_operator(client.post(&beats_url))
            .json(&serde_json::json!({
                "card_id": card_id,
                "beat_text": "Argus can target this beat without reading the character sheet.",
            }))
            .send()
            .await
            .expect("add beat"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{beat}");
    assert_eq!(str_field(&beat, "story_document_ref"), expected_story_ref);
    assert_eq!(beat["card_id"].as_str(), Some(card_id.as_str()));
    assert_eq!(
        beat["card_ref"].as_str(),
        Some(format!("atelier://story-card/{card_id}").as_str())
    );
    let beats: Vec<serde_json::Value> = client
        .get(&beats_url)
        .send()
        .await
        .expect("list beats")
        .json()
        .await
        .expect("beats json");
    assert_eq!(beats.len(), 1);

    let wrong_doc_beat = with_operator(client.post(format!(
        "{base_url}/atelier/character-documents/{moodboard_document_id}/story-beats"
    )))
    .json(&serde_json::json!({
        "card_id": null,
        "beat_text": "This must not attach to a moodboard document.",
    }))
    .send()
    .await
    .expect("wrong doc beat");
    assert_eq!(
        wrong_doc_beat.status(),
        StatusCode::BAD_REQUEST,
        "story beats must reject non-story documents instead of mixing story/moodboard state"
    );

    let layer_id = Uuid::now_v7();
    let moodboard_snapshot = serde_json::json!({
        "schema_id": "hsk.atelier.moodboard@1",
        "schema_version": 1,
        "moodboard_id": moodboard_document_id,
        "name": "Visual continuity board",
        "description": "Native Handshake moodboard snapshot, not Excalidraw.",
        "canvas": {
            "width": 1600.0,
            "height": 1000.0,
            "background_color": "#101418"
        },
        "layers": [{
            "layer_id": layer_id,
            "name": "Reference layer",
            "order": 1,
            "visible": true,
            "locked": false,
            "opacity": 1.0,
            "parent_layer_id": null
        }],
        "images": [],
        "text": [],
        "shapes": [],
        "connectors": [],
        "folders": [],
        "guides": [],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": false
        },
        "style": {
            "dominant_colors": ["#101418"],
            "mood_keywords": ["continuity"],
            "style_description": "native moodboard",
            "suggested_presets": []
        },
        "history": [{
            "history_id": Uuid::now_v7(),
            "at": "2026-06-29T00:00:00Z",
            "actor": "operator",
            "operation": "created",
            "summary": "Initial CKC moodboard snapshot"
        }]
    });
    let snapshot_json_text = serde_json::to_string(&moodboard_snapshot).expect("snapshot json");
    let snapshots_url = format!(
        "{base_url}/atelier/character-documents/{moodboard_document_id}/moodboard/snapshots"
    );
    let (status, snapshot) = json_body(
        with_operator(client.post(&snapshots_url))
            .json(&serde_json::json!({
                "raw_json_text": snapshot_json_text,
                "expected_document_version_id": first_moodboard_version_id,
            }))
            .send()
            .await
            .expect("record snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{snapshot}");
    let snapshot_id = str_field(&snapshot, "snapshot_id").to_owned();
    let expected_moodboard_ref = format!("atelier://moodboard/{snapshot_id}");
    assert_eq!(
        str_field(&snapshot, "document_ref"),
        expected_moodboard_document_ref
    );
    assert_eq!(str_field(&snapshot, "moodboard_ref"), expected_moodboard_ref);
    assert_eq!(
        snapshot["moodboard"]["name"].as_str(),
        Some("Visual continuity board")
    );
    assert_eq!(snapshot["author"].as_str(), Some("operator"));

    let (status, latest) = json_body(
        client
            .get(format!(
                "{base_url}/atelier/character-documents/{moodboard_document_id}/moodboard/latest"
            ))
            .send()
            .await
            .expect("latest snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        str_field(&latest, "moodboard_ref"),
        expected_moodboard_ref,
        "latest moodboard route returns the reusable snapshot ref"
    );

    let (status, moodboard_append) = json_body(
        with_operator(client.post(format!(
            "{base_url}/atelier/character-documents/{moodboard_document_id}/versions"
        )))
        .json(&serde_json::json!({
            "title": "Visual continuity board",
            "body_raw_text": snapshot_json_text,
            "tags": ["moodboard", "visual", "second"],
            "expected_parent_version_id": first_moodboard_version_id,
        }))
        .send()
        .await
        .expect("append moodboard doc version"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{moodboard_append}");
    let second_moodboard_version_id =
        str_field(&moodboard_append, "current_version_id").to_owned();
    let expected_first_moodboard_version_ref =
        format!("atelier://document/{moodboard_document_id}/version/{first_moodboard_version_id}");
    let expected_second_moodboard_version_ref =
        format!("atelier://document/{moodboard_document_id}/version/{second_moodboard_version_id}");

    let (status, stale_snapshot) = json_body(
        with_operator(client.post(&snapshots_url))
            .json(&serde_json::json!({
                "raw_json_text": snapshot_json_text,
                "expected_document_version_id": first_moodboard_version_id,
            }))
            .send()
            .await
            .expect("stale snapshot"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "stale CKC moodboard snapshot writes must not attach to a newer document head: {stale_snapshot}"
    );
    assert_eq!(
        stale_snapshot["error"].as_str(),
        Some("stale_moodboard_document_version")
    );
    assert_eq!(
        str_field(&stale_snapshot, "expected_document_version_ref"),
        expected_first_moodboard_version_ref
    );
    assert_eq!(
        str_field(&stale_snapshot, "current_head_document_version_ref"),
        expected_second_moodboard_version_ref
    );

    let wrong_doc_snapshot = with_operator(client.post(format!(
        "{base_url}/atelier/character-documents/{story_document_id}/moodboard/snapshots"
    )))
    .json(&serde_json::json!({
        "raw_json_text": snapshot_json_text,
    }))
    .send()
    .await
    .expect("wrong doc snapshot");
    assert_eq!(
        wrong_doc_snapshot.status(),
        StatusCode::BAD_REQUEST,
        "moodboard snapshots must reject story documents"
    );

    let wrong_doc_latest = client
        .get(format!(
            "{base_url}/atelier/character-documents/{story_document_id}/moodboard/latest"
        ))
        .send()
        .await
        .expect("wrong doc latest");
    assert_eq!(
        wrong_doc_latest.status(),
        StatusCode::BAD_REQUEST,
        "latest moodboard route must reject story documents, not report a missing snapshot"
    );

    let missing_document = client
        .get(format!(
            "{base_url}/atelier/character-documents/{}",
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("missing document");
    assert_eq!(missing_document.status(), StatusCode::NOT_FOUND);

    server.abort();
    harness.shutdown().await;
}

/// The model-operation lease guard on this lane's mutations, plus the sheet artifact-link routes
/// driven through a real lease: bare actors are refused, an operator declaration is accepted only
/// for the `operator` actor, and a lease is accepted only when it is bound to the exact thread
/// the route demands.
#[tokio::test]
async fn atelier_ckc_sheets_guarded_mutations_require_lease_or_operator() {
    let harness = AtelierSurrealHarness::create().await;
    let (base_url, client, server) = serve(app_state(&harness)).await;
    let actor = format!("model:mt059-{}", Uuid::now_v7());
    let session_id = format!("session-{}", Uuid::now_v7());
    let public_id = format!("mt059-lease-char-{}", Uuid::now_v7());
    let create_body = serde_json::json!({
        "public_id": public_id,
        "display_name": "MT-059 Lease Character",
    });

    let no_actor = client
        .post(format!("{base_url}/atelier/characters"))
        .json(&create_body)
        .send()
        .await
        .expect("no actor");
    assert_eq!(no_actor.status(), StatusCode::BAD_REQUEST, "missing x-hsk-actor-id");

    let bare_actor = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&create_body)
        .send()
        .await
        .expect("bare actor");
    assert_eq!(
        bare_actor.status(),
        StatusCode::BAD_REQUEST,
        "a model actor without a lease must not create a character"
    );

    let fake_operator = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .header("x-hsk-actor-kind", "operator")
        .json(&create_body)
        .send()
        .await
        .expect("fake operator");
    assert_eq!(
        fake_operator.status(),
        StatusCode::BAD_REQUEST,
        "x-hsk-actor-kind=operator is reserved for x-hsk-actor-id=operator"
    );

    let lease_without_session = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .header("x-hsk-model-lease-id", Uuid::now_v7().to_string())
        .json(&create_body)
        .send()
        .await
        .expect("lease without session");
    assert_eq!(lease_without_session.status(), StatusCode::BAD_REQUEST);

    let unknown_lease = with_lease(
        client.post(format!("{base_url}/atelier/characters")),
        &actor,
        Uuid::now_v7(),
        &session_id,
    )
    .json(&create_body)
    .send()
    .await
    .expect("unknown lease");
    assert_eq!(unknown_lease.status(), StatusCode::NOT_FOUND, "unknown claim id");

    let wrong_thread_claim = claim_lease(
        &harness,
        &ckc_character_create_model_operation_thread_id("some-other-character"),
        &actor,
        &session_id,
    )
    .await;
    let wrong_thread = with_lease(
        client.post(format!("{base_url}/atelier/characters")),
        &actor,
        wrong_thread_claim,
        &session_id,
    )
    .json(&create_body)
    .send()
    .await
    .expect("wrong thread lease");
    assert_eq!(
        wrong_thread.status(),
        StatusCode::CONFLICT,
        "a lease on another coordination thread must not authorise this mutation"
    );

    let create_claim = claim_lease(
        &harness,
        &ckc_character_create_model_operation_thread_id(&public_id),
        &actor,
        &session_id,
    )
    .await;
    let other_session = with_lease(
        client.post(format!("{base_url}/atelier/characters")),
        &actor,
        create_claim,
        "not-the-lease-session",
    )
    .json(&create_body)
    .send()
    .await
    .expect("other session");
    assert_eq!(
        other_session.status(),
        StatusCode::CONFLICT,
        "the lease is bound to actor+session"
    );

    let (status, character) = json_body(
        with_lease(
            client.post(format!("{base_url}/atelier/characters")),
            &actor,
            create_claim,
            &session_id,
        )
        .json(&create_body)
        .send()
        .await
        .expect("leased create"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{character}");
    let character_internal_id = str_field(&character, "internal_id").to_owned();

    // The sheet append demands the character thread; seed one version as operator, then drive the
    // artifact-link routes under a lease bound to the sheet-version artifacts thread.
    let (status, sheet) = json_body(
        with_operator(client.post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        )))
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-059"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await
        .expect("seed sheet version"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sheet}");
    let version_id = str_field(&sheet, "version_id").to_owned();
    let version_uuid = Uuid::parse_str(&version_id).expect("version uuid");
    let links_url = format!("{base_url}/atelier/sheet-versions/{version_id}/artifact-links");
    let artifact_ref = format!(
        "artifact://.handshake/artifacts/L1/{}/payload",
        Uuid::now_v7()
    );
    let attach_body = serde_json::json!({
        "artifact_kind": "openpose_png",
        "artifact_ref": artifact_ref,
        "manifest_ref": null,
        "source_ref": format!("posekit://rig/{}", Uuid::now_v7()),
        "label": "yaw +45 openpose conditioning",
        "reuse_role": "cui_openpose_conditioning",
        "metadata": { "yaw_degrees": 45 }
    });

    let bare_attach = client
        .post(&links_url)
        .header("x-hsk-actor-id", &actor)
        .json(&attach_body)
        .send()
        .await
        .expect("bare attach");
    assert_eq!(bare_attach.status(), StatusCode::BAD_REQUEST);

    let artifacts_claim = claim_lease(
        &harness,
        &ckc_sheet_artifacts_model_operation_thread_id(version_uuid),
        &actor,
        &session_id,
    )
    .await;
    let (status, link) = json_body(
        with_lease(client.post(&links_url), &actor, artifacts_claim, &session_id)
            .json(&attach_body)
            .send()
            .await
            .expect("leased attach"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{link}");
    let link_id = str_field(&link, "link_id").to_owned();
    assert_eq!(link["linked_by"].as_str(), Some(actor.as_str()));
    assert_eq!(link["artifact_kind"].as_str(), Some("openpose_png"));
    assert_eq!(
        str_field(&link, "typed_ref"),
        format!("atelier://sheet-artifact/{link_id}")
    );
    assert_eq!(
        str_field(&link, "sheet_version_ref"),
        format!("atelier://sheet/{character_internal_id}/{version_id}")
    );
    assert_eq!(link["metadata"]["yaw_degrees"], 45);

    let (status, repeat) = json_body(
        with_lease(client.post(&links_url), &actor, artifacts_claim, &session_id)
            .json(&attach_body)
            .send()
            .await
            .expect("repeat attach"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a repeat attach of the same active (kind, ref) returns the existing link with 200: {repeat}"
    );
    assert_eq!(repeat["link_id"].as_str(), Some(link_id.as_str()));
    let link_uuid = Uuid::parse_str(&link_id).expect("link uuid");
    assert_eq!(
        harness
            .atelier
            .count_events_for_aggregate(
                sheet_artifact_event_family::SHEET_ARTIFACT_LINKED,
                "atelier_sheet_artifact_link",
                &link_id,
            )
            .await
            .expect("count linked events"),
        1
    );
    assert_eq!(link_uuid.get_version_num(), 7);

    let rejected_ref = with_lease(client.post(&links_url), &actor, artifacts_claim, &session_id)
        .json(&serde_json::json!({
            "artifact_kind": "comfy_render",
            "artifact_ref": "D:\\renders\\out.png",
        }))
        .send()
        .await
        .expect("machine-local ref");
    assert_eq!(
        rejected_ref.status(),
        StatusCode::BAD_REQUEST,
        "machine-local artifact refs are ForbiddenStorage -> 400 bad_request over HTTP (reference contract)"
    );
    let bad_kind = with_lease(client.post(&links_url), &actor, artifacts_claim, &session_id)
        .json(&serde_json::json!({
            "artifact_kind": "poster",
            "artifact_ref": artifact_ref,
        }))
        .send()
        .await
        .expect("bad kind");
    assert_eq!(bad_kind.status(), StatusCode::BAD_REQUEST);

    let listed: Vec<serde_json::Value> = client
        .get(&links_url)
        .send()
        .await
        .expect("list links")
        .json()
        .await
        .expect("links json");
    assert_eq!(listed.len(), 1);
    let (status, resolved) = json_body(
        client
            .get(format!("{base_url}/atelier/sheet-artifact-links/{link_id}"))
            .send()
            .await
            .expect("resolve link"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resolved["artifact_ref"].as_str(), Some(artifact_ref.as_str()));

    let bare_detach = client
        .delete(format!("{base_url}/atelier/sheet-artifact-links/{link_id}"))
        .header("x-hsk-actor-id", &actor)
        .send()
        .await
        .expect("bare detach");
    assert_eq!(bare_detach.status(), StatusCode::BAD_REQUEST);

    let (status, detached) = json_body(
        with_lease(
            client.delete(format!("{base_url}/atelier/sheet-artifact-links/{link_id}")),
            &actor,
            artifacts_claim,
            &session_id,
        )
        .send()
        .await
        .expect("leased detach"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detached}");
    assert!(detached["detached_at_utc"].is_string());
    assert_eq!(detached["detached_by"].as_str(), Some(actor.as_str()));

    let gone = client
        .get(format!("{base_url}/atelier/sheet-artifact-links/{link_id}"))
        .send()
        .await
        .expect("resolve detached");
    assert_eq!(gone.status(), StatusCode::NOT_FOUND, "detached typed refs are not active");
    let after: Vec<serde_json::Value> = client
        .get(&links_url)
        .send()
        .await
        .expect("list after detach")
        .json()
        .await
        .expect("links json");
    assert!(after.is_empty());
    let second_detach = with_lease(
        client.delete(format!("{base_url}/atelier/sheet-artifact-links/{link_id}")),
        &actor,
        artifacts_claim,
        &session_id,
    )
    .send()
    .await
    .expect("second detach");
    assert_eq!(second_detach.status(), StatusCode::NOT_FOUND);

    let missing_version_links = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{}/artifact-links",
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("missing version links");
    assert_eq!(missing_version_links.status(), StatusCode::NOT_FOUND);

    server.abort();
    harness.shutdown().await;
}
