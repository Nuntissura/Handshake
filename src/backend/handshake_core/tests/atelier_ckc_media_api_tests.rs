//! WP-CKC-posekit-overhaul (SurrealDB port) MT-060: the CKC media lane over HTTP.
//!
//! Port of the reference `atelier_stealth_window_tests.rs` media/search proofs onto the embedded
//! SurrealDB harness. Every test drives the production Axum routers (`api::atelier` merged with
//! `api::atelier_ckc_media`) against a real embedded store and real ArtifactStore payloads; no
//! PostgreSQL, no SKIP path. Characters and sheet versions are created through the store because
//! the character routes belong to the `sheets` lane router.
//!
//! Differences from the reference, deliberate for this base: the model-operation lease guard
//! (`x-hsk-model-lease-id`) is not present on this branch (model leases are the `ops` lane), so
//! attribution is proven from authenticated sessions (spoof actor headers have no authority); the pagination proof crosses `LIST_CAP`
//! (>200 members); and one real concurrent-writer race (two `tokio::join!`ed reorders on one album)
//! is added per MT-056 F5/F9.

mod atelier_surreal_support;
mod account_session_support;
use account_session_support::{AccountFixture, OwnerSession};

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use atelier_surreal_support::{write_native_media_artifact_in_workspace, AtelierSurrealHarness};
use axum::Router;
use handshake_core::api::atelier as atelier_api;
use handshake_core::api::atelier_ckc_media;
use handshake_core::atelier::refs::{collection_ref, media_asset_ref};
use handshake_core::atelier::search::TagType;
use handshake_core::atelier::{AtelierStore, NewCharacter, NewMediaAsset, NewSheetVersion};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse, LlmClient,
    LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use uuid::Uuid;

const EMBEDDING_DIM: usize = 768;

/// One workspace root for the whole test binary (`HANDSHAKE_WORKSPACE_ROOT` is process-global).
fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        assert!(std::env::var("HANDSHAKE_WORKSPACE_ROOT").is_ok_and(|value| !value.trim().is_empty()),
            "CKC media fixtures require an isolated HANDSHAKE_WORKSPACE_ROOT");
        handshake_core::storage::artifacts::resolve_workspace_root()
            .expect("resolve configured CKC media fixture root")
    })
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

/// Deterministic token-hash embedding of the canonical width: the same text always embeds to the
/// same vector and texts sharing tokens have positive cosine, so the vector leg is exercised
/// through the real `LlmClient::embedding` surface without a model runtime.
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

struct TestLlmClient {
    profile: ModelProfile,
    embeddings: bool,
}

#[async_trait]
impl LlmClient for TestLlmClient {
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
        if !self.embeddings {
            return Err(LlmError::EmbeddingUnsupported);
        }
        Ok(EmbeddingResponse {
            vector: deterministic_test_embedding(&req.input, EMBEDDING_DIM),
            model_id: self.profile.model_id.clone(),
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// Route the lane's `tracing::error!(.., "db_error")` / `warn!` lines (which carry the real
/// `AtelierError` text the HTTP body deliberately hides) into the captured test output, once per
/// test binary, so a 500 in a proof names its cause.
fn init_tracing() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("handshake_core::atelier=warn")
            .with_test_writer()
            .try_init();
    });
}

fn app_state(harness: &AtelierSurrealHarness, embeddings: bool) -> AppState {
    init_tracing();
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: harness.database.clone(),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(TestLlmClient {
            profile: ModelProfile::new("mt060-ckc-media-test-embedder".to_string(), 4096),
            embeddings,
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

fn ckc_router(state: AppState) -> Router {
    atelier_api::routes(state.clone()).merge(atelier_ckc_media::routes(state))
}

async fn serve(state: AppState, owner: &OwnerSession) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, ckc_router(state))
            .await
            .expect("Atelier CKC media API server");
    });
    (format!("http://{addr}"), owner.client(), server)
}


/// Distinct authenticated actors share the existing account; no synthetic actor-header authority.
async fn actor_session(harness: &AtelierSurrealHarness, owner: &OwnerSession, actor: &str) -> OwnerSession {
    use sha2::{Digest, Sha256};
    let principal = harness.storage.provision_principal(
        &owner.actor_id, &format!("{}-{actor}", owner.actor_id), "human_account", actor,
        "Operator", &["fs.read".to_owned(), "fs.write".to_owned()], &owner.actor_id,
        Some(&hex::encode(Sha256::digest(owner.channel_binding_token.as_bytes()))),
        std::time::Duration::from_secs(3600),
    ).await.expect("provision same-account authenticated attribution actor");
    assert_eq!(principal.identity.account_id, owner.account_id);
    OwnerSession {
        session_token: principal.session.token,
        channel_binding_token: owner.channel_binding_token.clone(),
        account_id: principal.identity.account_id,
        principal_id: principal.identity.principal_id,
        access_space_id: principal.identity.access_space_id,
        session_id: principal.session.session_id,
        actor_id: actor.to_owned(),
    }
}

