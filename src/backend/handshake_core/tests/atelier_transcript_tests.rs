//! WP-KERNEL-005 atelier transcript/caption pipeline: live PostgreSQL
//! round-trip proofs for the governed-record + receipt model in
//! `handshake_core::atelier::transcript` (MT-203). Run with a live
//! DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_transcript_tests -- --nocapture
//!
//! No mocks: each test connects the real `AtelierStore` to a live Postgres,
//! ensures the schema, and exercises the probe -> transcript -> caption ->
//! receipt lineage with REAL data. This module has NO character FK; the lineage
//! anchor is the `sha256:<64hex>` `source_media_hash`. Tables persist between
//! runs, so every hash / job_id / artifact_ref is made unique per run via
//! `Uuid::new_v4()` to avoid colliding on the UNIQUE / ON CONFLICT keys. Only
//! `handshake_core` + `tokio` + `uuid` + `serde_json` (+ std) are used; sqlx is
//! never imported directly.

use chrono::Utc;
use handshake_core::atelier::transcript::{
    transcript_event_family, CaptionFormat, CaptionRenderRequest, MediaProbeReport,
    NewCaptionArtifact, NewMediaProbeReport, NewPipelineReceipt, NewTranscriptArtifact,
    ReceiptKind, ReceiptStatus,
};
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
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

/// A fresh, run-unique canonical `sha256:<64hex>` lineage key. The 64 hex chars
/// are derived from two simple-form UUIDs so the shape passes the module's
/// `validate_source_media_hash` and never collides across runs.
fn fresh_source_hash() -> String {
    let a = Uuid::new_v4().simple().to_string(); // 32 hex chars
    let b = Uuid::new_v4().simple().to_string(); // 32 hex chars
    format!("sha256:{a}{b}")
}

/// Persist a run-unique probe report for `hash` and return it. This is the
/// lineage root every transcript must bind to (6.11.6 LAW).
async fn fresh_probe(store: &AtelierStore, hash: &str) -> MediaProbeReport {
    store
        .record_media_probe(&NewMediaProbeReport {
            media_source_id: format!("PRIM-MediaSource:{}", Uuid::new_v4()),
            source_media_hash: hash.to_string(),
            container: "mp4".to_string(),
            duration_ms: 12_000,
            streams: serde_json::json!([
                { "index": 0, "kind": "audio", "codec": "aac", "sample_rate_hz": 48000 }
            ]),
            ffprobe_tool_version: "ffprobe 6.1".to_string(),
            artifact_ref: format!("artifact://atelier/probe/{}", Uuid::new_v4()),
            probed_at: Utc::now(),
        })
        .await
        .expect("record media probe")
}

#[tokio::test]
async fn atelier_transcript_rejects_legacy_runtime_artifact_refs() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_transcript_rejects_legacy_runtime_artifact_refs: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let hash = fresh_source_hash();

    let err = store
        .record_media_probe(&NewMediaProbeReport {
            media_source_id: format!("PRIM-MediaSource:{}", Uuid::new_v4()),
            source_media_hash: hash,
            container: "mp4".to_string(),
            duration_ms: 12_000,
            streams: serde_json::json!([
                { "index": 0, "kind": "audio", "codec": "aac", "sample_rate_hz": 48000 }
            ]),
            ffprobe_tool_version: "ffprobe 6.1".to_string(),
            artifact_ref: "file:///tmp/probe-report.json".to_string(),
            probed_at: Utc::now(),
        })
        .await
        .expect_err("file artifact refs are forbidden");
    assert!(
        err.to_string().contains("Handshake-native portable ref"),
        "unexpected transcript probe error: {err}"
    );
}

