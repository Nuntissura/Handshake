//! WP-KERNEL-005 atelier Stealth Reference Window: real PostgreSQL round-trip
//! proofs for the stealth_window submodule (MT-205). Run with a live
//! DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_stealth_window_tests -- --nocapture
//!
//! No mocks: each test connects the actual `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the stealth-window registry with REAL data,
//! and asserts the load-bearing invariants: idempotent window create on
//! (owner_actor, title), append-only seq monotonicity + UNIQUE(window, seq),
//! two-phase reorder permutation guard, idempotent capture receipt on
//! (window, manifest_id), governed-resolver LAW rejection, redaction assertion,
//! soft-close audit retention, and EventLedger emission via count_events.
//! Tables persist between runs, so all titles / resolvers / manifest ids are
//! made unique per run via `Uuid::new_v4()` to avoid cross-run collisions. Only
//! `handshake_core` + `tokio` + `uuid` (+ serde_json + std) are used for
//! behavioral proofs; the schema-hardening proof queries `information_schema`
//! through sqlx so direct database defaults cannot silently mint UUID v4 ids.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::atelier as atelier_api;
use handshake_core::atelier::search::search_event_family;
use handshake_core::atelier::search::TagType;
use handshake_core::atelier::stealth_window::stealth_ref_event_family::{
    STEALTH_REF_ADDED, STEALTH_REF_CAPTURED, STEALTH_REF_REMOVED, STEALTH_REF_REORDERED,
    STEALTH_REF_WINDOW_CLOSED, STEALTH_REF_WINDOW_CREATED,
};
use handshake_core::atelier::stealth_window::{
    ContentRefKind, NewContentRef, NewStealthWindow, QuietFlags, StealthRefStatus, VisibilityFlag,
};
use handshake_core::atelier::{
    character_ref, collection_ref, event_family, media_asset_ref, AtelierStore,
    MediaSidecarRelationKind, NewCharacter, NewMediaAsset, NewMediaSidecarRelation,
    NewSheetVersion,
};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse, LlmClient,
    LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::tests::optional_postgres_backend_with_pool_from_env;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use sqlx::Row;
use uuid::Uuid;

mod atelier_pg_support;

const SIDECAR_VISIBILITY_HEALTH_LOCK_ID: i64 = 5_023_022;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root resolves from handshake_core manifest")
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

async fn acquire_sidecar_visibility_health_lock(
    store: &AtelierStore,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut tx = store
        .pool()
        .begin()
        .await
        .expect("begin sidecar visibility health lock transaction");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(SIDECAR_VISIBILITY_HEALTH_LOCK_ID)
        .execute(&mut *tx)
        .await
        .expect("acquire sidecar visibility health lock");
    tx
}

async fn fresh_api_media_asset(store: &AtelierStore, label: &str) -> Uuid {
    let artifact = atelier_pg_support::write_native_media_artifact(label.as_bytes());
    store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some(format!("atelier-api-{label}")),
            artifact_ref: artifact.artifact_ref,
        })
        .await
        .expect("materialize API test media asset")
        .asset_id
}

fn quote_pg_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

async fn sidecar_visibility_constraints(store: &AtelierStore) -> Vec<(String, String)> {
    sqlx::query(
        r#"SELECT conname, pg_get_constraintdef(oid) AS constraint_def
           FROM pg_constraint
           WHERE conrelid = 'atelier_media_sidecar'::regclass
             AND contype = 'c'
             AND (
                pg_get_constraintdef(oid) ILIKE '%hidden_from_gallery%'
                OR pg_get_constraintdef(oid) ILIKE '%searchable_by_relation%'
             )
           ORDER BY conname"#,
    )
    .fetch_all(store.pool())
    .await
    .expect("read sidecar visibility constraints")
    .iter()
    .map(|row| {
        (
            row.get::<String, _>("conname"),
            row.get::<String, _>("constraint_def"),
        )
    })
    .collect()
}

async fn drop_sidecar_visibility_constraints(
    store: &AtelierStore,
    constraints: &[(String, String)],
) {
    for (name, _) in constraints {
        sqlx::query(&format!(
            "ALTER TABLE atelier_media_sidecar DROP CONSTRAINT {}",
            quote_pg_ident(name)
        ))
        .execute(store.pool())
        .await
        .expect("drop sidecar visibility constraint for API drift probe");
    }
}

async fn restore_sidecar_visibility_constraints(
    store: &AtelierStore,
    constraints: &[(String, String)],
    sidecar_id: Uuid,
) {
    sqlx::query(
        r#"UPDATE atelier_media_sidecar
           SET hidden_from_gallery = TRUE,
               searchable_by_relation = TRUE,
               updated_at_utc = NOW()
           WHERE sidecar_id = $1"#,
    )
    .bind(sidecar_id)
    .execute(store.pool())
    .await
    .expect("repair API drift sidecar row before restoring constraints");

    for (name, definition) in constraints {
        let check_sql = if definition.contains("hidden_from_gallery") {
            "CHECK (hidden_from_gallery = TRUE)"
        } else {
            "CHECK (searchable_by_relation = TRUE)"
        };
        sqlx::query(&format!(
            "ALTER TABLE atelier_media_sidecar ADD CONSTRAINT {} {}",
            quote_pg_ident(name),
            check_sql
        ))
        .execute(store.pool())
        .await
        .expect("restore API sidecar visibility constraint after drift probe");
    }
}

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
        id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        let _ = id;
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

impl NoopLlmClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new("atelier-api-test".to_string(), 4096),
        }
    }
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

    async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        Ok(EmbeddingResponse {
            vector: deterministic_test_embedding(
                &req.input,
                handshake_core::loom_search::LOOM_SEARCH_EMBEDDING_DIM,
            ),
            model_id: req.model_id,
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn deterministic_test_embedding(input: &str, dim: usize) -> Vec<f32> {
    let mut vector = vec![0.0f32; dim.max(1)];
    for token in input
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let digest = hasher.finalize();
        let idx = (u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) as usize)
            % vector.len();
        let sign = if digest[4] & 1 == 0 { 1.0 } else { -1.0 };
        vector[idx] += sign;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

async fn test_app_state_from_database_url() -> Option<AppState> {
    if std::env::var("POSTGRES_TEST_URL").is_err() {
        let Some(url) = atelier_pg_support::database_url().await else {
            eprintln!("SKIP atelier api state: no DATABASE_URL and managed PostgreSQL unavailable");
            return None;
        };
        std::env::set_var("POSTGRES_TEST_URL", url);
    }

    let backend = optional_postgres_backend_with_pool_from_env()
        .await
        .expect("create isolated postgres test backend")?;
    let store = AtelierStore::new(backend.postgres_pool.clone());
    store.ensure_schema().await.expect("ensure atelier schema");

    let recorder = Arc::new(NoopRecorder);
    Some(AppState {
        storage: backend.database,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient::new()),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: backend.postgres_pool,
    })
}

async fn start_atelier_api_server(
    state: AppState,
) -> Result<(String, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = atelier_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("atelier api server");
    });
    Ok((format!("http://{addr}"), server))
}

/// Build a run-unique `NewStealthWindow` with default (all-ON) quiet flags and
/// the non-intrusive off-screen-only visibility.
fn fresh_window_input() -> NewStealthWindow {
    NewStealthWindow {
        owner_actor: format!("operator-{}", Uuid::new_v4()),
        title: format!("stealth-window-{}", Uuid::new_v4()),
        visibility: VisibilityFlag::OffScreenOnly,
        quiet: QuietFlags::default(),
        layout: None,
    }
}

/// A run-unique governed resolver id (never a localhost / network / file
/// authority, never a machine-local path), accepted by `validate_resolver`.
fn governed_resolver() -> String {
    format!("artifact-manifest-{}", Uuid::new_v4())
}

fn assert_uuid_v7(id: Uuid, label: &str) {
    assert_eq!(id.get_version_num(), 7, "{label} must be UUID v7");
}

