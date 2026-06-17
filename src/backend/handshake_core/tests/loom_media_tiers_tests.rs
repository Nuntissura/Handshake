//! WP-KERNEL-009 MT-259 MediaCacheTiers — REAL PostgreSQL + REAL HTTP proof.
//!
//! GAP-LM-009 (cache-tiered media) and the inherited MT-244 gaps:
//!   * GAP-LM-009b — HTTP Range support on GET .../assets/:id/content (206 +
//!     Content-Range + Accept-Ranges; 416 unsatisfiable).
//!   * GAP-LM-244a — a real backend album/slideshow list-source
//!     (loom_collections + loom_collection_members), ordered enumeration.
//!
//! Authority = PostgreSQL (media_asset_tiers, loom_collections,
//! loom_collection_members) + the original asset blob on disk. Tiers are
//! derived/regenerable; the original is authority. No SQLite, no mock, no
//! synthetic proof. The HTTP routes are the actual `api::loom::routes` driven
//! over a loopback listener (quiet — no foreground window).

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::loom as loom_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    Database, JobKind, JobState, LoomBlockContentType, LoomBlockDerived, MediaTier,
    MediaTierStatus, MediaTierUpsert, NewAsset, NewLoomBlock, WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};

macro_rules! pg_or_skip {
    ($name:expr) => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-259 {}: PostgreSQL unavailable", $name);
                return;
            }
        }
    }};
}

/// A FlightRecorder that CAPTURES every event so a test can assert the
/// LoomPreviewGenerated receipt actually carries the per-tier pyramid.
#[derive(Default)]
struct NoopRecorder {
    events: std::sync::Mutex<Vec<FlightRecorderEvent>>,
}

