//! Media transcript + caption governed records (WP-KERNEL-005, MT-203).
//!
//! Spec authority: master-spec-v02.189 / 06-mechanical-integrations.md
//! Section 6.11 "Media Transcript and Caption Pipeline" (Normative, [ADD
//! v02.189]). This module persists the GOVERNED DATA + RECEIPT model only:
//! `MediaProbeReportV1` (6.11.3), `TranscriptArtifactV1` (6.11.4, segments +
//! timing anchors), `CaptionArtifactV1` (6.11.5), and the three typed receipts
//! `MediaProbeReceiptV1` / `TranscribeReceiptV1` / `CaptionRenderReceiptV1`
//! (6.11.10). It records the lineage chain
//! `PRIM-MediaSource -> MediaProbeReportV1 -> TranscriptArtifactV1 ->
//! CaptionArtifactV1` bound at every hop by a shared `source_media_hash`
//! (6.11.6 LAW: lineage chain); a hop whose hash does not match its upstream is
//! rejected with a typed validation error rather than persisted.
//!
//! legacy source source (intent only): legacy source `app backend ASR/ffmpeg` flow.
//! Handshake forbids the legacy source SQLite/Electron/localhost realization; only the
//! intent (probe -> extract -> transcribe -> caption, hash-bound lineage,
//! recoverable receipts) is carried across. Storage authority is the single
//! Handshake store + EventLedger + ArtifactStore (6.11.2 LAW: storage
//! authority). Persistence uses the embedded Surreal store owned by
//! [`AtelierStore`] (MT-138).
//!
//! HARD boundary: ffmpeg / ffprobe / Whisper run as governed Workflow-Engine
//! jobs ELSEWHERE (6.11.1 LAW: governed-job-only execution). This module NEVER
//! spawns a process, opens a socket, or calls an external endpoint. It only
//! stores the records a job writes through and emits EventLedger events. All
//! tool command lines / fetch contexts are redacted before persistence
//! (6.11.8 LAW: secret + log hygiene): secrets, cookies, tokens, and
//! credentials never appear in a stored record or an event payload.
//!
//! Microtasks: MT-203 (transcript + caption governed records), MT-005 (event
//! coverage), MT-004 (single-store-only authority).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

/// Transcript/caption job-lifecycle event families (MT-203, MT-005).
///
/// These mirror the Section 6.11.7 EventLedger families at the data-record
/// seam: every governed record this module persists emits the matching
/// `*.completed`-class family so the operator surface, Locus, and Flight
/// Recorder replay can reconstruct the probe -> transcribe -> caption pipeline.
/// The parent folds these into [`super::event_family::ALL`] for MT-005 coverage.
pub mod transcript_event_family {
    /// A `media.probe` report record was persisted (6.11.3 / 6.11.7).
    pub const MEDIA_PROBE_RECORDED: &str = "atelier.media_probe.recorded";
    /// A canonical `TranscriptArtifactV1` record was persisted (6.11.4).
    pub const TRANSCRIPT_RECORDED: &str = "atelier.transcript.recorded";
    /// A `CaptionArtifactV1` record was persisted (6.11.5).
    pub const CAPTION_RECORDED: &str = "atelier.caption.recorded";
    /// A typed pipeline receipt was filed (6.11.10): probe / transcribe /
    /// caption, success or typed-failure.
    pub const RECEIPT_FILED: &str = "atelier.transcript.receipt_filed";

    /// All transcript/caption event families, exported for parity/coverage
    /// proofs (mirrors the `event_family::ALL` shape used elsewhere).
    pub const ALL: &[&str] = &[
        MEDIA_PROBE_RECORDED,
        TRANSCRIPT_RECORDED,
        CAPTION_RECORDED,
        RECEIPT_FILED,
    ];
}

/// Re-export at module root so callers can write `transcript::TRANSCRIPT_RECORDED`.
pub use transcript_event_family::{
    CAPTION_RECORDED, MEDIA_PROBE_RECORDED, RECEIPT_FILED, TRANSCRIPT_RECORDED,
};

/// Caption sidecar format (6.11.5 `format: srt | vtt | ass`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionFormat {
    Srt,
    Vtt,
    Ass,
}

impl CaptionFormat {
    /// Stable lowercase DB token (also the spec `format` value).
    pub fn as_token(self) -> &'static str {
        match self {
            CaptionFormat::Srt => "srt",
            CaptionFormat::Vtt => "vtt",
            CaptionFormat::Ass => "ass",
        }
    }

    /// Parse a stored token. Unknown tokens are a validation error rather than
    /// a silent default, so a corrupt row never masquerades as a valid format.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "srt" => Ok(CaptionFormat::Srt),
            "vtt" => Ok(CaptionFormat::Vtt),
            "ass" => Ok(CaptionFormat::Ass),
            other => Err(AtelierError::Validation(format!(
                "unknown caption format token: {other}"
            ))),
        }
    }
}

/// The governed job kind a receipt attests to (6.11.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    /// `MediaProbeReceiptV1` for a `media.probe` job.
    MediaProbe,
    /// `TranscribeReceiptV1` for an `asr.transcribe` job.
    Transcribe,
    /// `CaptionRenderReceiptV1` for a `caption.render` job.
    CaptionRender,
}

impl ReceiptKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ReceiptKind::MediaProbe => "media_probe",
            ReceiptKind::Transcribe => "transcribe",
            ReceiptKind::CaptionRender => "caption_render",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "media_probe" => Ok(ReceiptKind::MediaProbe),
            "transcribe" => Ok(ReceiptKind::Transcribe),
            "caption_render" => Ok(ReceiptKind::CaptionRender),
            other => Err(AtelierError::Validation(format!(
                "unknown receipt kind token: {other}"
            ))),
        }
    }
}

/// Terminal status a receipt attests (6.11.7 / 6.11.10). `Completed` carries an
/// output artifact id; `Failed` carries a typed `error_class` and preserves any
/// partial-result artifact id (6.11.7: partial results preserved on failure).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Completed,
    Failed,
}

impl ReceiptStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ReceiptStatus::Completed => "completed",
            ReceiptStatus::Failed => "failed",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "completed" => Ok(ReceiptStatus::Completed),
            "failed" => Ok(ReceiptStatus::Failed),
            other => Err(AtelierError::Validation(format!(
                "unknown receipt status token: {other}"
            ))),
        }
    }
}