#[tokio::test]
async fn atelier_character_sheet_api_round_trips_refs_and_conflicts(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let pool = state.postgres_pool.clone();
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let actor = format!("mt009-agent-{}", Uuid::new_v4());
    let public_id = format!("mt009-char-{}", Uuid::new_v4());
    let projection_exists: bool = sqlx::query_scalar(
        "SELECT to_regclass('atelier_sheet_field_value_projection') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;
    assert!(
        projection_exists,
        "schema bootstrap must create the CKC field-value projection even on already-ready DBs"
    );
    let tag_note_exists: bool = sqlx::query_scalar(
        "SELECT to_regclass('atelier_tag_note') IS NOT NULL
              AND to_regclass('atelier_ckc_search_projection') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;
    assert!(
        tag_note_exists,
        "schema bootstrap must replay CKC search/tag-note migration on already-ready DBs"
    );

    let created = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": public_id,
            "display_name": "MT-009 Character Sheet API",
        }))
        .send()
        .await?;
    assert_eq!(created.status(), reqwest::StatusCode::CREATED);
    let character: serde_json::Value = created.json().await?;
    let character_internal_id = character["internal_id"]
        .as_str()
        .expect("character internal_id");
    let expected_character_ref = format!("atelier://character/{character_internal_id}");
    assert_eq!(
        character["character_ref"].as_str(),
        Some(expected_character_ref.as_str())
    );

    let duplicate = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": public_id,
            "display_name": "Duplicate MT-009 Character",
        }))
        .send()
        .await?;
    assert_eq!(
        duplicate.status(),
        reqwest::StatusCode::CONFLICT,
        "duplicate public_id must be a typed 409, not an infra 500"
    );
    let duplicate_body: serde_json::Value = duplicate.json().await?;
    assert_eq!(duplicate_body["error"].as_str(), Some("conflict"));

    let missing_owner = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": "name: MT-009\nrole: route proof",
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(
        missing_owner.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "full CKC sheet append must include CHAR-ID-001 so ownership is deterministic"
    );

    let duplicate_owner = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-001 — Character_ID: wrong-character-id"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(
        duplicate_owner.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "CKC sheet append must reject ambiguous duplicate CHAR-ID-001 ownership lines"
    );

    let first = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-006 — Primary_Role: route proof"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(first.status(), reqwest::StatusCode::CREATED);
    let first: serde_json::Value = first.json().await?;
    let first_version_id = first["version_id"].as_str().expect("version id");
    let expected_first_sheet_ref =
        format!("atelier://sheet/{character_internal_id}/{first_version_id}");
    assert_eq!(first["seq"], 1);
    assert_eq!(first["author"].as_str(), Some(actor.as_str()));
    assert_eq!(
        first["sheet_version_ref"].as_str(),
        Some(expected_first_sheet_ref.as_str())
    );
    let first_projection_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
           FROM atelier_sheet_field_value_projection
          WHERE sheet_version_id = $1
            AND field_id = 'CHAR-ID-006'
            AND value = 'route proof'",
    )
    .bind(Uuid::parse_str(first_version_id)?)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        first_projection_count, 1,
        "append-only CKC sheet writes must populate the field-value projection"
    );

    let second = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-009\nCHAR-ID-006 — Primary_Role: updated route proof"),
            "expected_parent_version_id": first_version_id,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(second.status(), reqwest::StatusCode::CREATED);
    let second: serde_json::Value = second.json().await?;
    assert_eq!(second["parent_version_id"].as_str(), Some(first_version_id));
    assert_eq!(second["seq"], 2);
    let second_version_id = second["version_id"].as_str().expect("second version id");
    let expected_second_sheet_ref =
        format!("atelier://sheet/{character_internal_id}/{second_version_id}");
    assert_eq!(
        second["sheet_version_ref"].as_str(),
        Some(expected_second_sheet_ref.as_str())
    );

    let stale = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: stale write"),
            "expected_parent_version_id": first_version_id,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(
        stale.status(),
        reqwest::StatusCode::CONFLICT,
        "stale expected_parent_version_id must not append over a newer head"
    );
    let stale_body: serde_json::Value = stale.json().await?;
    assert_eq!(stale_body["error"].as_str(), Some("stale_sheet_version"));
    assert_eq!(
        stale_body["character_ref"].as_str(),
        Some(expected_character_ref.as_str())
    );
    assert_eq!(
        stale_body["expected_parent_version_id"].as_str(),
        Some(first_version_id)
    );
    assert_eq!(
        stale_body["expected_parent_sheet_version_ref"].as_str(),
        Some(expected_first_sheet_ref.as_str())
    );
    assert_eq!(
        stale_body["expected_sheet_version_ref"].as_str(),
        Some(expected_first_sheet_ref.as_str())
    );
    assert_eq!(
        stale_body["current_head_version_id"].as_str(),
        Some(second_version_id)
    );
    assert_eq!(
        stale_body["current_head_sheet_version_ref"].as_str(),
        Some(expected_second_sheet_ref.as_str())
    );
    assert_eq!(
        stale_body["current_parent_version_id"].as_str(),
        Some(second_version_id)
    );
    assert_eq!(
        stale_body["current_sheet_version_ref"].as_str(),
        Some(expected_second_sheet_ref.as_str())
    );

    let conflict_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_event
         WHERE event_family = $1 AND aggregate_id = $2
           AND payload->>'reason_code' = 'stale_sheet_version'
           AND payload->>'current_head_sheet_version_ref' = $3",
    )
    .bind(event_family::SHEET_VERSION_CONFLICT)
    .bind(format!(
        "{}:conflict",
        character_ref(Uuid::parse_str(character_internal_id)?)
    ))
    .bind(&expected_second_sheet_ref)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        conflict_event_count, 1,
        "stale sheet writes must leave a durable conflict event"
    );

    let history = client
        .get(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .send()
        .await?;
    assert_eq!(history.status(), reqwest::StatusCode::OK);
    let history: Vec<serde_json::Value> = history.json().await?;
    assert_eq!(history.len(), 2);

    let missing_character_id = Uuid::new_v4();
    let missing_history = client
        .get(format!(
            "{base_url}/atelier/characters/{missing_character_id}/sheet-versions"
        ))
        .send()
        .await?;
    assert_eq!(
        missing_history.status(),
        reqwest::StatusCode::NOT_FOUND,
        "unknown character history must not look like a valid empty sheet"
    );

    server.abort();
    Ok(())
}

#[tokio::test]
async fn atelier_ckc_bundled_template_import_export_and_field_suggestions(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let actor = format!("mt009-template-agent-{}", Uuid::new_v4());
    let public_id = format!("mt009-template-char-{}", Uuid::new_v4());
    let display_name = "MT-009 Template Proof";

    let template = client
        .get(format!("{base_url}/atelier/sheet-templates/default"))
        .send()
        .await?;
    assert_eq!(
        template.status(),
        reqwest::StatusCode::OK,
        "CKC must expose the built-in v2.00 character sheet template"
    );
    let template: serde_json::Value = template.json().await?;
    assert_eq!(template["template_version"].as_str(), Some("v2.00"));
    assert_eq!(
        template["file_name"].as_str(),
        Some("CHARACTER_SHEET__v2.00.txt")
    );
    assert!(template["field_count"].as_i64().unwrap_or_default() > 100);
    let raw_template = template["raw_text"].as_str().expect("template raw text");
    assert!(raw_template.contains("CHARACTER SHEET"));
    assert!(raw_template.contains("CHAR-ID-001 — Character_ID: <string>"));
    assert!(raw_template.contains("CHAR-ID-002 — Name: <string>"));

    let safe_subset = client
        .get(format!(
            "{base_url}/atelier/sheet-templates/default/safe-subset"
        ))
        .send()
        .await?;
    assert_eq!(
        safe_subset.status(),
        reqwest::StatusCode::OK,
        "CKC must expose the original LLM-safe short field subset"
    );
    let safe_subset: serde_json::Value = safe_subset.json().await?;
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

    let created = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": public_id,
            "display_name": display_name,
            "create_default_sheet": true,
        }))
        .send()
        .await?;
    assert_eq!(created.status(), reqwest::StatusCode::CREATED);
    let character: serde_json::Value = created.json().await?;
    let character_internal_id = character["internal_id"]
        .as_str()
        .expect("character internal_id");

    let history = client
        .get(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .send()
        .await?;
    assert_eq!(history.status(), reqwest::StatusCode::OK);
    let history: Vec<serde_json::Value> = history.json().await?;
    assert_eq!(
        history.len(),
        1,
        "create_default_sheet=true must create the first v2.00 sheet version"
    );
    let first_version_id = history[0]["version_id"].as_str().expect("first version id");
    let first_raw = history[0]["raw_text"].as_str().expect("first raw text");
    assert!(first_raw.contains(&format!("CHAR-ID-001 — Character_ID: {public_id}")));
    assert!(first_raw.contains(&format!("CHAR-ID-002 — Name: {display_name}")));

    let txt_export = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{first_version_id}/export?format=txt"
        ))
        .send()
        .await?;
    assert_eq!(txt_export.status(), reqwest::StatusCode::OK);
    let txt_export: serde_json::Value = txt_export.json().await?;
    assert_eq!(txt_export["format"].as_str(), Some("txt"));
    assert_eq!(txt_export["content"].as_str(), Some(first_raw));
    assert!(txt_export["file_name"]
        .as_str()
        .expect("export file name")
        .ends_with(".txt"));

    let imported_raw = first_raw.replace(
        "CHAR-ID-006 — Primary_Role: <string>",
        "CHAR-ID-006 — Primary_Role: proof-primary-role",
    );
    let imported = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions/import"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": imported_raw,
            "expected_parent_version_id": first_version_id,
            "tool": "ckc-template-import-test",
        }))
        .send()
        .await?;
    assert_eq!(
        imported.status(),
        reqwest::StatusCode::CREATED,
        "CKC import must append a guarded sheet version, not mutate the current one"
    );
    let imported: serde_json::Value = imported.json().await?;
    let imported_version_id = imported["version_id"]
        .as_str()
        .expect("imported version id");
    assert_eq!(
        imported["parent_version_id"].as_str(),
        Some(first_version_id)
    );
    assert_eq!(imported["seq"].as_i64(), Some(2));

    let json_export = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{imported_version_id}/export?format=json"
        ))
        .send()
        .await?;
    assert_eq!(json_export.status(), reqwest::StatusCode::OK);
    let json_export: serde_json::Value = json_export.json().await?;
    assert_eq!(json_export["format"].as_str(), Some("json"));
    let json_export_content = json_export["content"]
        .as_str()
        .expect("json export content");
    assert!(json_export_content.contains("proof-primary-role"));

    let round_trip = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions/import"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": json_export_content,
            "expected_parent_version_id": imported_version_id,
            "tool": "ckc-template-json-round-trip-test",
        }))
        .send()
        .await?;
    assert_eq!(
        round_trip.status(),
        reqwest::StatusCode::CREATED,
        "CKC JSON export content must be importable as the next sheet version"
    );
    let round_trip: serde_json::Value = round_trip.json().await?;
    let round_trip_version_id = round_trip["version_id"]
        .as_str()
        .expect("round-trip version id");
    assert_eq!(
        round_trip["parent_version_id"].as_str(),
        Some(imported_version_id)
    );

    let mismatched_raw = first_raw.replace(
        &format!("CHAR-ID-001 — Character_ID: {public_id}"),
        "CHAR-ID-001 — Character_ID: wrong-character-id",
    );
    let mismatched = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions/import"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": mismatched_raw,
            "expected_parent_version_id": round_trip_version_id,
            "tool": "ckc-template-mismatch-test",
        }))
        .send()
        .await?;
    assert_eq!(
        mismatched.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "CKC import must reject a sheet whose CHAR-ID-001 belongs to another character"
    );

    let duplicate_owner_raw =
        format!("{first_raw}\nCHAR-ID-001 — Character_ID: wrong-character-id\n");
    let duplicate_owner = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions/import"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": duplicate_owner_raw,
            "expected_parent_version_id": round_trip_version_id,
            "tool": "ckc-template-duplicate-owner-test",
        }))
        .send()
        .await?;
    assert_eq!(
        duplicate_owner.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "CKC import must reject sheets with duplicate CHAR-ID-001 ownership lines"
    );

    let safe_export = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{round_trip_version_id}/export?format=safe-txt"
        ))
        .send()
        .await?;
    assert_eq!(safe_export.status(), reqwest::StatusCode::OK);
    let safe_export: serde_json::Value = safe_export.json().await?;
    assert_eq!(safe_export["format"].as_str(), Some("safe-txt"));
    let safe_content = safe_export["content"]
        .as_str()
        .expect("safe export content");
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
    let unsafe_variant = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions/import"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": unsafe_variant_raw,
            "expected_parent_version_id": round_trip_version_id,
            "tool": "ckc-template-safe-export-variant-test",
        }))
        .send()
        .await?;
    assert_eq!(unsafe_variant.status(), reqwest::StatusCode::CREATED);
    let unsafe_variant: serde_json::Value = unsafe_variant.json().await?;
    let unsafe_variant_version_id = unsafe_variant["version_id"]
        .as_str()
        .expect("unsafe variant version id");

    let safe_json_export = client
        .get(format!(
            "{base_url}/atelier/sheet-versions/{unsafe_variant_version_id}/export?format=safe-json"
        ))
        .send()
        .await?;
    assert_eq!(safe_json_export.status(), reqwest::StatusCode::OK);
    let safe_json_export: serde_json::Value = safe_json_export.json().await?;
    assert_eq!(safe_json_export["format"].as_str(), Some("safe-json"));
    let safe_json_content = safe_json_export["content"]
        .as_str()
        .expect("safe json export content");
    assert!(
        !safe_json_content.contains("CHAR-SEX-001"),
        "safe-json export must remove unsafe fields even when the separator has no spaces"
    );

    let suggestions = client
        .get(format!(
            "{base_url}/atelier/sheet-field-suggestions?field_id=CHAR-ID-006&limit=5"
        ))
        .send()
        .await?;
    assert_eq!(suggestions.status(), reqwest::StatusCode::OK);
    let suggestions: Vec<serde_json::Value> = suggestions.json().await?;
    assert!(
        suggestions
            .iter()
            .any(|row| row["field_id"].as_str() == Some("CHAR-ID-006")
                && row["value"].as_str() == Some("proof-primary-role")),
        "CKC should remember prior input values per field for future sheet suggestions"
    );

    let normalized_public_id = format!("mt009-normalized-{}", Uuid::new_v4());
    let normalized_created = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": format!("  {normalized_public_id}\n"),
            "display_name": "MT-009 Normalized Public ID",
            "create_default_sheet": true,
        }))
        .send()
        .await?;
    assert_eq!(normalized_created.status(), reqwest::StatusCode::CREATED);
    let normalized_character: serde_json::Value = normalized_created.json().await?;
    assert_eq!(
        normalized_character["public_id"].as_str(),
        Some(normalized_public_id.as_str()),
        "CKC public_id must normalize before storage and default-sheet creation"
    );
    let normalized_character_internal_id = normalized_character["internal_id"]
        .as_str()
        .expect("normalized character internal id");
    let normalized_history = client
        .get(format!(
            "{base_url}/atelier/characters/{normalized_character_internal_id}/sheet-versions"
        ))
        .send()
        .await?;
    assert_eq!(normalized_history.status(), reqwest::StatusCode::OK);
    let normalized_history: Vec<serde_json::Value> = normalized_history.json().await?;
    let normalized_raw = normalized_history[0]["raw_text"]
        .as_str()
        .expect("normalized default sheet raw text");
    assert!(normalized_raw.contains(&format!(
        "CHAR-ID-001 — Character_ID: {normalized_public_id}"
    )));
    assert!(!normalized_raw.contains(&format!("  {normalized_public_id}")));

    server.abort();
    Ok(())
}