impl NoopRecorder {
    fn captured(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().unwrap().push(event);
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
        _id: uuid::Uuid,
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

/// Build a real AppState against the isolated schema, returning it plus the
/// capturing recorder so tests can drive jobs AND assert receipts.
async fn loom_state(pg: &KnowledgePg) -> (AppState, Arc<NoopRecorder>) {
    let storage = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect AppState pool");
    let recorder = Arc::new(NoopRecorder::default());
    let state = AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder.clone(),
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("media-tiers-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    };
    (state, recorder)
}

/// Boot the real loom routes over loopback against the isolated schema.
async fn loom_server(pg: &KnowledgePg) -> (String, reqwest::Client, AppState, Arc<NoopRecorder>) {
    let (state, recorder) = loom_state(pg).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = loom_api::routes(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("loom api server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), state, recorder)
}

/// Create an `original` asset row AND write its blob to disk under the configured
/// HANDSHAKE_WORKSPACE_ROOT, returning (asset_id, content_hash, blob_path).
async fn make_original_asset(
    db: &PostgresDatabase,
    root: &std::path::Path,
    ws: &str,
    mime: &str,
    bytes: &[u8],
) -> (String, String, std::path::PathBuf) {
    use sha2::{Digest, Sha256};
    let content_hash = {
        let mut h = Sha256::new();
        h.update(bytes);
        hex::encode(h.finalize())
    };
    let ctx = WriteContext::human(None);
    let asset = db
        .create_asset(
            &ctx,
            NewAsset {
                workspace_id: ws.to_string(),
                kind: "original".to_string(),
                mime: mime.to_string(),
                original_filename: Some("media.bin".to_string()),
                content_hash: content_hash.clone(),
                size_bytes: bytes.len() as i64,
                width: None,
                height: None,
                classification: "low".to_string(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await
        .expect("create original asset");

    let blob_path = root
        .join("data")
        .join("workspaces")
        .join(ws)
        .join("assets")
        .join("original")
        .join(&content_hash);
    std::fs::create_dir_all(blob_path.parent().unwrap()).expect("mkdir blob dir");
    std::fs::write(&blob_path, bytes).expect("write original blob");
    (asset.asset_id, content_hash, blob_path)
}

// ---------------------------------------------------------------------------
// 1. Tier CRUD + ready rows + delete-tiers-leaves-original (negative proof).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_tier_rows_persist_and_delete_tiers_never_touches_original() {
    let pg = pg_or_skip!("tier_rows_persist");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    let original_bytes = vec![7u8; 4096];
    let (asset_id, original_hash, blob_path) =
        make_original_asset(&pg.db, tmp.path(), &ws, "image/png", &original_bytes).await;

    // Create a small derived thumb blob + asset so the thumb tier points at it.
    let thumb_bytes = vec![1u8; 64];
    let (thumb_asset_id, thumb_hash, _) =
        make_original_asset(&pg.db, tmp.path(), &ws, "image/png", &thumb_bytes).await;

    let ctx = WriteContext::human(None);
    pg.db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: asset_id.clone(),
                tier: MediaTier::Thumb,
                status: MediaTierStatus::Ready,
                tier_asset_id: Some(thumb_asset_id.clone()),
                content_hash: Some(thumb_hash.clone()),
                failure_reason: None,
            },
        )
        .await
        .expect("upsert thumb tier");
    pg.db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: asset_id.clone(),
                tier: MediaTier::Full,
                status: MediaTierStatus::Ready,
                tier_asset_id: Some(asset_id.clone()),
                content_hash: Some(original_hash.clone()),
                failure_reason: None,
            },
        )
        .await
        .expect("upsert full tier");

    let tiers = pg
        .db
        .list_media_tiers(&ws, &asset_id)
        .await
        .expect("list tiers");
    assert_eq!(tiers.len(), 2, "thumb + full tier rows persisted");
    assert!(tiers
        .iter()
        .all(|t| t.status == MediaTierStatus::Ready));

    // Negative proof: deleting derived tiers must NOT touch the original blob.
    let original_before = std::fs::read(&blob_path).expect("read original before");
    let deleted = pg
        .db
        .delete_media_tiers(&ctx, &ws, &asset_id)
        .await
        .expect("delete tiers");
    assert_eq!(deleted, 2, "both tier rows removed");
    assert!(
        pg.db
            .list_media_tiers(&ws, &asset_id)
            .await
            .expect("relist")
            .is_empty(),
        "tier rows gone after delete"
    );
    let original_after = std::fs::read(&blob_path).expect("read original after");
    assert_eq!(
        original_before, original_after,
        "ORIGINAL blob byte-identical after delete_media_tiers"
    );
    // And the original asset row still exists.
    let still = pg.db.get_asset(&ws, &asset_id).await.expect("original asset");
    assert_eq!(still.content_hash, original_hash);
}

// ---------------------------------------------------------------------------
// 2. Failed tier -> retry -> pending + attempt_count++ + visible retry queue.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_failed_tier_retry_bumps_attempt_count_and_is_in_failed_queue() {
    let pg = pg_or_skip!("failed_tier_retry");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    let (asset_id, _hash, _blob) =
        make_original_asset(&pg.db, tmp.path(), &ws, "video/mp4", &vec![9u8; 1024]).await;

    let ctx = WriteContext::human(None);
    // Honest video poster failure (no decoder bundled).
    pg.db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: asset_id.clone(),
                tier: MediaTier::Poster,
                status: MediaTierStatus::Failed,
                tier_asset_id: None,
                content_hash: None,
                failure_reason: Some("no_video_decoder_bundled".to_string()),
            },
        )
        .await
        .expect("upsert failed poster");

