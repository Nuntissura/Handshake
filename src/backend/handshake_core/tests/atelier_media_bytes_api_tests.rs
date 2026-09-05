//! WP-CKC-posekit-overhaul (SurrealDB port) MT-057 / MT-064: the ArtifactStore READ path and the
//! streaming, size-capped WRITE path over HTTP.
//!
//! Before this port the artifact tier was write-only over HTTP (`write_file_artifact` existed,
//! `read_file_artifact` did not), which is the blocker Studio's placed-asset binding
//! ([STU-ASSET-005] / `asset.resolve_bytes`) sits behind. Ingest was equally unusable for bulk
//! binary: the only path was a base64 JSON envelope with no size ceiling.
//!
//! Shape: ONE `#[tokio::test]` umbrella running every scenario sequentially against ONE
//! bootstrapped embedded SurrealDB and ONE workspace root. Bootstrapping the 292-table schema is
//! the dominant cost (minutes under host contention), and the artifact-store assertions count
//! directories in a shared root, so parallel per-test harnesses would be both slow and racy. Each
//! scenario is `catch_unwind`-attributed by name so a failure still says which one broke. This
//! mirrors `atelier_prompt_feedback_tests.rs`.

mod atelier_surreal_support;

use std::fs;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atelier_surreal_support::{
    write_native_media_artifact_in_workspace, AtelierSurrealHarness, NativeMediaArtifact,
};
use futures::FutureExt;
use handshake_core::api::atelier as atelier_api;
use handshake_core::atelier::{AtelierStore, MediaAssetBytesError, NewMediaAsset};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use uuid::Uuid;

/// Ingest ceiling for this test binary (bytes). The over-limit scenario streams a body above it;
/// every other scenario stays well below.
const TEST_INGEST_MAX_BYTES: u64 = 6 * 1024 * 1024;

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
            profile: ModelProfile::new("mt057-media-bytes-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// Everything a scenario needs: the live server base URL, an HTTP client, the store, and the
/// workspace root the ArtifactStore writes under.
struct Ctx {
    base: String,
    client: reqwest::Client,
    store: AtelierStore,
    workspace_root: PathBuf,
}

/// Deterministic pseudo-random payload so two scenarios never collide on content hash by accident
/// and the bytes are not trivially compressible.
fn pseudo_random_payload(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state & 0xff) as u8
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn l1_root(ctx: &Ctx) -> PathBuf {
    ctx.workspace_root
        .join(".handshake")
        .join("artifacts")
        .join("L1")
}

/// Directory names currently under the L1 layer. Sequential scenarios make set differences exact.
fn l1_dirs(ctx: &Ctx) -> Vec<String> {
    let Ok(entries) = fs::read_dir(l1_root(ctx)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// Any leftover atomic-write temp file anywhere under L1. A successful write renames its temp file;
/// an aborted one must remove it.
fn l1_tmp_files(ctx: &Ctx) -> usize {
    let Ok(entries) = fs::read_dir(l1_root(ctx)) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| fs::read_dir(e.path()).ok())
        .flat_map(|inner| inner.flatten())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".hsk_tmp_"))
        .count()
}

/// Catalogue an asset from a payload written directly through the ArtifactStore (the WRITE path
/// that already existed), so the READ scenarios do not depend on the ingest route.
async fn catalogued_asset(ctx: &Ctx, payload: &[u8]) -> (Uuid, NativeMediaArtifact) {
    let artifact = write_native_media_artifact_in_workspace(&ctx.workspace_root, payload);
    let asset = ctx
        .store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-057 media bytes read-path fixture".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize media asset against the real ArtifactStore payload");
    (asset.asset_id, artifact)
}

// ---------------------------------------------------------------------------------------------
// READ scenarios: GET /atelier/media-assets/:asset_id/bytes
// ---------------------------------------------------------------------------------------------