#[tokio::test]
async fn atelier_ckc_media_album_api_links_assets_notes_tags_and_refs(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let actor = format!("mt010-media-agent-{}", Uuid::new_v4());
    let public_id = format!("mt010-media-char-{}", Uuid::new_v4());

    let created = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": public_id,
            "display_name": "MT-010 Media Character",
        }))
        .send()
        .await?;
    assert_eq!(created.status(), reqwest::StatusCode::CREATED);
    let character: serde_json::Value = created.json().await?;
    let character_internal_id = character["internal_id"]
        .as_str()
        .expect("character internal_id");
    let expected_character_ref = format!("atelier://character/{character_internal_id}");

    let sheet = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: MT-010\nCHAR-ID-006 — Primary_Role: media album route proof"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(sheet.status(), reqwest::StatusCode::CREATED);
    let sheet: serde_json::Value = sheet.json().await?;
    let sheet_version_id = sheet["version_id"].as_str().expect("sheet version id");
    let expected_sheet_ref = format!("atelier://sheet/{character_internal_id}/{sheet_version_id}");

    let hero_asset = fresh_api_media_asset(&store, "mt010-hero").await;
    let detail_asset = fresh_api_media_asset(&store, "mt010-detail").await;

    let created_album = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/media-albums"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "name": format!("Hero reference album {}", Uuid::new_v4()),
            "notes": "Album notes stay separate from per-image notes.",
            "tags": [" hero ", "portrait", "hero"],
            "sheet_version_id": sheet_version_id,
        }))
        .send()
        .await?;
    assert_eq!(created_album.status(), reqwest::StatusCode::CREATED);
    let created_album: serde_json::Value = created_album.json().await?;
    let album_id = created_album["collection_id"].as_str().expect("album id");
    let expected_collection_ref = format!("atelier://collection/{album_id}");
    assert_eq!(
        created_album["character_ref"].as_str(),
        Some(expected_character_ref.as_str())
    );
    assert_eq!(
        created_album["sheet_version_ref"].as_str(),
        Some(expected_sheet_ref.as_str())
    );
    assert_eq!(
        created_album["collection_ref"].as_str(),
        Some(expected_collection_ref.as_str()),
        "album responses expose a typed collection ref, not a bare UUID"
    );
    assert_eq!(
        created_album["tags"],
        serde_json::json!(["hero", "portrait"]),
        "album tags are de-duplicated independently from media tags"
    );
    assert_eq!(created_album["member_count"].as_i64(), Some(0));
    assert!(
        created_album["members_next_offset"].is_null(),
        "empty albums have no next member page"
    );

    let add_items = client
        .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "asset_ids": [hero_asset, detail_asset, hero_asset],
        }))
        .send()
        .await?;
    assert_eq!(add_items.status(), reqwest::StatusCode::OK);
    let add_items: serde_json::Value = add_items.json().await?;
    assert_eq!(
        add_items["inserted"].as_i64(),
        Some(2),
        "duplicate asset ids must not duplicate album membership"
    );
    assert_eq!(
        add_items["member_count"].as_i64(),
        Some(2),
        "add-items response returns the true collection member count"
    );
    assert!(
        add_items["members_next_offset"].is_null(),
        "small albums are not marked as truncated"
    );

    let note_tags = client
        .post(format!(
            "{base_url}/atelier/media-assets/{hero_asset}/notes-tags"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "notes": "close-up face note for image only",
            "tags": ["face", "lighting", "face"],
            "review_status": "pass",
            "source_path_ref": "atelier://folder/reference-set-a",
        }))
        .send()
        .await?;
    assert_eq!(note_tags.status(), reqwest::StatusCode::OK);
    let note_tags: serde_json::Value = note_tags.json().await?;
    assert_eq!(
        note_tags["notes"].as_str(),
        Some("close-up face note for image only")
    );
    assert_eq!(note_tags["tags"], serde_json::json!(["face", "lighting"]));

    let albums = client
        .get(format!(
            "{base_url}/atelier/characters/{character_internal_id}/media-albums"
        ))
        .send()
        .await?;
    assert_eq!(albums.status(), reqwest::StatusCode::OK);
    let albums: Vec<serde_json::Value> = albums.json().await?;
    let album = albums
        .iter()
        .find(|row| row["collection_id"].as_str() == Some(album_id))
        .expect("created album listed for character");
    assert_eq!(album["member_count"].as_i64(), Some(2));
    assert!(
        album["members_next_offset"].is_null(),
        "listed small albums are not marked as truncated"
    );
    let members = album["members"].as_array().expect("album members");
    assert_eq!(members.len(), 2);
    let expected_hero_asset_id = hero_asset.to_string();
    let expected_hero_media_ref = format!("atelier://media/{hero_asset}");
    assert_eq!(
        members[0]["asset_id"].as_str(),
        Some(expected_hero_asset_id.as_str())
    );
    assert_eq!(
        members[0]["media_ref"].as_str(),
        Some(expected_hero_media_ref.as_str())
    );
    assert_eq!(
        members[0]["content_type"].as_str(),
        Some("image/png"),
        "album members expose media MIME/content type for frontend display"
    );
    assert!(
        !members[0]["file_name"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "album members expose a deterministic display file_name"
    );
    assert_eq!(
        members[0]["source_path_ref"].as_str(),
        Some("atelier://folder/reference-set-a"),
        "folder/source refs are linked through media provenance, not copied file paths"
    );
    assert_eq!(
        members[0]["notes"].as_str(),
        Some("close-up face note for image only"),
        "image notes are listed separately from the album notes"
    );
    assert_eq!(
        members[0]["tags"],
        serde_json::json!(["face", "lighting"]),
        "image tags are listed separately from album tags"
    );
    assert_eq!(
        album["notes"].as_str(),
        Some("Album notes stay separate from per-image notes.")
    );

    server.abort();
    Ok(())
}