    // It is in the visible retry queue.
    let failed = pg
        .db
        .list_failed_media_tiers(&ws)
        .await
        .expect("list failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].tier, MediaTier::Poster);
    assert_eq!(failed[0].attempt_count, 0);

    // Retry: failed -> pending bumps attempt_count.
    let after = pg
        .db
        .set_media_tier_status(
            &ctx,
            &ws,
            &asset_id,
            MediaTier::Poster,
            MediaTierStatus::Pending,
            None,
        )
        .await
        .expect("retry");
    assert_eq!(after.status, MediaTierStatus::Pending);
    assert_eq!(after.attempt_count, 1, "retry bumped attempt_count");
    assert!(after.failure_reason.is_none(), "failure cleared on retry");

    // No longer in failed queue.
    assert!(pg
        .db
        .list_failed_media_tiers(&ws)
        .await
        .expect("relist failed")
        .is_empty());

    // A second failure then retry -> attempt_count == 2.
    pg.db
        .set_media_tier_status(
            &ctx,
            &ws,
            &asset_id,
            MediaTier::Poster,
            MediaTierStatus::Failed,
            Some("still_no_decoder".to_string()),
        )
        .await
        .expect("fail again");
    let again = pg
        .db
        .set_media_tier_status(
            &ctx,
            &ws,
            &asset_id,
            MediaTier::Poster,
            MediaTierStatus::Pending,
            None,
        )
        .await
        .expect("retry 2");
    assert_eq!(again.attempt_count, 2, "second retry bumped attempt_count");
}

// ---------------------------------------------------------------------------
// 3. Backend album/collection enumerates ordered members (GAP-LM-244a).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_collection_enumerates_ordered_members_from_backend() {
    let pg = pg_or_skip!("collection_ordered");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    let mut ids = Vec::new();
    for i in 0..4u8 {
        let (id, _h, _p) =
            make_original_asset(&pg.db, tmp.path(), &ws, "image/png", &vec![i; 128]).await;
        ids.push(id);
    }

    let ctx = WriteContext::human(None);
    let collection = pg
        .db
        .create_loom_collection(&ctx, &ws, Some("Album".to_string()))
        .await
        .expect("create collection");

    // Set order: reversed.
    let order: Vec<String> = ids.iter().rev().cloned().collect();
    let with_members = pg
        .db
        .set_loom_collection_order(&ctx, &ws, &collection.collection_id, &order)
        .await
        .expect("set order");
    let got: Vec<String> = with_members.members.iter().map(|m| m.asset_id.clone()).collect();
    assert_eq!(got, order, "members enumerated in the set order");
    // Positions densely 0..n.
    for (i, m) in with_members.members.iter().enumerate() {
        assert_eq!(m.position, i as i32);
    }

    // Re-fetch from backend (no in-memory cache) -> still ordered.
    let refetched = pg
        .db
        .get_loom_collection(&ws, &collection.collection_id)
        .await
        .expect("get collection");
    let refetched_ids: Vec<String> =
        refetched.members.iter().map(|m| m.asset_id.clone()).collect();
    assert_eq!(refetched_ids, order, "ordered enumeration is durable");

    // Reorder again (densification): drop one, swap two.
    let new_order = vec![ids[1].clone(), ids[3].clone(), ids[0].clone()];
    let reordered = pg
        .db
        .set_loom_collection_order(&ctx, &ws, &collection.collection_id, &new_order)
        .await
        .expect("reorder");
    let reordered_ids: Vec<String> =
        reordered.members.iter().map(|m| m.asset_id.clone()).collect();
    assert_eq!(reordered_ids, new_order);
    assert_eq!(reordered.members.len(), 3);
}