#[tokio::test]
async fn atelier_transcript_rejects_legacy_runtime_receipt_and_mux_refs() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_transcript_rejects_legacy_runtime_receipt_and_mux_refs: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let hash = fresh_source_hash();
    let probe = fresh_probe(&store, &hash).await;
    let transcript = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: probe.media_source_id.clone(),
            source_media_hash: hash.clone(),
            language: "en".to_string(),
            model: serde_json::json!({ "family": "whisper" }),
            selection_path: "gpu_happy".to_string(),
            segments: serde_json::json!([]),
            timing_anchors: serde_json::json!([]),
            artifact_ref: format!("artifact://atelier/transcript/{}", Uuid::new_v4()),
        })
        .await
        .expect("record transcript for mux guard");

    let mux_err = store
        .record_caption(&NewCaptionArtifact {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Srt,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 800,
            max_cue_ms: 7000,
            cue_count: 0,
            artifact_ref: format!("artifact://atelier/caption/{}", Uuid::new_v4()),
            muxed_media_artifact_id: Some("file:///tmp/muxed.mp4".to_string()),
        })
        .await
        .expect_err("machine-local muxed media refs are forbidden");
    assert!(
        mux_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected mux error: {mux_err}"
    );

    let receipt_err = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::Transcribe,
            job_id: format!("job-receipt-ref-{}", Uuid::new_v4()),
            source_media_hash: hash.clone(),
            input_artifact_ids: serde_json::json!(["http://localhost:9000/in.wav"]),
            output_artifact_id: Some("artifact://atelier/transcript/out".to_string()),
            capability_grants: serde_json::json!([]),
            tool_versions: serde_json::json!({}),
            status: ReceiptStatus::Completed,
            error_class: None,
            partial_artifact_id: None,
            emitted_at: Utc::now(),
        })
        .await
        .expect_err("localhost input artifact refs are forbidden");
    assert!(
        receipt_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected receipt input error: {receipt_err}"
    );

    let receipt_err = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::Transcribe,
            job_id: format!("job-receipt-ref-{}", Uuid::new_v4()),
            source_media_hash: hash.clone(),
            input_artifact_ids: serde_json::json!(["artifact://atelier/transcript/in"]),
            output_artifact_id: Some(".GOV/out.json".to_string()),
            capability_grants: serde_json::json!([]),
            tool_versions: serde_json::json!({}),
            status: ReceiptStatus::Completed,
            error_class: None,
            partial_artifact_id: None,
            emitted_at: Utc::now(),
        })
        .await
        .expect_err(".GOV output artifact refs are forbidden");
    assert!(
        receipt_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected receipt output error: {receipt_err}"
    );

    let receipt_err = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::CaptionRender,
            job_id: format!("job-receipt-ref-{}", Uuid::new_v4()),
            source_media_hash: hash,
            input_artifact_ids: serde_json::json!(["artifact://atelier/transcript/in"]),
            output_artifact_id: None,
            capability_grants: serde_json::json!([]),
            tool_versions: serde_json::json!({}),
            status: ReceiptStatus::Failed,
            error_class: Some("render_error".to_string()),
            partial_artifact_id: Some(".GOV/partial.vtt".to_string()),
            emitted_at: Utc::now(),
        })
        .await
        .expect_err(".GOV partial artifact refs are forbidden");
    assert!(
        receipt_err
            .to_string()
            .contains("Handshake-native portable ref"),
        "unexpected receipt partial error: {receipt_err}"
    );
}