#[tokio::test]
async fn atelier_ckc_search_api_returns_fuzzy_vector_combined_refs_and_tag_notes(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let actor = format!("mt011-search-agent-{}", Uuid::new_v4());
    let public_id = format!("mt011-search-char-{}", Uuid::new_v4());

    let created = client
        .post(format!("{base_url}/atelier/characters"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "public_id": public_id,
            "display_name": "Silver Bob Reference",
        }))
        .send()
        .await?;
    assert_eq!(created.status(), reqwest::StatusCode::CREATED);
    let character: serde_json::Value = created.json().await?;
    let character_internal_id = character["internal_id"]
        .as_str()
        .expect("character internal id");
    let character_uuid = Uuid::parse_str(character_internal_id)?;
    let expected_character_ref = format!("atelier://character/{character_internal_id}");

    let sheet = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/sheet-versions"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "raw_text": format!("CHAR-ID-001 — Character_ID: {public_id}\nCHAR-ID-002 — Name: Silver Bob Reference\nCHAR-ID-006 — Primary_Role: facial close-up training avatar\nnotes: silver bob hair, green eyes, soft backlight"),
            "expected_parent_version_id": null,
            "tool": "argus",
        }))
        .send()
        .await?;
    assert_eq!(sheet.status(), reqwest::StatusCode::CREATED);
    let sheet: serde_json::Value = sheet.json().await?;
    let sheet_version_id = sheet["version_id"].as_str().expect("sheet version id");
    let expected_sheet_ref = format!("atelier://sheet/{character_internal_id}/{sheet_version_id}");

    store
        .tag_character(character_uuid, "silver-bob", TagType::Manual)
        .await
        .expect("tag character");

    let hero_asset = fresh_api_media_asset(&store, "mt011-hero").await;
    let decoy_asset = fresh_api_media_asset(&store, "mt011-decoy").await;
    store
        .upsert_similarity_projection(
            hero_asset,
            Some("0000000000000000"),
            serde_json::json!({"dominant":[{"hex":"#c0c0c0"}]}),
        )
        .await
        .expect("hero similarity projection");
    store
        .upsert_similarity_projection(
            decoy_asset,
            Some("ffffffffffffffff"),
            serde_json::json!({"dominant":[{"hex":"#111111"}]}),
        )
        .await
        .expect("decoy similarity projection");

    let created_album = client
        .post(format!(
            "{base_url}/atelier/characters/{character_internal_id}/media-albums"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "name": format!("Silver Bob close-up album {}", Uuid::new_v4()),
            "notes": "album note: approved close-up reference set",
            "tags": ["training", "face"],
            "sheet_version_id": sheet_version_id,
        }))
        .send()
        .await?;
    assert_eq!(created_album.status(), reqwest::StatusCode::CREATED);
    let created_album: serde_json::Value = created_album.json().await?;
    let album_id = created_album["collection_id"].as_str().expect("album id");
    let album_uuid = Uuid::parse_str(album_id)?;
    let expected_collection_ref = collection_ref(album_uuid);

    let add_items = client
        .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "asset_ids": [hero_asset, decoy_asset],
        }))
        .send()
        .await?;
    assert_eq!(add_items.status(), reqwest::StatusCode::OK);

    let media_note = client
        .post(format!(
            "{base_url}/atelier/media-assets/{hero_asset}/notes-tags"
        ))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "notes": "image note: silver bob close-up, soft backlight, CUI-ready face crop",
            "tags": ["training", "face", "approved"],
            "review_status": "pass",
            "source_path_ref": "atelier://folder/mt011-reference-set",
        }))
        .send()
        .await?;
    assert_eq!(media_note.status(), reqwest::StatusCode::OK);

    let tag_note = client
        .post(format!("{base_url}/atelier/ckc/tag-notes"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "tag_text": "training",
            "scope_ref": expected_collection_ref,
            "note": "Use this tag for LoRA-approved CKC image sets only.",
        }))
        .send()
        .await?;
    assert_eq!(
        tag_note.status(),
        reqwest::StatusCode::OK,
        "CKC tag notes must round-trip through a native route"
    );
    let tag_note: serde_json::Value = tag_note.json().await?;
    assert_eq!(tag_note["tag_text"].as_str(), Some("training"));
    assert_eq!(
        tag_note["scope_ref"].as_str(),
        Some(expected_collection_ref.as_str())
    );
    assert_eq!(
        tag_note["note"].as_str(),
        Some("Use this tag for LoRA-approved CKC image sets only.")
    );
    let invalid_tag_text = format!("mt011-invalid-tag-{}", Uuid::new_v4());
    let invalid_scope = client
        .post(format!("{base_url}/atelier/ckc/tag-notes"))
        .header("x-hsk-actor-id", &actor)
        .json(&serde_json::json!({
            "tag_text": invalid_tag_text.clone(),
            "scope_ref": collection_ref(Uuid::new_v4()),
            "note": "This should not attach to a missing album.",
        }))
        .send()
        .await?;
    assert_eq!(
        invalid_scope.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "CKC tag notes must reject syntactically valid refs whose target does not exist"
    );
    assert!(
        !store
            .list_all_tags()
            .await?
            .iter()
            .any(|tag| tag.text == invalid_tag_text),
        "rejected CKC tag-note writes must not leave an orphan tag dictionary row"
    );

    let fuzzy = client
        .post(format!("{base_url}/atelier/ckc/search"))
        .json(&serde_json::json!({
            "query": "silvr bob",
            "modes": ["fuzzy"],
            "limit": 10,
        }))
        .send()
        .await?;
    assert_eq!(fuzzy.status(), reqwest::StatusCode::OK);
    let fuzzy: serde_json::Value = fuzzy.json().await?;
    assert_eq!(fuzzy["query"].as_str(), Some("silvr bob"));
    assert!(fuzzy["search_modes"]
        .as_array()
        .expect("fuzzy modes")
        .iter()
        .any(|mode| mode.as_str() == Some("fuzzy")));
    assert!(fuzzy["results"]
        .as_array()
        .expect("fuzzy results")
        .iter()
        .any(|hit| hit["target_kind"].as_str() == Some("character")
            && hit["target_ref"].as_str() == Some(expected_character_ref.as_str())));

    let vector = client
        .post(format!("{base_url}/atelier/ckc/search"))
        .json(&serde_json::json!({
            "query": "soft backlight CUI-ready face crop",
            "modes": ["vector"],
            "similar_to_asset_id": hero_asset,
            "limit": 10,
        }))
        .send()
        .await?;
    assert_eq!(vector.status(), reqwest::StatusCode::OK);
    let vector: serde_json::Value = vector.json().await?;
    assert_eq!(vector["semantic_available"].as_bool(), Some(true));
    assert_eq!(
        vector["vector_source"].as_str(),
        Some("llm_embedding+pgvector_projection+dhash_similarity")
    );
    assert!(vector["results"]
        .as_array()
        .expect("vector results")
        .iter()
        .any(|hit| hit["target_kind"].as_str() == Some("media")
            && hit["target_ref"].as_str() == Some(media_asset_ref(hero_asset).as_str())
            && hit["match_modes"]
                .as_array()
                .expect("match modes")
                .iter()
                .any(|mode| mode.as_str() == Some("vector"))
            && hit["match_modes"]
                .as_array()
                .expect("match modes")
                .iter()
                .any(|mode| mode.as_str() == Some("image_similarity"))));

    let combined = client
        .post(format!("{base_url}/atelier/ckc/search"))
        .json(&serde_json::json!({
            "query": "backlight face",
            "modes": ["combined"],
            "tags": ["training"],
            "character_internal_id": character_internal_id,
            "similar_to_asset_id": hero_asset,
            "limit": 10,
        }))
        .send()
        .await?;
    assert_eq!(combined.status(), reqwest::StatusCode::OK);
    let combined: serde_json::Value = combined.json().await?;
    let media_hit = combined["results"]
        .as_array()
        .expect("combined results")
        .iter()
        .find(|hit| {
            hit["target_kind"].as_str() == Some("media")
                && hit["target_ref"].as_str() == Some(media_asset_ref(hero_asset).as_str())
        })
        .expect("combined search returns the tagged hero media hit");
    assert!(
        !combined["results"]
            .as_array()
            .expect("combined results")
            .iter()
            .any(|hit| hit["target_kind"].as_str() == Some("media")
                && hit["target_ref"].as_str() == Some(media_asset_ref(decoy_asset).as_str())),
        "combined CKC search must intersect text/tag constraints with the selected image-similarity leg"
    );
    assert_eq!(
        media_hit["character_ref"].as_str(),
        Some(expected_character_ref.as_str()),
        "combined CKC media hits carry the parent character ref"
    );
    assert_eq!(
        media_hit["sheet_version_ref"].as_str(),
        Some(expected_sheet_ref.as_str()),
        "combined CKC media hits carry the sheet version ref when known"
    );
    assert_eq!(
        media_hit["collection_ref"].as_str(),
        Some(expected_collection_ref.as_str()),
        "combined CKC media hits carry the album collection ref"
    );
    assert_eq!(
        media_hit["tag_notes"][0]["note"].as_str(),
        Some("Use this tag for LoRA-approved CKC image sets only."),
        "rich tag notes must be returned with matching CKC search hits"
    );

    server.abort();
    Ok(())
}

#[test]
fn stealth_ref_tauri_commands_are_registered_and_postgres_backed() {
    let repo = repo_root();
    let stealth_ref_rs =
        std::fs::read_to_string(repo.join("app/src-tauri/src/commands/stealth_ref.rs"))
            .expect("read stealth_ref Tauri command source");
    let lib_rs = std::fs::read_to_string(repo.join("app/src-tauri/src/lib.rs"))
        .expect("read Tauri lib source");

    for command in [
        "kernel_stealth_ref_list_windows",
        "kernel_stealth_ref_list_refs",
        "kernel_stealth_ref_resolve_ref",
    ] {
        assert!(
            stealth_ref_rs.contains(&format!("pub async fn {command}")),
            "missing Tauri command function {command}"
        );
        assert!(
            lib_rs.contains(&format!("commands::stealth_ref::{command}")),
            "missing invoke_handler registration for {command}"
        );
    }

    assert!(lib_rs.contains("pub mod stealth_ref"));
    assert!(lib_rs.contains("StealthRefIpcState::from_env_or_unavailable()"));
    assert!(stealth_ref_rs.contains("init_control_plane_storage"));
    assert!(stealth_ref_rs.contains("AtelierStore::with_event_ledger"));
    assert!(stealth_ref_rs.contains("list_stealth_windows"));
    assert!(stealth_ref_rs.contains("list_stealth_refs"));
    assert!(stealth_ref_rs.contains("resolve_stealth_ref"));
    assert!(stealth_ref_rs.contains("stealth_ref_postgres_unavailable"));
    assert!(!stealth_ref_rs.contains("InMemory"));
}

#[tokio::test]
async fn stealth_window_api_list_is_scoped_to_calling_actor(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());

    let caller_input = fresh_window_input();
    let caller_actor = caller_input.owner_actor.clone();
    let foreign_input = fresh_window_input();
    let foreign_actor = foreign_input.owner_actor.clone();

    let caller_window = store
        .create_stealth_window(&caller_input)
        .await
        .expect("create caller window");
    let foreign_window = store
        .create_stealth_window(&foreign_input)
        .await
        .expect("create foreign window");

    let (base_url, server) = start_atelier_api_server(state).await?;
    let response = reqwest::Client::new()
        .get(format!("{base_url}/atelier/stealth/windows"))
        .header("x-hsk-actor-id", &caller_actor)
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let rows: Vec<serde_json::Value> = response.json().await?;
    server.abort();

    let visible_ids: Vec<String> = rows
        .iter()
        .filter_map(|row| row.get("window_ref_id").and_then(serde_json::Value::as_str))
        .map(ToOwned::to_owned)
        .collect();

    assert!(
        visible_ids.contains(&caller_window.window_ref_id.to_string()),
        "caller must see its own stealth window"
    );
    assert!(
        !visible_ids.contains(&foreign_window.window_ref_id.to_string()),
        "stealth list route must not expose windows owned by another actor"
    );
    assert!(
        rows.iter().all(|row| {
            row.get("owner_actor").and_then(serde_json::Value::as_str) == Some(&caller_actor)
        }),
        "every listed stealth window must be scoped to the calling actor {caller_actor}; foreign actor was {foreign_actor}"
    );

    Ok(())
}