// ---------------------------------------------------------------------------
// 4. HTTP Range: 206 + Content-Range + Accept-Ranges; 416 unsatisfiable; and
//    content?tier=thumb serves the smaller derived blob, not the original.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_range_endpoint_and_tier_serving_over_http() {
    let pg = pg_or_skip!("range_endpoint");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    // Original is a 1000-byte ramp so a Range slice is byte-checkable.
    let original_bytes: Vec<u8> = (0..1000u32).map(|i| (i % 256) as u8).collect();
    let (asset_id, _hash, _blob) =
        make_original_asset(&pg.db, tmp.path(), &ws, "video/mp4", &original_bytes).await;

    // A small derived thumb blob + tier row pointing at it.
    let thumb_bytes = vec![5u8; 50];
    let (thumb_asset_id, thumb_hash, _) =
        make_original_asset(&pg.db, tmp.path(), &ws, "image/png", &thumb_bytes).await;
    let ctx = WriteContext::human(None);
    pg.db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: asset_id.clone(),
                tier: MediaTier::Thumb,
                status: MediaTierStatus::Ready,
                tier_asset_id: Some(thumb_asset_id.clone()),
                content_hash: Some(thumb_hash.clone()),
                failure_reason: None,
            },
        )
        .await
        .expect("upsert thumb tier");

    let (base, http, _state, _recorder) = loom_server(&pg).await;
    let content_url = format!("{base}/workspaces/{ws}/assets/{asset_id}/content");

    // (a) No Range -> 200 full + Accept-Ranges advertised.
    let resp = http.get(&content_url).send().await.expect("full send");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok()),
        Some("bytes"),
        "Accept-Ranges advertised on full response"
    );
    let full = resp.bytes().await.expect("full bytes");
    assert_eq!(full.len(), 1000, "full body is the whole file");

    // (b) Range bytes=100-199 -> 206 + correct Content-Range + correct slice.
    let resp = http
        .get(&content_url)
        .header("Range", "bytes=100-199")
        .send()
        .await
        .expect("range send");
    assert_eq!(resp.status(), 206, "partial content");
    assert_eq!(
        resp.headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some("bytes 100-199/1000"),
        "Content-Range matches the slice"
    );
    assert_eq!(
        resp.headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok()),
        Some("bytes")
    );
    let slice = resp.bytes().await.expect("range bytes");
    assert_eq!(slice.len(), 100);
    assert_eq!(&slice[..], &original_bytes[100..200], "slice is byte-exact");

    // (c) Open-ended suffix bytes=-10 -> last 10 bytes.
    let resp = http
        .get(&content_url)
        .header("Range", "bytes=-10")
        .send()
        .await
        .expect("suffix send");
    assert_eq!(resp.status(), 206);
    assert_eq!(
        resp.headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some("bytes 990-999/1000")
    );

    // (d) Unsatisfiable bytes=5000-6000 -> 416 + Content-Range "*".
    let resp = http
        .get(&content_url)
        .header("Range", "bytes=5000-6000")
        .send()
        .await
        .expect("unsat send");
    assert_eq!(resp.status(), 416, "range not satisfiable");
    assert_eq!(
        resp.headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some("bytes */1000")
    );

    // (e) tier=thumb serves the SMALLER derived blob, not the 1000-byte original.
    let resp = http
        .get(format!("{content_url}?tier=thumb"))
        .send()
        .await
        .expect("tier send");
    assert_eq!(resp.status(), 200);
    let thumb = resp.bytes().await.expect("thumb bytes");
    assert_eq!(thumb.len(), 50, "tier=thumb body is the derived thumb");
    assert!(
        thumb.len() < full.len(),
        "tier blob byte-len < original ({} < {})",
        thumb.len(),
        full.len()
    );

    // (f) tiers listing endpoint returns the thumb tier as ready.
    let resp = http
        .get(format!("{base}/workspaces/{ws}/assets/{asset_id}/tiers"))
        .send()
        .await
        .expect("tiers send");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("tiers json");
    let tiers = body["tiers"].as_array().expect("tiers array");
    assert!(tiers
        .iter()
        .any(|t| t["tier"] == "thumb" && t["status"] == "ready"));
}

/// Encode a real, decodable PNG of the given size so the generation job can
/// actually decode + downscale it (no synthetic bytes — the `image` crate
/// round-trips this).
fn real_png(width: u32, height: u32) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    });
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .expect("encode png");
    out
}

