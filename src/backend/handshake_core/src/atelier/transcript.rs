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
//! recoverable receipts) is carried across. Storage authority is PostgreSQL +
//! EventLedger + ArtifactStore (6.11.2 LAW: storage authority).
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
//! coverage), MT-004 (PostgreSQL-only authority).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

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

fn probe_from_row(row: &sqlx::postgres::PgRow) -> MediaProbeReport {
    MediaProbeReport {
        probe_report_id: row.get("probe_report_id"),
        media_source_id: row.get("media_source_id"),
        source_media_hash: row.get("source_media_hash"),
        container: row.get("container"),
        duration_ms: row.get("duration_ms"),
        streams: row.get("streams"),
        ffprobe_tool_version: row.get("ffprobe_tool_version"),
        artifact_ref: row.get("artifact_ref"),
        probed_at: row.get("probed_at"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn transcript_from_row(row: &sqlx::postgres::PgRow) -> TranscriptArtifact {
    TranscriptArtifact {
        transcript_id: row.get("transcript_id"),
        media_source_id: row.get("media_source_id"),
        source_media_hash: row.get("source_media_hash"),
        language: row.get("language"),
        model: row.get("model"),
        selection_path: row.get("selection_path"),
        segments: row.get("segments"),
        timing_anchors: row.get("timing_anchors"),
        format_version: row.get("format_version"),
        artifact_ref: row.get("artifact_ref"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn caption_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<CaptionArtifact> {
    let format_token: String = row.get("format");
    Ok(CaptionArtifact {
        caption_artifact_id: row.get("caption_artifact_id"),
        transcript_id: row.get("transcript_id"),
        media_source_id: row.get("media_source_id"),
        source_media_hash: row.get("source_media_hash"),
        format: CaptionFormat::from_token(&format_token)?,
        language: row.get("language"),
        max_line_chars: row.get("max_line_chars"),
        max_lines_per_cue: row.get("max_lines_per_cue"),
        min_cue_ms: row.get("min_cue_ms"),
        max_cue_ms: row.get("max_cue_ms"),
        cue_count: row.get("cue_count"),
        derived_from_timing_anchors: row.get("derived_from_timing_anchors"),
        artifact_ref: row.get("artifact_ref"),
        muxed_media_artifact_id: row.get("muxed_media_artifact_id"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn receipt_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PipelineReceipt> {
    let kind_token: String = row.get("kind");
    let status_token: String = row.get("status");
    Ok(PipelineReceipt {
        receipt_id: row.get("receipt_id"),
        kind: ReceiptKind::from_token(&kind_token)?,
        job_id: row.get("job_id"),
        feature_id: row.get("feature_id"),
        source_media_hash: row.get("source_media_hash"),
        input_artifact_ids: row.get("input_artifact_ids"),
        output_artifact_id: row.get("output_artifact_id"),
        capability_grants: row.get("capability_grants"),
        tool_versions: row.get("tool_versions"),
        status: ReceiptStatus::from_token(&status_token)?,
        error_class: row.get("error_class"),
        partial_artifact_id: row.get("partial_artifact_id"),
        emitted_at: row.get("emitted_at"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const PROBE_COLUMNS: &str = "probe_report_id, media_source_id, source_media_hash, container, \
                             duration_ms, streams, ffprobe_tool_version, artifact_ref, \
                             probed_at, created_at_utc";

const TRANSCRIPT_COLUMNS: &str = "transcript_id, media_source_id, source_media_hash, language, \
                                  model, selection_path, segments, timing_anchors, \
                                  format_version, artifact_ref, created_at_utc";

const CAPTION_COLUMNS: &str = "caption_artifact_id, transcript_id, media_source_id, \
                               source_media_hash, format, language, max_line_chars, \
                               max_lines_per_cue, min_cue_ms, max_cue_ms, cue_count, \
                               derived_from_timing_anchors, artifact_ref, \
                               muxed_media_artifact_id, created_at_utc";

const RECEIPT_COLUMNS: &str = "receipt_id, kind, job_id, feature_id, source_media_hash, \
                               input_artifact_ids, output_artifact_id, capability_grants, \
                               tool_versions, status, error_class, partial_artifact_id, \
                               emitted_at, created_at_utc";

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

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_media_probe_report
                 (media_source_id, source_media_hash, container, duration_ms, streams,
                  ffprobe_tool_version, artifact_ref, probed_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (source_media_hash) DO UPDATE
                 SET source_media_hash = EXCLUDED.source_media_hash
               RETURNING {PROBE_COLUMNS}"#
        ))
        .bind(&new.media_source_id)
        .bind(&new.source_media_hash)
        .bind(&new.container)
        .bind(new.duration_ms)
        .bind(&new.streams)
        .bind(&new.ffprobe_tool_version)
        .bind(&new.artifact_ref)
        .bind(new.probed_at)
        .fetch_one(self.pool())
        .await?;
        let report = probe_from_row(&row);

        self.record_event(
            MEDIA_PROBE_RECORDED,
            "atelier_media_probe_report",
            &report.probe_report_id.to_string(),
            serde_json::json!({
                "probe_report_id": report.probe_report_id,
                "media_source_id": report.media_source_id,
                "source_media_hash": report.source_media_hash,
                "container": report.container,
                "duration_ms": report.duration_ms,
            }),
        )
        .await?;
        Ok(report)
    }

    /// Fetch a probe report by its lineage `source_media_hash`.
    pub async fn get_media_probe_by_hash(
        &self,
        source_media_hash: &str,
    ) -> AtelierResult<Option<MediaProbeReport>> {
        let row = sqlx::query(&format!(
            r#"SELECT {PROBE_COLUMNS} FROM atelier_media_probe_report
               WHERE source_media_hash = $1"#
        ))
        .bind(source_media_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(probe_from_row))
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
        if self
            .get_media_probe_by_hash(&new.source_media_hash)
            .await?
            .is_none()
        {
            return Err(AtelierError::Validation(format!(
                "lineage break: no media_probe_report for source_media_hash {}",
                new.source_media_hash
            )));
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_transcript_artifact
                 (media_source_id, source_media_hash, language, model, selection_path,
                  segments, timing_anchors, format_version, artifact_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'TranscriptArtifactV1', $8)
               ON CONFLICT (artifact_ref) DO UPDATE
                 SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING {TRANSCRIPT_COLUMNS}"#
        ))
        .bind(&new.media_source_id)
        .bind(&new.source_media_hash)
        .bind(&new.language)
        .bind(&new.model)
        .bind(&new.selection_path)
        .bind(&new.segments)
        .bind(&new.timing_anchors)
        .bind(&new.artifact_ref)
        .fetch_one(self.pool())
        .await?;
        let transcript = transcript_from_row(&row);

        self.record_event(
            TRANSCRIPT_RECORDED,
            "atelier_transcript_artifact",
            &transcript.transcript_id.to_string(),
            serde_json::json!({
                "transcript_id": transcript.transcript_id,
                "media_source_id": transcript.media_source_id,
                "source_media_hash": transcript.source_media_hash,
                "language": transcript.language,
                "selection_path": transcript.selection_path,
                // model is reproducibility metadata; redact any stray secrets.
                "model": redact_secrets(&transcript.model),
            }),
        )
        .await?;
        Ok(transcript)
    }

    /// Fetch a transcript artifact by id.
    pub async fn get_transcript(&self, transcript_id: Uuid) -> AtelierResult<TranscriptArtifact> {
        let row = sqlx::query(&format!(
            r#"SELECT {TRANSCRIPT_COLUMNS} FROM atelier_transcript_artifact
               WHERE transcript_id = $1"#
        ))
        .bind(transcript_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("transcript_id={transcript_id}")))?;
        Ok(transcript_from_row(&row))
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

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_caption_artifact
                 (transcript_id, media_source_id, source_media_hash, format, language,
                  max_line_chars, max_lines_per_cue, min_cue_ms, max_cue_ms, cue_count,
                  derived_from_timing_anchors, artifact_ref, muxed_media_artifact_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, TRUE, $11, $12)
               ON CONFLICT (artifact_ref) DO UPDATE
                 SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING {CAPTION_COLUMNS}"#
        ))
        .bind(transcript.transcript_id)
        .bind(&transcript.media_source_id)
        .bind(&transcript.source_media_hash)
        .bind(new.format.as_token())
        .bind(&new.language)
        .bind(new.max_line_chars)
        .bind(new.max_lines_per_cue)
        .bind(new.min_cue_ms)
        .bind(new.max_cue_ms)
        .bind(new.cue_count)
        .bind(&new.artifact_ref)
        .bind(&new.muxed_media_artifact_id)
        .fetch_one(self.pool())
        .await?;
        let caption = caption_from_row(&row)?;

        self.record_event(
            CAPTION_RECORDED,
            "atelier_caption_artifact",
            &caption.caption_artifact_id.to_string(),
            serde_json::json!({
                "caption_artifact_id": caption.caption_artifact_id,
                "transcript_id": caption.transcript_id,
                "source_media_hash": caption.source_media_hash,
                "format": caption.format.as_token(),
                "language": caption.language,
                "cue_count": caption.cue_count,
                "muxed": caption.muxed_media_artifact_id.is_some(),
            }),
        )
        .await?;
        Ok(caption)
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
        let rows = sqlx::query(&format!(
            r#"SELECT {CAPTION_COLUMNS} FROM atelier_caption_artifact
               WHERE transcript_id = $1
               ORDER BY created_at_utc ASC"#
        ))
        .bind(transcript_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(caption_from_row).collect()
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

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_transcript_receipt
                 (kind, job_id, feature_id, source_media_hash, input_artifact_ids,
                  output_artifact_id, capability_grants, tool_versions, status,
                  error_class, partial_artifact_id, emitted_at)
               VALUES ($1, $2, 'FEAT-ASR', $3, $4, $5, $6, $7, $8, $9, $10, $11)
               ON CONFLICT (job_id) DO UPDATE
                 SET job_id = EXCLUDED.job_id
               RETURNING {RECEIPT_COLUMNS}"#
        ))
        .bind(new.kind.as_token())
        .bind(&new.job_id)
        .bind(&new.source_media_hash)
        .bind(&input_artifact_ids)
        .bind(&new.output_artifact_id)
        .bind(&capability_grants)
        .bind(&tool_versions)
        .bind(new.status.as_token())
        .bind(&new.error_class)
        .bind(&new.partial_artifact_id)
        .bind(new.emitted_at)
        .fetch_one(self.pool())
        .await?;
        let receipt = receipt_from_row(&row)?;

        self.record_event(
            RECEIPT_FILED,
            "atelier_transcript_receipt",
            &receipt.receipt_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt.receipt_id,
                "kind": receipt.kind.as_token(),
                "job_id": receipt.job_id,
                "feature_id": receipt.feature_id,
                "source_media_hash": receipt.source_media_hash,
                "status": receipt.status.as_token(),
                "error_class": receipt.error_class,
                "output_artifact_id": receipt.output_artifact_id,
                "partial_artifact_id": receipt.partial_artifact_id,
                // Already-redacted reproducibility metadata.
                "tool_versions": receipt.tool_versions,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Fetch a receipt by its originating job id.
    pub async fn get_receipt_by_job(&self, job_id: &str) -> AtelierResult<Option<PipelineReceipt>> {
        let row = sqlx::query(&format!(
            r#"SELECT {RECEIPT_COLUMNS} FROM atelier_transcript_receipt
               WHERE job_id = $1"#
        ))
        .bind(job_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(receipt_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List all receipts bound to a given lineage `source_media_hash`, newest
    /// first; the auditable evidence trail for one media source (6.11.6/6.11.7).
    pub async fn list_receipts_for_source(
        &self,
        source_media_hash: &str,
    ) -> AtelierResult<Vec<PipelineReceipt>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {RECEIPT_COLUMNS} FROM atelier_transcript_receipt
               WHERE source_media_hash = $1
               ORDER BY emitted_at DESC"#
        ))
        .bind(source_media_hash)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(receipt_from_row).collect()
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
