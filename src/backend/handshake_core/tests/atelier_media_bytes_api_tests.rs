//! WP-CKC-posekit-overhaul (SurrealDB port) MT-057: the ArtifactStore READ path over HTTP.
//!
//! Before this port the artifact tier was write-only over HTTP (`write_file_artifact` existed,
//! `read_file_artifact` did not), which is the blocker Studio's placed-asset binding
//! ([STU-ASSET-005] / `asset.resolve_bytes`) sits behind. These tests drive the production Axum
//! route `GET /atelier/media-assets/:asset_id/bytes` against a real embedded SurrealDB catalog row
//! and a real ArtifactStore payload on disk, and prove the four fail-closed outcomes:
//!
//! - the exact stored bytes come back with the catalog MIME, an ETag carrying the content hash,
//!   and the artifact ref;
//! - an unknown asset id is 404, never an empty 200;
//! - a payload deleted from disk after cataloguing is 404 (row exists, bytes do not);
//! - a payload tampered on disk (same length, different bytes) is a hard 500, never the tampered
//!   bytes.

mod atelier_surreal_support;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use atelier_surreal_support::{
    write_native_media_artifact_in_workspace, AtelierSurrealHarness, NativeMediaArtifact,
};
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

/// One workspace root for the whole test binary. `HANDSHAKE_WORKSPACE_ROOT` is process-global and
/// tests run on parallel threads, so every test writes its own artifact (distinct UUID) into this
/// single root instead of racing on the env var.
/// Ingest ceiling for this test binary (bytes). Set once alongside the workspace root; the
/// over-limit test streams a body above it, every other test stays well below it.
const TEST_INGEST_MAX_BYTES: u64 = 6 * 1024 * 1024;

fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir()
            .expect("create isolated media-bytes workspace root")
            .keep();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &root);
        std::env::set_var(
            "HANDSHAKE_MEDIA_INGEST_MAX_BYTES",
            TEST_INGEST_MAX_BYTES.to_string(),
        );
        root
    })
}

/// Deterministic pseudo-random payload so two tests never collide on content hash by accident and
/// the bytes are not compressible-trivial.
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

/// Count artifact directories and stray temp files under the L1 layer of the shared root.
fn l1_artifact_inventory() -> (usize, usize) {
    let l1 = shared_workspace_root()
        .join(".handshake")
        .join("artifacts")
        .join("L1");
    let Ok(entries) = fs::read_dir(&l1) else {
        return (0, 0);
    };
    let mut dirs = 0;
    let mut tmp_files = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs += 1;
            if let Ok(inner) = fs::read_dir(&path) {
                tmp_files += inner
                    .flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with(".hsk_tmp_"))
                    .count();
            }
        }
    }
    (dirs, tmp_files)
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

