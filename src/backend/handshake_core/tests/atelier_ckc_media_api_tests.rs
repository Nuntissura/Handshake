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
//! attribution is proven from `x-hsk-actor-id` alone; the pagination proof crosses `LIST_CAP`
//! (>200 members); and one real concurrent-writer race (two `tokio::join!`ed reorders on one album)
//! is added per MT-056 F5/F9.

mod atelier_surreal_support;

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
        let root = tempfile::tempdir()
            .expect("create isolated ckc-media workspace root")
            .keep();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &root);
        root
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

fn app_state(harness: &AtelierSurrealHarness, embeddings: bool) -> AppState {
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

async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, ckc_router(state))
            .await
            .expect("Atelier CKC media API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
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
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let actor = format!("mt010-media-agent-{}", Uuid::now_v7());

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
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_album_unlink_reorder_and_link_ref_edit() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .header("x-hsk-actor-id", "operator")
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
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_rows_preserve_actor_attribution() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let actor_a = "album-agent-a";
    let actor_b = "album-agent-b";
    let actor_c = "album-agent-c";
    let actor_d = "album-agent-d";
    let actor_e = "album-agent-e";
    let actor_f = "album-agent-f";
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
            .header("x-hsk-actor-id", actor_a)
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
            .header("x-hsk-actor-id", actor_b)
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
            .header("x-hsk-actor-id", actor_c)
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
            .header("x-hsk-actor-id", actor_d)
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
            .header("x-hsk-actor-id", actor_e)
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
            .header("x-hsk-actor-id", actor_f)
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
        client
            .post(format!("{base_url}/atelier/media-albums/{album_id}/items"))
            .json(&serde_json::json!({ "asset_ids": [second_asset] }))
            .send()
            .await
            .expect("no actor"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "mutations without x-hsk-actor-id are rejected: {missing_actor}"
    );

    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_source_refs_are_validated_and_link_scoped() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .header("x-hsk-actor-id", "operator")
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
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_media_album_large_library_pagination() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let with_operator = |request: reqwest::RequestBuilder| {
        request
            .header("x-hsk-actor-id", "operator")
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
    harness.shutdown().await;
}

/// MT-056 F5/F9: two writers reorder the same album concurrently. Neither may 500, neither may
/// leave a torn order; the album ends in exactly one of the two requested dense orders.
#[tokio::test]
async fn atelier_ckc_media_album_concurrent_reorders_never_tear_the_order() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
    let character_internal_id =
        fresh_character(&store, "mt056-race-char", "MT-056 Reorder Race Character").await;
    let (status, album) = json_of(
        client
            .post(format!(
                "{base_url}/atelier/characters/{character_internal_id}/media-albums"
            ))
            .header("x-hsk-actor-id", "race-writer-setup")
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
            .header("x-hsk-actor-id", "race-writer-setup")
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
        .header("x-hsk-actor-id", "race-writer-a")
        .json(&payload_for(&order_a))
        .send();
    let request_b = client
        .patch(format!(
            "{base_url}/atelier/media-albums/{album_id}/items/reorder"
        ))
        .header("x-hsk-actor-id", "race-writer-b")
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
    harness.shutdown().await;
}

#[tokio::test]
async fn atelier_ckc_search_api_returns_fuzzy_vector_combined_refs_and_tag_notes() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, true)).await;
    let actor = format!("mt011-search-agent-{}", Uuid::now_v7());

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
    harness.shutdown().await;
}

/// Without an embedding endpoint the vector leg degrades honestly: no fabricated vectors,
/// `semantic_available=false`, dHash similarity still works.
#[tokio::test]
async fn atelier_ckc_search_degrades_without_embedding_model() {
    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let (base_url, client, server) = serve(app_state(&harness, false)).await;
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
            .header("x-hsk-actor-id", "degrade-proof")
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
            .header("x-hsk-actor-id", "degrade-proof")
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
    harness.shutdown().await;
}