#[tokio::test]
async fn atelier_probe_idempotent_and_transcript_lineage_roundtrip() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_probe_idempotent_and_transcript_lineage_roundtrip: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let probe_before = store
        .count_events(transcript_event_family::MEDIA_PROBE_RECORDED)
        .await
        .expect("count probe events before");
    let transcript_before = store
        .count_events(transcript_event_family::TRANSCRIPT_RECORDED)
        .await
        .expect("count transcript events before");

    let hash = fresh_source_hash();

    // --- probe is idempotent on source_media_hash (re-probe returns same id) ---
    let probe = fresh_probe(&store, &hash).await;
    assert_eq!(
        probe.source_media_hash, hash,
        "probe stores the lineage key"
    );
    assert_eq!(probe.container, "mp4");
    assert_eq!(probe.duration_ms, 12_000);

    let probe_again = store
        .record_media_probe(&NewMediaProbeReport {
            media_source_id: format!("PRIM-MediaSource:{}", Uuid::new_v4()),
            source_media_hash: hash.clone(),
            container: "mkv".to_string(),
            duration_ms: 99_999,
            streams: serde_json::json!([]),
            ffprobe_tool_version: "ffprobe 7.0".to_string(),
            artifact_ref: format!("artifact://atelier/probe/{}", Uuid::new_v4()),
            probed_at: Utc::now(),
        })
        .await
        .expect("re-probe same source bytes");
    assert_eq!(
        probe.probe_report_id, probe_again.probe_report_id,
        "re-probing the same source_media_hash returns the existing report (no lineage fork)"
    );

    // --- fetch by lineage hash round-trips ---
    let fetched = store
        .get_media_probe_by_hash(&hash)
        .await
        .expect("get probe by hash")
        .expect("probe present");
    assert_eq!(fetched.probe_report_id, probe.probe_report_id);

    // --- transcript binds to the existing probe and round-trips ---
    let t_ref = format!("artifact://atelier/transcript/{}", Uuid::new_v4());
    let transcript = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: probe.media_source_id.clone(),
            source_media_hash: hash.clone(),
            language: "en".to_string(),
            model: serde_json::json!({
                "family": "whisper", "variant": "large-v3",
                "runtime": "ct2", "precision": "fp16"
            }),
            selection_path: "gpu_happy".to_string(),
            segments: serde_json::json!([
                { "segment_id": "s0", "start_ms": 0, "end_ms": 1500,
                  "text": "hello", "confidence": 0.92, "speaker": "A", "source": "asr" }
            ]),
            timing_anchors: serde_json::json!([
                { "anchor_id": "a0", "t_ms": 0, "segment_id": "s0", "kind": "start" }
            ]),
            artifact_ref: t_ref.clone(),
        })
        .await
        .expect("record transcript bound to probe");
    assert_eq!(
        transcript.source_media_hash, hash,
        "transcript inherits the lineage hash from its probe"
    );
    assert_eq!(transcript.format_version, "TranscriptArtifactV1");
    assert_eq!(transcript.selection_path, "gpu_happy");

    let got = store
        .get_transcript(transcript.transcript_id)
        .await
        .expect("get transcript");
    assert_eq!(got.transcript_id, transcript.transcript_id);
    assert!(got.segments.is_array(), "segments stored as JSON array");

    // --- transcript is idempotent on artifact_ref (stable id) ---
    let transcript_dup = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: probe.media_source_id.clone(),
            source_media_hash: hash.clone(),
            language: "de".to_string(),
            model: serde_json::json!({ "family": "whisper" }),
            selection_path: "cpu_only".to_string(),
            segments: serde_json::json!([]),
            timing_anchors: serde_json::json!([]),
            artifact_ref: t_ref.clone(),
        })
        .await
        .expect("re-record transcript with same artifact_ref");
    assert_eq!(
        transcript.transcript_id, transcript_dup.transcript_id,
        "re-recording the same artifact_ref returns the existing transcript (ON CONFLICT stable)"
    );

    // --- INVARIANT: lineage LAW (6.11.6) rejects a transcript with no probe ---
    let orphan_hash = fresh_source_hash();
    let orphan = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: format!("PRIM-MediaSource:{}", Uuid::new_v4()),
            source_media_hash: orphan_hash,
            language: "en".to_string(),
            model: serde_json::json!({}),
            selection_path: "gpu_happy".to_string(),
            segments: serde_json::json!([]),
            timing_anchors: serde_json::json!([]),
            artifact_ref: format!("artifact://atelier/transcript/{}", Uuid::new_v4()),
        })
        .await;
    assert!(
        orphan.is_err(),
        "a transcript whose hash has no matching probe report is a lineage break and must be rejected"
    );

    // --- events emitted for the persisted probe + transcript ---
    let probe_after = store
        .count_events(transcript_event_family::MEDIA_PROBE_RECORDED)
        .await
        .expect("count probe events after");
    let transcript_after = store
        .count_events(transcript_event_family::TRANSCRIPT_RECORDED)
        .await
        .expect("count transcript events after");
    assert!(
        probe_after > probe_before,
        "recording a probe emits a MEDIA_PROBE_RECORDED event"
    );
    assert!(
        transcript_after > transcript_before,
        "recording a transcript emits a TRANSCRIPT_RECORDED event"
    );
}