/// A `MediaProbeReportV1` record (6.11.3). `source_media_hash` is the lineage
/// key that binds every downstream transcript and caption to its exact input.
/// `streams` is the ffprobe-derived stream facts JSON array. The actual ffprobe
/// invocation ran in a governed `media.probe` job; this is the persisted result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaProbeReport {
    pub probe_report_id: Uuid,
    /// `PRIM-MediaSource` reference (Section 6.2.3.3); free-form portable ref.
    pub media_source_id: String,
    /// `sha256:<hex>` lineage key over the source bytes (6.11.6).
    pub source_media_hash: String,
    pub container: String,
    pub duration_ms: i64,
    /// ffprobe stream facts: `[{index, kind, codec, sample_rate_hz, ...}]`.
    pub streams: serde_json::Value,
    /// ffprobe tool version recorded for reproducibility.
    pub ffprobe_tool_version: String,
    /// ArtifactStore ref for the materialized probe-report artifact.
    pub artifact_ref: String,
    pub probed_at: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to persist a probe report (written by a `media.probe` job).
#[derive(Clone, Debug)]
pub struct NewMediaProbeReport {
    pub media_source_id: String,
    pub source_media_hash: String,
    pub container: String,
    pub duration_ms: i64,
    pub streams: serde_json::Value,
    pub ffprobe_tool_version: String,
    pub artifact_ref: String,
    pub probed_at: DateTime<Utc>,
}

/// A canonical `TranscriptArtifactV1` record (6.11.4). `segments` and
/// `timing_anchors` are stored as JSONB so transcript positions are
/// independently addressable for Loom/Lens time-span bridging without
/// re-deriving timing. `model` + `selection_path` are reproducibility metadata
/// (Section 6.2.2.4.5). Bound to its probe report by `source_media_hash`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptArtifact {
    pub transcript_id: Uuid,
    pub media_source_id: String,
    /// Lineage key; MUST equal the upstream probe report hash (6.11.6).
    pub source_media_hash: String,
    pub language: String,
    /// `{family, variant, runtime, precision}` reproducibility metadata.
    pub model: serde_json::Value,
    /// `gpu_happy | gpu_constrained | cpu_only | ...` (Section 6.2.2.4.5).
    pub selection_path: String,
    /// `[{segment_id, start_ms, end_ms, text, confidence, speaker, source}]`.
    pub segments: serde_json::Value,
    /// `[{anchor_id, t_ms, segment_id, kind}]`.
    pub timing_anchors: serde_json::Value,
    pub format_version: String,
    /// ArtifactStore ref for the materialized transcript artifact.
    pub artifact_ref: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to persist a transcript artifact (written by an `asr.transcribe` job).
#[derive(Clone, Debug)]
pub struct NewTranscriptArtifact {
    pub media_source_id: String,
    pub source_media_hash: String,
    pub language: String,
    pub model: serde_json::Value,
    pub selection_path: String,
    pub segments: serde_json::Value,
    pub timing_anchors: serde_json::Value,
    pub artifact_ref: String,
}

/// A `CaptionArtifactV1` record (6.11.5). Derived deterministically from a
/// transcript's segments + timing anchors; the same transcript + caption
/// profile MUST produce byte-identical output. Bound to its transcript by
/// `source_media_hash`. `muxed_media_artifact_id` is optional (sidecar-only).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptionArtifact {
    pub caption_artifact_id: Uuid,
    pub transcript_id: Uuid,
    pub media_source_id: String,
    /// Lineage key; MUST equal the parent transcript hash (6.11.6).
    pub source_media_hash: String,
    pub format: CaptionFormat,
    pub language: String,
    pub max_line_chars: i64,
    pub max_lines_per_cue: i64,
    pub min_cue_ms: i64,
    pub max_cue_ms: i64,
    pub cue_count: i64,
    pub derived_from_timing_anchors: bool,
    /// ArtifactStore ref for the caption sidecar bytes.
    pub artifact_ref: String,
    /// Optional ArtifactStore ref for a muxed-media derivative.
    pub muxed_media_artifact_id: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Caption profile + outputs to persist (written by a `caption.render` job).
#[derive(Clone, Debug)]
pub struct NewCaptionArtifact {
    pub transcript_id: Uuid,
    pub format: CaptionFormat,
    pub language: String,
    pub max_line_chars: i64,
    pub max_lines_per_cue: i64,
    pub min_cue_ms: i64,
    pub max_cue_ms: i64,
    pub cue_count: i64,
    pub artifact_ref: String,
    pub muxed_media_artifact_id: Option<String>,
}

/// Request for deterministic `caption.render` sidecar generation (6.11.5).
/// This in-process renderer only transforms persisted transcript segments into
/// sidecar bytes; it does not spawn ffmpeg/Whisper or mutate media.
#[derive(Clone, Debug)]
pub struct CaptionRenderRequest {
    pub transcript_id: Uuid,
    pub format: CaptionFormat,
    pub language: String,
    pub max_line_chars: i64,
    pub max_lines_per_cue: i64,
    pub min_cue_ms: i64,
    pub max_cue_ms: i64,
    pub muxed_media_artifact_id: Option<String>,
}

/// Output of `caption.render`: deterministic sidecar bytes plus the canonical
/// caption artifact record and recoverable caption-render receipt.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderedCaptionArtifact {
    pub caption: CaptionArtifact,
    pub receipt: PipelineReceipt,
    pub sidecar_text: String,
    pub sidecar_sha256: String,
}

/// A typed pipeline receipt (6.11.10): `MediaProbeReceiptV1` /
/// `TranscribeReceiptV1` / `CaptionRenderReceiptV1`. The recoverable evidence
/// unit (6.11.7): success carries `output_artifact_id`; failure carries
/// `error_class` and preserves `partial_artifact_id`. `tool_versions` and
/// `capability_grants` are reproducibility metadata. Any credential-bearing
/// tool argument is redacted before this row is written (6.11.8).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineReceipt {
    pub receipt_id: Uuid,
    pub kind: ReceiptKind,
    /// Originating governed job id (idempotency key for the receipt).
    pub job_id: String,
    /// Always `FEAT-ASR` per 6.11.10.
    pub feature_id: String,
    pub source_media_hash: String,
    /// Upstream artifact ids consumed by the job.
    pub input_artifact_ids: serde_json::Value,
    pub output_artifact_id: Option<String>,
    pub capability_grants: serde_json::Value,
    /// `{ffprobe: "...", ffmpeg: "...", whisper: "..."}` (redacted).
    pub tool_versions: serde_json::Value,
    pub status: ReceiptStatus,
    pub error_class: Option<String>,
    pub partial_artifact_id: Option<String>,
    pub emitted_at: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to file a typed pipeline receipt (written by the governed job).