/// A real catalog asset backed by a real ArtifactStore payload in the shared workspace root.
async fn fresh_api_media_asset(store: &AtelierStore, label: &str) -> Uuid {
    let payload = format!("{label}-{}", Uuid::now_v7()).into_bytes();
    let artifact = write_native_media_artifact_in_workspace(shared_workspace_root(), &payload);
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

async fn fresh_character(store: &AtelierStore, prefix: &str, display_name: &str) -> Uuid {
    store
        .create_character(&NewCharacter {
            public_id: format!("{prefix}-{}", Uuid::now_v7()),
            display_name: display_name.to_owned(),
        })
        .await
        .expect("create test character")
        .internal_id
}

fn media_album_rows_from_response(value: serde_json::Value) -> Vec<serde_json::Value> {
    value
        .get("albums")
        .and_then(|albums| albums.as_array())
        .cloned()
        .or_else(|| value.as_array().cloned())
        .unwrap_or_default()
}

fn member_asset_ids(value: &serde_json::Value) -> Vec<String> {
    value["members"]
        .as_array()
        .expect("members array")
        .iter()
        .map(|member| member["asset_id"].as_str().unwrap_or_default().to_owned())
        .collect()
}

async fn json_of(response: reqwest::Response) -> (reqwest::StatusCode, serde_json::Value) {
    let status = response.status();
    let body = response.text().await.expect("read body");
    let value = serde_json::from_str(&body).unwrap_or(serde_json::Value::String(body));
    (status, value)
}

#[tokio::test]
async fn atelier_ckc_media_album_api_links_assets_notes_tags_and_refs() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let actor = owner.actor_id.clone();

    let character_internal_id =
        fresh_character(&store, "mt010-media-char", "MT-010 Media Character").await;
    let expected_character_ref = format!("atelier://character/{character_internal_id}");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id,
            raw_text: "CHAR-ID-001 — Character_ID: mt010\nCHAR-ID-002 — Name: MT-010\nCHAR-ID-006 — Primary_Role: media album route proof".to_owned(),
            author: actor.clone(),
            tool: Some("argus".to_owned()),
        })
        .await
        .expect("append sheet version");
    let sheet_version_id = sheet.version_id;
    let expected_sheet_ref = format!("atelier://sheet/{character_internal_id}/{sheet_version_id}");

    let hero_asset = fresh_api_media_asset(&store, "mt010-hero").await;
    let detail_asset = fresh_api_media_asset(&store, "mt010-detail").await;

    let album_name = format!("Hero reference album {}", Uuid::now_v7());
    let (status, created_album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "name": album_name,
                "notes": "Album notes stay separate from per-image notes.",
                "tags": [" hero ", "portrait", "hero"],
                "sheet_version_id": sheet_version_id,
            }))
            .send()
            .await
            .expect("create album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{created_album}");
    let album_id = created_album["collection_id"]
        .as_str()
        .expect("album id")
        .to_owned();
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
    assert!(created_album["members_next_offset"].is_null());
    assert_eq!(created_album["created_by"].as_str(), Some(actor.as_str()));

    let (status, add_items) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({ "asset_ids": [hero_asset, detail_asset, hero_asset] }))
            .send()
            .await
            .expect("add items"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{add_items}");
    assert_eq!(
        add_items["inserted"].as_i64(),
        Some(2),
        "duplicate asset ids must not duplicate album membership"
    );
    assert_eq!(add_items["member_count"].as_i64(), Some(2));
    assert!(add_items["members_next_offset"].is_null());

    let (status, note_tags) = json_of(
        client
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
            .await
            .expect("notes-tags"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{note_tags}");
    assert_eq!(
        note_tags["notes"].as_str(),
        Some("close-up face note for image only")
    );
    assert_eq!(note_tags["tags"], serde_json::json!(["face", "lighting"]));
    assert_eq!(note_tags["review_status"].as_str(), Some("approved"));
    assert_eq!(
        note_tags["source_path_ref"].as_str(),
        Some("atelier://folder/reference-set-a")
    );

    let (status, albums) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .send()
            .await
            .expect("list albums"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{albums}");
    let albums = media_album_rows_from_response(albums);
    let album = albums
        .iter()
        .find(|row| row["collection_id"].as_str() == Some(album_id.as_str()))
        .expect("created album listed for character");
    assert_eq!(album["member_count"].as_i64(), Some(2));
    assert!(album["members_next_offset"].is_null());
    let members = album["members"].as_array().expect("album members");
    assert_eq!(members.len(), 2);
    assert_eq!(
        members[0]["asset_id"].as_str(),
        Some(hero_asset.to_string().as_str())
    );
    assert_eq!(
        members[0]["media_ref"].as_str(),
        Some(format!("atelier://media/{hero_asset}").as_str())
    );
    assert_eq!(members[0]["content_type"].as_str(), Some("image/png"));
    assert!(!members[0]["file_name"]
        .as_str()
        .unwrap_or_default()
        .is_empty());
    assert_eq!(
        members[0]["source_path_ref"].as_str(),
        Some("atelier://folder/reference-set-a"),
        "folder/source refs are linked through media provenance, not copied file paths"
    );
    assert_eq!(
        members[0]["source_path_ref_origin"].as_str(),
        Some("asset_fallback")
    );
    assert_eq!(
        members[0]["notes"].as_str(),
        Some("close-up face note for image only")
    );
    assert_eq!(members[0]["tags"], serde_json::json!(["face", "lighting"]));
    assert_eq!(
        album["notes"].as_str(),
        Some("Album notes stay separate from per-image notes.")
    );

    let (status, page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_id}/items?offset=1&limit=9999"
            ))
            .send()
            .await
            .expect("page items"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    assert_eq!(page["member_count"].as_i64(), Some(2));
    assert!(page["members_next_offset"].is_null());
    assert_eq!(
        page["limit"].as_i64(),
        Some(200),
        "over-large limits are capped at LIST_CAP"
    );
    let page_members = page["members"].as_array().expect("paged album members");
    assert_eq!(page_members.len(), 1);
    assert_eq!(
        page_members[0]["asset_id"].as_str(),
        Some(detail_asset.to_string().as_str())
    );

    for (label, invalid_payload) in [
        (
            "padded folder ref",
            serde_json::json!({
                "notes": "this invalid provenance write must not persist",
                "tags": ["badtag"],
                "review_status": "reject",
                "source_path_ref": " atelier://folder/padded-invalid ",
            }),
        ),
        (
            "machine-local url ref",
            serde_json::json!({
                "notes": "this local provenance write must not persist",
                "tags": ["localbad"],
                "review_status": "reject",
                "source_url_ref": "file://operator/reference-set",
            }),
        ),
    ] {
        let (status, body) = json_of(
            client
                .post(format!(
                    "{base_url}/atelier/media-assets/{hero_asset}/notes-tags"
                ))
                .header("x-hsk-actor-id", &actor)
                .json(&invalid_payload)
                .send()
                .await
                .expect("invalid notes-tags"),
        )
        .await;
        assert_eq!(
            status,
            reqwest::StatusCode::BAD_REQUEST,
            "{label}: invalid provenance refs must be rejected before mutating media metadata: {body}"
        );
        let (_, page) = json_of(
            client
                .get(format!(
                    "{base_url}/atelier/media-albums/{album_id}/items?offset=0&limit=20"
                ))
                .send()
                .await
                .expect("page after invalid"),
        )
        .await;
        assert_eq!(
            page["members"][0]["notes"].as_str(),
            Some("close-up face note for image only"),
            "{label}: rejected writes must not partially replace media notes"
        );
        assert_eq!(
            page["members"][0]["tags"],
            serde_json::json!(["face", "lighting"]),
            "{label}: rejected writes must not partially replace media tags"
        );
        assert_eq!(
            page["members"][0]["review_status"].as_str(),
            Some("approved"),
            "{label}: rejected writes must not change the review status"
        );
    }

    let (status, missing_add) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({ "asset_ids": [Uuid::now_v7()] }))
            .send()
            .await
            .expect("missing add"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "CKC album linking must reject UUIDs that are not Atelier media assets: {missing_add}"
    );
    let (status, invalid_ref_add) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "asset_ids": [hero_asset],
                "source_path_ref": "file://operator/reference-set",
            }))
            .send()
            .await
            .expect("invalid ref add"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "machine-local source refs on album membership writes must be rejected: {invalid_ref_add}"
    );

    // Album names are character-scoped, not globally unique.
    let duplicate_character_internal_id = fresh_character(
        &store,
        "mt010-media-char-dup",
        "MT-010 Duplicate Album Scope Character",
    )
    .await;
    let (status, duplicate_album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{duplicate_character_internal_id}/media-albums"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "name": created_album["name"].as_str().expect("created album name"),
                "notes": "Same album name on another character must be allowed.",
                "tags": ["cross-character"],
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("duplicate album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{duplicate_album}");
    let duplicate_album_id = duplicate_album["collection_id"]
        .as_str()
        .expect("duplicate album id")
        .to_owned();
    // ... but the same name on the SAME character is a typed conflict, not a 500.
    let (status, same_scope_duplicate) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "name": created_album["name"].as_str().expect("created album name"),
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("same-scope duplicate album"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::CONFLICT,
        "duplicate album name within one character scope is a conflict: {same_scope_duplicate}"
    );

    let (status, _) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/media-albums/{duplicate_album_id}/items"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "asset_ids": [hero_asset],
                "source_path_ref": "atelier://folder/reference-set-b",
            }))
            .send()
            .await
            .expect("duplicate add"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let (_, duplicate_page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{duplicate_album_id}/items"
            ))
            .send()
            .await
            .expect("duplicate page"),
    )
    .await;
    assert_eq!(
        duplicate_page["members"][0]["source_path_ref"].as_str(),
        Some("atelier://folder/reference-set-b"),
        "link-scoped provenance must override asset-global provenance for that album membership"
    );
    assert_eq!(
        duplicate_page["members"][0]["source_path_ref_origin"].as_str(),
        Some("link")
    );
    let (_, first_page) = json_of(
        client
            .get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .send()
            .await
            .expect("first page"),
    )
    .await;
    assert_eq!(
        first_page["members"][0]["source_path_ref"].as_str(),
        Some("atelier://folder/reference-set-a"),
        "link-scoped provenance in another album must not overwrite the first album's visible source ref"
    );

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_album_unlink_reorder_and_link_ref_edit() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let operator_headers = actor_session(&harness, &owner, "operator").await.headers();
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .headers(operator_headers.clone()).header("x-hsk-actor-id", "forged-attribution")
            .header("x-hsk-actor-kind", "operator")
    };
    let character_internal_id =
        fresh_character(&store, "mt034-media-char", "MT-034 Media Album Character").await;

    let hero_asset = fresh_api_media_asset(&store, "mt034-hero").await;
    let detail_asset = fresh_api_media_asset(&store, "mt034-detail").await;
    let third_asset = fresh_api_media_asset(&store, "mt034-third").await;
    let hero_asset_id = hero_asset.to_string();
    let detail_asset_id = detail_asset.to_string();
    let third_asset_id = third_asset.to_string();

    let (status, body) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-assets/{hero_asset}/notes-tags"
        )))
        .json(&serde_json::json!({
            "notes": "hero global image note",
            "tags": ["hero", "global"],
            "review_status": "approved",
            "source_path_ref": "atelier://folder/mt034-global",
            "source_url_ref": "https://example.invalid/mt034-global",
        }))
        .send()
        .await
        .expect("global refs"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    let (status, body) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-assets/{detail_asset}/notes-tags"
        )))
        .json(&serde_json::json!({
            "notes": "detail note must survive unlink",
            "tags": ["detail", "unlink-proof"],
            "review_status": "review",
        }))
        .send()
        .await
        .expect("detail notes"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let mut album_ids = Vec::new();
    for (name, notes, tags) in [
        ("MT-034 album one", "primary album", vec!["mt034"]),
        (
            "MT-034 album two",
            "secondary album",
            vec!["mt034", "secondary"],
        ),
    ] {
        let (status, album) = json_of(
            with_operator(client.post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            )))
            .json(&serde_json::json!({
                "name": format!("{name} {}", Uuid::now_v7()),
                "notes": notes,
                "tags": tags,
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("create album"),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
        album_ids.push(
            album["collection_id"]
                .as_str()
                .expect("album id")
                .to_owned(),
        );
    }
    let album_one_id = album_ids[0].clone();
    let album_two_id = album_ids[1].clone();

    let (status, body) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items"
        )))
        .json(&serde_json::json!({
            "asset_ids": [hero_asset, detail_asset, third_asset],
            "source_path_ref": "atelier://folder/mt034-primary-link",
            "source_url_ref": "https://example.invalid/mt034-primary-link",
        }))
        .send()
        .await
        .expect("add primary"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    let (status, body) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_two_id}/items"
        )))
        .json(&serde_json::json!({
            "asset_ids": [hero_asset],
            "source_path_ref": "atelier://folder/mt034-secondary-link",
        }))
        .send()
        .await
        .expect("add secondary"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let (status, edit) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({
            "source_path_ref": "atelier://folder/mt034-primary-edited",
            "source_url_ref": "https://example.invalid/mt034-primary-edited",
        }))
        .send()
        .await
        .expect("edit link"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{edit}");
    assert_eq!(edit["mutation"].as_str(), Some("link_ref_edit"));
    assert_eq!(edit["actor_id"].as_str(), Some("operator"));
    assert_eq!(
        edit["concurrency_policy"].as_str(),
        Some("collection_single_statement_transaction_snapshot")
    );
    let edited_hero = edit["members"]
        .as_array()
        .expect("edited members")
        .iter()
        .find(|member| member["asset_id"].as_str() == Some(hero_asset_id.as_str()))
        .expect("hero remains in edited album");
    assert_eq!(
        edited_hero["link_source_path_ref"].as_str(),
        Some("atelier://folder/mt034-primary-edited")
    );
    assert_eq!(edited_hero["source_path_ref_origin"].as_str(), Some("link"));
    let global_after_edit = store
        .get_media_source_provenance_refs(hero_asset)
        .await
        .expect("read provenance")
        .expect("global media provenance exists");
    assert_eq!(
        global_after_edit.source_path_ref.as_deref(),
        Some("atelier://folder/mt034-global"),
        "link edit must not overwrite global media provenance"
    );

    let (status, clear_link) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({
            "clear_source_path_ref": true,
            "clear_source_url_ref": true,
        }))
        .send()
        .await
        .expect("clear link"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{clear_link}");
    let cleared_hero = clear_link["members"]
        .as_array()
        .expect("cleared members")
        .iter()
        .find(|member| member["asset_id"].as_str() == Some(hero_asset_id.as_str()))
        .expect("hero remains after clear");
    assert!(cleared_hero["link_source_path_ref"].is_null());
    assert_eq!(
        cleared_hero["source_path_ref"].as_str(),
        Some("atelier://folder/mt034-global"),
        "cleared link-local refs should fall back to asset provenance visibly"
    );
    assert_eq!(
        cleared_hero["source_path_ref_origin"].as_str(),
        Some("asset_fallback")
    );
    assert_eq!(
        cleared_hero["link_source_path_ref_status"].as_str(),
        Some("none")
    );
    assert_eq!(
        cleared_hero["asset_source_path_ref_status"].as_str(),
        Some("present")
    );
    let (status, conflicting_clear) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({
            "clear_source_path_ref": true,
            "source_path_ref": "atelier://folder/also-set",
        }))
        .send()
        .await
        .expect("conflicting clear"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "clear + set on the same field is rejected: {conflicting_clear}"
    );
    let (status, empty_edit) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("empty edit"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "an edit that sets nothing is rejected: {empty_edit}"
    );

    let (_, secondary_page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_two_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("secondary page"),
    )
    .await;
    assert_eq!(
        secondary_page["members"][0]["source_path_ref"].as_str(),
        Some("atelier://folder/mt034-secondary-link"),
        "editing album one must not change album two's link-scoped refs"
    );

    let (status, reorder) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/reorder"
        )))
        .json(&serde_json::json!({
            "items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": hero_asset, "sort_order": 1},
                {"asset_id": detail_asset, "sort_order": 2},
            ],
        }))
        .send()
        .await
        .expect("reorder"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{reorder}");
    assert_eq!(reorder["mutation"].as_str(), Some("reorder"));
    assert_eq!(
        reorder["concurrency_policy"].as_str(),
        Some("collection_single_statement_transaction_full_dense_membership_verified")
    );
    assert_eq!(reorder["reordered"].as_i64(), Some(3));
    let reordered_ids = member_asset_ids(&reorder);
    assert_eq!(
        reordered_ids,
        vec![
            third_asset_id.clone(),
            hero_asset_id.clone(),
            detail_asset_id.clone()
        ],
        "reorder response must be dense and visible immediately"
    );
    let reordered_members = reorder["members"].as_array().expect("reordered members");
    assert_eq!(reordered_members[0]["sort_order"].as_i64(), Some(0));
    assert_eq!(reordered_members[1]["sort_order"].as_i64(), Some(1));
    assert_eq!(reordered_members[2]["sort_order"].as_i64(), Some(2));

    let foreign_asset = fresh_api_media_asset(&store, "mt034-foreign").await;
    let rejected_reorders: Vec<(&str, serde_json::Value, reqwest::StatusCode)> = vec![
        (
            "duplicate asset ids",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": third_asset, "sort_order": 1},
                {"asset_id": hero_asset, "sort_order": 2},
            ]}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
        (
            "duplicate positions",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": hero_asset, "sort_order": 0},
                {"asset_id": detail_asset, "sort_order": 2},
            ]}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
        (
            "partial set",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": hero_asset, "sort_order": 1},
            ]}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
        (
            "foreign asset",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": hero_asset, "sort_order": 1},
                {"asset_id": foreign_asset, "sort_order": 2},
            ]}),
            reqwest::StatusCode::NOT_FOUND,
        ),
        (
            "gapped positions",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": 0},
                {"asset_id": hero_asset, "sort_order": 2},
                {"asset_id": detail_asset, "sort_order": 3},
            ]}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
        (
            "negative position",
            serde_json::json!({"items": [
                {"asset_id": third_asset, "sort_order": -1},
                {"asset_id": hero_asset, "sort_order": 0},
                {"asset_id": detail_asset, "sort_order": 1},
            ]}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
        (
            "empty set",
            serde_json::json!({"items": []}),
            reqwest::StatusCode::BAD_REQUEST,
        ),
    ];
    for (label, payload, expected) in rejected_reorders {
        let (status, body) = json_of(
            with_operator(client.patch(format!(
                "{base_url}/atelier/media-albums/{album_one_id}/items/reorder"
            )))
            .json(&payload)
            .send()
            .await
            .expect("rejected reorder"),
        )
        .await;
        assert_eq!(status, expected, "{label}: {body}");
    }
    let (_, after_failed) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_one_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("after failed reorders"),
    )
    .await;
    assert_eq!(
        member_asset_ids(&after_failed),
        reordered_ids,
        "failed reorder requests must leave the last accepted order intact"
    );

    let (status, unlink) = json_of(
        with_operator(client.delete(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{detail_asset}"
        )))
        .send()
        .await
        .expect("unlink"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{unlink}");
    assert_eq!(unlink["mutation"].as_str(), Some("unlink"));
    assert_eq!(
        unlink["concurrency_policy"].as_str(),
        Some("collection_single_statement_transaction_snapshot")
    );
    assert_eq!(unlink["removed"].as_i64(), Some(1));
    assert_eq!(unlink["removed_by"].as_str(), Some("operator"));
    let unlink_receipt_id = Uuid::parse_str(
        unlink["unlink_receipt_id"]
            .as_str()
            .expect("unlink response exposes the persisted receipt id"),
    )
    .expect("receipt id is a uuid");
    assert!(unlink["unlinked_at_utc"].as_str().is_some());
    assert!(
        !member_asset_ids(&unlink).contains(&detail_asset_id),
        "unlink removes only album membership"
    );
    let receipt = store
        .get_collection_item_unlink_receipt(unlink_receipt_id)
        .await
        .expect("read receipt")
        .expect("unlink receipt row persisted");
    assert_eq!(receipt.asset_id, detail_asset);
    assert_eq!(receipt.prior_sort_order, 2);
    assert_eq!(
        receipt.prior_source_path_ref.as_deref(),
        Some("atelier://folder/mt034-primary-link")
    );
    assert_eq!(receipt.unlinked_by, "operator");

    let (status, missing_unlink) = json_of(
        with_operator(client.delete(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{detail_asset}"
        )))
        .send()
        .await
        .expect("missing unlink"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "unlinking a non-member must not report a successful no-op mutation: {missing_unlink}"
    );
    assert!(
        store
            .get_media_asset(detail_asset)
            .await
            .expect("read asset")
            .is_some(),
        "unlinked media asset row is preserved"
    );
    let detail_metadata = store
        .get_media_review_metadata(detail_asset)
        .await
        .expect("read metadata")
        .expect("detail metadata survives unlink");
    assert_eq!(
        detail_metadata.notes.as_deref(),
        Some("detail note must survive unlink")
    );
    let detail_tags = store
        .list_media_asset_tags(detail_asset)
        .await
        .expect("list tags")
        .into_iter()
        .map(|tag| tag.text)
        .collect::<Vec<_>>();
    assert_eq!(
        detail_tags,
        vec!["detail".to_owned(), "unlink-proof".to_owned()],
        "image tags survive album unlink"
    );

    let (status, relink) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items"
        )))
        .json(&serde_json::json!({ "asset_ids": [detail_asset] }))
        .send()
        .await
        .expect("relink"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{relink}");
    assert_eq!(relink["inserted"].as_i64(), Some(1));
    assert_eq!(
        member_asset_ids(&relink),
        vec![third_asset_id, hero_asset_id, detail_asset_id],
        "a re-linked asset is appended after the current members"
    );

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_rows_preserve_actor_attribution() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let actor_a = "album-agent-a";
    let session_a = actor_session(&harness, &owner, actor_a).await;
    let actor_b = "album-agent-b";
    let session_b = actor_session(&harness, &owner, actor_b).await;
    let actor_c = "album-agent-c";
    let session_c = actor_session(&harness, &owner, actor_c).await;
    let actor_d = "album-agent-d";
    let session_d = actor_session(&harness, &owner, actor_d).await;
    let actor_e = "album-agent-e";
    let session_e = actor_session(&harness, &owner, actor_e).await;
    let actor_f = "album-agent-f";
    let session_f = actor_session(&harness, &owner, actor_f).await;
    let character_internal_id = fresh_character(
        &store,
        "mt036-media-char",
        "MT-036 Media Attribution Character",
    )
    .await;
    let first_asset = fresh_api_media_asset(&store, "mt036-first").await;
    let second_asset = fresh_api_media_asset(&store, "mt036-second").await;
    let first_asset_id = first_asset.to_string();
    let second_asset_id = second_asset.to_string();

    let (status, created_album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .headers(session_a.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "name": format!("MT-036 actor album {}", Uuid::now_v7()),
                "notes": "row-level actor proof",
                "tags": ["mt036"],
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("create album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{created_album}");
    let album_id = created_album["collection_id"]
        .as_str()
        .expect("album id")
        .to_owned();
    assert_eq!(created_album["created_by"].as_str(), Some(actor_a));
    assert_eq!(created_album["updated_by"].as_str(), Some(actor_a));

    let (status, linked) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .headers(session_b.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "asset_ids": [first_asset, second_asset],
                "source_path_ref": "atelier://folder/mt036-linked",
                "source_url_ref": "https://example.invalid/mt036-linked",
            }))
            .send()
            .await
            .expect("link"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{linked}");
    let linked_members = linked["members"].as_array().expect("linked members");
    assert_eq!(linked_members.len(), 2);
    for member in linked_members {
        assert_eq!(member["linked_by"].as_str(), Some(actor_b));
        assert_eq!(member["member_updated_by"].as_str(), Some(actor_b));
        assert!(member["member_updated_at_utc"].as_str().is_some());
    }

    let (status, edited) = json_of(
        client
            .patch(format!(
                "{base_url}/atelier/media-albums/{album_id}/items/{first_asset}"
            ))
            .headers(session_c.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "source_path_ref": "atelier://folder/mt036-edited",
                "source_url_ref": "https://example.invalid/mt036-edited",
            }))
            .send()
            .await
            .expect("edit"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{edited}");
    let edited_first = edited["members"]
        .as_array()
        .expect("edited members")
        .iter()
        .find(|member| member["asset_id"].as_str() == Some(first_asset_id.as_str()))
        .expect("first member after edit");
    assert_eq!(edited_first["linked_by"].as_str(), Some(actor_b));
    assert_eq!(edited_first["member_updated_by"].as_str(), Some(actor_c));

    let (status, notes) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/media-assets/{first_asset}/notes-tags"
            ))
            .headers(session_d.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "notes": "first asset note attributed to actor D",
                "tags": ["mt036", "actor-d"],
                "review_status": "approved",
            }))
            .send()
            .await
            .expect("notes"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{notes}");
    assert_eq!(notes["updated_by"].as_str(), Some(actor_d));
    assert!(notes["updated_at_utc"].as_str().is_some());

    let (_, listed) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("list"),
    )
    .await;
    let listed_first = listed["members"]
        .as_array()
        .expect("listed members")
        .iter()
        .find(|member| member["asset_id"].as_str() == Some(first_asset_id.as_str()))
        .expect("first member after notes");
    assert_eq!(listed_first["linked_by"].as_str(), Some(actor_b));
    assert_eq!(listed_first["member_updated_by"].as_str(), Some(actor_c));
    assert_eq!(listed_first["notes_updated_by"].as_str(), Some(actor_d));
    assert!(listed_first["notes_updated_at_utc"].as_str().is_some());

    let (status, reordered) = json_of(
        client
            .patch(format!(
                "{base_url}/atelier/media-albums/{album_id}/items/reorder"
            ))
            .headers(session_e.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "items": [
                    {"asset_id": second_asset, "sort_order": 0},
                    {"asset_id": first_asset, "sort_order": 1},
                ],
            }))
            .send()
            .await
            .expect("reorder"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{reordered}");
    for member in reordered["members"].as_array().expect("reordered members") {
        assert_eq!(member["linked_by"].as_str(), Some(actor_b));
        assert_eq!(member["member_updated_by"].as_str(), Some(actor_e));
    }

    let (status, unlink) = json_of(
        client
            .delete(format!(
                "{base_url}/atelier/media-albums/{album_id}/items/{second_asset}"
            ))
            .headers(session_f.headers()).header("x-hsk-actor-id", "forged-attribution")
            .send()
            .await
            .expect("unlink"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{unlink}");
    assert_eq!(unlink["mutation"].as_str(), Some("unlink"));
    assert_eq!(unlink["actor_id"].as_str(), Some(actor_f));
    assert_eq!(unlink["removed_by"].as_str(), Some(actor_f));
    let unlink_receipt_id = Uuid::parse_str(
        unlink["unlink_receipt_id"]
            .as_str()
            .expect("unlink response must expose persisted receipt id"),
    )
    .expect("receipt id uuid");
    assert!(unlink["unlinked_at_utc"].as_str().is_some());

    let receipt = store
        .get_collection_item_unlink_receipt(unlink_receipt_id)
        .await
        .expect("read receipt")
        .expect("receipt persisted");
    assert_eq!(receipt.collection_id.to_string(), album_id);
    assert_eq!(receipt.asset_id, second_asset);
    assert_eq!(receipt.linked_by, actor_b);
    assert_eq!(receipt.member_updated_by, actor_e);
    assert_eq!(receipt.unlinked_by, actor_f);
    assert_eq!(receipt.prior_sort_order, 0);
    assert_eq!(
        harness
            .row_count_by_field(
                "atelier_collection_item_unlink_receipt",
                "unlinked_by",
                actor_f
            )
            .await,
        1
    );

    let (_, after_unlink) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("after unlink"),
    )
    .await;
    let remaining_ids = member_asset_ids(&after_unlink);
    assert_eq!(remaining_ids, vec![first_asset_id]);
    assert!(!remaining_ids.contains(&second_asset_id));
    let album_after = store
        .get_collection(Uuid::parse_str(&album_id).expect("album uuid"))
        .await
        .expect("album row");
    assert_eq!(album_after.created_by, actor_a);
    assert_eq!(
        album_after.updated_by, actor_f,
        "the last album mutation actor is stored on the collection row"
    );

    let (status, missing_actor) = json_of(
        reqwest::Client::new()
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .json(&serde_json::json!({ "asset_ids": [second_asset] }))
            .send()
            .await
            .expect("no actor"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "mutations without authenticated credentials are rejected: {missing_actor}"
    );

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_source_refs_are_validated_and_link_scoped() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let operator_headers = actor_session(&harness, &owner, "operator").await.headers();
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .headers(operator_headers.clone()).header("x-hsk-actor-id", "forged-attribution")
            .header("x-hsk-actor-kind", "operator")
    };
    let character_internal_id =
        fresh_character(&store, "mt035-media-char", "MT-035 Source Ref Character").await;
    let hero_asset = fresh_api_media_asset(&store, "mt035-hero").await;
    let detail_asset = fresh_api_media_asset(&store, "mt035-detail").await;
    let hero_asset_id = hero_asset.to_string();

    let mut album_ids = Vec::new();
    for (name, notes, tags) in [
        (
            "MT-035 album one",
            "primary source ref proof",
            vec!["mt035"],
        ),
        (
            "MT-035 album two",
            "secondary source ref proof",
            vec!["mt035", "secondary"],
        ),
    ] {
        let (status, album) = json_of(
            with_operator(client.post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            )))
            .json(&serde_json::json!({
                "name": format!("{name} {}", Uuid::now_v7()),
                "notes": notes,
                "tags": tags,
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("create album"),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
        album_ids.push(
            album["collection_id"]
                .as_str()
                .expect("album id")
                .to_owned(),
        );
    }
    let album_one_id = album_ids[0].clone();
    let album_two_id = album_ids[1].clone();

    let (status, add_primary) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items"
        )))
        .json(&serde_json::json!({
            "asset_ids": [hero_asset],
            "source_path_ref": "atelier://folder/mt035-primary-link",
            "source_url_ref": "https://example.invalid/mt035-primary-link",
        }))
        .send()
        .await
        .expect("add primary"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{add_primary}");
    let primary_hero = add_primary["members"]
        .as_array()
        .expect("primary album members")
        .iter()
        .find(|member| member["asset_id"].as_str() == Some(hero_asset_id.as_str()))
        .expect("hero linked in primary album");
    assert_eq!(
        primary_hero["source_path_ref_kind"].as_str(),
        Some("folder")
    );
    assert_eq!(
        primary_hero["source_url_ref_kind"].as_str(),
        Some("source_url")
    );
    assert_eq!(
        primary_hero["source_path_ref_readout"]["value"].as_str(),
        Some("atelier://folder/mt035-primary-link")
    );
    assert_eq!(
        primary_hero["source_url_ref_readout"]["value"].as_str(),
        Some("https://example.invalid/mt035-primary-link")
    );

    let (status, _) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_two_id}/items"
        )))
        .json(&serde_json::json!({
            "asset_ids": [hero_asset],
            "source_path_ref": "atelier://folder/mt035-secondary-link",
            "source_url_ref": "https://example.invalid/mt035-secondary-link",
        }))
        .send()
        .await
        .expect("add secondary"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);

    let (status, body) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({ "source_path_ref": "https://example.invalid/not-a-folder" }))
        .send()
        .await
        .expect("invalid path kind"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "source_path_ref must reject source URL refs before mutation: {body}"
    );
    let (status, body) = json_of(
        with_operator(client.patch(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items/{hero_asset}"
        )))
        .json(&serde_json::json!({ "source_url_ref": "atelier://folder/not-a-source-url" }))
        .send()
        .await
        .expect("invalid url kind"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "source_url_ref must reject folder refs before mutation: {body}"
    );
    let (status, body) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{album_one_id}/items"
        )))
        .json(&serde_json::json!({
            "asset_ids": [detail_asset],
            "source_url_ref": "atelier://folder/not-a-source-url",
        }))
        .send()
        .await
        .expect("invalid add"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "invalid add-item source refs must reject before album membership mutation: {body}"
    );

    let (_, primary_after_invalid) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_one_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("primary after invalid"),
    )
    .await;
    assert_eq!(
        primary_after_invalid["member_count"].as_i64(),
        Some(1),
        "invalid source-ref add must not insert a partial album member"
    );
    let primary_hero_after = &primary_after_invalid["members"][0];
    assert_eq!(
        primary_hero_after["source_path_ref"].as_str(),
        Some("atelier://folder/mt035-primary-link")
    );
    assert_eq!(
        primary_hero_after["source_url_ref"].as_str(),
        Some("https://example.invalid/mt035-primary-link")
    );
    let (_, secondary_after_invalid) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{album_two_id}/items?offset=0&limit=20"
            ))
            .send()
            .await
            .expect("secondary after invalid"),
    )
    .await;
    assert_eq!(
        secondary_after_invalid["members"][0]["source_path_ref"].as_str(),
        Some("atelier://folder/mt035-secondary-link")
    );
    assert_eq!(
        secondary_after_invalid["members"][0]["source_url_ref"].as_str(),
        Some("https://example.invalid/mt035-secondary-link")
    );

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_album_large_library_pagination() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let operator_headers = actor_session(&harness, &owner, "operator").await.headers();
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .headers(operator_headers.clone()).header("x-hsk-actor-id", "forged-attribution")
            .header("x-hsk-actor-kind", "operator")
    };
    let character_internal_id = fresh_character(
        &store,
        "mt033-large-media-char",
        "MT-033 Large Media Character",
    )
    .await;

    let mut album_ids = Vec::new();
    for idx in 0..3 {
        let (status, created_album) = json_of(
            with_operator(client.post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            )))
            .json(&serde_json::json!({
                "name": format!("MT-033 album page {idx}"),
                "notes": format!("large-library album {idx}"),
                "tags": ["mt033", "large-library"],
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("create album"),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{created_album}");
        album_ids.push(
            created_album["collection_id"]
                .as_str()
                .expect("album id")
                .to_owned(),
        );
    }
    let large_album_id = album_ids[0].clone();

    // 205 members: two more than LIST_CAP (200) plus a margin, so the default page truncates.
    const LARGE_MEMBER_COUNT: usize = 205;
    let mut large_assets = Vec::with_capacity(LARGE_MEMBER_COUNT);
    for idx in 0..LARGE_MEMBER_COUNT {
        large_assets.push(fresh_api_media_asset(&store, &format!("mt033-large-{idx}")).await);
    }
    let (status, add_items) = json_of(
        with_operator(client.post(format!(
            "{base_url}/atelier/media-albums/{large_album_id}/items"
        )))
        .json(&serde_json::json!({ "asset_ids": large_assets }))
        .send()
        .await
        .expect("add large set"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{add_items}");
    assert_eq!(
        add_items["inserted"].as_i64(),
        Some(LARGE_MEMBER_COUNT as i64)
    );
    assert_eq!(
        add_items["member_count"].as_i64(),
        Some(LARGE_MEMBER_COUNT as i64),
        "the mutation response reports the canonical count, not the rendered page"
    );
    assert_eq!(
        add_items["members"].as_array().map(Vec::len),
        Some(200),
        "the mutation response page is capped at LIST_CAP"
    );
    assert_eq!(add_items["members_next_offset"].as_i64(), Some(200));

    // Default member page: LIST_CAP rows, a real next offset, then the 5-row tail.
    let (status, default_page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{large_album_id}/items"
            ))
            .send()
            .await
            .expect("default page"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{default_page}");
    assert_eq!(default_page["limit"].as_i64(), Some(200));
    assert_eq!(
        default_page["member_count"].as_i64(),
        Some(LARGE_MEMBER_COUNT as i64)
    );
    assert_eq!(default_page["members_next_offset"].as_i64(), Some(200));
    let first_page_ids = member_asset_ids(&default_page);
    assert_eq!(first_page_ids.len(), 200);
    let (status, tail_page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{large_album_id}/items?offset=200"
            ))
            .send()
            .await
            .expect("tail page"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{tail_page}");
    let tail_ids = member_asset_ids(&tail_page);
    assert_eq!(tail_ids.len(), LARGE_MEMBER_COUNT - 200);
    assert!(tail_page["members_next_offset"].is_null());
    let mut all_ids = first_page_ids.clone();
    all_ids.extend(tail_ids.iter().cloned());
    let expected_ids: Vec<String> = large_assets.iter().map(Uuid::to_string).collect();
    assert_eq!(
        all_ids, expected_ids,
        "pages concatenate to the exact linked order with no duplicates or gaps"
    );
    let sort_orders: Vec<i64> = default_page["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| member["sort_order"].as_i64().expect("sort_order"))
        .collect();
    assert_eq!(
        sort_orders,
        (0..200).collect::<Vec<i64>>(),
        "appended memberships are dense from 0"
    );

    let (status, page_one) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums?offset=0&limit=2&member_limit=1"
            ))
            .send()
            .await
            .expect("album page one"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page_one}");
    assert_eq!(page_one["offset"].as_i64(), Some(0));
    assert_eq!(page_one["limit"].as_i64(), Some(2));
    assert_eq!(page_one["member_limit"].as_i64(), Some(1));
    assert_eq!(page_one["album_count"].as_i64(), Some(3));
    assert_eq!(page_one["albums_next_offset"].as_i64(), Some(2));
    let page_one_albums = page_one["albums"].as_array().expect("album page object");
    assert_eq!(page_one_albums.len(), 2);
    let page_one_album_ids = page_one_albums
        .iter()
        .map(|album| {
            album["collection_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();

    let (_, page_one_repeat) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums?offset=0&limit=2&member_limit=1"
            ))
            .send()
            .await
            .expect("album page one repeat"),
    )
    .await;
    let page_one_repeat_ids = page_one_repeat["albums"]
        .as_array()
        .expect("album repeat page object")
        .iter()
        .map(|album| {
            album["collection_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        page_one_repeat_ids, page_one_album_ids,
        "album-list pagination order must be stable across repeated offset reads"
    );

    let (status, page_two) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums?offset=2&limit=2&member_limit=1"
            ))
            .send()
            .await
            .expect("album page two"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page_two}");
    assert_eq!(page_two["offset"].as_i64(), Some(2));
    assert_eq!(page_two["album_count"].as_i64(), Some(3));
    assert!(page_two["albums_next_offset"].is_null());
    let page_two_albums = page_two["albums"]
        .as_array()
        .expect("album page two object");
    assert_eq!(page_two_albums.len(), 1);

    let paged_album_ids = page_one_albums
        .iter()
        .chain(page_two_albums.iter())
        .map(|album| {
            album["collection_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(paged_album_ids.len(), 3);
    assert_eq!(
        paged_album_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3,
        "album pages must not duplicate albums across offsets: {paged_album_ids:?}"
    );
    let large_album = page_one_albums
        .iter()
        .chain(page_two_albums.iter())
        .find(|album| album["collection_id"].as_str() == Some(large_album_id.as_str()))
        .expect("large album appears in one album-list page");
    assert_eq!(
        large_album["member_count"].as_i64(),
        Some(LARGE_MEMBER_COUNT as i64)
    );
    assert_eq!(
        large_album["members"].as_array().map(Vec::len),
        Some(1),
        "album-list response must use bounded member preview"
    );
    assert_eq!(large_album["members_next_offset"].as_i64(), Some(1));

    let (status, member_page) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{large_album_id}/items?offset=1&limit=1"
            ))
            .send()
            .await
            .expect("member page"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{member_page}");
    assert_eq!(member_page["offset"].as_i64(), Some(1));
    assert_eq!(member_page["limit"].as_i64(), Some(1));
    assert_eq!(
        member_page["member_count"].as_i64(),
        Some(LARGE_MEMBER_COUNT as i64)
    );
    assert_eq!(member_page["members_next_offset"].as_i64(), Some(2));
    assert_eq!(
        member_asset_ids(&member_page),
        vec![large_assets[1].to_string()],
        "offset one returns exactly the second linked asset"
    );
    let (_, member_page_repeat) = json_of(
        client
            .get(format!(
                "{base_url}/atelier/media-albums/{large_album_id}/items?offset=1&limit=1"
            ))
            .send()
            .await
            .expect("member page repeat"),
    )
    .await;
    assert_eq!(
        member_asset_ids(&member_page_repeat),
        member_asset_ids(&member_page),
        "member pagination order must be stable across repeated offset reads"
    );

    for (label, path) in [
        ("negative offset", "items?offset=-1"),
        ("zero limit", "items?limit=0"),
    ] {
        let (status, body) = json_of(
            client
                .get(format!(
                    "{base_url}/atelier/media-albums/{large_album_id}/{path}"
                ))
                .send()
                .await
                .expect("bad page query"),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{label}: {body}");
    }

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

/// MT-056 F5/F9: two writers reorder the same album concurrently. Neither may 500, neither may
/// leave a torn order; the album ends in exactly one of the two requested dense orders.
#[tokio::test]
async fn atelier_ckc_media_album_concurrent_reorders_never_tear_the_order() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let character_internal_id =
        fresh_character(&store, "mt056-race-char", "MT-056 Reorder Race Character").await;
    let (status, album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .headers(actor_session(&harness, &owner, "race-writer-setup").await.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "name": format!("MT-056 race album {}", Uuid::now_v7()),
                "sheet_version_id": null,
            }))
            .send()
            .await
            .expect("create album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
    let album_id = album["collection_id"]
        .as_str()
        .expect("album id")
        .to_owned();

    let mut assets = Vec::new();
    for idx in 0..6 {
        assets.push(fresh_api_media_asset(&store, &format!("mt056-race-{idx}")).await);
    }
    let (status, body) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .headers(actor_session(&harness, &owner, "race-writer-setup").await.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({ "asset_ids": assets }))
            .send()
            .await
            .expect("add"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let order_a: Vec<Uuid> = assets.iter().rev().copied().collect();
    let order_b: Vec<Uuid> = {
        let mut rotated = assets.clone();
        rotated.rotate_left(2);
        rotated
    };
    let payload_for = |order: &[Uuid]| {
        serde_json::json!({
            "items": order
                .iter()
                .enumerate()
                .map(|(sort_order, asset_id)| serde_json::json!({
                    "asset_id": asset_id,
                    "sort_order": sort_order,
                }))
                .collect::<Vec<_>>()
        })
    };
    let request_a = client
        .patch(format!(
            "{base_url}/atelier/media-albums/{album_id}/items/reorder"
        ))
        .headers(actor_session(&harness, &owner, "race-writer-a").await.headers()).header("x-hsk-actor-id", "forged-attribution")
        .json(&payload_for(&order_a))
        .send();
    let request_b = client
        .patch(format!(
            "{base_url}/atelier/media-albums/{album_id}/items/reorder"
        ))
        .headers(actor_session(&harness, &owner, "race-writer-b").await.headers()).header("x-hsk-actor-id", "forged-attribution")
        .json(&payload_for(&order_b))
        .send();
    let (response_a, response_b) = tokio::join!(request_a, request_b);
    let (status_a, body_a) = json_of(response_a.expect("reorder a")).await;
    let (status_b, body_b) = json_of(response_b.expect("reorder b")).await;
    for (label, status, body) in [("a", status_a, &body_a), ("b", status_b, &body_b)] {
        assert!(
            status == reqwest::StatusCode::OK || status == reqwest::StatusCode::CONFLICT,
            "writer {label} must succeed or report a typed conflict, got {status}: {body}"
        );
    }
    assert!(
        status_a == reqwest::StatusCode::OK || status_b == reqwest::StatusCode::OK,
        "at least one concurrent reorder must be applied: a={status_a} b={status_b}"
    );

    let (_, final_page) = json_of(
        client
            .get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .send()
            .await
            .expect("final page"),
    )
    .await;
    let final_ids = member_asset_ids(&final_page);
    let expected_a: Vec<String> = order_a.iter().map(Uuid::to_string).collect();
    let expected_b: Vec<String> = order_b.iter().map(Uuid::to_string).collect();
    assert!(
        final_ids == expected_a || final_ids == expected_b,
        "the album must end in exactly one writer's dense order, got {final_ids:?}"
    );
    let final_sort_orders: Vec<i64> = final_page["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| member["sort_order"].as_i64().expect("sort_order"))
        .collect();
    assert_eq!(
        final_sort_orders,
        (0..6).collect::<Vec<i64>>(),
        "order stays dense"
    );
    let winner = if final_ids == expected_a {
        "race-writer-a"
    } else {
        "race-writer-b"
    };
    for member in final_page["members"].as_array().expect("members") {
        assert_eq!(
            member["member_updated_by"].as_str(),
            Some(winner),
            "every member carries the winning writer's attribution"
        );
    }

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_search_api_returns_fuzzy_vector_combined_refs_and_tag_notes() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, true);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let actor = owner.actor_id.clone();

    let character_uuid = fresh_character(&store, "mt011-search-char", "Silver Bob Reference").await;
    let expected_character_ref = format!("atelier://character/{character_uuid}");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character_uuid,
            raw_text: "CHAR-ID-001 — Character_ID: mt011\nCHAR-ID-002 — Name: Silver Bob Reference\nCHAR-ID-006 — Primary_Role: facial close-up training avatar\nnotes: silver bob hair, green eyes, soft backlight".to_owned(),
            author: actor.clone(),
            tool: Some("argus".to_owned()),
        })
        .await
        .expect("append sheet version");
    let expected_sheet_ref = format!("atelier://sheet/{character_uuid}/{}", sheet.version_id);
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

    let (status, created_album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_uuid}/media-albums"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "name": format!("Silver Bob close-up album {}", Uuid::now_v7()),
                "notes": "album note: approved close-up reference set",
                "tags": ["training", "face"],
                "sheet_version_id": sheet.version_id,
            }))
            .send()
            .await
            .expect("create album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{created_album}");
    let album_uuid = Uuid::parse_str(created_album["collection_id"].as_str().expect("album id"))
        .expect("album uuid");
    let expected_collection_ref = collection_ref(album_uuid);

    let (status, body) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/media-albums/{album_uuid}/items"
            ))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({ "asset_ids": [hero_asset, decoy_asset] }))
            .send()
            .await
            .expect("add items"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    let (status, body) = json_of(
        client
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
            .await
            .expect("media note"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let (status, tag_note) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/tag-notes"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "tag_text": "training",
                "scope_ref": expected_collection_ref,
                "note": "Use this tag for LoRA-approved CKC image sets only.",
            }))
            .send()
            .await
            .expect("tag note"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "CKC tag notes must round-trip through a native route: {tag_note}"
    );
    assert_eq!(tag_note["tag_text"].as_str(), Some("training"));
    assert_eq!(
        tag_note["scope_ref"].as_str(),
        Some(expected_collection_ref.as_str())
    );
    assert_eq!(
        tag_note["note"].as_str(),
        Some("Use this tag for LoRA-approved CKC image sets only.")
    );
    assert_eq!(tag_note["updated_by"].as_str(), Some(actor.as_str()));
    let first_note_id = tag_note["tag_note_id"]
        .as_str()
        .expect("tag note id")
        .to_owned();
    // Same (tag, scope) again is an update of the same note row, not a second row.
    let (status, tag_note_again) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/tag-notes"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "tag_text": "Training",
                "scope_ref": expected_collection_ref,
                "note": "Use this tag for LoRA-approved CKC image sets only (revised).",
            }))
            .send()
            .await
            .expect("tag note again"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{tag_note_again}");
    assert_eq!(
        tag_note_again["tag_note_id"].as_str(),
        Some(first_note_id.as_str()),
        "re-noting the same tag+scope updates in place"
    );
    // A global note (no scope) is a separate row and stays separate from the scoped one.
    let (status, global_note) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/tag-notes"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "tag_text": "training",
                "note": "Global training-tag guidance.",
            }))
            .send()
            .await
            .expect("global tag note"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{global_note}");
    assert!(global_note["scope_ref"].is_null());
    assert_ne!(
        global_note["tag_note_id"].as_str(),
        Some(first_note_id.as_str())
    );

    let invalid_tag_text = format!("mt011-invalid-tag-{}", Uuid::now_v7());
    let (status, invalid_scope) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/tag-notes"))
            .header("x-hsk-actor-id", &actor)
            .json(&serde_json::json!({
                "tag_text": invalid_tag_text.clone(),
                "scope_ref": collection_ref(Uuid::now_v7()),
                "note": "This should not attach to a missing album.",
            }))
            .send()
            .await
            .expect("invalid scope"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "CKC tag notes must reject syntactically valid refs whose target does not exist: {invalid_scope}"
    );
    assert!(
        !store
            .list_all_tags()
            .await
            .expect("list tags")
            .iter()
            .any(|tag| tag.text == invalid_tag_text),
        "rejected CKC tag-note writes must not leave an orphan tag dictionary row"
    );

    let (status, fuzzy) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/search"))
            .json(&serde_json::json!({
                "query": "silvr bob",
                "modes": ["fuzzy"],
                "limit": 10,
            }))
            .send()
            .await
            .expect("fuzzy"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{fuzzy}");
    assert_eq!(fuzzy["query"].as_str(), Some("silvr bob"));
    assert_eq!(fuzzy["vector_source"].as_str(), Some("not_requested"));
    assert!(fuzzy["search_modes"]
        .as_array()
        .expect("fuzzy modes")
        .iter()
        .any(|mode| mode.as_str() == Some("fuzzy")));
    assert!(
        fuzzy["results"]
            .as_array()
            .expect("fuzzy results")
            .iter()
            .any(|hit| hit["target_kind"].as_str() == Some("character")
                && hit["target_ref"].as_str() == Some(expected_character_ref.as_str())),
        "trigram fuzzy search finds the misspelled character: {fuzzy}"
    );

    let (status, vector) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/search"))
            .json(&serde_json::json!({
                "query": "soft backlight CUI-ready face crop",
                "modes": ["vector"],
                "similar_to_asset_id": hero_asset,
                "limit": 10,
            }))
            .send()
            .await
            .expect("vector"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{vector}");
    assert_eq!(vector["semantic_available"].as_bool(), Some(true));
    assert_eq!(
        vector["vector_source"].as_str(),
        Some("llm_embedding+surreal_vector_projection+dhash_similarity")
    );
    assert!(
        vector["results"]
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
                    .any(|mode| mode.as_str() == Some("image_similarity"))),
        "vector search returns the hero media hit with vector + image_similarity legs: {vector}"
    );
    assert_eq!(
        harness
            .row_count_by_field(
                "atelier_ckc_search_projection",
                "embedding_model",
                "mt060-ckc-media-test-embedder",
            )
            .await
            > 0,
        true,
        "vector search persists embeddings on atelier_ckc_search_projection"
    );

    let (status, combined) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/search"))
            .json(&serde_json::json!({
                "query": "backlight face",
                "modes": ["combined"],
                "tags": ["training"],
                "character_internal_id": character_uuid,
                "similar_to_asset_id": hero_asset,
                "limit": 10,
            }))
            .send()
            .await
            .expect("combined"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{combined}");
    let results = combined["results"].as_array().expect("combined results");
    let media_hit = results
        .iter()
        .find(|hit| {
            hit["target_kind"].as_str() == Some("media")
                && hit["target_ref"].as_str() == Some(media_asset_ref(hero_asset).as_str())
        })
        .unwrap_or_else(|| panic!("combined search returns the tagged hero media hit: {combined}"));
    assert!(
        !results.iter().any(|hit| hit["target_kind"].as_str() == Some("media")
            && hit["target_ref"].as_str() == Some(media_asset_ref(decoy_asset).as_str())),
        "combined CKC search must intersect text/tag constraints with the selected image-similarity leg"
    );
    assert_eq!(
        media_hit["character_ref"].as_str(),
        Some(expected_character_ref.as_str())
    );
    assert_eq!(
        media_hit["sheet_version_ref"].as_str(),
        Some(expected_sheet_ref.as_str())
    );
    assert_eq!(
        media_hit["collection_ref"].as_str(),
        Some(expected_collection_ref.as_str())
    );
    let notes = media_hit["tag_notes"].as_array().expect("tag notes");
    assert!(
        notes.iter().any(|note| note["note"].as_str()
            == Some("Use this tag for LoRA-approved CKC image sets only (revised).")
            && note["scope_ref"].as_str() == Some(expected_collection_ref.as_str())),
        "rich scoped tag notes must be returned with matching CKC search hits: {media_hit}"
    );
    assert!(
        notes.iter().any(
            |note| note["note"].as_str() == Some("Global training-tag guidance.")
                && note["scope_ref"].is_null()
        ),
        "global tag notes accompany every hit carrying the tag: {media_hit}"
    );

    let (status, bad_mode) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/search"))
            .json(&serde_json::json!({ "query": "x", "modes": ["telepathic"] }))
            .send()
            .await
            .expect("bad mode"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{bad_mode}");

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

/// Without an embedding endpoint the vector leg degrades honestly: no fabricated vectors,
/// `semantic_available=false`, dHash similarity still works.
#[tokio::test]
async fn atelier_ckc_search_degrades_without_embedding_model() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    let character_uuid = fresh_character(&store, "mt011-degrade-char", "Degrade Proof").await;
    let hero_asset = fresh_api_media_asset(&store, "mt011-degrade-hero").await;
    store
        .upsert_similarity_projection(hero_asset, Some("0f0f0f0f0f0f0f0f"), serde_json::json!({}))
        .await
        .expect("projection");
    let (status, album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_uuid}/media-albums"
            ))
            .headers(actor_session(&harness, &owner, "degrade-proof").await.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({ "name": "degrade album", "sheet_version_id": null }))
            .send()
            .await
            .expect("album"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
    let album_id = album["collection_id"]
        .as_str()
        .expect("album id")
        .to_owned();
    let (status, body) = json_of(
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .headers(actor_session(&harness, &owner, "degrade-proof").await.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({ "asset_ids": [hero_asset] }))
            .send()
            .await
            .expect("add"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");

    let (status, vector) = json_of(
        client
            .post(format!("{base_url}/atelier/ckc/search"))
            .json(&serde_json::json!({
                "query": "degrade",
                "modes": ["vector"],
                "similar_to_asset_id": hero_asset,
            }))
            .send()
            .await
            .expect("vector without embeddings"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{vector}");
    assert_eq!(vector["semantic_available"].as_bool(), Some(false));
    assert_eq!(
        vector["vector_source"].as_str(),
        Some("semantic_unavailable_no_embedding_model+dhash_similarity")
    );
    let hit = vector["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|hit| hit["target_ref"].as_str() == Some(media_asset_ref(hero_asset).as_str()))
        .unwrap_or_else(|| panic!("dhash leg still returns the media hit: {vector}"));
    assert_eq!(hit["similarity_distance"].as_i64(), Some(0));
    assert!(hit["match_modes"]
        .as_array()
        .expect("match modes")
        .iter()
        .all(|mode| mode.as_str() != Some("vector")));
    assert_eq!(
        harness
            .row_count_by_field("atelier_ckc_search_projection", "target_kind", "media",)
            .await,
        0,
        "no embedding rows are fabricated when the runtime has no embedding endpoint"
    );

    server.abort();

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_notes_tags_commit_all_event_families() {
    use handshake_core::atelier::{collections::collections_event_family, event_family};
    use sha2::{Digest, Sha256};

    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
    let asset_id = fresh_api_media_asset(&harness.atelier, "notes-tags-events").await;
    let (base_url, client, server) = serve(state.clone(), &owner).await;
    for (actor, tag, notes) in [("event-seed", "retired-tag", "before"),
                               ("event-writer", "replacement-tag", "after")] {
        let (status, body) = json_of(client
            .post(format!("{base_url}/atelier/media-assets/{asset_id}/notes-tags"))
            .headers(actor_session(&harness, &owner, actor).await.headers()).header("x-hsk-actor-id", "forged-attribution")
            .json(&serde_json::json!({
                "notes": notes, "tags": [tag],
                "source_path_ref": format!("source://mt060/{notes}")
            }))
            .send().await.expect("save notes/tags over HTTP")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{body}");
        assert_eq!(body["updated_by"], actor);
    }
    let hash_ref = |value: &str| format!("sha256:{}", hex::encode(Sha256::digest(value.as_bytes())));
    let asset_key = asset_id.to_string();
    for (aggregate_type, aggregate_id, family, actor_field, expected_actor) in [
        ("atelier_media_review_metadata", asset_key.clone(),
         event_family::MEDIA_REVIEW_METADATA_UPDATED, "requested_by_ref", hash_ref("event-writer")),
        ("atelier_media_asset_tag", hash_ref(&format!("media-asset-untag:{asset_id}:retired-tag")),
         collections_event_family::MEDIA_ASSET_UNTAGGED, "", String::new()),
        ("atelier_media_asset_tag", hash_ref(&format!("media-asset-tag:{asset_id}:replacement-tag")),
         collections_event_family::MEDIA_ASSET_TAGGED, "tag_source_ref", hash_ref("event-writer")),
        ("atelier_media_asset", asset_key,
         event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET, "updated_by", "event-writer".to_owned()),
    ] {
        let events = harness.database.list_kernel_events_for_aggregate(aggregate_type, &aggregate_id)
            .await.expect("re-read canonical committed ledger");
        let matching: Vec<_> = events.iter()
            .filter(|event| event.payload["event_family"] == family)
            .filter(|event| actor_field.is_empty()
                || event.payload["atelier_payload"][actor_field] == expected_actor)
            .collect();
        assert_eq!(matching.len(), 1, "exactly one writer event for {family}");
        assert_eq!(matching[0].payload["atelier_payload"]["asset_id"], asset_id.to_string());
        assert_eq!(harness.row_count_by_field("atelier_event", "kernel_event_id",
            &matching[0].event_id).await, 1, "one projection for the committed event");
    }
    server.abort();
    let _ = server.await;
    drop(client);

    }).await.expect("authenticated account-scoped CKC test setup");
    harness.shutdown().await;
}

fn cursor_member_uuids(page: &serde_json::Value) -> Vec<Uuid> {
    page["members"].as_array().expect("cursor members").iter()
        .map(|row| Uuid::parse_str(row["asset_id"].as_str().expect("asset UUID string"))
            .expect("canonical asset UUID")).collect()
}

async fn create_cursor_test_album(
    client: &reqwest::Client, base_url: &str, character_id: Uuid, label: &str,
) -> serde_json::Value {
    let (status, album) = json_of(client
        .post(format!("{base_url}/atelier/characters/{character_id}/media-albums"))
        .json(&serde_json::json!({"name": format!("{label}-{}", Uuid::now_v7()), "sheet_version_id": null}))
        .send().await.expect("create cursor album")).await;
    assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
    album
}

async fn cursor_member_page(
    client: &reqwest::Client, base_url: &str, album_id: &str, cursor: &str, limit: Option<usize>,
) -> serde_json::Value {
    let request = client.get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
        .query(&[("cursor", cursor)]);
    let request = match limit { Some(limit) => request.query(&[("limit", limit)]), None => request };
    let (status, page) = json_of(request.send().await.expect("member cursor request")).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    assert_eq!(page["traversal_order"].as_str(), Some("asset_id_asc"));
    assert!(cursor_member_uuids(&page).windows(2)
        .all(|pair| pair[0].as_u128() < pair[1].as_u128()), "native UUID order: {page}");
    page
}

async fn collect_cursor_members(
    client: &reqwest::Client, base_url: &str, album_id: &str,
    initial_cursor: &str, initial_limit: Option<usize>, max_pages: usize,
) -> Vec<Uuid> {
    let mut cursor = initial_cursor.to_owned();
    let mut limit = initial_limit;
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for _ in 0..max_pages {
        assert!(seen.insert(cursor.clone()), "continuation must advance");
        let page = cursor_member_page(client, base_url, album_id, &cursor, limit).await;
        let ids = cursor_member_uuids(&page);
        assert!(ids.len() <= 200);
        result.extend(ids);
        if let Some(next) = page["next_cursor"].as_str() {
            assert!(!page["members"].as_array().unwrap().is_empty());
            cursor = next.to_owned();
            limit = None;
        } else {
            assert!(page["next_cursor"].is_null());
            return result;
        }
    }
    panic!("cursor did not exhaust within the known fixture bound");
}
#[tokio::test]
async fn atelier_ckc_member_cursor_is_bounded_scope_checked_and_stable_under_interleaved_mutations() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
        let store = harness.atelier.clone();
        let (base_url, client, server) = serve(state.clone(), &owner).await;
        let character_id = fresh_character(&store, "cursor-member-char", "Cursor member character").await;
        let album = create_cursor_test_album(&client, &base_url, character_id, "cursor-members").await;
        let other_album = create_cursor_test_album(&client, &base_url, character_id, "cursor-empty").await;
        let album_id = album["collection_id"].as_str().expect("album id").to_owned();
        let other_id = other_album["collection_id"].as_str().expect("other album id").to_owned();
        let mut pool = Vec::with_capacity(207);
        for index in 0..207 {
            pool.push(fresh_api_media_asset(&store, &format!("cursor-member-{index}")).await);
        }
        pool.sort_unstable_by_key(Uuid::as_u128);
        assert_eq!(pool.iter().collect::<std::collections::HashSet<_>>().len(), 207);
        let lower = pool[0];
        let upper = pool[206];
        let initial = pool[1..206].to_vec();
        assert_eq!(initial.len(), 205);
        let (status, linked) = json_of(client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .json(&serde_json::json!({"asset_ids": &initial}))
            .send().await.expect("link 205 cursor members")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{linked}");
        assert_eq!(linked["inserted"].as_u64(), Some(205));
        assert_eq!(linked["member_count"].as_u64(), Some(205));
        assert!(linked["next_cursor"].is_null() && linked["traversal_order"].is_null());

        let full = collect_cursor_members(&client, &base_url, &album_id, "start", Some(17), 206).await;
        assert_eq!(full, initial, "all 205 native UUID keys appear once through seek pages");
        assert_eq!(full.iter().collect::<std::collections::HashSet<_>>().len(), 205);
        let cap_page = cursor_member_page(&client, &base_url, &album_id, "start", Some(200)).await;
        assert_eq!(cursor_member_uuids(&cap_page).len(), 200);
        let cap_tail = cursor_member_page(&client, &base_url, &album_id,
            cap_page["next_cursor"].as_str().expect("200-row page has a sentinel"), None).await;
        assert_eq!(cursor_member_uuids(&cap_tail), initial[200..].to_vec());
        assert!(cap_tail["next_cursor"].is_null());
        let empty = cursor_member_page(&client, &base_url, &other_id, "start", Some(17)).await;
        assert!(cursor_member_uuids(&empty).is_empty() && empty["next_cursor"].is_null());

        let first = cursor_member_page(&client, &base_url, &album_id, "start", Some(17)).await;
        let prefix = cursor_member_uuids(&first);
        assert_eq!(prefix, initial[..17].to_vec());
        let continuation = first["next_cursor"].as_str().expect("first cursor").to_owned();
        let page_two = cursor_member_page(&client, &base_url, &album_id, &continuation, None).await;
        let retry = cursor_member_page(&client, &base_url, &album_id, &continuation, None).await;
        assert_eq!(page_two["limit"].as_u64(), Some(17), "omitted limit retains token binding");
        assert_eq!(cursor_member_uuids(&retry), cursor_member_uuids(&page_two));
        assert_eq!(retry["next_cursor"], page_two["next_cursor"], "retry does not advance its input cursor");
        let parts: Vec<_> = continuation.split(':').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0], "v1");
        assert_eq!(parts[1], "m");
        assert_eq!(Uuid::parse_str(parts[2]).unwrap(), Uuid::parse_str(&album_id).unwrap());
        assert_eq!(parts[3], "17");
        assert_eq!(Uuid::parse_str(parts[4]).unwrap(), *prefix.last().unwrap(),
            "cursor anchors the last emitted row, never the sentinel");
        let bad_cursors = vec![
            String::new(), "start ".to_owned(), "v2:m:bad".to_owned(),
            continuation.replacen("v1:", "v2:", 1),
            continuation.replacen("v1:m:", "v1:a:", 1),
            format!("{continuation}:extra"), format!("v1:m:{album_id}:17"),
            format!("v1:m:not-a-uuid:17:{}", prefix.last().unwrap()),
            format!("v1:m:{album_id}:17:not-a-uuid"),
            format!("v1:m:{other_id}:17:{}", prefix.last().unwrap()),
            format!("v1:m:{album_id}:0:{}", prefix.last().unwrap()),
            format!("v1:m:{album_id}:201:{}", prefix.last().unwrap()),
            format!("v1:m:{album_id}:no-limit:{}", prefix.last().unwrap()),
        ];
        for bad in bad_cursors {
            let (status, body) = json_of(client
                .get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
                .query(&[("cursor", bad.as_str())]).send().await.expect("invalid member cursor")).await;
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "cursor={bad:?}: {body}");
        }
        for (cursor, extra) in [
            ("start", vec![("offset", "0")]),
            (continuation.as_str(), vec![("offset", "1")]),
            ("start", vec![("limit", "0")]),
            ("start", vec![("limit", "201")]),
            (continuation.as_str(), vec![("limit", "18")]),
        ] {
            let (status, body) = json_of(client
                .get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
                .query(&[("cursor", cursor)]).query(&extra)
                .send().await.expect("conflicting cursor query")).await;
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{cursor} {extra:?}: {body}");
        }

        // Completed writes interleave pages; this does not establish DB transaction overlap.
        let (status, inserted) = json_of(client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .json(&serde_json::json!({"asset_ids": [lower, upper]}))
            .send().await.expect("insert both sides of cursor")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{inserted}");
        assert_eq!(inserted["inserted"].as_u64(), Some(2));
        let removed_emitted = prefix[0];
        let removed_unseen = initial[initial.len() - 2];
        assert!(!prefix.contains(&removed_unseen));
        for removed in [removed_emitted, removed_unseen] {
            let (status, body) = json_of(client
                .delete(format!("{base_url}/atelier/media-albums/{album_id}/items/{removed}"))
                .send().await.expect("unlink cursor fixture member")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{body}");
            assert_eq!(body["removed"].as_u64(), Some(1));
        }
        let current: Vec<_> = pool.iter().copied()
            .filter(|id| *id != removed_emitted && *id != removed_unseen).collect();
        assert_eq!(current.len(), 205);
        let manual_order: Vec<_> = current.iter().rev().copied().collect();
        let reorder_items: Vec<_> = manual_order.iter().enumerate()
            .map(|(sort_order, asset_id)| serde_json::json!({"asset_id": asset_id, "sort_order": sort_order}))
            .collect();
        let (status, reordered) = json_of(client
            .patch(format!("{base_url}/atelier/media-albums/{album_id}/items/reorder"))
            .json(&serde_json::json!({"items": reorder_items}))
            .send().await.expect("reverse manual order during cursor traversal")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{reordered}");
        assert_eq!(reordered["reordered"].as_u64(), Some(205));
        let rest = collect_cursor_members(&client, &base_url, &album_id, &continuation, None, 208).await;
        let mut continued = prefix.clone();
        continued.extend(rest);
        let mut expected_continued: Vec<_> = initial.iter().copied().filter(|id| *id != removed_unseen).collect();
        expected_continued.push(upper);
        assert_eq!(continued, expected_continued, "insert/unlink/reorder cannot shift the immutable anchor");
        for survivor in initial.iter().filter(|id| **id != removed_emitted && **id != removed_unseen) {
            assert_eq!(continued.iter().filter(|id| *id == survivor).count(), 1, "continuous survivor {survivor}");
        }
        assert_eq!(continued.iter().filter(|id| **id == upper).count(), 1);
        assert!(!continued.contains(&lower));
        assert!(!continued.contains(&removed_unseen));
        assert_eq!(continued.iter().filter(|id| **id == removed_emitted).count(), 1,
            "emitted-unlinked item remains only in the retained prefix");
        let restarted = collect_cursor_members(&client, &base_url, &album_id, "start", Some(17), 208).await;
        assert_eq!(restarted, current, "fresh traversal is the exact current canonical set");
        assert!(restarted.contains(&lower) && restarted.contains(&upper));
        assert!(!restarted.contains(&removed_emitted) && !restarted.contains(&removed_unseen));
        let fresh_page = cursor_member_page(&client, &base_url, &album_id, "start", Some(17)).await;
        assert_eq!(fresh_page["members"][0]["sort_order"].as_u64(), Some(204),
            "manual sort_order remains row data, not traversal order");
        let (status, legacy) = json_of(client
            .get(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .query(&[("offset", 0), ("limit", 17)]).send().await.expect("legacy manual page")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{legacy}");
        assert_eq!(cursor_member_uuids(&legacy), manual_order[..17].to_vec());
        assert!(legacy["next_cursor"].is_null() && legacy["traversal_order"].is_null());
        server.abort();
        let _ = server.await;
        drop(client);
    }).await.expect("authenticated cursor member fixture");
    harness.shutdown().await;
}
fn cursor_album_key(album: &serde_json::Value) -> (chrono::DateTime<chrono::Utc>, Uuid) {
    (chrono::DateTime::parse_from_rfc3339(album["created_at_utc"].as_str().expect("album creation time"))
        .expect("canonical album timestamp").with_timezone(&chrono::Utc),
     Uuid::parse_str(album["collection_id"].as_str().expect("album UUID")).expect("canonical album UUID"))
}

fn expected_cursor_album_ids(albums: &[serde_json::Value]) -> Vec<Uuid> {
    let mut keys: Vec<_> = albums.iter().map(cursor_album_key).collect();
    keys.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.as_u128().cmp(&b.1.as_u128())));
    keys.into_iter().map(|(_, id)| id).collect()
}

fn cursor_album_ids(page: &serde_json::Value) -> Vec<Uuid> {
    page["albums"].as_array().expect("cursor albums").iter()
        .map(|album| cursor_album_key(album).1).collect()
}

async fn cursor_album_page(
    client: &reqwest::Client, base_url: &str, character_id: Uuid, cursor: &str,
    limit: Option<usize>, member_limit: Option<usize>,
) -> serde_json::Value {
    let request = client.get(format!("{base_url}/atelier/characters/{character_id}/media-albums"))
        .query(&[("cursor", cursor)]);
    let request = match limit { Some(limit) => request.query(&[("limit", limit)]), None => request };
    let request = match member_limit {
        Some(limit) => request.query(&[("member_limit", limit)]), None => request,
    };
    let (status, page) = json_of(request.send().await.expect("album cursor request")).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    assert_eq!(page["traversal_order"].as_str(), Some("created_at_desc_collection_id_asc"));
    let albums = page["albums"].as_array().expect("albums");
    assert_eq!(cursor_album_ids(&page), expected_cursor_album_ids(albums));
    for album in albums {
        assert!(album["members"].as_array().expect("preview members").len()
            <= page["member_limit"].as_u64().expect("bound preview size") as usize);
        assert!(album["next_cursor"].is_null() && album["traversal_order"].is_null(),
            "manual member previews are not cursor traversals");
    }
    page
}

async fn collect_cursor_albums(
    client: &reqwest::Client, base_url: &str, character_id: Uuid, cursor: &str,
    limit: Option<usize>, member_limit: Option<usize>, max_pages: usize,
) -> Vec<Uuid> {
    let mut cursor = cursor.to_owned();
    let mut limit = limit;
    let mut member_limit = member_limit;
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for _ in 0..max_pages {
        assert!(seen.insert(cursor.clone()), "album continuation must advance");
        let page = cursor_album_page(client, base_url, character_id, &cursor, limit, member_limit).await;
        result.extend(cursor_album_ids(&page));
        if let Some(next) = page["next_cursor"].as_str() {
            assert!(!page["albums"].as_array().unwrap().is_empty());
            cursor = next.to_owned();
            limit = None;
            member_limit = None;
        } else {
            assert!(page["next_cursor"].is_null());
            return result;
        }
    }
    panic!("album cursor did not exhaust within the known fixture bound");
}

#[tokio::test]
async fn atelier_ckc_album_cursor_orders_timestamps_and_uuid_ties_and_binds_scope_and_limits() {
    let harness = AtelierSurrealHarness::create().await;
    let owner = AccountFixture::install(&harness.storage).await;
    let state = app_state(&harness, false);
    atelier_api::with_account_session(&state, &owner.headers(), async {
        let store = harness.atelier.clone();
        let (base_url, client, server) = serve(state.clone(), &owner).await;
        let character_id = fresh_character(&store, "cursor-album-char", "Cursor album character").await;
        let empty_character = fresh_character(&store, "cursor-empty-char", "Cursor empty character").await;
        let mut albums = Vec::new();
        for index in 0..5 {
            albums.push(create_cursor_test_album(&client, &base_url, character_id,
                &format!("cursor-album-{index}")).await);
        }
        let expected = expected_cursor_album_ids(&albums);
        let populated_album_id = expected[2];
        let populated_album_key = populated_album_id.to_string();
        let preview_assets = [
            fresh_api_media_asset(&store, "cursor-preview-first").await,
            fresh_api_media_asset(&store, "cursor-preview-second").await,
        ];
        let (status, linked) = json_of(client
            .post(format!("{base_url}/atelier/media-albums/{populated_album_id}/items"))
            .json(&serde_json::json!({"asset_ids": &preview_assets}))
            .send().await.expect("populate continuation album preview")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{linked}");
        assert_eq!(linked["inserted"].as_u64(), Some(2));
        assert_eq!(linked["member_count"].as_u64(), Some(2));
        let (status, expanded) = json_of(client
            .get(format!("{base_url}/atelier/media-albums/{populated_album_id}/items"))
            .query(&[("offset", 0), ("limit", 2)])
            .send().await.expect("expand populated album members")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{expanded}");
        assert_eq!(expanded["member_count"].as_u64(), Some(2));
        let expanded_ids = cursor_member_uuids(&expanded);
        assert_eq!(expanded_ids.len(), 2);
        assert_eq!(expanded_ids.into_iter().collect::<std::collections::HashSet<_>>(),
            preview_assets.iter().copied().collect::<std::collections::HashSet<_>>());
        assert!(expanded["next_cursor"].is_null() && expanded["traversal_order"].is_null());
        let traversed = collect_cursor_albums(&client, &base_url, character_id, "start", Some(2), Some(1), 6).await;
        assert_eq!(traversed, expected);
        assert_eq!(traversed.iter().collect::<std::collections::HashSet<_>>().len(), 5);
        let empty = cursor_album_page(&client, &base_url, empty_character, "start", Some(2), Some(1)).await;
        assert!(cursor_album_ids(&empty).is_empty() && empty["next_cursor"].is_null());

        let first = cursor_album_page(&client, &base_url, character_id, "start", Some(2), Some(1)).await;
        let prefix = cursor_album_ids(&first);
        assert_eq!(prefix, expected[..2].to_vec());
        let continuation = first["next_cursor"].as_str().expect("first album cursor").to_owned();
        let second = cursor_album_page(&client, &base_url, character_id, &continuation, None, None).await;
        let retry = cursor_album_page(&client, &base_url, character_id, &continuation, None, None).await;
        assert_eq!(second["limit"].as_u64(), Some(2));
        assert_eq!(second["member_limit"].as_u64(), Some(1));
        let populated_preview = second["albums"].as_array().expect("continuation albums").iter()
            .find(|album| album["collection_id"].as_str() == Some(populated_album_key.as_str()))
            .expect("populated album is on the continuation page");
        assert_eq!(populated_preview["member_count"].as_u64(), Some(2));
        let preview_ids = cursor_member_uuids(populated_preview);
        assert_eq!(preview_ids.len(), 1, "bound member_limit truncates a genuinely populated album");
        assert!(preview_assets.contains(&preview_ids[0]));
        assert!(populated_preview["next_cursor"].is_null() && populated_preview["traversal_order"].is_null(),
            "album previews remain manual previews, not member cursor traversals");
        assert_eq!(cursor_album_ids(&second), expected[2..4].to_vec());
        assert_eq!(cursor_album_ids(&retry), cursor_album_ids(&second));
        assert_eq!(retry["next_cursor"], second["next_cursor"]);
        let parts: Vec<_> = continuation.split(':').collect();
        assert_eq!(parts.len(), 7);
        assert_eq!(&parts[..2], &["v1", "a"]);
        assert_eq!(Uuid::parse_str(parts[2]).unwrap(), character_id);
        assert_eq!(parts[3], "2");
        assert_eq!(parts[4], "1");
        let encoded_time = String::from_utf8(hex::decode(parts[5]).expect("hex time")).expect("UTF-8 time");
        let last_emitted = first["albums"].as_array().unwrap().last().unwrap();
        let (last_time, last_id) = cursor_album_key(last_emitted);
        assert_eq!(encoded_time, last_time.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true));
        assert_eq!(Uuid::parse_str(parts[6]).unwrap(), last_id, "last emitted album, not sentinel");
        let bad_cursors = vec![
            String::new(), "start ".to_owned(), continuation.replacen("v1:", "v2:", 1),
            continuation.replacen("v1:a:", "v1:m:", 1), format!("{continuation}:extra"),
            format!("v1:a:{character_id}:2:1:{}", parts[5]),
            format!("v1:a:not-a-uuid:2:1:{}:{last_id}", parts[5]),
            format!("v1:a:{empty_character}:2:1:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:2:1:{}:not-a-uuid", parts[5]),
            format!("v1:a:{character_id}:0:1:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:201:1:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:two:1:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:2:0:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:2:201:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:2:one:{}:{last_id}", parts[5]),
            format!("v1:a:{character_id}:2:1:gg:{last_id}"),
            format!("v1:a:{character_id}:2:1:0:{last_id}"),
            format!("v1:a:{character_id}:2:1:ff:{last_id}"),
            format!("v1:a:{character_id}:2:1:{}:{last_id}", hex::encode(b"not-a-time")),
            format!("v1:a:{character_id}:2:1:{}:{last_id}", hex::encode(b"2020-01-01T00:00:00+01:00")),
        ];
        for bad in bad_cursors {
            let (status, body) = json_of(client
                .get(format!("{base_url}/atelier/characters/{character_id}/media-albums"))
                .query(&[("cursor", bad.as_str())]).send().await.expect("invalid album cursor")).await;
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "cursor={bad:?}: {body}");
        }
        for (cursor, extra) in [
            ("start", vec![("offset", "0")]),
            (continuation.as_str(), vec![("offset", "1")]),
            ("start", vec![("limit", "0")]),
            ("start", vec![("limit", "201")]),
            ("start", vec![("member_limit", "0")]),
            ("start", vec![("member_limit", "201")]),
            (continuation.as_str(), vec![("limit", "3")]),
            (continuation.as_str(), vec![("member_limit", "2")]),
        ] {
            let (status, body) = json_of(client
                .get(format!("{base_url}/atelier/characters/{character_id}/media-albums"))
                .query(&[("cursor", cursor)]).query(&extra)
                .send().await.expect("conflicting album cursor query")).await;
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{cursor} {extra:?}: {body}");
        }

        let later = create_cursor_test_album(&client, &base_url, character_id, "cursor-album-later").await;
        albums.push(later);
        let current = expected_cursor_album_ids(&albums);
        let anchor = current.iter().position(|id| *id == last_id).expect("original anchor survives");
        let after_insert = collect_cursor_albums(&client, &base_url, character_id,
            &continuation, None, None, 7).await;
        assert_eq!(after_insert, current[anchor + 1..].to_vec(),
            "current-state seek excludes any new key before the original anchor");
        let fresh = collect_cursor_albums(&client, &base_url, character_id, "start", Some(2), Some(1), 7).await;
        assert_eq!(fresh, current, "fresh start includes the exact current six-album set");
        let (status, legacy) = json_of(client
            .get(format!("{base_url}/atelier/characters/{character_id}/media-albums"))
            .query(&[("offset", 0), ("limit", 2), ("member_limit", 1)])
            .send().await.expect("legacy album page")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{legacy}");
        assert_eq!(cursor_album_ids(&legacy), current[..2].to_vec());
        assert!(legacy["next_cursor"].is_null() && legacy["traversal_order"].is_null());
        server.abort();
        let _ = server.await;
        drop(client);
    }).await.expect("authenticated cursor album fixture");
    harness.shutdown().await;
}
type CkcTestPanic = Box<dyn std::any::Any + Send + 'static>;

async fn ckc_capture_test_panic<F: std::future::Future>(
    future: F,
) -> Result<F::Output, CkcTestPanic> {
    let mut future = Some(Box::pin(future));
    std::future::poll_fn(|context| {
        let polled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::future::Future::poll(
                future.as_mut().expect("panic-capture future already finished").as_mut(), context,
            )
        }));
        match polled {
            Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
            Ok(std::task::Poll::Ready(output)) => {
                future.take();
                std::task::Poll::Ready(Ok(output))
            }
            Err(primary) => {
                future.take();
                std::task::Poll::Ready(Err(primary))
            }
        }
    }).await
}

async fn ckc_finish_owned_test(
    harness: AtelierSurrealHarness,
    server: Option<tokio::task::JoinHandle<()>>,
    requests: Vec<tokio::task::JoinHandle<Result<reqwest::Response, reqwest::Error>>>,
    outcome: Result<(), CkcTestPanic>,
) {
    for request in &requests {
        request.abort();
    }
    for request in requests {
        let _ = request.await;
    }
    if let Some(server) = server {
        server.abort();
        let _ = server.await;
    }
    let shutdown = ckc_capture_test_panic(harness.storage.shutdown_with_report()).await;
    let cleanup = match shutdown {
        Ok(Ok(report)) => {
            if !report.drained {
                eprintln!("CKC fixture closed after cooperative cancellation: {report:?}");
            }
            drop(harness);
            Ok(())
        }
        Ok(result) => {
            let failure = format!("CKC fixture drain failed; fixture retained without Drop: {result:?}");
            eprintln!("{failure}");
            std::mem::forget(harness);
            Err(Box::new(failure) as CkcTestPanic)
        }
        Err(failure) => {
            eprintln!("CKC fixture drain panicked; fixture retained without Drop");
            std::mem::forget(harness);
            Err(failure)
        }
    };
    if let Err(primary) = outcome {
        if cleanup.is_err() {
            eprintln!("CKC cleanup also failed; preserving the original test panic");
        }
        std::panic::resume_unwind(primary);
    }
    if let Err(failure) = cleanup {
        std::panic::resume_unwind(failure);
    }
}


#[derive(Default)]
struct AlbumMutationRejectingRecorder {
    reject_family: std::sync::Mutex<Option<&'static str>>,
    rejected: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl FlightRecorder for AlbumMutationRejectingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        let mut reject_family = self.reject_family.lock().expect("recorder control lock");
        if reject_family.is_some_and(|family| event.payload["event_family"] == family) {
            *reject_family = None;
            self.rejected.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return Err(RecorderError::SinkError("MT060 precommit recorder rejection".to_owned()));
        }
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(&self, _filter: EventFilter) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn atelier_ckc_media_membership_recorder_failure_rolls_back_membership_and_ledger() {
    use handshake_core::atelier::collections::collections_event_family;

    let harness = AtelierSurrealHarness::create().await;
    let recorder = Arc::new(AlbumMutationRejectingRecorder::default());
    let mut owned_server = None;
    let outcome = ckc_capture_test_panic(async {
    let owner = AccountFixture::install(&harness.storage).await;
    let mut state = app_state(&harness, false);
    state.flight_recorder = recorder.clone();
    atelier_api::with_account_session(&state, &owner.headers(), async {
        let store = harness.atelier.clone();
        let character = fresh_character(&store, "mt060-fr-rollback", "Recorder rollback").await;
        let assets = [
            fresh_api_media_asset(&store, "mt060-fr-rollback-a").await,
            fresh_api_media_asset(&store, "mt060-fr-rollback-b").await,
        ];
        let (base_url, client, server) = serve(state.clone(), &owner).await;
        owned_server = Some(server);
        let (status, album) = json_of(client
            .post(format!("{base_url}/atelier/characters/{character}/media-albums"))
            .json(&serde_json::json!({"name": format!("rollback-{}", Uuid::now_v7()), "sheet_version_id": null}))
            .send().await.expect("create rollback album")).await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
        let album_id = album["collection_id"].as_str().expect("album id").to_owned();
        let items_url = format!("{base_url}/atelier/media-albums/{album_id}/items");
        let (status, added) = json_of(client.post(&items_url)
            .json(&serde_json::json!({"asset_ids": assets}))
            .send().await.expect("add rollback members")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{added}");

        for (family, method, suffix, payload) in [
            (collections_event_family::MEDIA_ALBUM_ITEMS_REORDERED,
             reqwest::Method::PATCH, "/reorder".to_owned(),
             Some(serde_json::json!({"items": [
                 {"asset_id": assets[1], "sort_order": 0},
                 {"asset_id": assets[0], "sort_order": 1},
             ]}))),
            (collections_event_family::MEDIA_ALBUM_ITEM_LINK_REFS_UPDATED,
             reqwest::Method::PATCH, format!("/{}", assets[0]),
             Some(serde_json::json!({"source_path_ref": "atelier://folder/mt060-fr-rollback"}))),
            (collections_event_family::MEDIA_ALBUM_ITEM_UNLINKED,
             reqwest::Method::DELETE, format!("/{}", assets[0]), None),
        ] {
            let (status, before) = json_of(client.get(&items_url)
                .send().await.expect("read before rejected mutation")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{before}");
            let before_events = harness.database
                .list_kernel_events_for_aggregate("atelier_collection", &album_id)
                .await.expect("read canonical ledger before rejected mutation");
            let before_ids: Vec<_> = before_events.iter().map(|event| event.event_id.clone()).collect();
            let rejected_before = recorder.rejected.load(std::sync::atomic::Ordering::SeqCst);
            *recorder.reject_family.lock().expect("arm recorder rejection") = Some(family);

            let make_request = || {
                let request = client.request(method.clone(), format!("{items_url}{suffix}"));
                match &payload {
                    Some(payload) => request.json(payload),
                    None => request,
                }
            };
            let (status, rejected) = json_of(make_request().send().await
                .expect("send mutation with rejected recorder callback")).await;
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR, "{family}: {rejected}");
            assert_eq!(recorder.rejected.load(std::sync::atomic::Ordering::SeqCst),
                rejected_before + 1, "the real precommit callback must have rejected {family}");
            assert!(recorder.reject_family.lock().expect("read recorder arm").is_none());

            let (status, after) = json_of(client.get(&items_url)
                .send().await.expect("read after rejected mutation")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{after}");
            assert_eq!(after["members"], before["members"], "{family} membership rolled back");
            assert_eq!(after["member_count"], before["member_count"]);
            let after_events = harness.database
                .list_kernel_events_for_aggregate("atelier_collection", &album_id)
                .await.expect("read canonical ledger after rejected mutation");
            let after_ids: Vec<_> = after_events.iter().map(|event| event.event_id.clone()).collect();
            assert_eq!(after_ids, before_ids, "{family} canonical ledger rolled back");

            let (status, committed) = json_of(make_request().send().await
                .expect("send the same valid mutation after rejection")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{family}: {committed}");
            let committed_events = harness.database
                .list_kernel_events_for_aggregate("atelier_collection", &album_id)
                .await.expect("read canonical ledger after successful retry");
            let new_events: Vec<_> = committed_events.iter()
                .filter(|event| !before_ids.contains(&event.event_id))
                .filter(|event| event.payload["event_family"] == family)
                .collect();
            assert_eq!(new_events.len(), 1, "one committed canonical event for {family}");
            let (status, final_page) = json_of(client.get(&items_url)
                .send().await.expect("read committed membership")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{final_page}");
            assert_ne!(final_page["members"], before["members"], "{family} membership committed");
        }

        drop(client);
    }).await.expect("authenticated account-scoped rollback test");
    }).await;
    if let Ok(mut family) = recorder.reject_family.lock() {
        *family = None;
    }
    ckc_finish_owned_test(harness, owned_server, Vec::new(), outcome).await;
}


struct AlbumMutationOverlapRecorder {
    album_id: std::sync::Mutex<Option<String>>,
    entered: std::sync::atomic::AtomicUsize,
    entry: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
}

#[async_trait]
impl FlightRecorder for AlbumMutationOverlapRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        use handshake_core::atelier::collections::collections_event_family;
        let selected = {
            let album_id = self.album_id.lock().expect("overlap album lock");
            event.payload["event_family"] == collections_event_family::MEDIA_ALBUM_ITEMS_REORDERED
                && album_id.as_ref().is_some_and(|id| event.payload["aggregate_id"] == id.as_str())
        };
        if selected {
            self.entered.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.entry.notify_one();
            self.release.acquire().await
                .map_err(|_| RecorderError::SinkError("overlap gate closed".to_owned()))?
                .forget();
        }
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(&self, _filter: EventFilter) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn atelier_ckc_media_reorders_overlap_inside_live_protected_transactions() {
    use handshake_core::atelier::collections::collections_event_family;

    let harness = AtelierSurrealHarness::create().await;
    let recorder = Arc::new(AlbumMutationOverlapRecorder {
        album_id: std::sync::Mutex::new(None),
        entered: std::sync::atomic::AtomicUsize::new(0),
        entry: tokio::sync::Notify::new(),
        release: tokio::sync::Semaphore::new(0),
    });
    let mut owned_server = None;
    let mut request_a = None;
    let mut request_b = None;
    let mut response_a = None;
    let mut response_b = None;
    let outcome = ckc_capture_test_panic(async {
    let owner = AccountFixture::install(&harness.storage).await;
    let mut state = app_state(&harness, false);
    state.flight_recorder = recorder.clone();
    atelier_api::with_account_session(&state, &owner.headers(), async {
        let store = harness.atelier.clone();
        let character = fresh_character(&store, "mt060-overlap", "Protected transaction overlap").await;
        let assets = [
            fresh_api_media_asset(&store, "mt060-overlap-a").await,
            fresh_api_media_asset(&store, "mt060-overlap-b").await,
        ];
        let (base_url, client, server) = serve(state.clone(), &owner).await;
        owned_server = Some(server);
        let (status, album) = json_of(client
            .post(format!("{base_url}/atelier/characters/{character}/media-albums"))
            .json(&serde_json::json!({"name": format!("overlap-{}", Uuid::now_v7()), "sheet_version_id": null}))
            .send().await.expect("create overlap album")).await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
        let album_id = album["collection_id"].as_str().expect("album id").to_owned();
        let items_url = format!("{base_url}/atelier/media-albums/{album_id}/items");
        let (status, added) = json_of(client.post(&items_url)
            .json(&serde_json::json!({"asset_ids": assets}))
            .send().await.expect("add overlap members")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{added}");
        let before_events = harness.database
            .list_kernel_events_for_aggregate("atelier_collection", &album_id)
            .await.expect("read canonical ledger before overlap");
        let before_ids: Vec<_> = before_events.iter().map(|event| event.event_id.clone()).collect();
        *recorder.album_id.lock().expect("arm overlap album") = Some(album_id.clone());
        let headers_a = actor_session(&harness, &owner, "mt060-overlap-writer-a").await.headers();
        let headers_b = actor_session(&harness, &owner, "mt060-overlap-writer-b").await.headers();
        let spawn_reorder = |order: [Uuid; 2], headers| {
            let client = client.clone();
            let url = format!("{items_url}/reorder");
            tokio::spawn(async move {
                client.patch(url).headers(headers)
                    .json(&serde_json::json!({"items": [
                        {"asset_id": order[0], "sort_order": 0},
                        {"asset_id": order[1], "sort_order": 1},
                    ]}))
                    .send().await
            })
        };
        request_a = Some(spawn_reorder([assets[1], assets[0]], headers_a));
        request_b = Some(spawn_reorder([assets[0], assets[1]], headers_b));
        let overlap = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                let entered = recorder.entry.notified();
                if recorder.entered.load(std::sync::atomic::Ordering::SeqCst) >= 2 {
                    break;
                }
                entered.await;
            }
        }).await;
        // Always release parked real callbacks before checking the witness.
        recorder.release.add_permits(2);
        *recorder.album_id.lock().expect("disarm overlap album") = None;
        let responses = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            async {
                tokio::join!(
                    async { response_a = Some(request_a.as_mut().expect("writer a handle").await); },
                    async { response_b = Some(request_b.as_mut().expect("writer b handle").await); },
                );
            },
        ).await;
        assert!(responses.is_ok(), "overlap requests did not settle; transaction settlement proof is blocked");
        request_a = None;
        request_b = None;
        let acquired = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let response_a = response_a.take().ok_or("writer a result missing")?
                .map_err(|_| "writer a task failed")?
                .map_err(|_| "writer a HTTP failed")?;
            let status_a = response_a.status();
            let body_a = response_a.json::<serde_json::Value>().await
                .map_err(|_| "writer a body read failed")?;
            let response_b = response_b.take().ok_or("writer b result missing")?
                .map_err(|_| "writer b task failed")?
                .map_err(|_| "writer b HTTP failed")?;
            let status_b = response_b.status();
            let body_b = response_b.json::<serde_json::Value>().await
                .map_err(|_| "writer b body read failed")?;
            let final_response = client.get(&items_url).send().await
                .map_err(|_| "membership observation HTTP failed")?;
            let status = final_response.status();
            let final_page = final_response.json::<serde_json::Value>().await
                .map_err(|_| "membership observation body read failed")?;
            Ok::<_, &str>((status_a, body_a, status_b, body_b, status, final_page))
        }).await.map_err(|_| "HTTP observation timeout").and_then(|result| result);
        drop(client);
        let (status_a, body_a, status_b, body_b, status, final_page) =
            acquired.expect("overlap HTTP observation failed; worker settlement proof remains required");
        assert_eq!(status, reqwest::StatusCode::OK, "{final_page}");
        let final_ids = member_asset_ids(&final_page);
        assert!(final_ids == vec![assets[0].to_string(), assets[1].to_string()]
            || final_ids == vec![assets[1].to_string(), assets[0].to_string()]);
        let members = final_page["members"].as_array().expect("final members");
        assert_eq!(members.len(), 2);
        for (index, member) in members.iter().enumerate() {
            assert_eq!(member["sort_order"], index as i64);
            assert_eq!(member["member_updated_by"], members[0]["member_updated_by"]);
        }
        let committed_events = harness.database
            .list_kernel_events_for_aggregate("atelier_collection", &album_id)
            .await.expect("read canonical ledger after overlapping writers");
        let committed: Vec<_> = committed_events.iter()
            .filter(|event| !before_ids.contains(&event.event_id))
            .filter(|event| event.payload["event_family"] == collections_event_family::MEDIA_ALBUM_ITEMS_REORDERED)
            .collect();
        let successful = usize::from(status_a == reqwest::StatusCode::OK)
            + usize::from(status_b == reqwest::StatusCode::OK);
        let unknown = usize::from(status_a == reqwest::StatusCode::SERVICE_UNAVAILABLE)
            + usize::from(status_b == reqwest::StatusCode::SERVICE_UNAVAILABLE);
        assert!(committed.len() >= successful && committed.len() <= successful + unknown,
            "canonical events must cover successful writers; Unknown is not a rollback");
        assert!(committed.iter().any(|event| {
            let payload = &event.payload["atelier_payload"];
            payload["requested_by"] == members[0]["member_updated_by"]
                && payload["items"].as_array().is_some_and(|items|
                    items.iter().all(|item| item["sort_order"].as_u64().is_some_and(|position|
                        final_ids.get(position as usize).is_some_and(|asset_id|
                            item["asset_id"] == asset_id.as_str()))))
        }), "the final order and actor must match one committed canonical reorder event");

        assert!(overlap.is_ok(),
            "two real post-write precommit callbacks must overlap; spawned requests are insufficient");
        assert!(recorder.entered.load(std::sync::atomic::Ordering::SeqCst) >= 2);
        for (status, body) in [(status_a, &body_a), (status_b, &body_b)] {
            assert!(status == reqwest::StatusCode::OK || status == reqwest::StatusCode::CONFLICT
                || (status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                    && body["error"] == "album_mutation_outcome_unknown"),
                "overlap must return a typed outcome: {status} {body}");
        }
        assert_eq!(unknown, 0,
            "503 Unknown retains unresolved operation custody; independent settlement proof is blocked, no replay");
        assert!(successful >= 1);
    }).await.expect("authenticated account-scoped overlap test");
    }).await;
    recorder.release.add_permits(2);
    if let Ok(mut album_id) = recorder.album_id.lock() {
        *album_id = None;
    }
    let mut owned_requests = Vec::new();
    if response_a.is_none() {
        owned_requests.extend(request_a);
    }
    if response_b.is_none() {
        owned_requests.extend(request_b);
    }
    ckc_finish_owned_test(harness, owned_server, owned_requests, outcome).await;
}


struct AlbumOperationProofCustody {
    control: Arc<handshake_core::storage::surreal::AlbumMutationProofControl>,
    operation: Option<handshake_core::storage::surreal::AlbumMutationOperationReference>,
    started: bool,
    acknowledged: bool,
}

async fn settle_album_operation_proofs(
    harness: &AtelierSurrealHarness,
    state: &AppState,
    owner: Option<&OwnerSession>,
    custody: &mut [AlbumOperationProofCustody],
) -> Result<(), String> {
    use handshake_core::storage::surreal::AlbumMutationSettlement;
    for proof in custody.iter() {
        proof.control.release();
    }
    if custody.iter().all(|proof| !proof.started || proof.acknowledged) {
        return Ok(());
    }
    let owner = owner.ok_or("actual account fixture unavailable for operation cleanup")?;
    atelier_api::with_account_session(state, &owner.headers(), async {
        for proof in custody.iter_mut().filter(|proof| proof.started && !proof.acknowledged) {
            if proof.operation.is_none() {
                proof.operation = Some(tokio::time::timeout(
                    std::time::Duration::from_secs(30), proof.control.wait_for_entry(),
                ).await.map_err(|_| "worker entry remained unobserved during cleanup")?);
            }
            let settlement = tokio::time::timeout(
                std::time::Duration::from_secs(60), proof.control.wait_for_settlement(),
            ).await.map_err(|_| "actual worker settlement remained unobserved during cleanup")?;
            if matches!(settlement, AlbumMutationSettlement::Pending | AlbumMutationSettlement::Unobserved) {
                return Err(format!("actual worker is not terminal: {settlement:?}"));
            }
            let operation = proof.operation.as_ref().ok_or("original operation identity unavailable")?;
            let observed = harness.storage.observe_album_membership_transaction(operation)
                .map_err(|error| format!("same-account terminal observation failed: {error}"))?;
            if observed.operation != *operation || observed.settlement != settlement {
                return Err("authenticated observation differs from the actual worker settlement".to_owned());
            }
            if !harness.storage.acknowledge_album_membership_transaction(operation)
                .map_err(|error| format!("same-account terminal acknowledgement failed: {error}"))? {
                return Err("terminal operation acknowledgement was not accepted".to_owned());
            }
            proof.acknowledged = true;
        }
        Ok(())
    }).await.map_err(|_| "could not restore the actual account scope for cleanup".to_owned())?
}

#[tokio::test]
async fn atelier_ckc_media_original_operations_survive_pending_until_observed_settlement() {
    use handshake_core::atelier::collections::CollectionItemReorder;
    use handshake_core::atelier::AtelierError;
    use handshake_core::storage::surreal::{
        with_album_mutation_proof_control, AlbumMutationProofControl,
        AlbumMutationSettlement, SurrealStorageError,
    };

    let harness = AtelierSurrealHarness::create().await;
    let state = app_state(&harness, false);
    let mut owner = None;
    let mut owned_server = None;
    let mut custody = Vec::<AlbumOperationProofCustody>::new();
    let outcome = ckc_capture_test_panic(async {
        owner = Some(AccountFixture::install(&harness.storage).await);
        let owner = owner.as_ref().expect("installed account fixture");
        atelier_api::with_account_session(&state, &owner.headers(), async {
            let store = harness.atelier.clone();
            let character = fresh_character(&store, "mt060-operation", "Original operation settlement").await;
            let assets = [
                fresh_api_media_asset(&store, "mt060-operation-a").await,
                fresh_api_media_asset(&store, "mt060-operation-b").await,
            ];
            let (base_url, client, server) = serve(state.clone(), owner).await;
            owned_server = Some(server);
            let (status, album) = json_of(client
                .post(format!("{base_url}/atelier/characters/{character}/media-albums"))
                .json(&serde_json::json!({"name": format!("operation-{}", Uuid::now_v7()), "sheet_version_id": null}))
                .send().await.expect("create operation proof album")).await;
            assert_eq!(status, reqwest::StatusCode::CREATED, "{album}");
            let album_id = album["collection_id"].as_str().expect("album identity").to_owned();
            let collection_id = Uuid::parse_str(&album_id).expect("canonical collection UUID");
            let items_url = format!("{base_url}/atelier/media-albums/{album_id}/items");
            let (status, added) = json_of(client.post(&items_url)
                .json(&serde_json::json!({"asset_ids": assets}))
                .send().await.expect("add operation proof members")).await;
            assert_eq!(status, reqwest::StatusCode::OK, "{added}");
            for (force_pending, force_collision) in [(true, false), (false, false), (false, true)] {
                let desired_assets = if force_collision { [assets[0], assets[1]] } else { [assets[1], assets[0]] };
                let desired = [
                    CollectionItemReorder { asset_id: desired_assets[0], sort_order: 0 },
                    CollectionItemReorder { asset_id: desired_assets[1], sort_order: 1 },
                ];
                let (status, before) = json_of(client.get(&items_url)
                    .send().await.expect("read membership before operation")).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{before}");
                let before_events = harness.database
                    .list_kernel_events_for_aggregate("atelier_collection", &album_id)
                    .await.expect("read original canonical ledger");
                let control = Arc::new(AlbumMutationProofControl::new());
                let occupied = if force_collision {
                    let event = before_events.first().expect("legitimate fixture supplied a canonical event").clone();
                    control.collide_next_ledger_create_with(event.event_id.clone());
                    Some(event)
                } else {
                    None
                };
                let proof_index = custody.len();
                custody.push(AlbumOperationProofCustody {
                    control: control.clone(), operation: None, started: true, acknowledged: false,
                });
                // This scope drives the real store/worker. HTTP tasks do not inherit this task-local control.
                let mut mutation = Box::pin(with_album_mutation_proof_control(control.clone(),
                    store.reorder_media_album_items(collection_id, &desired, &owner.actor_id)));
                let operation = tokio::time::timeout(std::time::Duration::from_secs(30), async {
                    tokio::select! {
                        operation = control.wait_for_entry() => operation,
                        _ = &mut mutation => panic!("mutation completed before the actual worker pause"),
                    }
                }).await.expect("actual worker entered its post-callback pre-settlement pause");
                custody[proof_index].operation = Some(operation.clone());
                assert_ne!(operation.operation_id.to_string(), operation.event_id,
                    "worker and canonical event identities are distinct");
                assert!(!operation.idempotency_key.is_empty());
                assert!(before_events.iter().all(|event| event.event_id != operation.event_id
                    && event.idempotency_key != operation.idempotency_key),
                    "the original operation has a fresh canonical event and idempotency key");
                let initial = harness.storage.observe_album_membership_transaction(&operation)
                    .expect("same-account observation of paused original operation");
                assert_eq!(initial.operation, operation);
                assert!(matches!(initial.settlement, AlbumMutationSettlement::Pending | AlbumMutationSettlement::Unobserved));
                assert!(!harness.storage.acknowledge_album_membership_transaction(&operation)
                    .expect("nonterminal acknowledgement check"));

                if force_collision {
                    use surrealdb::types::{AlreadyExistsError, ErrorDetails, RecordId, ToSql};
                    let occupied = occupied.as_ref().expect("actual occupied canonical row");
                    let expected_record = RecordId::new("kernel_event_ledger", occupied.event_id.clone()).to_sql();
                    control.release();
                    let result = tokio::time::timeout(std::time::Duration::from_secs(60), &mut mutation)
                        .await.expect("occupied canonical CREATE returns its real database error");
                    match result {
                        Err(AtelierError::Database(SurrealStorageError::Database(source))) => {
                            let mut cause = Some(&source);
                            let mut rejected_record = None;
                            while let Some(error) = cause {
                                if let ErrorDetails::AlreadyExists(Some(AlreadyExistsError::Record { id })) = error.details() {
                                    rejected_record = Some(id.clone());
                                    break;
                                }
                                cause = error.cause();
                            }
                            assert_eq!(rejected_record.as_deref(), Some(expected_record.as_str()),
                                "only the typed occupied canonical CREATE rejection proves the insertion fault: {source:?}");
                        }
                        other => panic!("expected the occupied canonical CREATE database error, got {other:?}"),
                    }
                } else if force_pending {
                    let result = tokio::time::timeout(std::time::Duration::from_secs(90), &mut mutation)
                        .await.expect("real caller deadline returns Pending without a synthetic result");
                    match result {
                        Err(AtelierError::Database(SurrealStorageError::AlbumMutationUnknown {
                            operation: returned, source,
                        })) => {
                            assert_eq!(returned, operation, "Unknown keeps the original worker/event/key association");
                            assert!(matches!(*source, SurrealStorageError::TransactionSettlementPending { operation_id }
                                if operation_id == operation.operation_id));
                        }
                        other => panic!("expected the actual Pending worker outcome, got {other:?}"),
                    }
                    let pending = harness.storage.observe_album_membership_transaction(&operation)
                        .expect("observe retained original operation after Pending response");
                    assert!(matches!(pending.settlement, AlbumMutationSettlement::Pending | AlbumMutationSettlement::Unobserved));
                    assert!(!harness.storage.acknowledge_album_membership_transaction(&operation)
                        .expect("Pending must retain observation custody"));
                    control.release();
                } else {
                    control.release();
                    assert_eq!(tokio::time::timeout(std::time::Duration::from_secs(60), &mut mutation)
                        .await.expect("released worker completes its actual commit").expect("commit reorder"), 2);
                }
                let settlement = tokio::time::timeout(
                    std::time::Duration::from_secs(60), control.wait_for_settlement(),
                ).await.expect("actual worker settlement was reported");
                let expected = if force_pending || force_collision {
                    AlbumMutationSettlement::CancelNoEffectObserved
                } else {
                    AlbumMutationSettlement::CommitObserved
                };
                assert_eq!(settlement, expected, "no terminal outcome is inferred from a pending-map disappearance");
                let terminal = harness.storage.observe_album_membership_transaction(&operation)
                    .expect("same-account terminal observation");
                assert_eq!(terminal.operation, operation);
                assert_eq!(terminal.settlement, settlement);
                let after_events = harness.database
                    .list_kernel_events_for_aggregate("atelier_collection", &album_id)
                    .await.expect("read canonical ledger after actual terminal observation");
                let matching: Vec<_> = after_events.iter()
                    .filter(|event| event.event_id == operation.event_id).collect();
                let (status, after) = json_of(client.get(&items_url)
                    .send().await.expect("read membership after actual terminal observation")).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{after}");
                if force_pending || force_collision {
                    assert!(matching.is_empty(), "observed cancellation leaves no associated canonical event");
                    assert!(after_events.iter().all(|event| event.idempotency_key != operation.idempotency_key),
                        "observed cancellation leaves no associated canonical idempotency key");
                    assert_eq!(after_events, before_events, "all returned canonical event fields remain unchanged");
                    assert_eq!(after["members"], before["members"]);
                } else {
                    assert_eq!(matching.len(), 1, "observed commit retains its exact canonical event");
                    assert_eq!(matching[0].idempotency_key, operation.idempotency_key,
                        "the committed canonical event retains its original operation association");
                    assert_eq!(member_asset_ids(&after), [assets[1].to_string(), assets[0].to_string()]);
                    assert_eq!(matching[0].payload["atelier_payload"]["requested_by"], owner.actor_id);
                }
                assert!(harness.storage.acknowledge_album_membership_transaction(&operation)
                    .expect("acknowledge only the observed original terminal operation"));
                custody[proof_index].acknowledged = true;
            }
            drop(client);
        }).await.expect("actual authenticated operation proof scope");
    }).await;
    let cleanup = ckc_capture_test_panic(settle_album_operation_proofs(
        &harness, &state, owner.as_deref(), &mut custody,
    )).await;
    let cleanup_error = match cleanup {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(Box::new(error) as CkcTestPanic),
        Err(error) => Some(error),
    };
    if let Some(error) = cleanup_error {
        if let Some(server) = owned_server {
            server.abort();
            let _ = server.await;
        }
        eprintln!("Original album operation remains unobserved; retaining fixture without engine shutdown or Drop");
        std::mem::forget(harness);
        match outcome {
            Err(primary) => std::panic::resume_unwind(primary),
            Ok(()) => std::panic::resume_unwind(error),
        }
    }
    ckc_finish_owned_test(harness, owned_server, Vec::new(), outcome).await;
}