#[tokio::test]
async fn atelier_caption_inherits_hash_and_is_idempotent() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_caption_inherits_hash_and_is_idempotent: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let caption_before = store
        .count_events(transcript_event_family::CAPTION_RECORDED)
        .await
        .expect("count caption events before");

    // Build the lineage root: probe -> transcript.
    let hash = fresh_source_hash();
    let probe = fresh_probe(&store, &hash).await;
    let transcript = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: probe.media_source_id.clone(),
            source_media_hash: hash.clone(),
            language: "en".to_string(),
            model: serde_json::json!({ "family": "whisper", "variant": "large-v3" }),
            selection_path: "gpu_happy".to_string(),
            segments: serde_json::json!([
                { "segment_id": "s0", "start_ms": 0, "end_ms": 2000, "text": "line one" }
            ]),
            timing_anchors: serde_json::json!([
                { "anchor_id": "a0", "t_ms": 0, "segment_id": "s0", "kind": "start" }
            ]),
            artifact_ref: format!("artifact://atelier/transcript/{}", Uuid::new_v4()),
        })
        .await
        .expect("record transcript");

    // --- caption derives from the transcript and INHERITS its lineage hash ---
    let cap_ref = format!("artifact://atelier/caption/{}", Uuid::new_v4());
    let caption = store
        .record_caption(&NewCaptionArtifact {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Srt,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 800,
            max_cue_ms: 7000,
            cue_count: 1,
            artifact_ref: cap_ref.clone(),
            muxed_media_artifact_id: None,
        })
        .await
        .expect("record caption derived from transcript");
    assert_eq!(
        caption.source_media_hash, hash,
        "caption inherits the parent transcript's source_media_hash (lineage binding)"
    );
    assert_eq!(
        caption.transcript_id, transcript.transcript_id,
        "caption is bound to its parent transcript"
    );
    assert_eq!(
        caption.format,
        CaptionFormat::Srt,
        "caption format round-trips"
    );
    assert_eq!(caption.cue_count, 1);
    assert!(
        caption.derived_from_timing_anchors,
        "caption records it was derived from timing anchors"
    );

    // --- caption is idempotent on artifact_ref (stable id, no duplicate) ---
    let caption_dup = store
        .record_caption(&NewCaptionArtifact {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Vtt, // ignored on conflict; id stays stable
            language: "de".to_string(),
            max_line_chars: 80,
            max_lines_per_cue: 3,
            min_cue_ms: 100,
            max_cue_ms: 9000,
            cue_count: 99,
            artifact_ref: cap_ref.clone(),
            muxed_media_artifact_id: None,
        })
        .await
        .expect("re-record caption with same artifact_ref");
    assert_eq!(
        caption.caption_artifact_id, caption_dup.caption_artifact_id,
        "re-recording the same caption artifact_ref returns the existing row (ON CONFLICT stable)"
    );

    // --- listing returns exactly one caption (no duplicate from the upsert) ---
    let listed = store
        .list_captions_for_transcript(transcript.transcript_id)
        .await
        .expect("list captions for transcript");
    let matches = listed
        .iter()
        .filter(|c| c.caption_artifact_id == caption.caption_artifact_id)
        .count();
    assert_eq!(matches, 1, "the upsert did not duplicate the caption row");

    // --- event emitted for the persisted caption ---
    let caption_after = store
        .count_events(transcript_event_family::CAPTION_RECORDED)
        .await
        .expect("count caption events after");
    assert!(
        caption_after > caption_before,
        "recording a caption emits a CAPTION_RECORDED event"
    );
}