#[derive(Clone, Debug)]
pub struct NewPipelineReceipt {
    pub kind: ReceiptKind,
    pub job_id: String,
    pub source_media_hash: String,
    pub input_artifact_ids: serde_json::Value,
    pub output_artifact_id: Option<String>,
    pub capability_grants: serde_json::Value,
    pub tool_versions: serde_json::Value,
    pub status: ReceiptStatus,
    pub error_class: Option<String>,
    pub partial_artifact_id: Option<String>,
    pub emitted_at: DateTime<Utc>,
}

/// Tokens that signal a value is credential-bearing and must be redacted before
/// persistence (6.11.8 LAW: secret + log hygiene). Matched case-insensitively
/// against JSON object keys in tool-version / capability / arg payloads.
const SECRET_KEY_HINTS: &[&str] = &[
    "secret",
    "token",
    "cookie",
    "password",
    "passwd",
    "credential",
    "authorization",
    "auth",
    "api_key",
    "apikey",
    "bearer",
    "session",
];

const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Recursively redact any credential-bearing values in a JSON payload so no raw
/// secret material is persisted to a record or echoed into an event payload
/// (6.11.8). Object values whose key matches a [`SECRET_KEY_HINTS`] token are
/// replaced with `[REDACTED]`; arrays and nested objects are walked. This is
/// the transcript-module analogue of `settings.rs` redaction.
fn redact_secrets(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, val) in map {
                let lowered = key.to_ascii_lowercase();
                let is_secret = SECRET_KEY_HINTS.iter().any(|hint| lowered.contains(hint));
                if is_secret {
                    out.insert(
                        key.clone(),
                        serde_json::Value::String(REDACTED_PLACEHOLDER.into()),
                    );
                } else {
                    out.insert(key.clone(), redact_secrets(val));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(redact_secrets).collect())
        }
        other => other.clone(),
    }
}

/// Validate a `source_media_hash` is the canonical `sha256:<hex>` lineage key
/// shape (6.11.6). A malformed hash can never anchor lineage, so it is rejected
/// before persistence rather than silently stored.
fn validate_source_media_hash(hash: &str) -> AtelierResult<()> {
    let trimmed = hash.trim();
    let hex = trimmed.strip_prefix("sha256:").ok_or_else(|| {
        AtelierError::Validation(format!(
            "source_media_hash must be 'sha256:<hex>', got {trimmed:?}"
        ))
    })?;
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AtelierError::Validation(format!(
            "source_media_hash sha256 digest must be 64 hex chars, got {hex:?}"
        )))
    }
}

#[derive(Clone, Debug)]
struct CaptionCue {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

fn caption_cues_from_transcript(
    transcript: &TranscriptArtifact,
    request: &CaptionRenderRequest,
) -> AtelierResult<Vec<CaptionCue>> {
    if request.max_line_chars <= 0 {
        return Err(AtelierError::Validation(
            "caption max_line_chars must be > 0".into(),
        ));
    }
    if request.max_lines_per_cue <= 0 {
        return Err(AtelierError::Validation(
            "caption max_lines_per_cue must be > 0".into(),
        ));
    }
    if request.min_cue_ms < 0 || request.max_cue_ms < 0 {
        return Err(AtelierError::Validation(
            "caption cue duration bounds must be >= 0".into(),
        ));
    }

    let segments = transcript.segments.as_array().ok_or_else(|| {
        AtelierError::Validation("transcript segments must be a JSON array".into())
    })?;
    let mut cues = Vec::with_capacity(segments.len());
    for (idx, segment) in segments.iter().enumerate() {
        let start_ms = segment
            .get("start_ms")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                AtelierError::Validation(format!("segment {idx} missing integer start_ms"))
            })?;
        let mut end_ms = segment
            .get("end_ms")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                AtelierError::Validation(format!("segment {idx} missing integer end_ms"))
            })?;
        let text = segment
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();

        if start_ms < 0 || end_ms <= start_ms {
            return Err(AtelierError::Validation(format!(
                "segment {idx} has invalid cue timing"
            )));
        }
        if request.min_cue_ms > 0 && end_ms - start_ms < request.min_cue_ms {
            end_ms = start_ms + request.min_cue_ms;
        }
        if request.max_cue_ms > 0 && end_ms - start_ms > request.max_cue_ms {
            end_ms = start_ms + request.max_cue_ms;
        }

        cues.push(CaptionCue {
            start_ms,
            end_ms,
            text: wrap_caption_text(
                text,
                request.max_line_chars as usize,
                request.max_lines_per_cue as usize,
            ),
        });
    }
    Ok(cues)
}

fn wrap_caption_text(text: &str, max_line_chars: usize, max_lines: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if !current.is_empty() && candidate_len > max_line_chars {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        return String::new();
    }
    if lines.len() > max_lines {
        let mut kept = lines[..max_lines].to_vec();
        let overflow = lines[max_lines..].join(" ");
        if let Some(last) = kept.last_mut() {
            if !last.is_empty() && !overflow.is_empty() {
                last.push(' ');
            }
            last.push_str(&overflow);
        }
        kept.join("\n")
    } else {
        lines.join("\n")
    }
}