#[tokio::test]
async fn atelier_filesystem_health_api_records_read_only_check(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let sidecar_health_lock = acquire_sidecar_visibility_health_lock(&store).await;
    let parent = fresh_api_media_asset(&store, "health-parent").await;
    let sidecar = fresh_api_media_asset(&store, "health-sidecar").await;
    let sidecar_relation = store
        .record_media_sidecar_relation(&NewMediaSidecarRelation {
            parent_asset_id: parent,
            sidecar_asset_id: sidecar,
            relation_kind: MediaSidecarRelationKind::OpenPoseJson,
            created_by: "operator-health".to_string(),
        })
        .await
        .expect("record sidecar relation for API health drift proof");
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();

    let constraints = sidecar_visibility_constraints(&store).await;
    drop_sidecar_visibility_constraints(&store, &constraints).await;
    sqlx::query(
        r#"UPDATE atelier_media_sidecar
           SET hidden_from_gallery = FALSE,
               searchable_by_relation = FALSE,
               updated_at_utc = NOW()
           WHERE sidecar_id = $1"#,
    )
    .bind(sidecar_relation.sidecar_id)
    .execute(store.pool())
    .await
    .expect("simulate sidecar visibility drift before API health check");
    let check_response_result = client
        .post(format!("{base_url}/atelier/filesystem-health/checks"))
        .header("x-hsk-actor-id", "operator-health")
        .json(&serde_json::json!({ "scope_label": "api-health" }))
        .send()
        .await;
    restore_sidecar_visibility_constraints(&store, &constraints, sidecar_relation.sidecar_id).await;
    sidecar_health_lock
        .commit()
        .await
        .expect("release sidecar visibility health lock");
    let check_response = check_response_result?;
    assert_eq!(check_response.status(), reqwest::StatusCode::CREATED);
    let report: serde_json::Value = check_response.json().await?;
    let check_id = report
        .get("check")
        .and_then(|check| check.get("check_id"))
        .and_then(serde_json::Value::as_str)
        .expect("health response includes check_id")
        .to_string();
    assert_eq!(
        report
            .get("check")
            .and_then(|check| check.get("requested_by")),
        Some(&serde_json::json!("operator-health")),
        "health check route must attribute durable diagnostic snapshot from x-hsk-actor-id"
    );
    assert_eq!(
        report
            .get("check")
            .and_then(|check| check.get("summary"))
            .and_then(|summary| summary.get("auto_resync")),
        Some(&serde_json::json!(false)),
        "health route must not auto-resync"
    );
    assert_eq!(
        report
            .get("check")
            .and_then(|check| check.get("summary"))
            .and_then(|summary| summary.get("auto_delete")),
        Some(&serde_json::json!(false)),
        "health route must not auto-delete"
    );
    let anomaly_target_id = sidecar_relation.sidecar_id.to_string();
    let report_findings = report
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .expect("health response includes findings");
    assert!(
        report_findings.iter().any(|finding| {
            finding.get("finding_kind") == Some(&serde_json::json!("sidecar_visibility_anomaly"))
                && finding.get("target_id").and_then(serde_json::Value::as_str)
                    == Some(anomaly_target_id.as_str())
        }),
        "health API must serialize anomaly finding_kind as snake_case"
    );
    assert_eq!(
        report
            .get("check")
            .and_then(|check| check.get("summary"))
            .and_then(|summary| summary.get("sidecar_visibility_anomalies_count")),
        Some(&serde_json::json!(1)),
        "health route summary must count the seeded sidecar anomaly"
    );

    let findings_response = client
        .get(format!(
            "{base_url}/atelier/filesystem-health/checks/{check_id}/findings"
        ))
        .send()
        .await?;
    assert_eq!(findings_response.status(), reqwest::StatusCode::OK);
    let findings: serde_json::Value = findings_response.json().await?;
    assert!(
        findings.is_array(),
        "health findings route must return a list even when no issues are present"
    );
    assert!(
        findings
            .as_array()
            .expect("findings list response is an array")
            .iter()
            .any(|finding| {
                finding.get("finding_kind")
                    == Some(&serde_json::json!("sidecar_visibility_anomaly"))
                    && finding.get("target_id").and_then(serde_json::Value::as_str)
                        == Some(anomaly_target_id.as_str())
            }),
        "health findings list route must preserve snake_case anomaly token"
    );
    server.abort();

    Ok(())
}

#[tokio::test]
async fn atelier_deletion_controls_api_preview_archive_and_restore(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("api-delete-{}", Uuid::new_v4()),
            display_name: "API Delete Subject".to_string(),
        })
        .await
        .expect("create API deletion character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "API delete sheet".to_string(),
            author: "api-delete-test".to_string(),
            tool: Some("api-delete-test".to_string()),
        })
        .await
        .expect("append API deletion sheet");
    let asset_id = fresh_api_media_asset(&store, "deletion-controls").await;
    let targets = serde_json::json!([
        { "target_type": "media_asset", "target_id": asset_id },
        { "target_type": "sheet_version", "target_id": sheet.version_id },
    ]);
    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();

    let preview_response = client
        .post(format!("{base_url}/atelier/deletion/impact-preview"))
        .header("x-hsk-actor-id", "operator-delete")
        .json(&serde_json::json!({
            "targets": targets,
            "reason": "api preview",
        }))
        .send()
        .await?;
    assert_eq!(preview_response.status(), reqwest::StatusCode::OK);
    let preview: serde_json::Value = preview_response.json().await?;
    assert_eq!(preview["requested_by"], "operator-delete");
    assert_eq!(preview["target_count"], 2);
    assert_eq!(preview["would_archive_count"], 2);
    assert_eq!(preview["already_archived_count"], 0);
    assert!(
        !store
            .is_media_asset_trashed(asset_id)
            .await
            .expect("media marker after API preview"),
        "API preview must not archive media"
    );
    assert!(
        !store
            .is_sheet_version_trashed(sheet.version_id)
            .await
            .expect("sheet marker after API preview"),
        "API preview must not archive sheet versions"
    );

    let archive_response = client
        .post(format!("{base_url}/atelier/deletion/archive"))
        .header("x-hsk-actor-id", "operator-delete")
        .json(&serde_json::json!({
            "targets": preview["targets"],
            "reason": "api archive",
        }))
        .send()
        .await?;
    assert_eq!(archive_response.status(), reqwest::StatusCode::CREATED);
    let archive: serde_json::Value = archive_response.json().await?;
    assert_eq!(archive["operation"], "archive_deletion_targets");
    assert_eq!(archive["target_count"], 2);
    assert!(store
        .is_media_asset_trashed(asset_id)
        .await
        .expect("media marker after API archive"));
    assert!(store
        .is_sheet_version_trashed(sheet.version_id)
        .await
        .expect("sheet marker after API archive"));

    let restore_response = client
        .post(format!("{base_url}/atelier/deletion/restore"))
        .header("x-hsk-actor-id", "operator-delete")
        .json(&serde_json::json!({
            "targets": preview["targets"],
            "reason": "api restore",
        }))
        .send()
        .await?;
    assert_eq!(restore_response.status(), reqwest::StatusCode::CREATED);
    let restore: serde_json::Value = restore_response.json().await?;
    assert_eq!(restore["operation"], "restore_deletion_targets");
    assert_eq!(restore["target_count"], 2);
    assert!(!store
        .is_media_asset_trashed(asset_id)
        .await
        .expect("media marker after API restore"));
    assert!(!store
        .is_sheet_version_trashed(sheet.version_id)
        .await
        .expect("sheet marker after API restore"));
    server.abort();

    Ok(())
}

#[tokio::test]
async fn atelier_image_import_api_records_clipboard_and_url_imports(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let artifact = atelier_pg_support::write_native_media_artifact(b"mt-025 api clipboard");
    let url_source = format!(
        "https://example.com/api-import/{}.png?token=api-secret#fragment",
        Uuid::new_v4()
    );
    let before_url_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_image_import_request WHERE source_kind = 'url'",
    )
    .fetch_one(store.pool())
    .await
    .expect("count URL import rows before API negative path");

    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();

    let clipboard_response = client
        .post(format!("{base_url}/atelier/image-import/clipboard"))
        .header("x-hsk-actor-id", "operator-import-api")
        .json(&serde_json::json!({
            "idempotency_key": format!("api-clipboard-import-{}", Uuid::new_v4()),
            "mime": "image/png",
            "content_hash": artifact.content_hash,
            "byte_len": artifact.byte_len,
            "artifact_ref": artifact.artifact_ref,
            "source_application": "system-clipboard",
        }))
        .send()
        .await?;
    assert_eq!(clipboard_response.status(), reqwest::StatusCode::CREATED);
    let clipboard: serde_json::Value = clipboard_response.json().await?;
    assert_eq!(clipboard["source_kind"], "clipboard");
    assert_eq!(clipboard["status"], "materialized");
    assert_eq!(clipboard["requested_by"], "operator-import-api");
    assert!(
        clipboard
            .get("asset_id")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "clipboard API response must expose the materialized media asset id"
    );

    let url_response = client
        .post(format!("{base_url}/atelier/image-import/url"))
        .header("x-hsk-actor-id", "operator-import-api")
        .json(&serde_json::json!({
            "idempotency_key": format!("api-url-import-{}", Uuid::new_v4()),
            "source_url": url_source,
            "expected_mime": "image/png",
            "source_label": "api url import",
            "capability_profile_id": "MediaDownloader",
            "capability_grant_ref": format!(
                "capgrant://media_downloader/MediaDownloader/evidence-{}",
                Uuid::new_v4()
            ),
        }))
        .send()
        .await?;
    assert_eq!(url_response.status(), reqwest::StatusCode::CREATED);
    let url_record: serde_json::Value = url_response.json().await?;
    assert_eq!(url_record["source_kind"], "url");
    assert_eq!(url_record["status"], "queued");
    assert_eq!(url_record["requested_by"], "operator-import-api");
    assert_eq!(url_record["asset_id"], serde_json::Value::Null);
    assert!(
        url_record["source_url_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "URL API response must expose hashed source provenance"
    );
    let url_response_json = url_record.to_string();
    assert!(
        !url_response_json.contains("api-secret")
            && !url_response_json.contains("fragment")
            && !url_response_json.contains(url_source.as_str()),
        "URL API response must not leak query secrets, fragments, or raw source URLs"
    );

    let rejected_response = client
        .post(format!("{base_url}/atelier/image-import/url"))
        .header("x-hsk-actor-id", "operator-import-api")
        .json(&serde_json::json!({
            "idempotency_key": format!("api-url-import-blocked-{}", Uuid::new_v4()),
            "source_url": "http://127.0.0.1/private.png",
            "expected_mime": "image/png",
            "source_label": null,
            "capability_profile_id": "MediaDownloader",
            "capability_grant_ref": format!(
                "capgrant://media_downloader/MediaDownloader/evidence-{}",
                Uuid::new_v4()
            ),
        }))
        .send()
        .await?;
    assert_eq!(rejected_response.status(), reqwest::StatusCode::BAD_REQUEST);
    let after_url_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_image_import_request WHERE source_kind = 'url'",
    )
    .fetch_one(store.pool())
    .await
    .expect("count URL import rows after API negative path");
    assert_eq!(
        after_url_rows,
        before_url_rows + 1,
        "only the accepted URL import should persist; blocked localhost must not create a row"
    );
    server.abort();

    Ok(())
}