#[tokio::test]
async fn atelier_caption_render_produces_deterministic_sidecars_and_receipt() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_caption_render_produces_deterministic_sidecars_and_receipt: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let hash = fresh_source_hash();
    let probe = fresh_probe(&store, &hash).await;
    let transcript = store
        .record_transcript(&NewTranscriptArtifact {
            media_source_id: probe.media_source_id.clone(),
            source_media_hash: hash.clone(),
            language: "en".to_string(),
            model: serde_json::json!({ "family": "whisper", "variant": "large-v3" }),
            selection_path: "gpu_happy".to_string(),
            segments: serde_json::json!([
                { "segment_id": "s0", "start_ms": 0, "end_ms": 1500, "text": "Hello world." },
                { "segment_id": "s1", "start_ms": 1500, "end_ms": 3250, "text": "Second cue" }
            ]),
            timing_anchors: serde_json::json!([
                { "anchor_id": "a0", "t_ms": 0, "segment_id": "s0", "kind": "start" },
                { "anchor_id": "a1", "t_ms": 1500, "segment_id": "s1", "kind": "start" }
            ]),
            artifact_ref: format!("artifact://atelier/transcript/{}", Uuid::new_v4()),
        })
        .await
        .expect("record transcript");

    let rendered_srt = store
        .render_caption(&CaptionRenderRequest {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Srt,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 500,
            max_cue_ms: 7000,
            muxed_media_artifact_id: None,
        })
        .await
        .expect("render srt caption");
    assert_eq!(rendered_srt.caption.format, CaptionFormat::Srt);
    assert_eq!(rendered_srt.caption.cue_count, 2);
    assert_eq!(rendered_srt.receipt.kind, ReceiptKind::CaptionRender);
    assert_eq!(rendered_srt.receipt.status, ReceiptStatus::Completed);
    assert_eq!(
        rendered_srt.receipt.output_artifact_id.as_deref(),
        Some(rendered_srt.caption.artifact_ref.as_str())
    );
    assert_eq!(
        rendered_srt.sidecar_text,
        "1\n00:00:00,000 --> 00:00:01,500\nHello world.\n\n2\n00:00:01,500 --> 00:00:03,250\nSecond cue\n"
    );
    assert!(
        rendered_srt
            .caption
            .artifact_ref
            .starts_with("artifact://atelier/caption/srt/sha256:"),
        "rendered caption artifact_ref must be content-addressed"
    );

    let rendered_srt_again = store
        .render_caption(&CaptionRenderRequest {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Srt,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 500,
            max_cue_ms: 7000,
            muxed_media_artifact_id: None,
        })
        .await
        .expect("render srt caption again");
    assert_eq!(
        rendered_srt.caption.caption_artifact_id, rendered_srt_again.caption.caption_artifact_id,
        "rendering the same transcript/profile is idempotent on sidecar content hash"
    );
    assert_eq!(
        rendered_srt.sidecar_sha256, rendered_srt_again.sidecar_sha256,
        "same transcript/profile produces byte-identical sidecar text"
    );

    let rendered_vtt = store
        .render_caption(&CaptionRenderRequest {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Vtt,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 500,
            max_cue_ms: 7000,
            muxed_media_artifact_id: None,
        })
        .await
        .expect("render vtt caption");
    assert!(rendered_vtt.sidecar_text.starts_with("WEBVTT\n\n"));
    assert!(rendered_vtt
        .sidecar_text
        .contains("00:00:01.500 --> 00:00:03.250"));

    let rendered_ass = store
        .render_caption(&CaptionRenderRequest {
            transcript_id: transcript.transcript_id,
            format: CaptionFormat::Ass,
            language: "en".to_string(),
            max_line_chars: 42,
            max_lines_per_cue: 2,
            min_cue_ms: 500,
            max_cue_ms: 7000,
            muxed_media_artifact_id: None,
        })
        .await
        .expect("render ass caption");
    assert!(rendered_ass.sidecar_text.contains("[Events]"));
    assert!(rendered_ass
        .sidecar_text
        .contains("Dialogue: 0,0:00:00.00,0:00:01.50"));
}