fn format_srt_time(ms: i64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn format_vtt_time(ms: i64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn format_ass_time(ms: i64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let centis = (ms % 1_000) / 10;
    format!("{hours}:{minutes:02}:{seconds:02}.{centis:02}")
}

fn escape_ass_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\N")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

fn render_caption_sidecar(
    transcript: &TranscriptArtifact,
    request: &CaptionRenderRequest,
) -> AtelierResult<String> {
    let cues = caption_cues_from_transcript(transcript, request)?;
    let mut out = String::new();
    match request.format {
        CaptionFormat::Srt => {
            for (idx, cue) in cues.iter().enumerate() {
                out.push_str(&(idx + 1).to_string());
                out.push('\n');
                out.push_str(&format_srt_time(cue.start_ms));
                out.push_str(" --> ");
                out.push_str(&format_srt_time(cue.end_ms));
                out.push('\n');
                out.push_str(&cue.text);
                out.push('\n');
                if idx + 1 < cues.len() {
                    out.push('\n');
                }
            }
        }
        CaptionFormat::Vtt => {
            out.push_str("WEBVTT\n\n");
            for cue in &cues {
                out.push_str(&format_vtt_time(cue.start_ms));
                out.push_str(" --> ");
                out.push_str(&format_vtt_time(cue.end_ms));
                out.push('\n');
                out.push_str(&cue.text);
                out.push_str("\n\n");
            }
        }
        CaptionFormat::Ass => {
            out.push_str("[Script Info]\nScriptType: v4.00+\n\n");
            out.push_str("[V4+ Styles]\n");
            out.push_str("Format: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\n");
            out.push_str("Style: Default,Arial,36,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,2,32,32,32,1\n\n");
            out.push_str("[Events]\n");
            out.push_str(
                "Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n",
            );
            for cue in &cues {
                out.push_str("Dialogue: 0,");
                out.push_str(&format_ass_time(cue.start_ms));
                out.push(',');
                out.push_str(&format_ass_time(cue.end_ms));
                out.push_str(",Default,,0,0,0,,");
                out.push_str(&escape_ass_text(&cue.text));
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn sha256_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("sha256:{digest:x}")
}

fn reject_legacy_runtime_refs_in_json_array(
    field: &str,
    value: &serde_json::Value,
) -> AtelierResult<()> {
    let Some(items) = value.as_array() else {
        return Err(AtelierError::Validation(format!(
            "{field} must be a JSON array"
        )));
    };
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(AtelierError::Validation(format!(
                "{field} entries must be artifact ref strings"
            )));
        };
        reject_legacy_runtime_ref(field, text)?;
    }
    Ok(())
}

#[derive(SurrealValue)]
struct MediaProbeRow {
    probe_report_id: SurrealUuid,
    media_source_id: String,
    source_media_hash: String,
    container: String,
    duration_ms: i64,
    streams: serde_json::Value,
    ffprobe_tool_version: String,
    artifact_ref: String,
    probed_at: Datetime,
    created_at_utc: Datetime,
}

impl From<MediaProbeRow> for MediaProbeReport {
    fn from(row: MediaProbeRow) -> Self {
        Self {
            probe_report_id: row.probe_report_id.into(),
            media_source_id: row.media_source_id,
            source_media_hash: row.source_media_hash,
            container: row.container,
            duration_ms: row.duration_ms,
            streams: row.streams,
            ffprobe_tool_version: row.ffprobe_tool_version,
            artifact_ref: row.artifact_ref,
            probed_at: row.probed_at.into(),
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct TranscriptRow {
    transcript_id: SurrealUuid,
    media_source_id: String,
    source_media_hash: String,
    language: String,
    model: serde_json::Value,
    selection_path: String,
    segments: serde_json::Value,
    timing_anchors: serde_json::Value,
    format_version: String,
    artifact_ref: String,
    created_at_utc: Datetime,
}

impl From<TranscriptRow> for TranscriptArtifact {
    fn from(row: TranscriptRow) -> Self {
        Self {
            transcript_id: row.transcript_id.into(),
            media_source_id: row.media_source_id,
            source_media_hash: row.source_media_hash,
            language: row.language,
            model: row.model,
            selection_path: row.selection_path,
            segments: row.segments,
            timing_anchors: row.timing_anchors,
            format_version: row.format_version,
            artifact_ref: row.artifact_ref,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CaptionRow {
    caption_artifact_id: SurrealUuid,
    transcript_id: SurrealUuid,
    media_source_id: String,
    source_media_hash: String,
    format: String,
    language: String,
    max_line_chars: i64,
    max_lines_per_cue: i64,
    min_cue_ms: i64,
    max_cue_ms: i64,
    cue_count: i64,
    derived_from_timing_anchors: bool,
    artifact_ref: String,
    muxed_media_artifact_id: Option<String>,
    created_at_utc: Datetime,
}

impl TryFrom<CaptionRow> for CaptionArtifact {
    type Error = AtelierError;

    fn try_from(row: CaptionRow) -> AtelierResult<Self> {
        Ok(Self {
            caption_artifact_id: row.caption_artifact_id.into(),
            transcript_id: row.transcript_id.into(),
            media_source_id: row.media_source_id,
            source_media_hash: row.source_media_hash,
            format: CaptionFormat::from_token(&row.format)?,
            language: row.language,
            max_line_chars: row.max_line_chars,
            max_lines_per_cue: row.max_lines_per_cue,
            min_cue_ms: row.min_cue_ms,
            max_cue_ms: row.max_cue_ms,
            cue_count: row.cue_count,
            derived_from_timing_anchors: row.derived_from_timing_anchors,
            artifact_ref: row.artifact_ref,
            muxed_media_artifact_id: row.muxed_media_artifact_id,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct PipelineReceiptRow {
    receipt_id: SurrealUuid,
    kind: String,
    job_id: String,
    feature_id: String,
    source_media_hash: String,
    input_artifact_ids: serde_json::Value,
    output_artifact_id: Option<String>,
    capability_grants: serde_json::Value,
    tool_versions: serde_json::Value,
    status: String,
    error_class: Option<String>,
    partial_artifact_id: Option<String>,
    emitted_at: Datetime,
    created_at_utc: Datetime,
}

impl TryFrom<PipelineReceiptRow> for PipelineReceipt {
    type Error = AtelierError;

    fn try_from(row: PipelineReceiptRow) -> AtelierResult<Self> {
        Ok(Self {
            receipt_id: row.receipt_id.into(),
            kind: ReceiptKind::from_token(&row.kind)?,
            job_id: row.job_id,
            feature_id: row.feature_id,
            source_media_hash: row.source_media_hash,
            input_artifact_ids: row.input_artifact_ids,
            output_artifact_id: row.output_artifact_id,
            capability_grants: row.capability_grants,
            tool_versions: row.tool_versions,
            status: ReceiptStatus::from_token(&row.status)?,
            error_class: row.error_class,
            partial_artifact_id: row.partial_artifact_id,
            emitted_at: row.emitted_at.into(),
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

fn stable_transcript_uuid(kind: &str, key: &str) -> Uuid {
    let digest = Sha256::digest(format!("atelier.transcript:{kind}:{key}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[derive(Clone, SurrealValue)]
struct WriteProbeBindings {
    rid: RecordId,
    probe_report_id: SurrealUuid,
    media_source_id: String,
    source_media_hash: String,
    container: String,
    duration_ms: i64,
    streams: serde_json::Value,
    ffprobe_tool_version: String,
    artifact_ref: String,
    probed_at: Datetime,
}

#[derive(SurrealValue)]
struct SourceHashBinding {
    source_media_hash: String,
}

#[derive(Clone, SurrealValue)]
struct WriteTranscriptBindings {
    rid: RecordId,
    transcript_id: SurrealUuid,
    probe_ref: RecordId,
    media_source_id: String,
    source_media_hash: String,
    language: String,
    model: serde_json::Value,
    selection_path: String,
    segments: serde_json::Value,
    timing_anchors: serde_json::Value,
    artifact_ref: String,
}

#[derive(SurrealValue)]
struct TranscriptIdBinding {
    transcript_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct WriteCaptionBindings {
    rid: RecordId,
    caption_artifact_id: SurrealUuid,
    transcript_ref: RecordId,
    media_source_id: String,
    source_media_hash: String,
    format: String,
    language: String,
    max_line_chars: i64,
    max_lines_per_cue: i64,
    min_cue_ms: i64,
    max_cue_ms: i64,
    cue_count: i64,
    artifact_ref: String,
    muxed_media_artifact_id: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct WriteReceiptBindings {
    rid: RecordId,
    receipt_id: SurrealUuid,
    kind: String,
    job_id: String,
    source_media_hash: String,
    input_artifact_ids: serde_json::Value,
    output_artifact_id: Option<String>,
    capability_grants: serde_json::Value,
    tool_versions: serde_json::Value,
    status: String,
    error_class: Option<String>,
    partial_artifact_id: Option<String>,
    emitted_at: Datetime,
}

#[derive(SurrealValue)]
struct JobIdBinding {
    job_id: String,
}

const WRITE_PROBE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing = (SELECT VALUE id FROM atelier_media_probe_report \
                        WHERE source_media_hash = $domain.source_media_hash LIMIT 1); \
       IF $existing = [] { \
         CREATE $domain.rid CONTENT { \
           probe_report_id: $domain.probe_report_id, media_source_id: $domain.media_source_id, \
           source_media_hash: $domain.source_media_hash, container: $domain.container, \
           duration_ms: $domain.duration_ms, streams: $domain.streams, \
           ffprobe_tool_version: $domain.ffprobe_tool_version, artifact_ref: $domain.artifact_ref, \
           probed_at: $domain.probed_at \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN (SELECT probe_report_id, media_source_id, source_media_hash, container, \
                    duration_ms, streams, ffprobe_tool_version, artifact_ref, \
                    probed_at, created_at_utc \
             FROM atelier_media_probe_report \
             WHERE source_media_hash = $domain.source_media_hash LIMIT 1); };"
);

const GET_PROBE_BY_HASH_STATEMENT: &str =
    "SELECT probe_report_id, media_source_id, source_media_hash, container, duration_ms, \
            streams, ffprobe_tool_version, artifact_ref, probed_at, created_at_utc \
     FROM atelier_media_probe_report WHERE source_media_hash = $source_media_hash LIMIT 1;";

const WRITE_TRANSCRIPT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing = (SELECT VALUE id FROM atelier_transcript_artifact \
                        WHERE artifact_ref = $domain.artifact_ref LIMIT 1); \
       IF $existing = [] { \
         CREATE $domain.rid CONTENT { \
           transcript_id: $domain.transcript_id, media_source_id: $domain.media_source_id, \
           source_media_hash: $domain.probe_ref, language: $domain.language, model: $domain.model, \
           selection_path: $domain.selection_path, segments: $domain.segments, \
           timing_anchors: $domain.timing_anchors, format_version: 'TranscriptArtifactV1', \
           artifact_ref: $domain.artifact_ref \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN (SELECT transcript_id, media_source_id, source_media_hash.source_media_hash AS source_media_hash, \
                    language, model, selection_path, segments, timing_anchors, format_version, \
                    artifact_ref, created_at_utc \
             FROM atelier_transcript_artifact \
             WHERE artifact_ref = $domain.artifact_ref LIMIT 1); };"
);

const GET_TRANSCRIPT_STATEMENT: &str =
    "SELECT transcript_id, media_source_id, source_media_hash.source_media_hash AS source_media_hash, \
            language, model, selection_path, segments, timing_anchors, format_version, \
            artifact_ref, created_at_utc \
     FROM atelier_transcript_artifact WHERE transcript_id = $transcript_id LIMIT 1;";

const WRITE_CAPTION_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing = (SELECT VALUE id FROM atelier_caption_artifact \
                        WHERE artifact_ref = $domain.artifact_ref LIMIT 1); \
       IF $existing = [] { \
         CREATE $domain.rid CONTENT { \
           caption_artifact_id: $domain.caption_artifact_id, transcript_id: $domain.transcript_ref, \
           media_source_id: $domain.media_source_id, source_media_hash: $domain.source_media_hash, \
           format: $domain.format, language: $domain.language, \
           max_line_chars: $domain.max_line_chars, max_lines_per_cue: $domain.max_lines_per_cue, \
           min_cue_ms: $domain.min_cue_ms, max_cue_ms: $domain.max_cue_ms, \
           cue_count: $domain.cue_count, derived_from_timing_anchors: true, \
           artifact_ref: $domain.artifact_ref, \
           muxed_media_artifact_id: $domain.muxed_media_artifact_id \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN (SELECT caption_artifact_id, record::id(transcript_id) AS transcript_id, \
                    media_source_id, source_media_hash, format, language, max_line_chars, \
                    max_lines_per_cue, min_cue_ms, max_cue_ms, cue_count, \
                    derived_from_timing_anchors, artifact_ref, muxed_media_artifact_id, created_at_utc \
             FROM atelier_caption_artifact \
             WHERE artifact_ref = $domain.artifact_ref LIMIT 1); };"
);

const LIST_CAPTIONS_STATEMENT: &str =
    "SELECT caption_artifact_id, record::id(transcript_id) AS transcript_id, media_source_id, \
            source_media_hash, format, language, max_line_chars, max_lines_per_cue, min_cue_ms, \
            max_cue_ms, cue_count, derived_from_timing_anchors, artifact_ref, \
            muxed_media_artifact_id, created_at_utc \
     FROM atelier_caption_artifact \
     WHERE transcript_id = type::record('atelier_transcript_artifact', $transcript_id) \
     ORDER BY created_at_utc ASC;";

const WRITE_RECEIPT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $existing = (SELECT VALUE id FROM atelier_transcript_receipt \
                        WHERE job_id = $domain.job_id LIMIT 1); \
       IF $existing = [] { \
         CREATE $domain.rid CONTENT { \
           receipt_id: $domain.receipt_id, kind: $domain.kind, job_id: $domain.job_id, \
           feature_id: 'FEAT-ASR', source_media_hash: $domain.source_media_hash, \
           input_artifact_ids: $domain.input_artifact_ids, \
           output_artifact_id: $domain.output_artifact_id, \
           capability_grants: $domain.capability_grants, tool_versions: $domain.tool_versions, \
           status: $domain.status, error_class: $domain.error_class, \
           partial_artifact_id: $domain.partial_artifact_id, emitted_at: $domain.emitted_at \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN (SELECT receipt_id, kind, job_id, feature_id, source_media_hash, \
                    input_artifact_ids, output_artifact_id, capability_grants, tool_versions, \
                    status, error_class, partial_artifact_id, emitted_at, created_at_utc \
             FROM atelier_transcript_receipt WHERE job_id = $domain.job_id LIMIT 1); };"
);

const GET_RECEIPT_BY_JOB_STATEMENT: &str =
    "SELECT receipt_id, kind, job_id, feature_id, source_media_hash, input_artifact_ids, \
            output_artifact_id, capability_grants, tool_versions, status, error_class, \
            partial_artifact_id, emitted_at, created_at_utc \
     FROM atelier_transcript_receipt WHERE job_id = $job_id LIMIT 1;";

const LIST_RECEIPTS_BY_SOURCE_STATEMENT: &str =
    "SELECT receipt_id, kind, job_id, feature_id, source_media_hash, input_artifact_ids, \
            output_artifact_id, capability_grants, tool_versions, status, error_class, \
            partial_artifact_id, emitted_at, created_at_utc \
     FROM atelier_transcript_receipt WHERE source_media_hash = $source_media_hash \
     ORDER BY emitted_at DESC;";

impl AtelierStore {
    /// Persist a `MediaProbeReportV1` record (6.11.3), written by a governed
    /// `media.probe` job. Idempotent on `source_media_hash`: re-probing the same
    /// source bytes returns the existing report rather than duplicating it, so a
    /// job retry never forks lineage. The `sha256:<hex>` shape is validated up
    /// front (6.11.6). Emits [`MEDIA_PROBE_RECORDED`].
    ///
    /// This NEVER runs ffprobe; the tool executed in the governed job and this
    /// method only stores the result and its ArtifactStore ref (6.11.1).
    pub async fn record_media_probe(
        &self,
        new: &NewMediaProbeReport,
    ) -> AtelierResult<MediaProbeReport> {
        validate_source_media_hash(&new.source_media_hash)?;
        if new.media_source_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "media_source_id must not be empty".into(),
            ));
        }
        if new.artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "probe artifact_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", &new.artifact_ref)?;
        if new.duration_ms < 0 {
            return Err(AtelierError::Validation(
                "probe duration_ms must be >= 0".into(),
            ));
        }

        let probe_report_id = stable_transcript_uuid("probe", &new.source_media_hash);
        let row: MediaProbeRow = self
            .write_with_event(
                WRITE_PROBE_STATEMENT,
                WriteProbeBindings {
                    rid: RecordId::new(
                        "atelier_media_probe_report",
                        SurrealUuid::from(probe_report_id),
                    ),
                    probe_report_id: SurrealUuid::from(probe_report_id),
                    media_source_id: new.media_source_id.clone(),
                    source_media_hash: new.source_media_hash.clone(),
                    container: new.container.clone(),
                    duration_ms: new.duration_ms,
                    streams: new.streams.clone(),
                    ffprobe_tool_version: new.ffprobe_tool_version.clone(),
                    artifact_ref: new.artifact_ref.clone(),
                    probed_at: Datetime::from(new.probed_at),
                },
                MEDIA_PROBE_RECORDED,
                "atelier_media_probe_report",
                &probe_report_id.to_string(),
                serde_json::json!({
                    "probe_report_id": probe_report_id,
                    "media_source_id": new.media_source_id,
                    "source_media_hash": new.source_media_hash,
                    "container": new.container,
                    "duration_ms": new.duration_ms,
                }),
            )
            .await?
            .ok_or_else(|| {
                AtelierError::Internal("recording media probe returned no row".to_owned())
            })?;
        Ok(row.into())
    }

    /// Fetch a probe report by its lineage `source_media_hash`.
    pub async fn get_media_probe_by_hash(
        &self,
        source_media_hash: &str,
    ) -> AtelierResult<Option<MediaProbeReport>> {
        let bindings = SourceHashBinding {
            source_media_hash: source_media_hash.to_owned(),
        };
        let row: Option<MediaProbeRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_first(GET_PROBE_BY_HASH_STATEMENT, bindings).await },
                )
            })
            .await?;
        Ok(row.map(Into::into))
    }

    /// Persist a canonical `TranscriptArtifactV1` record (6.11.4), written by a
    /// governed `asr.transcribe` job. Enforces the lineage LAW (6.11.6): the
    /// transcript's `source_media_hash` MUST match an existing probe report's
    /// hash, otherwise it is a lineage break and is rejected with a typed error
    /// rather than persisted. Segments + timing anchors are stored as JSONB so
    /// transcript positions stay independently addressable. Emits
    /// [`TRANSCRIPT_RECORDED`].
    ///
    /// This NEVER runs ffmpeg or Whisper; inference executed in the governed
    /// job and this method only stores the canonical artifact (6.11.1).
    pub async fn record_transcript(
        &self,
        new: &NewTranscriptArtifact,
    ) -> AtelierResult<TranscriptArtifact> {
        validate_source_media_hash(&new.source_media_hash)?;
        if new.artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "transcript artifact_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", &new.artifact_ref)?;
        if !new.segments.is_array() {
            return Err(AtelierError::Validation(
                "transcript segments must be a JSON array".into(),
            ));
        }
        if !new.timing_anchors.is_array() {
            return Err(AtelierError::Validation(
                "transcript timing_anchors must be a JSON array".into(),
            ));
        }

        // Lineage LAW (6.11.6): a transcript may only bind to an existing probe
        // report sharing the same source_media_hash.
        let probe = self
            .get_media_probe_by_hash(&new.source_media_hash)
            .await?
            .ok_or_else(|| {
                AtelierError::Validation(format!(
                    "lineage break: no media_probe_report for source_media_hash {}",
                    new.source_media_hash
                ))
            })?;

        let transcript_id = stable_transcript_uuid("transcript", &new.artifact_ref);
        let row: TranscriptRow = self
            .write_with_event(
                WRITE_TRANSCRIPT_STATEMENT,
                WriteTranscriptBindings {
                    rid: RecordId::new(
                        "atelier_transcript_artifact",
                        SurrealUuid::from(transcript_id),
                    ),
                    transcript_id: SurrealUuid::from(transcript_id),
                    probe_ref: RecordId::new(
                        "atelier_media_probe_report",
                        SurrealUuid::from(probe.probe_report_id),
                    ),
                    media_source_id: new.media_source_id.clone(),
                    source_media_hash: new.source_media_hash.clone(),
                    language: new.language.clone(),
                    model: new.model.clone(),
                    selection_path: new.selection_path.clone(),
                    segments: new.segments.clone(),
                    timing_anchors: new.timing_anchors.clone(),
                    artifact_ref: new.artifact_ref.clone(),
                },
                TRANSCRIPT_RECORDED,
                "atelier_transcript_artifact",
                &transcript_id.to_string(),
                serde_json::json!({
                    "transcript_id": transcript_id,
                    "media_source_id": new.media_source_id,
                    "source_media_hash": new.source_media_hash,
                    "language": new.language,
                    "selection_path": new.selection_path,
                    "model": redact_secrets(&new.model),
                }),
            )
            .await?
            .ok_or_else(|| {
                AtelierError::Internal("recording transcript returned no row".to_owned())
            })?;
        Ok(row.into())
    }

    /// Fetch a transcript artifact by id.
    pub async fn get_transcript(&self, transcript_id: Uuid) -> AtelierResult<TranscriptArtifact> {
        let bindings = TranscriptIdBinding {
            transcript_id: SurrealUuid::from(transcript_id),
        };
        let row: Option<TranscriptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_TRANSCRIPT_STATEMENT, bindings).await })
            })
            .await?;
        row.map(Into::into)
            .ok_or_else(|| AtelierError::NotFound(format!("transcript_id={transcript_id}")))
    }

    /// Persist a `CaptionArtifactV1` record (6.11.5), written by a governed
    /// `caption.render` job. Enforces lineage (6.11.6): the parent transcript
    /// must exist and the caption's `source_media_hash` MUST match the parent
    /// transcript's hash; a mismatch is a lineage break and is rejected. Caption
    /// rendering MUST NOT re-run ASR (this method only stores the derived
    /// sidecar record). Idempotent on `artifact_ref`. Emits [`CAPTION_RECORDED`].
    pub async fn record_caption(&self, new: &NewCaptionArtifact) -> AtelierResult<CaptionArtifact> {
        if new.artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "caption artifact_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", &new.artifact_ref)?;
        if let Some(muxed_media_artifact_id) = &new.muxed_media_artifact_id {
            reject_legacy_runtime_ref("muxed_media_artifact_id", muxed_media_artifact_id)?;
        }
        if new.cue_count < 0 {
            return Err(AtelierError::Validation(
                "caption cue_count must be >= 0".into(),
            ));
        }

        // Lineage LAW (6.11.6): resolve the parent transcript and inherit its
        // source_media_hash + media_source_id; reject if it does not exist.
        let transcript = self.get_transcript(new.transcript_id).await?;

        let caption_artifact_id = stable_transcript_uuid("caption", &new.artifact_ref);
        let row: CaptionRow = self
            .write_with_event(
                WRITE_CAPTION_STATEMENT,
                WriteCaptionBindings {
                    rid: RecordId::new(
                        "atelier_caption_artifact",
                        SurrealUuid::from(caption_artifact_id),
                    ),
                    caption_artifact_id: SurrealUuid::from(caption_artifact_id),
                    transcript_ref: RecordId::new(
                        "atelier_transcript_artifact",
                        SurrealUuid::from(transcript.transcript_id),
                    ),
                    media_source_id: transcript.media_source_id.clone(),
                    source_media_hash: transcript.source_media_hash.clone(),
                    format: new.format.as_token().to_owned(),
                    language: new.language.clone(),
                    max_line_chars: new.max_line_chars,
                    max_lines_per_cue: new.max_lines_per_cue,
                    min_cue_ms: new.min_cue_ms,
                    max_cue_ms: new.max_cue_ms,
                    cue_count: new.cue_count,
                    artifact_ref: new.artifact_ref.clone(),
                    muxed_media_artifact_id: new.muxed_media_artifact_id.clone(),
                },
                CAPTION_RECORDED,
                "atelier_caption_artifact",
                &caption_artifact_id.to_string(),
                serde_json::json!({
                    "caption_artifact_id": caption_artifact_id,
                    "transcript_id": transcript.transcript_id,
                    "source_media_hash": transcript.source_media_hash,
                    "format": new.format.as_token(),
                    "language": new.language,
                    "cue_count": new.cue_count,
                    "muxed": new.muxed_media_artifact_id.is_some(),
                }),
            )
            .await?
            .ok_or_else(|| {
                AtelierError::Internal("recording caption returned no row".to_owned())
            })?;
        CaptionArtifact::try_from(row)
    }

    /// Deterministically render a transcript into an SRT/VTT/ASS caption
    /// sidecar, persist the resulting `CaptionArtifactV1`, and file a
    /// `CaptionRenderReceiptV1`. The returned sidecar bytes are content
    /// addressed by sha256; callers can materialize them into ArtifactStore
    /// using the returned `caption.artifact_ref`.
    pub async fn render_caption(
        &self,
        request: &CaptionRenderRequest,
    ) -> AtelierResult<RenderedCaptionArtifact> {
        let transcript = self.get_transcript(request.transcript_id).await?;
        let sidecar_text = render_caption_sidecar(&transcript, request)?;
        let sidecar_sha256 = sha256_text(&sidecar_text);
        let artifact_ref = format!(
            "artifact://atelier/caption/{}/{}",
            request.format.as_token(),
            sidecar_sha256
        );
        let cue_count = caption_cues_from_transcript(&transcript, request)?.len() as i64;
        let caption = self
            .record_caption(&NewCaptionArtifact {
                transcript_id: transcript.transcript_id,
                format: request.format,
                language: request.language.clone(),
                max_line_chars: request.max_line_chars,
                max_lines_per_cue: request.max_lines_per_cue,
                min_cue_ms: request.min_cue_ms,
                max_cue_ms: request.max_cue_ms,
                cue_count,
                artifact_ref: artifact_ref.clone(),
                muxed_media_artifact_id: request.muxed_media_artifact_id.clone(),
            })
            .await?;
        let receipt = self
            .file_pipeline_receipt(&NewPipelineReceipt {
                kind: ReceiptKind::CaptionRender,
                job_id: format!(
                    "caption-render:{}:{}:{}",
                    transcript.transcript_id,
                    request.format.as_token(),
                    sidecar_sha256
                ),
                source_media_hash: transcript.source_media_hash.clone(),
                input_artifact_ids: serde_json::json!([transcript.artifact_ref]),
                output_artifact_id: Some(caption.artifact_ref.clone()),
                capability_grants: serde_json::json!({
                    "workflow": "caption.render",
                    "external_process": false,
                    "renderer": "hsk.atelier.caption_render@1",
                }),
                tool_versions: serde_json::json!({
                    "caption_renderer": "hsk.atelier.caption_render@1",
                }),
                status: ReceiptStatus::Completed,
                error_class: None,
                partial_artifact_id: None,
                emitted_at: Utc::now(),
            })
            .await?;

        Ok(RenderedCaptionArtifact {
            caption,
            receipt,
            sidecar_text,
            sidecar_sha256,
        })
    }

    /// List caption artifacts derived from a transcript (creation order).
    pub async fn list_captions_for_transcript(
        &self,
        transcript_id: Uuid,
    ) -> AtelierResult<Vec<CaptionArtifact>> {
        let bindings = TranscriptIdBinding {
            transcript_id: SurrealUuid::from(transcript_id),
        };
        let rows: Vec<CaptionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_CAPTIONS_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter().map(CaptionArtifact::try_from).collect()
    }

    /// File a typed pipeline receipt (6.11.10): `MediaProbeReceiptV1` /
    /// `TranscribeReceiptV1` / `CaptionRenderReceiptV1`. The recoverable
    /// evidence unit (6.11.7): a `Completed` receipt carries an
    /// `output_artifact_id`; a `Failed` receipt carries a typed `error_class`
    /// and preserves any `partial_artifact_id`. Idempotent on `job_id`:
    /// re-filing the same job's receipt returns the existing row. `tool_versions`
    /// and `capability_grants` are deep-redacted before persistence so no
    /// credential-bearing argument is stored (6.11.8). Emits [`RECEIPT_FILED`].
    pub async fn file_pipeline_receipt(
        &self,
        new: &NewPipelineReceipt,
    ) -> AtelierResult<PipelineReceipt> {
        validate_source_media_hash(&new.source_media_hash)?;
        if new.job_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "receipt job_id must not be empty".into(),
            ));
        }
        // Status/field consistency (6.11.7): completed -> output; failed -> error_class.
        match new.status {
            ReceiptStatus::Completed if new.output_artifact_id.is_none() => {
                return Err(AtelierError::Validation(
                    "completed receipt must carry an output_artifact_id".into(),
                ));
            }
            ReceiptStatus::Failed if new.error_class.is_none() => {
                return Err(AtelierError::Validation(
                    "failed receipt must carry a typed error_class".into(),
                ));
            }
            _ => {}
        }

        // Secret hygiene (6.11.8): deep-redact tool-version + capability payloads
        // before they are persisted or echoed into the event ledger.
        let tool_versions = redact_secrets(&new.tool_versions);
        let capability_grants = redact_secrets(&new.capability_grants);
        reject_legacy_runtime_refs_in_json_array("input_artifact_ids", &new.input_artifact_ids)?;
        if let Some(output_artifact_id) = &new.output_artifact_id {
            reject_legacy_runtime_ref("output_artifact_id", output_artifact_id)?;
        }
        if let Some(partial_artifact_id) = &new.partial_artifact_id {
            reject_legacy_runtime_ref("partial_artifact_id", partial_artifact_id)?;
        }
        let input_artifact_ids = new.input_artifact_ids.clone();

        let receipt_id = stable_transcript_uuid("receipt", &new.job_id);
        let row: PipelineReceiptRow = self
            .write_with_event(
                WRITE_RECEIPT_STATEMENT,
                WriteReceiptBindings {
                    rid: RecordId::new("atelier_transcript_receipt", SurrealUuid::from(receipt_id)),
                    receipt_id: SurrealUuid::from(receipt_id),
                    kind: new.kind.as_token().to_owned(),
                    job_id: new.job_id.clone(),
                    source_media_hash: new.source_media_hash.clone(),
                    input_artifact_ids,
                    output_artifact_id: new.output_artifact_id.clone(),
                    capability_grants,
                    tool_versions: tool_versions.clone(),
                    status: new.status.as_token().to_owned(),
                    error_class: new.error_class.clone(),
                    partial_artifact_id: new.partial_artifact_id.clone(),
                    emitted_at: Datetime::from(new.emitted_at),
                },
                RECEIPT_FILED,
                "atelier_transcript_receipt",
                &receipt_id.to_string(),
                serde_json::json!({
                    "receipt_id": receipt_id,
                    "kind": new.kind.as_token(),
                    "job_id": new.job_id,
                    "feature_id": "FEAT-ASR",
                    "source_media_hash": new.source_media_hash,
                    "status": new.status.as_token(),
                    "error_class": new.error_class,
                    "output_artifact_id": new.output_artifact_id,
                    "partial_artifact_id": new.partial_artifact_id,
                    "tool_versions": tool_versions,
                }),
            )
            .await?
            .ok_or_else(|| {
                AtelierError::Internal("filing transcript receipt returned no row".to_owned())
            })?;
        PipelineReceipt::try_from(row)
    }

    /// Fetch a receipt by its originating job id.
    pub async fn get_receipt_by_job(&self, job_id: &str) -> AtelierResult<Option<PipelineReceipt>> {
        let bindings = JobIdBinding {
            job_id: job_id.to_owned(),
        };
        let row: Option<PipelineReceiptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_RECEIPT_BY_JOB_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.map(PipelineReceipt::try_from).transpose()
    }

    /// List all receipts bound to a given lineage `source_media_hash`, newest
    /// first; the auditable evidence trail for one media source (6.11.6/6.11.7).
    pub async fn list_receipts_for_source(
        &self,
        source_media_hash: &str,
    ) -> AtelierResult<Vec<PipelineReceipt>> {
        let bindings = SourceHashBinding {
            source_media_hash: source_media_hash.to_owned(),
        };
        let rows: Vec<PipelineReceiptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_RECEIPTS_BY_SOURCE_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(PipelineReceipt::try_from).collect()
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    #[test]
    fn redacts_secret_keys_recursively() {
        let payload = serde_json::json!({
            "ffmpeg": "6.1",
            "auth_token": "abc123",
            "nested": {"cookie": "sid=xyz", "ok": "keep"},
            "args": [{"bearer": "t"}, {"flag": "-v"}]
        });
        let redacted = redact_secrets(&payload);
        assert_eq!(redacted["ffmpeg"], "6.1");
        assert_eq!(redacted["auth_token"], REDACTED_PLACEHOLDER);
        assert_eq!(redacted["nested"]["cookie"], REDACTED_PLACEHOLDER);
        assert_eq!(redacted["nested"]["ok"], "keep");
        assert_eq!(redacted["args"][0]["bearer"], REDACTED_PLACEHOLDER);
        assert_eq!(redacted["args"][1]["flag"], "-v");
    }

    #[test]
    fn validates_source_media_hash_shape() {
        assert!(validate_source_media_hash(&format!("sha256:{}", "a".repeat(64))).is_ok());
        assert!(validate_source_media_hash("sha256:tooshort").is_err());
        assert!(validate_source_media_hash("md5:deadbeef").is_err());
        assert!(validate_source_media_hash(&"a".repeat(64)).is_err());
    }
}