#[tokio::test]
async fn atelier_image_import_api_rejects_caller_supplied_artifact_workspace_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let hostile_workspace =
        tempfile::tempdir().expect("create hostile caller-controlled ArtifactStore root");
    let artifact = atelier_pg_support::write_native_media_artifact_in_workspace(
        hostile_workspace.path(),
        b"mt-016 hostile workspace root",
    );
    let assets_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM atelier_media_asset WHERE content_hash = $1")
            .bind(format!("sha256:{}", artifact.content_hash))
            .fetch_one(store.pool())
            .await
            .expect("count media assets before hostile root proof");
    let imports_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_image_import_request WHERE artifact_ref = $1",
    )
    .bind(&artifact.artifact_ref)
    .fetch_one(store.pool())
    .await
    .expect("count image imports before hostile root proof");

    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base_url}/atelier/image-import/clipboard"))
        .header("x-hsk-actor-id", "operator-import-api")
        .json(&serde_json::json!({
            "idempotency_key": format!("api-hostile-root-import-{}", Uuid::new_v4()),
            "mime": "image/png",
            "content_hash": artifact.content_hash,
            "byte_len": artifact.byte_len,
            "artifact_ref": artifact.artifact_ref,
            "artifact_workspace_root": artifact.workspace_root.to_string_lossy(),
            "source_application": "system-clipboard",
        }))
        .send()
        .await?;
    server.abort();

    assert_ne!(
        response.status(),
        reqwest::StatusCode::CREATED,
        "clipboard import must not follow caller-supplied ArtifactStore workspace roots"
    );
    assert_eq!(
        assets_before,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM atelier_media_asset WHERE content_hash = $1",
        )
        .bind(format!("sha256:{}", artifact.content_hash))
        .fetch_one(store.pool())
        .await
        .expect("count media assets after hostile root proof"),
        "rejected hostile ArtifactStore root must not create a media asset"
    );
    assert_eq!(
        imports_before,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM atelier_image_import_request WHERE artifact_ref = $1",
        )
        .bind(&artifact.artifact_ref)
        .fetch_one(store.pool())
        .await
        .expect("count image imports after hostile root proof"),
        "rejected hostile ArtifactStore root must not create an image import row"
    );

    Ok(())
}

#[tokio::test]
async fn atelier_ai_tag_suggestion_api_exposes_review_lifecycle(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("api-ai-suggest-{}", Uuid::new_v4()),
            display_name: "API AI Suggestion Subject".to_string(),
        })
        .await
        .expect("create character for API AI tag suggestion");

    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();
    let record_response = client
        .post(format!("{base_url}/atelier/ai-tag-suggestions"))
        .json(&serde_json::json!({
            "character_internal_id": character.internal_id,
            "asset_id": null,
            "tag_text": "  Cinematic Lighting  ",
            "confidence": 0.91,
            "model_receipt_ref": format!("receipt://atelier/model/{}", Uuid::new_v4()),
            "tool_receipt_ref": format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            "suggested_by": "api-model-worker",
        }))
        .send()
        .await?;

    assert_eq!(record_response.status(), reqwest::StatusCode::CREATED);
    let recorded: serde_json::Value = record_response.json().await?;
    let suggestion_id = recorded
        .get("suggestion_id")
        .and_then(serde_json::Value::as_str)
        .expect("record response includes suggestion_id")
        .to_string();
    assert_eq!(recorded.get("status"), Some(&serde_json::json!("proposed")));
    assert_eq!(
        recorded.get("tag_text"),
        Some(&serde_json::json!("cinematic lighting")),
        "route must expose normalized proposal text without applying it"
    );

    let listed: Vec<serde_json::Value> = client
        .get(format!(
            "{base_url}/atelier/ai-tag-suggestions/characters/{}",
            character.internal_id
        ))
        .send()
        .await?
        .json()
        .await?;
    assert!(
        listed.iter().any(|row| {
            row.get("suggestion_id").and_then(serde_json::Value::as_str)
                == Some(suggestion_id.as_str())
        }),
        "list route must expose recorded proposals by character"
    );

    let accept_response = client
        .post(format!(
            "{base_url}/atelier/ai-tag-suggestions/{suggestion_id}/accept"
        ))
        .header("x-hsk-actor-id", "operator-api-reviewer")
        .json(&serde_json::json!({ "reason": "matches image" }))
        .send()
        .await?;
    assert_eq!(accept_response.status(), reqwest::StatusCode::OK);
    let accepted: serde_json::Value = accept_response.json().await?;
    assert_eq!(accepted.get("status"), Some(&serde_json::json!("accepted")));
    assert_eq!(
        accepted.get("decided_by"),
        Some(&serde_json::json!("operator-api-reviewer")),
        "decision route must attribute reviewer from x-hsk-actor-id"
    );

    let reject_record_response = client
        .post(format!("{base_url}/atelier/ai-tag-suggestions"))
        .json(&serde_json::json!({
            "character_internal_id": character.internal_id,
            "asset_id": null,
            "tag_text": "  Reject Candidate  ",
            "confidence": 0.42,
            "model_receipt_ref": format!("receipt://atelier/model/{}", Uuid::new_v4()),
            "tool_receipt_ref": format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            "suggested_by": "api-model-worker",
        }))
        .send()
        .await?;
    assert_eq!(
        reject_record_response.status(),
        reqwest::StatusCode::CREATED
    );
    let reject_recorded: serde_json::Value = reject_record_response.json().await?;
    let reject_suggestion_id = reject_recorded
        .get("suggestion_id")
        .and_then(serde_json::Value::as_str)
        .expect("reject record response includes suggestion_id")
        .to_string();
    let reject_response = client
        .post(format!(
            "{base_url}/atelier/ai-tag-suggestions/{reject_suggestion_id}/reject"
        ))
        .header("x-hsk-actor-id", "operator-api-rejecter")
        .json(&serde_json::json!({ "reason": "does not match image" }))
        .send()
        .await?;
    assert_eq!(reject_response.status(), reqwest::StatusCode::OK);
    let rejected: serde_json::Value = reject_response.json().await?;
    assert_eq!(rejected.get("status"), Some(&serde_json::json!("rejected")));
    assert_eq!(
        rejected.get("decided_by"),
        Some(&serde_json::json!("operator-api-rejecter")),
        "reject route must attribute reviewer from x-hsk-actor-id"
    );

    let apply_response = client
        .post(format!(
            "{base_url}/atelier/ai-tag-suggestions/{suggestion_id}/apply"
        ))
        .header("x-hsk-actor-id", "operator-api-reviewer")
        .send()
        .await?;
    assert_eq!(apply_response.status(), reqwest::StatusCode::OK);
    let applied: serde_json::Value = apply_response.json().await?;
    server.abort();

    assert_eq!(applied.get("status"), Some(&serde_json::json!("applied")));
    assert!(
        applied
            .get("applied_tag_id")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "apply route must promote accepted proposal into reviewed manual tag surface"
    );

    Ok(())
}

#[tokio::test]
async fn atelier_ai_tag_suggestion_api_rejects_non_receipt_refs(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = test_app_state_from_database_url().await else {
        return Ok(());
    };
    let store = AtelierStore::new(state.postgres_pool.clone());
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("api-ai-receipt-{}", Uuid::new_v4()),
            display_name: "API AI Receipt Subject".to_string(),
        })
        .await
        .expect("create character for API AI receipt validation");
    let before_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_ai_tag_suggestion WHERE character_internal_id = $1",
    )
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI suggestions before invalid API receipt refs");
    let before_events = store
        .count_events(search_event_family::AI_TAG_SUGGESTION_RECORDED)
        .await
        .expect("count AI suggestion events before invalid API receipt refs");

    let (base_url, server) = start_atelier_api_server(state).await?;
    let client = reqwest::Client::new();

    let invalid_model = client
        .post(format!("{base_url}/atelier/ai-tag-suggestions"))
        .json(&serde_json::json!({
            "character_internal_id": character.internal_id,
            "asset_id": null,
            "tag_text": "receipt check",
            "confidence": 0.74,
            "model_receipt_ref": "model-worker-output-1",
            "tool_receipt_ref": format!("receipt://atelier/tool/{}", Uuid::new_v4()),
            "suggested_by": "api-model-worker",
        }))
        .send()
        .await?;
    assert_eq!(invalid_model.status(), reqwest::StatusCode::BAD_REQUEST);

    let invalid_tool = client
        .post(format!("{base_url}/atelier/ai-tag-suggestions"))
        .json(&serde_json::json!({
            "character_internal_id": character.internal_id,
            "asset_id": null,
            "tag_text": "receipt check",
            "confidence": 0.74,
            "model_receipt_ref": format!("receipt://atelier/model/{}", Uuid::new_v4()),
            "tool_receipt_ref": "tool-output-1",
            "suggested_by": "api-model-worker",
        }))
        .send()
        .await?;
    assert_eq!(invalid_tool.status(), reqwest::StatusCode::BAD_REQUEST);
    server.abort();

    let after_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM atelier_ai_tag_suggestion WHERE character_internal_id = $1",
    )
    .bind(character.internal_id)
    .fetch_one(store.pool())
    .await
    .expect("count AI suggestions after invalid API receipt refs");
    assert_eq!(
        after_rows, before_rows,
        "invalid API receipt refs must not persist proposal rows"
    );
    assert_eq!(
        store
            .count_events(search_event_family::AI_TAG_SUGGESTION_RECORDED)
            .await
            .expect("count AI suggestion events after invalid API receipt refs"),
        before_events,
        "invalid API receipt refs must not emit proposal events"
    );

    Ok(())
}