async fn read_returns_exact_stored_bytes_with_integrity_headers(ctx: &Ctx) {
    let payload = pseudo_random_payload(0x5701, 3 * 1024 * 1024);
    let (asset_id, artifact) = catalogued_asset(ctx, &payload).await;

    let response = ctx
        .client
        .get(format!("{}/atelier/media-assets/{asset_id}/bytes", ctx.base))
        .send()
        .await
        .expect("send media-asset bytes request");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.expect("read body");

    assert!(status.is_success(), "expected 200, got {status}");
    assert_eq!(
        headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/png"),
        "Content-Type must be the catalog-authoritative MIME"
    );
    assert_eq!(
        headers
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok()),
        Some(payload.len()),
        "Content-Length must equal the manifest size"
    );
    assert_eq!(
        headers
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok()),
        Some(format!("\"sha256-{}\"", artifact.content_hash).as_str()),
        "ETag carries the content hash a placed-asset link stores as resolved_content_hash"
    );
    assert_eq!(
        headers
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("private, immutable")
    );
    assert_eq!(
        headers
            .get("x-hsk-artifact-ref")
            .and_then(|v| v.to_str().ok()),
        Some(artifact.artifact_ref.as_str())
    );
    assert_eq!(
        headers
            .get("x-hsk-content-sha256")
            .and_then(|v| v.to_str().ok()),
        Some(artifact.content_hash.as_str())
    );
    assert_eq!(
        body.as_ref(),
        artifact.stored_payload.as_slice(),
        "byte route must return the EXACT stored ArtifactStore payload bytes"
    );
}