#[tokio::test]
async fn atelier_receipt_redaction_idempotency_and_partial_preservation() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_receipt_redaction_idempotency_and_partial_preservation: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let receipt_before = store
        .count_events(transcript_event_family::RECEIPT_FILED)
        .await
        .expect("count receipt events before");

    let hash = fresh_source_hash();
    let job_id = format!("job-{}", Uuid::new_v4());

    // --- a Completed receipt with secret-bearing tool_versions/capability_grants ---
    let receipt = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::Transcribe,
            job_id: job_id.clone(),
            source_media_hash: hash.clone(),
            input_artifact_ids: serde_json::json!(["artifact://in/0"]),
            output_artifact_id: Some("artifact://out/0".to_string()),
            capability_grants: serde_json::json!({
                "gpu": "cuda:0",
                "api_key": "should-not-persist",
                "nested": { "session": "secret-sid" }
            }),
            tool_versions: serde_json::json!({
                "ffmpeg": "6.1",
                "whisper": "1.5.0",
                "auth_token": "super-secret"
            }),
            status: ReceiptStatus::Completed,
            error_class: None,
            partial_artifact_id: None,
            emitted_at: Utc::now(),
        })
        .await
        .expect("file completed receipt");
    assert_eq!(receipt.kind, ReceiptKind::Transcribe);
    assert_eq!(receipt.status, ReceiptStatus::Completed);
    assert_eq!(
        receipt.feature_id, "FEAT-ASR",
        "receipt is pinned to FEAT-ASR"
    );
    assert_eq!(
        receipt.output_artifact_id.as_deref(),
        Some("artifact://out/0")
    );

    // --- INVARIANT: secret redaction in stored tool_versions / capability_grants ---
    assert_eq!(
        receipt.tool_versions["ffmpeg"], "6.1",
        "non-secret tool versions are preserved"
    );
    assert_eq!(
        receipt.tool_versions["auth_token"], "[REDACTED]",
        "credential-bearing tool_versions key is redacted before persistence (6.11.8)"
    );
    assert_eq!(
        receipt.capability_grants["api_key"], "[REDACTED]",
        "credential-bearing capability_grants key is redacted"
    );
    assert_eq!(
        receipt.capability_grants["nested"]["session"], "[REDACTED]",
        "nested credential-bearing key is recursively redacted"
    );
    assert_eq!(
        receipt.capability_grants["gpu"], "cuda:0",
        "non-secret capability grants are preserved"
    );

    // --- receipt is idempotent on job_id (re-file returns same row) ---
    let receipt_again = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::Transcribe,
            job_id: job_id.clone(),
            source_media_hash: hash.clone(),
            input_artifact_ids: serde_json::json!([]),
            output_artifact_id: Some("artifact://out/different".to_string()),
            capability_grants: serde_json::json!({}),
            tool_versions: serde_json::json!({}),
            status: ReceiptStatus::Completed,
            error_class: None,
            partial_artifact_id: None,
            emitted_at: Utc::now(),
        })
        .await
        .expect("re-file same job's receipt");
    assert_eq!(
        receipt.receipt_id, receipt_again.receipt_id,
        "re-filing the same job_id returns the existing receipt (idempotent)"
    );

    let by_job = store
        .get_receipt_by_job(&job_id)
        .await
        .expect("get receipt by job")
        .expect("receipt present");
    assert_eq!(by_job.receipt_id, receipt.receipt_id);

    // --- INVARIANT: a Failed receipt preserves its partial_artifact_id (6.11.7) ---
    let fail_job = format!("job-fail-{}", Uuid::new_v4());
    let failed = store
        .file_pipeline_receipt(&NewPipelineReceipt {
            kind: ReceiptKind::CaptionRender,
            job_id: fail_job.clone(),
            source_media_hash: hash.clone(),
            input_artifact_ids: serde_json::json!(["artifact://in/1"]),
            output_artifact_id: None,
            capability_grants: serde_json::json!({}),
            tool_versions: serde_json::json!({ "ffmpeg": "6.1" }),
            status: ReceiptStatus::Failed,
            error_class: Some("E_CAPTION_TIMING".to_string()),
            partial_artifact_id: Some("artifact://partial/0".to_string()),
            emitted_at: Utc::now(),
        })
        .await
        .expect("file failed receipt with partial result");
    assert_eq!(failed.status, ReceiptStatus::Failed);
    assert_eq!(
        failed.error_class.as_deref(),
        Some("E_CAPTION_TIMING"),
        "failed receipt carries a typed error_class"
    );
    assert_eq!(
        failed.partial_artifact_id.as_deref(),
        Some("artifact://partial/0"),
        "failed receipt preserves its partial_artifact_id (partial results not discarded)"
    );

    // --- both receipts are listed against the shared lineage hash ---
    let trail = store
        .list_receipts_for_source(&hash)
        .await
        .expect("list receipts for source");
    assert!(
        trail.iter().any(|r| r.receipt_id == receipt.receipt_id),
        "the completed receipt is in the source's evidence trail"
    );
    assert!(
        trail.iter().any(|r| r.receipt_id == failed.receipt_id),
        "the failed receipt is in the source's evidence trail"
    );

    // --- events emitted for both filed receipts ---
    let receipt_after = store
        .count_events(transcript_event_family::RECEIPT_FILED)
        .await
        .expect("count receipt events after");
    assert!(
        receipt_after >= receipt_before + 2,
        "filing receipts emits RECEIPT_FILED events"
    );
}