#[tokio::test]
async fn stealth_window_runtime_ids_are_uuid_v7() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_runtime_ids_are_uuid_v7: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");
    assert_uuid_v7(window.window_ref_id, "window_ref_id");
    let next_window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create next stealth window");
    assert_uuid_v7(next_window.window_ref_id, "next window_ref_id");
    assert!(
        window.window_ref_id.as_u128() <= next_window.window_ref_id.as_u128(),
        "sequential stealth window UUID v7 values are nondecreasing"
    );

    let content_ref = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add content ref");
    assert_uuid_v7(content_ref.ref_id, "ref_id");
    let next_content_ref = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add next content ref");
    assert_uuid_v7(next_content_ref.ref_id, "next ref_id");
    assert!(
        content_ref.ref_id.as_u128() <= next_content_ref.ref_id.as_u128(),
        "sequential stealth ref UUID v7 values are nondecreasing"
    );

    let receipt = store
        .record_stealth_capture(
            window.window_ref_id,
            &governed_resolver(),
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await
        .expect("record capture receipt");
    assert_uuid_v7(receipt.capture_id, "capture_id");
    let next_receipt = store
        .record_stealth_capture(
            window.window_ref_id,
            &governed_resolver(),
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await
        .expect("record next capture receipt");
    assert_uuid_v7(next_receipt.capture_id, "next capture_id");
    assert!(
        receipt.capture_id.as_u128() <= next_receipt.capture_id.as_u128(),
        "sequential stealth capture UUID v7 values are nondecreasing"
    );
}

#[tokio::test]
async fn stealth_window_id_columns_have_no_database_defaults() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_id_columns_have_no_database_defaults: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let defaults: Vec<Option<String>> = sqlx::query_scalar(
        r#"SELECT column_default
           FROM information_schema.columns
           WHERE table_schema = ANY(current_schemas(false))
             AND (
               (table_name = 'atelier_stealth_window' AND column_name = 'window_ref_id')
               OR (table_name = 'atelier_stealth_ref' AND column_name = 'ref_id')
               OR (table_name = 'atelier_stealth_capture' AND column_name = 'capture_id')
             )
           ORDER BY table_name, column_name"#,
    )
    .fetch_all(store.pool())
    .await
    .expect("query stealth id column defaults");

    assert_eq!(
        defaults,
        vec![None, None, None],
        "stealth ids must be application-bound UUID v7 values with no database UUID v4 fallback"
    );

    let direct_insert_without_id = sqlx::query(
        "INSERT INTO atelier_stealth_window (owner_actor, title, visibility)
         VALUES ($1, $2, 'off_screen_only')",
    )
    .bind(format!("operator-{}", Uuid::new_v4()))
    .bind(format!("stealth-window-no-db-default-{}", Uuid::new_v4()))
    .execute(store.pool())
    .await;
    assert!(
        direct_insert_without_id.is_err(),
        "direct database insert without window_ref_id must fail instead of minting a fallback UUID"
    );
}

#[tokio::test]
async fn stealth_window_create_idempotent_and_quiet_default() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_create_idempotent_and_quiet_default: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- create a window; round-trips with quiet-default + open status ---
    let input = fresh_window_input();
    let window = store
        .create_stealth_window(&input)
        .await
        .expect("create stealth window");
    assert_eq!(window.owner_actor, input.owner_actor, "owner round-trips");
    assert_eq!(window.title, input.title, "title round-trips");
    assert_eq!(
        window.visibility,
        VisibilityFlag::OffScreenOnly,
        "visibility round-trips as off-screen-only"
    );
    assert!(
        window.quiet.all_quiet(),
        "default quiet flags are all ON (non-intrusive by construction, HBR-QUIET)"
    );
    assert_eq!(window.status, StealthRefStatus::Open, "new window is Open");
    assert_eq!(window.revision, 1, "fresh window starts at revision 1");

    // --- IDEMPOTENCY: re-creating the same (owner, title) wins, no duplicate ---
    let window_again = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: input.owner_actor.clone(),
            title: input.title.clone(),
            // Even with different visibility input, the existing entry wins.
            visibility: VisibilityFlag::DiagnosticEmbed,
            quiet: QuietFlags::default(),
            layout: Some(serde_json::json!({ "panels": ["a", "b"] })),
        })
        .await
        .expect("re-create same (owner, title)");
    assert_eq!(
        window.window_ref_id, window_again.window_ref_id,
        "re-creating the same (owner, title) returns the existing window id"
    );
    assert_eq!(
        window_again.visibility,
        VisibilityFlag::OffScreenOnly,
        "the existing entry wins; the second call does not overwrite visibility"
    );

    // get-by-title resolves to the same single registry row.
    let by_title = store
        .get_stealth_window_by_title(&input.owner_actor, &input.title)
        .await
        .expect("get by title")
        .expect("window present by title");
    assert_eq!(by_title.window_ref_id, window.window_ref_id);

    // Only ONE window for this owner exists (idempotent create did not duplicate).
    let listed = store
        .list_stealth_windows(&input.owner_actor, None, 100)
        .await
        .expect("list windows for owner");
    assert_eq!(
        listed.len(),
        1,
        "idempotent create yields exactly one registry row for the owner"
    );

    // --- INVARIANT: non-quiet off-screen window is rejected (HBR-QUIET) ---
    let mut loud = fresh_window_input();
    loud.quiet = QuietFlags {
        no_focus_steal: true,
        no_foreground: false, // inverted outside a foreground-exception window
        no_taskbar: true,
        no_global_shortcut: true,
        no_synthetic_input: true,
    };
    let loud_err = store.create_stealth_window(&loud).await;
    assert!(
        loud_err.is_err(),
        "a non-quiet off-screen-only window must be rejected (quiet flags must stay ON)"
    );

    // --- EVENT EMISSION: exactly one new window_created (idempotent re-create + ---
    // --- the rejected loud window emit nothing) ---
    let created = store
        .count_events_for_aggregate(
            STEALTH_REF_WINDOW_CREATED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count window_created events for window");
    assert_eq!(
        created, 1,
        "exactly one window_created event for the single materialized window"
    );
}

#[tokio::test]
async fn stealth_window_add_refs_seq_monotonic_and_resolver_law() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP stealth_window_add_refs_seq_monotonic_and_resolver_law: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");

    // --- append two refs; seq is append-only monotonic 0, 1 ---
    let ref0 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add first content ref");
    assert_eq!(ref0.seq, 0, "first appended ref is seq 0");
    assert!(ref0.redaction_state, "redaction assertion round-trips");
    assert_eq!(
        ref0.window_ref_id, window.window_ref_id,
        "ref is bound to its window"
    );

    let ref1 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::SpecAnchor,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add second content ref");
    assert_eq!(
        ref1.seq, 1,
        "second appended ref is seq 1 (monotonic increment)"
    );

    // round-trip via the read-only projection, ascending by seq.
    let refs = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs");
    assert_eq!(refs.len(), 2, "both refs present");
    assert_eq!(refs[0].ref_id, ref0.ref_id, "ordered by seq ascending");
    assert_eq!(refs[1].ref_id, ref1.ref_id);
    assert_eq!(
        refs[0].ref_kind,
        ContentRefKind::Artifact,
        "kind round-trips"
    );
    assert_eq!(refs[1].ref_kind, ContentRefKind::SpecAnchor);
    assert!(
        refs[1].seq > refs[0].seq,
        "append-only sequence is strictly monotonic"
    );

    // --- INVARIANT: a non-governed resolver (machine-local path) is rejected ---
    let path_err = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Screenshot,
                resolver: "C:\\Users\\op\\capture.png".to_string(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await;
    assert!(
        path_err.is_err(),
        "a machine-local filesystem path resolver must be rejected (governed-id LAW)"
    );

    // --- INVARIANT: a non-redacted ref is rejected (secret hygiene) ---
    let redact_err = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: false,
            },
        )
        .await;
    assert!(
        redact_err.is_err(),
        "a ref that does not assert redaction_state must be rejected"
    );

    // The two rejected adds did not append anything.
    let refs_after = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after rejected adds");
    assert_eq!(refs_after.len(), 2, "rejected adds appended no rows");

    // --- EVENT EMISSION: exactly two ref-added events (rejected adds emit none) ---
    let added = store
        .count_events_for_aggregate(
            STEALTH_REF_ADDED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count added events for window");
    assert_eq!(
        added, 2,
        "exactly two ref-added events for the two successful appends"
    );
}

#[tokio::test]
async fn stealth_window_resolve_ref_returns_redacted_governed_single_ref_view() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP stealth_window_resolve_ref_returns_redacted_governed_single_ref_view: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");
    let other_window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create other stealth window");
    let resolver = governed_resolver();
    let content_sha256 = format!("sha256-{}", Uuid::new_v4());
    let content_ref = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: resolver.clone(),
                content_sha256: content_sha256.clone(),
                redaction_state: true,
            },
        )
        .await
        .expect("add content ref");
    let added_events_before = store
        .count_events_for_aggregate(
            STEALTH_REF_ADDED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count added events before resolve");

    let resolved = store
        .resolve_stealth_ref(window.window_ref_id, content_ref.ref_id)
        .await
        .expect("resolve governed content ref");
    assert_eq!(resolved.ref_id, content_ref.ref_id);
    assert_eq!(resolved.window_ref_id, window.window_ref_id);
    assert_eq!(resolved.ref_kind, ContentRefKind::Artifact);
    assert_eq!(resolved.resolver, resolver);
    assert_eq!(resolved.content_sha256, content_sha256);
    assert!(resolved.redaction_state, "resolved view stays redacted");
    assert_eq!(
        resolved.source_authority, "artifact_store",
        "artifact refs resolve through ArtifactStore authority metadata"
    );
    assert!(
        !resolved.payload_included,
        "resolve_ref returns metadata only, never raw payload"
    );
    let resolved_json = serde_json::to_value(&resolved).expect("serialize resolved ref");
    assert!(
        resolved_json.get("payload").is_none() && resolved_json.get("raw_payload").is_none(),
        "resolved view must not expose raw payload fields"
    );
    let added_events_after = store
        .count_events_for_aggregate(
            STEALTH_REF_ADDED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count added events after resolve");
    assert_eq!(
        added_events_after, added_events_before,
        "read-only resolve_ref does not emit mutation events"
    );

    let wrong_window = store
        .resolve_stealth_ref(other_window.window_ref_id, content_ref.ref_id)
        .await;
    assert!(
        wrong_window.is_err(),
        "resolve_ref must not resolve a ref through the wrong window id"
    );

    let poisoned_ref_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO atelier_stealth_ref
             (ref_id, window_ref_id, seq, ref_kind, resolver, content_sha256, redaction_state)
           VALUES ($1, $2, 99, 'artifact', $3, $4, false)"#,
    )
    .bind(poisoned_ref_id)
    .bind(window.window_ref_id)
    .bind(format!("artifact-manifest-poisoned-{}", Uuid::new_v4()))
    .bind(format!("sha256-{}", Uuid::new_v4()))
    .execute(store.pool())
    .await
    .expect("insert intentionally poisoned unredacted ref");
    let poisoned = store
        .resolve_stealth_ref(window.window_ref_id, poisoned_ref_id)
        .await;
    assert!(
        poisoned
            .expect_err("poisoned unredacted ref must fail resolve")
            .to_string()
            .contains("redacted"),
        "resolve_ref refuses to return an unredacted view even if a bad row exists"
    );
}