async fn read_unknown_asset_is_404_not_empty_200(ctx: &Ctx) {
    let response = ctx
        .client
        .get(format!(
            "{}/atelier/media-assets/{}/bytes",
            ctx.base,
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("send missing media-asset bytes request");
    let status = response.status();
    let body = response.text().await.expect("read body");

    assert_eq!(status.as_u16(), 404, "unknown asset must be 404, body={body}");
    assert!(
        body.contains("not_found"),
        "typed error body expected, got {body}"
    );
}

async fn read_missing_payload_on_disk_is_404(ctx: &Ctx) {
    let payload = pseudo_random_payload(0x5702, 64 * 1024);
    let (asset_id, artifact) = catalogued_asset(ctx, &payload).await;
    // The catalog row is durable; the bytes vanish (operator deleted the file, a copy lost the
    // payload). The route must report the absence, never a fabricated body.
    fs::remove_file(&artifact.payload_path).expect("delete payload after cataloguing");

    let response = ctx
        .client
        .get(format!("{}/atelier/media-assets/{asset_id}/bytes", ctx.base))
        .send()
        .await
        .expect("send request for asset whose payload is gone");
    assert_eq!(response.status().as_u16(), 404, "missing payload must be 404");

    let direct = ctx.store.read_media_asset_bytes(asset_id).await;
    assert!(
        matches!(direct, Err(MediaAssetBytesError::PayloadMissing)),
        "store-level read must classify the missing payload, got {direct:?}"
    );
}

async fn read_tampered_payload_is_hard_error_never_tampered_bytes(ctx: &Ctx) {
    let payload = pseudo_random_payload(0x5703, 64 * 1024);
    let (asset_id, artifact) = catalogued_asset(ctx, &payload).await;
    // Same length, different content: the size pre-check passes, the sha256 re-hash must not.
    let mut tampered = payload.clone();
    tampered[0] ^= 0xff;
    fs::write(&artifact.payload_path, &tampered).expect("overwrite payload with tampered bytes");

    let response = ctx
        .client
        .get(format!("{}/atelier/media-assets/{asset_id}/bytes", ctx.base))
        .send()
        .await
        .expect("send request for tampered asset");
    let status = response.status();
    let body = response.bytes().await.expect("read body");

    assert_eq!(
        status.as_u16(),
        500,
        "tampered payload must fail closed as a hard error"
    );
    assert_ne!(
        body.as_ref(),
        tampered.as_slice(),
        "tampered bytes must never be served"
    );
    assert_ne!(
        body.as_ref(),
        payload.as_slice(),
        "original bytes cannot be served either: they no longer exist on disk"
    );

    let direct = ctx.store.read_media_asset_bytes(asset_id).await;
    assert!(
        matches!(direct, Err(MediaAssetBytesError::Artifact(_))),
        "store-level read must surface the integrity failure, got {direct:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// WRITE scenarios: POST /atelier/media-assets (raw body, streamed, size-capped)
// ---------------------------------------------------------------------------------------------

async fn ingest_streams_raw_body_into_artifact_store_and_serves_it_back(ctx: &Ctx) {
    let payload = pseudo_random_payload(0x5711, 3 * 1024 * 1024);
    let expected_hash = sha256_hex(&payload);
    let before = l1_dirs(ctx);

    let response = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "video/mp4")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .header("x-hsk-source-provenance", "mt-057 streaming ingest fixture")
        .header("x-hsk-filename-hint", "clip.mp4")
        .body(payload.clone())
        .send()
        .await
        .expect("send streaming ingest request");
    let status = response.status();
    let body: serde_json::Value = response.json().await.expect("ingest response json");

    assert_eq!(
        status.as_u16(),
        201,
        "first ingest must be 201 Created, got {body}"
    );
    assert_eq!(body["content_hash"], expected_hash);
    assert_eq!(body["byte_len"], payload.len() as i64);
    assert_eq!(body["mime"], "video/mp4");
    assert_eq!(body["dedup_hit"], false);
    let asset_id = body["asset_id"].as_str().expect("asset_id").to_owned();
    let artifact_ref = body["artifact_ref"]
        .as_str()
        .expect("artifact_ref")
        .to_owned();
    assert!(
        artifact_ref.starts_with("artifact://.handshake/artifacts/L1/")
            && artifact_ref.ends_with("/payload"),
        "artifact_ref must be a native single-file payload ref, got {artifact_ref}"
    );

    let after = l1_dirs(ctx);
    let added: Vec<&String> = after.iter().filter(|d| !before.contains(d)).collect();
    assert_eq!(added.len(), 1, "exactly one artifact directory is created");
    assert!(
        artifact_ref.contains(added[0].as_str()),
        "the created directory must be the one the response names"
    );
    assert_eq!(l1_tmp_files(ctx), 0, "no temp residue after a successful ingest");

    // The bytes round-trip through the READ path with the ingest hash as ETag.
    let read = ctx
        .client
        .get(format!("{}/atelier/media-assets/{asset_id}/bytes", ctx.base))
        .send()
        .await
        .expect("read back ingested bytes");
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(
        read.headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok()),
        Some(format!("\"sha256-{expected_hash}\"").as_str())
    );
    assert_eq!(
        read.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("video/mp4")
    );
    let read_bytes = read.bytes().await.expect("read body");
    assert_eq!(read_bytes.as_ref(), payload.as_slice());
}

async fn ingest_dedups_identical_bytes_and_removes_the_duplicate_blob(ctx: &Ctx) {
    let payload = pseudo_random_payload(0x5712, 512 * 1024);

    let first: serde_json::Value = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(payload.clone())
        .send()
        .await
        .expect("first ingest")
        .json()
        .await
        .expect("first json");
    let after_first = l1_dirs(ctx);

    let second_response = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "model:mt-057-second-writer")
        .body(payload.clone())
        .send()
        .await
        .expect("second ingest");
    let second_status = second_response.status();
    let second: serde_json::Value = second_response.json().await.expect("second json");
    let after_second = l1_dirs(ctx);

    assert_eq!(
        second_status.as_u16(),
        200,
        "dedup returns 200, not 201: {second}"
    );
    assert_eq!(second["dedup_hit"], true);
    assert_eq!(second["asset_id"], first["asset_id"], "same catalog identity");
    assert_eq!(second["artifact_ref"], first["artifact_ref"], "same blob");
    assert_eq!(
        after_second, after_first,
        "the duplicate streamed blob must be removed, not accumulated"
    );
    assert_eq!(l1_tmp_files(ctx), 0);
}

async fn ingest_over_limit_is_413_and_leaves_no_artifact_behind(ctx: &Ctx) {
    let before = l1_dirs(ctx);
    let oversized = pseudo_random_payload(0x5713, (TEST_INGEST_MAX_BYTES as usize) + 4096);

    // Declared length above the ceiling: refused before any byte is read.
    let declared = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "video/mp4")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(oversized.clone())
        .send()
        .await
        .expect("send oversized (declared) ingest");
    assert_eq!(declared.status().as_u16(), 413);

    // Chunked transfer with no Content-Length: refused while streaming, temp file removed.
    let stream = futures::stream::iter(
        oversized
            .chunks(64 * 1024)
            .map(|chunk| Ok::<_, std::io::Error>(chunk.to_vec()))
            .collect::<Vec<_>>(),
    );
    let chunked = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "video/mp4")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await
        .expect("send oversized (chunked) ingest");

    assert_eq!(
        chunked.status().as_u16(),
        413,
        "streaming cap must abort with 413"
    );
    assert_eq!(
        l1_dirs(ctx),
        before,
        "no artifact directory may survive a capped upload"
    );
    assert_eq!(l1_tmp_files(ctx), 0, "no temp residue may survive a capped upload");
}