async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, atelier_api::routes(state))
            .await
            .expect("Atelier API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

async fn catalogued_asset(
    store: &AtelierStore,
    payload: &[u8],
) -> (Uuid, NativeMediaArtifact) {
    let artifact = write_native_media_artifact_in_workspace(shared_workspace_root(), payload);
    let asset = store
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

#[tokio::test]
async fn media_asset_bytes_route_returns_exact_stored_bytes_with_integrity_headers() {
    let harness = AtelierSurrealHarness::create().await;
    let payload = format!("mt-057 exact stored payload {}", Uuid::now_v7()).into_bytes();
    let (asset_id, artifact) = catalogued_asset(&harness.atelier, &payload).await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let response = client
        .get(format!("{base}/atelier/media-assets/{asset_id}/bytes"))
        .send()
        .await
        .expect("send media-asset bytes request");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.expect("read body");
    server.abort();

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
        headers.get(reqwest::header::ETAG).and_then(|v| v.to_str().ok()),
        Some(format!("\"sha256-{}\"", artifact.content_hash).as_str()),
        "ETag must carry the content hash so a placed-asset link can store resolved_content_hash"
    );
    assert_eq!(
        headers
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("private, immutable")
    );
    assert_eq!(
        headers.get("x-hsk-artifact-ref").and_then(|v| v.to_str().ok()),
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
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_bytes_route_unknown_asset_is_404_not_empty_200() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let response = client
        .get(format!(
            "{base}/atelier/media-assets/{}/bytes",
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("send missing media-asset bytes request");
    let status = response.status();
    let body = response.text().await.expect("read body");
    server.abort();

    assert_eq!(status.as_u16(), 404, "unknown asset must be 404, body={body}");
    assert!(body.contains("not_found"), "typed error body expected, got {body}");
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_bytes_route_missing_payload_on_disk_is_404() {
    let harness = AtelierSurrealHarness::create().await;
    let payload = format!("mt-057 payload to delete {}", Uuid::now_v7()).into_bytes();
    let (asset_id, artifact) = catalogued_asset(&harness.atelier, &payload).await;
    // The catalog row is durable; the bytes vanish (operator deleted the workspace file, a copy
    // lost the payload). The route must report the absence, not serve a fabricated body.
    fs::remove_file(&artifact.payload_path).expect("delete payload after cataloguing");
    let (base, client, server) = serve(app_state(&harness)).await;

    let response = client
        .get(format!("{base}/atelier/media-assets/{asset_id}/bytes"))
        .send()
        .await
        .expect("send request for asset whose payload is gone");
    let status = response.status();
    server.abort();

    assert_eq!(status.as_u16(), 404, "missing payload must be 404");

    let direct = harness.atelier.read_media_asset_bytes(asset_id).await;
    assert!(
        matches!(direct, Err(MediaAssetBytesError::PayloadMissing)),
        "store-level read must classify the missing payload, got {direct:?}"
    );
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_bytes_route_tampered_payload_is_hard_error_never_tampered_bytes() {
    let harness = AtelierSurrealHarness::create().await;
    let payload = format!("mt-057 payload to tamper {}", Uuid::now_v7()).into_bytes();
    let (asset_id, artifact) = catalogued_asset(&harness.atelier, &payload).await;
    // Same length, different content: the size pre-check passes, the sha256 re-hash must not.
    let mut tampered = payload.clone();
    tampered[0] ^= 0xff;
    fs::write(&artifact.payload_path, &tampered).expect("overwrite payload with tampered bytes");
    let (base, client, server) = serve(app_state(&harness)).await;

    let response = client
        .get(format!("{base}/atelier/media-assets/{asset_id}/bytes"))
        .send()
        .await
        .expect("send request for tampered asset");
    let status = response.status();
    let body = response.bytes().await.expect("read body");
    server.abort();

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

    let direct = harness.atelier.read_media_asset_bytes(asset_id).await;
    assert!(
        matches!(direct, Err(MediaAssetBytesError::Artifact(_))),
        "store-level read must surface the integrity failure, got {direct:?}"
    );
    harness.shutdown().await;
}

// ---------------------------------------------------------------------------------------------
// Streaming ingest: POST /atelier/media-assets (raw body, size-capped, blob -> verify -> row).
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn media_asset_ingest_streams_raw_body_into_artifact_store_and_serves_it_back() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let payload = pseudo_random_payload(0x5701, 3 * 1024 * 1024);
    let expected_hash = sha256_hex(&payload);
    let (dirs_before, _) = l1_artifact_inventory();

    let response = client
        .post(format!("{base}/atelier/media-assets"))
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
    assert_eq!(status.as_u16(), 201, "first ingest must be 201 Created, got {body}");
    assert_eq!(body["content_hash"], expected_hash);
    assert_eq!(body["byte_len"], payload.len() as i64);
    assert_eq!(body["mime"], "video/mp4");
    assert_eq!(body["dedup_hit"], false);
    let asset_id = body["asset_id"].as_str().expect("asset_id").to_owned();
    let artifact_ref = body["artifact_ref"].as_str().expect("artifact_ref").to_owned();
    assert!(
        artifact_ref.starts_with("artifact://.handshake/artifacts/L1/")
            && artifact_ref.ends_with("/payload"),
        "artifact_ref must be a native single-file payload ref, got {artifact_ref}"
    );
    let (dirs_after, tmp_after) = l1_artifact_inventory();
    assert_eq!(dirs_after, dirs_before + 1, "exactly one artifact directory is created");
    assert_eq!(tmp_after, 0, "no .hsk_tmp_ residue after a successful ingest");

    // The bytes round-trip through the READ path with the ingest hash as ETag.
    let read = client
        .get(format!("{base}/atelier/media-assets/{asset_id}/bytes"))
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
    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_ingest_dedups_identical_bytes_and_removes_the_duplicate_blob() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let payload = pseudo_random_payload(0x5702, 512 * 1024);

    let first: serde_json::Value = client
        .post(format!("{base}/atelier/media-assets"))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(payload.clone())
        .send()
        .await
        .expect("first ingest")
        .json()
        .await
        .expect("first json");
    let (dirs_after_first, _) = l1_artifact_inventory();

    let second_response = client
        .post(format!("{base}/atelier/media-assets"))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "model:mt-057-second-writer")
        .body(payload.clone())
        .send()
        .await
        .expect("second ingest");
    let second_status = second_response.status();
    let second: serde_json::Value = second_response.json().await.expect("second json");
    let (dirs_after_second, tmp_after_second) = l1_artifact_inventory();
    server.abort();

    assert_eq!(second_status.as_u16(), 200, "dedup returns 200, not 201: {second}");
    assert_eq!(second["dedup_hit"], true);
    assert_eq!(second["asset_id"], first["asset_id"], "same catalog identity");
    assert_eq!(second["artifact_ref"], first["artifact_ref"], "same blob");
    assert_eq!(
        dirs_after_second, dirs_after_first,
        "the duplicate streamed blob must be removed, not accumulated"
    );
    assert_eq!(tmp_after_second, 0);
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_ingest_over_limit_is_413_and_leaves_no_artifact_behind() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let (dirs_before, _) = l1_artifact_inventory();
    let oversized = pseudo_random_payload(0x5703, (TEST_INGEST_MAX_BYTES as usize) + 4096);

    // Declared length above the ceiling: refused before any byte is read.
    let declared = client
        .post(format!("{base}/atelier/media-assets"))
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
    let chunked = client
        .post(format!("{base}/atelier/media-assets"))
        .header("content-type", "video/mp4")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await
        .expect("send oversized (chunked) ingest");
    let chunked_status = chunked.status();
    let (dirs_after, tmp_after) = l1_artifact_inventory();
    server.abort();

    assert_eq!(chunked_status.as_u16(), 413, "streaming cap must abort with 413");
    assert_eq!(dirs_after, dirs_before, "no artifact directory may survive a capped upload");
    assert_eq!(tmp_after, 0, "no .hsk_tmp_ residue may survive a capped upload");
    harness.shutdown().await;
}

#[tokio::test]
async fn media_asset_ingest_requires_actor_and_content_type_and_rejects_empty_body() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let (dirs_before, _) = l1_artifact_inventory();

    let no_actor = client
        .post(format!("{base}/atelier/media-assets"))
        .header("content-type", "image/png")
        .body(b"abc".to_vec())
        .send()
        .await
        .expect("send without actor");
    assert_eq!(no_actor.status().as_u16(), 400);

    let no_mime = client
        .post(format!("{base}/atelier/media-assets"))
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(b"abc".to_vec())
        .send()
        .await
        .expect("send without content-type");
    assert_eq!(no_mime.status().as_u16(), 400);

    let empty = client
        .post(format!("{base}/atelier/media-assets"))
        .header("content-type", "image/png")
        .header("x-hsk-actor-id", "operator:mt-057-test")
        .body(Vec::<u8>::new())
        .send()
        .await
        .expect("send empty body");
    let empty_status = empty.status();
    let (dirs_after, tmp_after) = l1_artifact_inventory();
    server.abort();

    assert_eq!(empty_status.as_u16(), 400, "empty payload is rejected");
    assert_eq!(dirs_after, dirs_before);
    assert_eq!(tmp_after, 0);
    harness.shutdown().await;
}