#[tokio::test]
async fn stealth_window_reorder_permutation_guard_and_remove() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_reorder_permutation_guard_and_remove: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");

    // Append three refs at seq 0, 1, 2.
    let mut ref_ids = Vec::new();
    for _ in 0..3 {
        let r = store
            .add_stealth_ref(
                window.window_ref_id,
                &NewContentRef {
                    ref_kind: ContentRefKind::Artifact,
                    resolver: governed_resolver(),
                    content_sha256: format!("sha256-{}", Uuid::new_v4()),
                    redaction_state: true,
                },
            )
            .await
            .expect("add ref");
        ref_ids.push(r.ref_id);
    }

    // --- INVARIANT: reorder must be an exact permutation (no missing ids) ---
    let partial_err = store
        .reorder_stealth_refs(window.window_ref_id, &ref_ids[..2], None)
        .await;
    assert!(
        partial_err.is_err(),
        "a reorder list missing a current ref must be rejected (no silent drop)"
    );

    // --- INVARIANT: reorder rejects duplicate ids in the supplied order ---
    let dup_err = store
        .reorder_stealth_refs(
            window.window_ref_id,
            &[ref_ids[0], ref_ids[0], ref_ids[1]],
            None,
        )
        .await;
    assert!(
        dup_err.is_err(),
        "a reorder list with a duplicate ref id must be rejected"
    );

    // --- valid reorder (two-phase) reverses the order and repins layout ---
    let new_order = vec![ref_ids[2], ref_ids[1], ref_ids[0]];
    let new_layout = serde_json::json!({ "panels": ["c", "b", "a"] });
    let window_after = store
        .reorder_stealth_refs(window.window_ref_id, &new_order, Some(&new_layout))
        .await
        .expect("reorder refs");
    assert_eq!(
        window_after.layout, new_layout,
        "the optional new layout is repinned in the same mutation"
    );
    assert!(
        window_after.revision > window.revision,
        "reorder bumps the window revision monotonically"
    );

    let refs_reordered = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after reorder");
    assert_eq!(
        refs_reordered.iter().map(|r| r.ref_id).collect::<Vec<_>>(),
        new_order,
        "refs now list in the requested new order"
    );
    // Seqs are re-assigned 0..N (no gaps, no UNIQUE(window, seq) collision).
    assert_eq!(refs_reordered[0].seq, 0);
    assert_eq!(refs_reordered[1].seq, 1);
    assert_eq!(refs_reordered[2].seq, 2);

    // --- remove one ref; remaining rows are preserved (no silent cascade) ---
    let removed = store
        .remove_stealth_ref(window.window_ref_id, new_order[0])
        .await
        .expect("remove ref");
    assert!(removed, "removing an existing ref returns true");
    let removed_again = store
        .remove_stealth_ref(window.window_ref_id, new_order[0])
        .await
        .expect("remove already-removed ref");
    assert!(
        !removed_again,
        "removing an already-removed ref returns false (no spurious event)"
    );
    let refs_post_remove = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after remove");
    assert_eq!(refs_post_remove.len(), 2, "exactly one ref removed");

    // --- EVENT EMISSION: exactly one reorder event (rejected reorders emit none) ---
    let reordered_events = store
        .count_events_for_aggregate(
            STEALTH_REF_REORDERED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count reordered events for window");
    assert_eq!(
        reordered_events, 1,
        "exactly one reorder event for the single valid reorder"
    );
    // And the remove path emitted a removed event for the single real removal.
    let removed_events = store
        .count_events_for_aggregate(
            STEALTH_REF_REMOVED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count removed events for window");
    assert_eq!(removed_events, 1, "one ref-removed event was emitted");
}

#[tokio::test]
async fn stealth_window_capture_idempotent_and_close_audit() {
    let Some(url) = database_url() else {
        eprintln!("SKIP stealth_window_capture_idempotent_and_close_audit: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let window = store
        .create_stealth_window(&fresh_window_input())
        .await
        .expect("create stealth window");
    let pinned_ref = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add ref before close");

    // --- record a capture receipt; round-trips manifest id + hash ---
    let manifest_id = governed_resolver();
    let sha = format!("sha256-{}", Uuid::new_v4());
    let receipt = store
        .record_stealth_capture(window.window_ref_id, &manifest_id, &sha)
        .await
        .expect("record capture receipt");
    assert_eq!(
        receipt.window_ref_id, window.window_ref_id,
        "receipt bound to window"
    );
    assert_eq!(
        receipt.artifact_manifest_id, manifest_id,
        "manifest id round-trips"
    );
    assert_eq!(receipt.content_sha256, sha, "content hash round-trips");

    // --- IDEMPOTENCY: re-recording the same (window, manifest_id) is stable ---
    let receipt_again = store
        .record_stealth_capture(window.window_ref_id, &manifest_id, &sha)
        .await
        .expect("re-record same capture");
    assert_eq!(
        receipt.capture_id, receipt_again.capture_id,
        "re-recording the same (window, manifest_id) returns the same receipt id (ON CONFLICT)"
    );
    let captures = store
        .list_stealth_captures(window.window_ref_id)
        .await
        .expect("list captures");
    assert_eq!(
        captures.len(),
        1,
        "idempotent capture did not duplicate the receipt row"
    );

    // --- INVARIANT: a non-governed manifest id (URL authority) is rejected ---
    let bad_manifest = store
        .record_stealth_capture(
            window.window_ref_id,
            "http://localhost:9222/screenshot",
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await;
    assert!(
        bad_manifest.is_err(),
        "a localhost / network manifest id must be rejected (governed-id LAW)"
    );

    // --- soft-close retains the row for audit; status flips to Closed ---
    let closed = store
        .close_stealth_window(window.window_ref_id)
        .await
        .expect("close window");
    assert_eq!(
        closed.status,
        StealthRefStatus::Closed,
        "status flips to Closed"
    );
    assert!(
        closed.revision > window.revision,
        "close bumps the window revision"
    );
    // Audit retention: the closed row is still fetchable, not deleted.
    let fetched = store
        .get_stealth_window(window.window_ref_id)
        .await
        .expect("get closed window (retained for audit)");
    assert_eq!(
        fetched.status,
        StealthRefStatus::Closed,
        "closed row retained"
    );
    // The capture receipt survives the close (no silent cascade delete).
    let captures_after_close = store
        .list_stealth_captures(window.window_ref_id)
        .await
        .expect("list captures after close");
    assert_eq!(
        captures_after_close.len(),
        1,
        "capture receipt retained after close"
    );
    let closed_again = store
        .close_stealth_window(window.window_ref_id)
        .await
        .expect("closing an already-closed window is idempotent");
    assert_eq!(
        closed_again.status,
        StealthRefStatus::Closed,
        "second close keeps the window closed"
    );
    assert_eq!(
        closed_again.revision, closed.revision,
        "second close returns the existing closed revision without churn"
    );
    let closed_revision = closed.revision;

    // --- INVARIANT: a closed window refuses new refs ---
    let add_on_closed = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: governed_resolver(),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await;
    assert!(
        add_on_closed.is_err(),
        "a closed window must refuse new content refs"
    );
    let remove_on_closed = store
        .remove_stealth_ref(window.window_ref_id, pinned_ref.ref_id)
        .await;
    assert!(
        remove_on_closed.is_err(),
        "a closed window must refuse ref removal so audit-retained refs cannot be deleted"
    );
    let refs_after_rejected_remove = store
        .list_stealth_refs(window.window_ref_id)
        .await
        .expect("list refs after rejected closed-window remove");
    assert_eq!(
        refs_after_rejected_remove
            .iter()
            .map(|r| r.ref_id)
            .collect::<Vec<_>>(),
        vec![pinned_ref.ref_id],
        "rejected closed-window remove preserves the pinned ref"
    );
    let after_rejected_remove = store
        .get_stealth_window(window.window_ref_id)
        .await
        .expect("get window after rejected closed-window remove");
    assert_eq!(
        after_rejected_remove.revision, closed_revision,
        "rejected closed-window remove does not bump revision"
    );

    let blocked_existing_manifest_hash = format!("sha256-{}", Uuid::new_v4());
    let capture_update_on_closed = store
        .record_stealth_capture(
            window.window_ref_id,
            &manifest_id,
            &blocked_existing_manifest_hash,
        )
        .await;
    assert!(
        capture_update_on_closed.is_err(),
        "a closed window must refuse updates to an existing capture receipt"
    );
    let capture_on_closed = store
        .record_stealth_capture(
            window.window_ref_id,
            &governed_resolver(),
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await;
    assert!(
        capture_on_closed.is_err(),
        "a closed window must refuse new capture receipts so closed audit state stays immutable"
    );
    let captures_after_rejected_closed_capture = store
        .list_stealth_captures(window.window_ref_id)
        .await
        .expect("list captures after rejected closed-window capture");
    assert_eq!(
        captures_after_rejected_closed_capture.len(),
        1,
        "rejected closed-window capture does not append another capture receipt"
    );
    assert_eq!(
        captures_after_rejected_closed_capture[0].capture_id, receipt.capture_id,
        "rejected closed-window capture keeps the original receipt id"
    );
    assert_eq!(
        captures_after_rejected_closed_capture[0].content_sha256, sha,
        "rejected closed-window capture cannot update the existing receipt hash"
    );
    let after_rejected_captures = store
        .get_stealth_window(window.window_ref_id)
        .await
        .expect("get window after rejected closed-window captures");
    assert_eq!(
        after_rejected_captures.revision, closed_revision,
        "rejected closed-window capture attempts do not bump revision"
    );

    // --- EVENT EMISSION: capture + close each emitted (idempotent re-record ---
    // --- and rejected capture do not inflate the capture count beyond +2) ---
    let captured_events = store
        .count_events_for_aggregate(
            STEALTH_REF_CAPTURED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count captured events for window");
    // Two successful captures (initial + idempotent re-record both emit), the
    // rejected bad-manifest capture emits nothing.
    assert_eq!(
        captured_events, 2,
        "two capture events (initial + idempotent re-record); rejected capture emits none"
    );
    let closed_events = store
        .count_events_for_aggregate(
            STEALTH_REF_WINDOW_CLOSED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count closed events for window");
    assert_eq!(
        closed_events, 1,
        "exactly one window_closed event for the single close"
    );
    let removed_events = store
        .count_events_for_aggregate(
            STEALTH_REF_REMOVED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
        )
        .await
        .expect("count removed events for window");
    assert_eq!(
        removed_events, 0,
        "rejected closed-window remove emits no ref-removed event"
    );
}