async fn ingest_requires_actor_and_content_type_and_rejects_empty_body(ctx: &Ctx) {
    let before = l1_dirs(ctx);

    let no_actor = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "image/png")
        .body(b"abc".to_vec())
        .send()
        .await
        .expect("send without actor");
    assert_eq!(no_actor.status().as_u16(), 400);

    let no_mime = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(b"abc".to_vec())
        .send()
        .await
        .expect("send without content-type");
    assert_eq!(no_mime.status().as_u16(), 400);

    let empty = ctx
        .client
        .post(format!("{}/atelier/media-assets", ctx.base))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(Vec::<u8>::new())
        .send()
        .await
        .expect("send empty body");

    assert_eq!(empty.status().as_u16(), 400, "empty payload is rejected");
    assert_eq!(l1_dirs(ctx), before, "a rejected ingest creates no artifact");
    assert_eq!(l1_tmp_files(ctx), 0);
}

// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn media_artifact_byte_read_and_streaming_ingest_proof_on_one_embedded_store() {
    let workspace_root = tempfile::tempdir()
        .expect("create isolated media-bytes workspace root")
        .keep();
    // Process-global for this binary only: `resolve_workspace_root` and the ingest ceiling are read
    // from the environment by the production code under test.
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &workspace_root);
    std::env::set_var(
        "HANDSHAKE_MEDIA_INGEST_MAX_BYTES",
        TEST_INGEST_MAX_BYTES.to_string(),
    );

    let harness = AtelierSurrealHarness::create().await;
    let store = harness.atelier.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let state = app_state(&harness);
    let server = tokio::spawn(async move {
        axum::serve(listener, atelier_api::routes(state))
            .await
            .expect("Atelier API server");
    });

    let ctx = Ctx {
        base: format!("http://{addr}"),
        client: reqwest::Client::new(),
        store,
        workspace_root,
    };

    let mut failures: Vec<String> = Vec::new();
    macro_rules! scenario {
        ($name:literal, $call:expr) => {
            if AssertUnwindSafe($call).catch_unwind().await.is_err() {
                failures.push($name.to_string());
            }
        };
    }

    scenario!(
        "read_returns_exact_stored_bytes_with_integrity_headers",
        read_returns_exact_stored_bytes_with_integrity_headers(&ctx)
    );
    scenario!(
        "read_unknown_asset_is_404_not_empty_200",
        read_unknown_asset_is_404_not_empty_200(&ctx)
    );
    scenario!(
        "read_missing_payload_on_disk_is_404",
        read_missing_payload_on_disk_is_404(&ctx)
    );
    scenario!(
        "read_tampered_payload_is_hard_error_never_tampered_bytes",
        read_tampered_payload_is_hard_error_never_tampered_bytes(&ctx)
    );
    scenario!(
        "ingest_streams_raw_body_into_artifact_store_and_serves_it_back",
        ingest_streams_raw_body_into_artifact_store_and_serves_it_back(&ctx)
    );
    scenario!(
        "ingest_dedups_identical_bytes_and_removes_the_duplicate_blob",
        ingest_dedups_identical_bytes_and_removes_the_duplicate_blob(&ctx)
    );
    scenario!(
        "ingest_over_limit_is_413_and_leaves_no_artifact_behind",
        ingest_over_limit_is_413_and_leaves_no_artifact_behind(&ctx)
    );
    scenario!(
        "ingest_requires_actor_and_content_type_and_rejects_empty_body",
        ingest_requires_actor_and_content_type_and_rejects_empty_body(&ctx)
    );

    server.abort();
    harness.shutdown().await;

    assert!(
        failures.is_empty(),
        "media artifact byte scenarios failed: {failures:?}"
    );
}