/// Create an `original` IMAGE asset (real PNG on disk) plus a loom file block
/// pointing at it, so the preview-generate job has a (block, asset) to operate
/// on. Returns (block_id, asset_id, content_hash).
async fn make_image_block(
    db: &PostgresDatabase,
    root: &std::path::Path,
    ws: &str,
) -> (String, String, String) {
    let bytes = real_png(800, 600);
    let (asset_id, content_hash, _path) =
        make_original_asset(db, root, ws, "image/png", &bytes).await;
    let ctx = WriteContext::human(None);
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some("media tiers fixture".to_string());
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.to_string(),
                content_type: LoomBlockContentType::File,
                document_id: None,
                asset_id: Some(asset_id.clone()),
                title: Some("Pyramid source".to_string()),
                original_filename: Some("media.png".to_string()),
                content_hash: Some(content_hash.clone()),
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived,
            },
        )
        .await
        .expect("create file block");
    (block.block_id, asset_id, content_hash)
}

/// Poll the AI job until it leaves the running states (bounded).
async fn wait_for_job_done(state: &AppState, job_id: &str) -> JobState {
    for _ in 0..200 {
        let job = state.storage.get_ai_job(job_id).await.expect("get job");
        if matches!(
            job.state,
            JobState::Completed
                | JobState::CompletedWithIssues
                | JobState::Failed
                | JobState::Poisoned
                | JobState::Cancelled
        ) {
            return job.state;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("job {job_id} did not finish within the bound");
}

// ---------------------------------------------------------------------------
// 5. The REAL background generation JOB produces the thumb+preview+full pyramid
//    and emits a LoomPreviewGenerated receipt carrying the per-tier dimension.
//    (acceptance #1: tier generation job real-PG proven + receipts.)
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_preview_generate_job_builds_pyramid_and_receipt_carries_tiers() {
    let pg = pg_or_skip!("preview_generate_job");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    let (block_id, asset_id, _hash) = make_image_block(&pg.db, tmp.path(), &ws).await;

    let (state, recorder) = loom_state(&pg).await;
    // Dispatch the REAL job through the production protocol (create_job +
    // start_workflow_for_job), exactly as import does — no direct storage write.
    let profile = state
        .capability_registry
        .profile_for_job_request(JobKind::LoomPreviewGenerate.as_str(), "hsk.loom.preview_generate@v1")
        .expect("profile");
    let job = handshake_core::jobs::create_job(
        &state,
        JobKind::LoomPreviewGenerate,
        "hsk.loom.preview_generate@v1",
        profile.id.as_str(),
        Some(serde_json::json!({
            "workspace_id": ws.clone(),
            "block_id": block_id.clone(),
            "asset_id": asset_id.clone(),
        })),
        Vec::new(),
    )
    .await
    .expect("create job");
    let job_id = job.job_id.to_string();
    handshake_core::workflows::start_workflow_for_job(&state, job)
        .await
        .expect("start workflow");

    let final_state = wait_for_job_done(&state, &job_id).await;
    assert_eq!(final_state, JobState::Completed, "preview job completed");

    // The pyramid is real: thumb + preview + full tier rows, all ready, each
    // pointing at a derived (or original) asset.
    let tiers = pg.db.list_media_tiers(&ws, &asset_id).await.expect("tiers");
    let mut by_tier: std::collections::HashMap<MediaTier, _> = std::collections::HashMap::new();
    for t in tiers {
        assert_eq!(t.status, MediaTierStatus::Ready, "tier {:?} ready", t.tier);
        assert!(t.tier_asset_id.is_some(), "tier {:?} has a blob asset", t.tier);
        by_tier.insert(t.tier, t);
    }
    assert!(by_tier.contains_key(&MediaTier::Thumb), "thumb tier produced");
    assert!(by_tier.contains_key(&MediaTier::Preview), "preview tier produced");
    assert!(by_tier.contains_key(&MediaTier::Full), "full tier produced");
    // full tier points back at the ORIGINAL asset (tiers derived; original auth).
    assert_eq!(
        by_tier[&MediaTier::Full].tier_asset_id.as_deref(),
        Some(asset_id.as_str()),
        "full tier resolves to the original asset"
    );
    // thumb/preview point at DERIVED assets, not the original.
    assert_ne!(by_tier[&MediaTier::Thumb].tier_asset_id.as_deref(), Some(asset_id.as_str()));
    assert_ne!(by_tier[&MediaTier::Preview].tier_asset_id.as_deref(), Some(asset_id.as_str()));

    // The receipt carries the tier dimension (thumb/preview/full), not a single
    // hardcoded slot.
    let events = recorder.captured();
    let receipt = events
        .iter()
        .find(|e| {
            e.event_type == handshake_core::flight_recorder::FlightRecorderEventType::LoomPreviewGenerated
        })
        .expect("LoomPreviewGenerated receipt emitted");
    let receipt_tiers = receipt.payload["tiers"]
        .as_array()
        .expect("receipt carries a tiers array");
    let names: std::collections::HashSet<&str> = receipt_tiers
        .iter()
        .filter_map(|t| t["tier"].as_str())
        .collect();
    assert!(names.contains("thumb"), "receipt tier=thumb");
    assert!(names.contains("preview"), "receipt tier=preview");
    assert!(names.contains("full"), "receipt tier=full");
    assert_eq!(
        receipt.payload["asset_id"].as_str(),
        Some(asset_id.as_str()),
        "receipt asset_id is the original source asset"
    );
}

// ---------------------------------------------------------------------------
// 6. End-to-end HTTP retry: a FAILED tier -> POST /tiers/:tier/retry flips it to
//    pending, bumps attempt_count, and requeues a real generation job. Then the
//    tiers list shows the tier no longer failed. (acceptance #3 over HTTP.)
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt259_http_retry_endpoint_requeues_failed_tier() {
    let pg = pg_or_skip!("http_retry");
    let ws = pg.create_workspace().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", tmp.path());

    // A real image block so find_loom_block_by_asset_id resolves (the retry
    // endpoint requeues the job keyed by the owning block).
    let (_block_id, asset_id, _hash) = make_image_block(&pg.db, tmp.path(), &ws).await;

    // Seed a FAILED poster tier (the honest video-poster failure shape).
    let ctx = WriteContext::human(None);
    pg.db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: asset_id.clone(),
                tier: MediaTier::Poster,
                status: MediaTierStatus::Failed,
                tier_asset_id: None,
                content_hash: None,
                failure_reason: Some("no_video_decoder_bundled".to_string()),
            },
        )
        .await
        .expect("seed failed poster");

    let (base, http, _state, _recorder) = loom_server(&pg).await;

    // It is in the failed queue before retry.
    assert_eq!(
        pg.db.list_failed_media_tiers(&ws).await.expect("failed").len(),
        1
    );

    // POST the retry endpoint.
    let resp = http
        .post(format!("{base}/workspaces/{ws}/assets/{asset_id}/tiers/poster/retry"))
        .send()
        .await
        .expect("retry send");
    assert_eq!(resp.status(), 200, "retry accepted");
    let body: serde_json::Value = resp.json().await.expect("retry json");
    assert_eq!(body["tier"], "poster");
    assert_eq!(body["status"], "pending", "tier flipped to pending");
    assert_eq!(body["attempt_count"], 1, "attempt_count bumped on retry");
    assert_eq!(body["requeued"], true, "a real job was requeued");

    // No longer in the failed queue (it is pending / requeued).
    assert!(pg
        .db
        .list_failed_media_tiers(&ws)
        .await
        .expect("relist failed")
        .is_empty());

    // The persisted row reflects pending + attempt_count == 1.
    let row = pg
        .db
        .get_media_tier(&ws, &asset_id, MediaTier::Poster)
        .await
        .expect("get tier")
        .expect("tier row");
    assert_eq!(row.status, MediaTierStatus::Pending);
    assert_eq!(row.attempt_count, 1);
}
