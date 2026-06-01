//! Pure render + redaction for the per-session EXPORT (ROI #5).
//!
//! GOAL (governance glue): export a recorded session's consolidated record to a
//! portable, secret-REDACTED file — markdown for humans, json for machines — for
//! archive / handoff / sharing. The merge itself is NOT re-implemented here: the
//! command (`commands/session_transcript.rs`) builds the ordered
//! [`SessionTranscriptEntry`] list via the SAME aggregator
//! `build_response`/`merge_transcript` path that `kernel_session_transcript_get`
//! uses, then hands the entries + a small [`ExportHeader`] to this module.
//!
//! This file is the SHARED, Tauri-free piece (mirroring the search/aggregator
//! split): it owns the pure markdown+json render, the redaction pass, and the
//! safe-filename helper, so it is unit-testable in `cargo test -p handshake_core`
//! with no Tauri runtime. The IPC surface + IO glue (read/fetch/write) live in
//! the app crate.
//!
//! ## Secret safety (the load-bearing contract)
//!
//! Every emitted text field is routed through the SAME
//! [`SecretRedactor`](crate::terminal::redaction::SecretRedactor) the shipped
//! cross-session search already trusts (`make_snippet` -> `redact_command`), so
//! the export is exactly as safe as search — never a second, divergent secret
//! policy. `FrEvent`/`Process`/agent `detail` payloads are compact-stringified
//! (identical to `searchable_text`), redacted as one string, then re-parsed; on a
//! re-parse failure the redacted string is stored under a `_redacted_text`
//! envelope so a secret can NEVER leak through a parse fallback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::session_transcript::SessionTranscriptEntry;
use crate::terminal::redaction::SecretRedactor;

/// Output format selector. Maps 1:1 to the IPC `format` string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Markdown,
    Json,
    Both,
}

impl ExportFormat {
    /// Parse the IPC string form. Accepts `markdown`/`md`, `json`, `both`
    /// (case-insensitive, trimmed). Unknown strings yield `None` so the caller
    /// rejects honestly rather than guessing.
    pub fn from_ipc(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "markdown" | "md" => Some(Self::Markdown),
            "json" => Some(Self::Json),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    pub fn wants_markdown(self) -> bool {
        matches!(self, Self::Markdown | Self::Both)
    }

    pub fn wants_json(self) -> bool {
        matches!(self, Self::Json | Self::Both)
    }
}

/// Per-lane entry counts for the export header table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCounts {
    pub chat: u64,
    pub fr: u64,
    pub terminal: u64,
    pub process: u64,
}

/// The small, Tauri-free header the command builds from the session summary and
/// passes into [`render`]. Keeping this local means `export.rs` does NOT depend
/// on the app crate's `SessionSummary`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportHeader {
    pub session_id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<DateTime<Utc>>,
    pub counts: ExportCounts,
}

/// The rendered output(s) + redaction telemetry. `markdown`/`json` are `Some`
/// only for the requested format(s).
#[derive(Clone, Debug)]
pub struct RenderedExport {
    pub markdown: Option<String>,
    pub json: Option<String>,
    /// Total emitted text fields that matched a secret pattern and were masked.
    /// The honest "N secrets redacted" telemetry — never the secret itself.
    pub redacted_field_count: u64,
}

/// The export JSON schema version. Bumped if the wrapper shape changes.
pub const EXPORT_SCHEMA_VERSION: u32 = 1;

/// The mask the markdown banner advertises (matches the `PatternRedactor` token).
const REDACTION_TOKEN: &str = "***REDACTED***";

/// Render an export from already-merged transcript entries + a header.
///
/// `redactor` is passed by `&dyn` so this stays decoupled from the concrete
/// `PatternRedactor` and tests can inject a spy to PROVE every emitted text field
/// is routed through the redactor (not only the ones that happen to match a
/// pattern).
///
/// Redaction is applied to a CLONE of each entry BEFORE emission; the original
/// entries are never mutated. The json render emits the redacted entries under
/// the existing `SessionTranscriptResponse`-shaped contract (camelCase, tagged
/// `kind`), so the json export is automatically validated by the serde-casing
/// gate that already exists for that struct.
pub fn render(
    entries: &[SessionTranscriptEntry],
    header: &ExportHeader,
    fmt: ExportFormat,
    redactor: &dyn SecretRedactor,
) -> RenderedExport {
    let exported_at = Utc::now();

    // Redact a clone of every entry up front so both renders share the SAME
    // redacted view and the redaction count is computed once.
    let mut redacted_field_count: u64 = 0;
    let redacted: Vec<SessionTranscriptEntry> = entries
        .iter()
        .map(|e| redact_entry(e, redactor, &mut redacted_field_count))
        .collect();

    let markdown = if fmt.wants_markdown() {
        Some(render_markdown(
            &redacted,
            header,
            exported_at,
            redacted_field_count,
        ))
    } else {
        None
    };

    let json = if fmt.wants_json() {
        Some(render_json(
            &redacted,
            header,
            exported_at,
            redacted_field_count,
        ))
    } else {
        None
    };

    RenderedExport {
        markdown,
        json,
        redacted_field_count,
    }
}

// ---------------------------------------------------------------------------
// Redaction pass (every emitted text field routed through the redactor)
// ---------------------------------------------------------------------------

/// Redact a single text field through the redactor, incrementing the counter
/// when the field matched a secret pattern. Returns the masked string.
fn redact_field(text: &str, redactor: &dyn SecretRedactor, count: &mut u64) -> String {
    let result = redactor.redact_command(text);
    if result.matched {
        *count += 1;
    }
    result.redacted
}

/// Redact an optional text field in place.
fn redact_opt(
    text: &Option<String>,
    redactor: &dyn SecretRedactor,
    count: &mut u64,
) -> Option<String> {
    text.as_ref().map(|t| redact_field(t, redactor, count))
}

/// Redact a JSON payload by compact-stringifying it (IDENTICAL to
/// `searchable_text`'s payload handling), redacting that string, then re-parsing.
/// On a re-parse failure the redacted string is stored under a `_redacted_text`
/// envelope so a secret can NEVER leak through a raw-value fallback.
fn redact_payload(payload: &Value, redactor: &dyn SecretRedactor, count: &mut u64) -> Value {
    // `null`/empty payloads carry nothing to redact.
    if payload.is_null() {
        return payload.clone();
    }
    let compact = serde_json::to_string(payload).unwrap_or_default();
    let result = redactor.redact_command(&compact);
    if !result.matched {
        // No secret matched: keep the original structured value untouched.
        return payload.clone();
    }
    *count += 1;
    // A secret WAS masked. Try to re-parse the masked compact JSON back to a
    // structured Value; on failure, NEVER fall back to the raw payload — wrap the
    // redacted string so the secret cannot leak.
    match serde_json::from_str::<Value>(&result.redacted) {
        Ok(parsed) => parsed,
        Err(_) => json!({ "_redacted_text": result.redacted }),
    }
}

/// Rebuild an entry with every text field redacted. The entry shape (variant,
/// `ts`, `seq`, ids) is preserved; only human/secret-bearing text is masked.
fn redact_entry(
    entry: &SessionTranscriptEntry,
    redactor: &dyn SecretRedactor,
    count: &mut u64,
) -> SessionTranscriptEntry {
    match entry {
        SessionTranscriptEntry::ChatTurn {
            ts,
            seq,
            role,
            model_role,
            content,
            message_id,
        } => SessionTranscriptEntry::ChatTurn {
            ts: *ts,
            seq: *seq,
            role: redact_field(role, redactor, count),
            model_role: redact_opt(model_role, redactor, count),
            content: redact_field(content, redactor, count),
            message_id: message_id.clone(),
        },
        SessionTranscriptEntry::FrEvent {
            ts,
            seq,
            event_type,
            fr_event,
            actor,
            model_id,
            payload,
            event_id,
        } => SessionTranscriptEntry::FrEvent {
            ts: *ts,
            seq: *seq,
            event_type: event_type.clone(),
            fr_event: fr_event.clone(),
            actor: actor.clone(),
            model_id: model_id.clone(),
            payload: redact_payload(payload, redactor, count),
            event_id: event_id.clone(),
        },
        SessionTranscriptEntry::AgentActivity {
            ts,
            seq,
            activity_kind,
            name,
            detail,
            text,
            event_id,
        } => SessionTranscriptEntry::AgentActivity {
            ts: *ts,
            seq: *seq,
            activity_kind: activity_kind.clone(),
            name: redact_opt(name, redactor, count),
            detail: detail
                .as_ref()
                .map(|d| redact_payload(d, redactor, count)),
            text: redact_opt(text, redactor, count),
            event_id: event_id.clone(),
        },
        SessionTranscriptEntry::TerminalChunk {
            ts,
            seq,
            terminal_session_id,
            fr_event,
            command,
            text,
        } => SessionTranscriptEntry::TerminalChunk {
            ts: *ts,
            seq: *seq,
            terminal_session_id: terminal_session_id.clone(),
            fr_event: fr_event.clone(),
            command: redact_opt(command, redactor, count),
            text: redact_opt(text, redactor, count),
        },
        SessionTranscriptEntry::Process {
            ts,
            seq,
            process_uuid,
            phase,
            model_id,
            payload,
        } => SessionTranscriptEntry::Process {
            ts: *ts,
            seq: *seq,
            process_uuid: process_uuid.clone(),
            phase: phase.clone(),
            model_id: model_id.clone(),
            payload: redact_payload(payload, redactor, count),
        },
    }
}

// ---------------------------------------------------------------------------
// Markdown render
// ---------------------------------------------------------------------------

fn fmt_ts(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339()
}

fn fmt_ts_opt(ts: &Option<DateTime<Utc>>) -> String {
    ts.map(|t| t.to_rfc3339()).unwrap_or_else(|| "—".to_string())
}

/// Compact-pretty a JSON value into a fenced ```json block body. Falls back to
/// the compact form on the (practically impossible) pretty-serialize error.
fn json_block(value: &Value) -> String {
    let body = serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| serde_json::to_string(value).unwrap_or_default());
    format!("```json\n{body}\n```")
}

fn render_markdown(
    entries: &[SessionTranscriptEntry],
    header: &ExportHeader,
    exported_at: DateTime<Utc>,
    redacted_field_count: u64,
) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Session Export — {}\n\n", header.session_id));

    // Header table.
    let provider_model = match (&header.provider, &header.model_id) {
        (Some(p), Some(m)) => format!("{p} · {m}"),
        (Some(p), None) => p.clone(),
        (None, Some(m)) => m.clone(),
        (None, None) => "—".to_string(),
    };
    out.push_str("| Field | Value |\n|---|---|\n");
    out.push_str(&format!("| Session ID | {} |\n", header.session_id));
    out.push_str(&format!("| Kind | {} |\n", header.kind));
    out.push_str(&format!("| Provider / Model | {provider_model} |\n"));
    out.push_str(&format!(
        "| Worktree | {} |\n",
        header.worktree_id.as_deref().unwrap_or("—")
    ));
    out.push_str(&format!(
        "| Started | {} |\n",
        fmt_ts_opt(&header.started_at)
    ));
    out.push_str(&format!(
        "| Last activity | {} |\n",
        fmt_ts_opt(&header.last_activity_at)
    ));
    out.push_str(&format!(
        "| Counts | chat {} · fr {} · terminal {} · process {} |\n",
        header.counts.chat, header.counts.fr, header.counts.terminal, header.counts.process
    ));
    out.push_str(&format!("| Secrets redacted | {redacted_field_count} |\n"));
    out.push_str(&format!("| Exported | {} |\n", fmt_ts(&exported_at)));
    out.push('\n');

    out.push_str(&format!(
        "> Secret-redacted export. Text matching secret patterns is masked as `{REDACTION_TOKEN}`. Redaction is pattern-based and not guaranteed exhaustive.\n\n"
    ));
    out.push_str("---\n\n");

    if entries.is_empty() {
        out.push_str("_No entries recorded for this session._\n");
        return out;
    }

    out.push_str(&format!("## Timeline ({} entries)\n\n", entries.len()));

    for entry in entries {
        render_entry_markdown(&mut out, entry);
        out.push('\n');
    }

    out
}

/// Render one entry as a typed markdown section. Mirrors the panel's
/// `renderEntryBody` typing so the markdown matches what the operator saw.
fn render_entry_markdown(out: &mut String, entry: &SessionTranscriptEntry) {
    match entry {
        SessionTranscriptEntry::ChatTurn {
            ts,
            seq,
            role,
            model_role,
            content,
            ..
        } => {
            let role_label = match model_role {
                Some(m) if !m.is_empty() => format!("{role} ({m})"),
                _ => role.clone(),
            };
            out.push_str(&format!(
                "### [seq {seq}] {} · chat · {role_label}\n",
                fmt_ts(ts)
            ));
            out.push_str(content);
            out.push('\n');
        }
        SessionTranscriptEntry::AgentActivity {
            ts,
            seq,
            activity_kind,
            name,
            detail,
            text,
            ..
        } => {
            match activity_kind.as_str() {
                "tool_call" => {
                    let tool = name.as_deref().unwrap_or("tool");
                    out.push_str(&format!(
                        "### [seq {seq}] {} · agent · tool_call · {tool}\n",
                        fmt_ts(ts)
                    ));
                    if let Some(d) = detail {
                        out.push_str(&json_block(d));
                        out.push('\n');
                    }
                }
                "thinking" => {
                    out.push_str(&format!(
                        "### [seq {seq}] {} · agent · thinking\n",
                        fmt_ts(ts)
                    ));
                    if let Some(t) = text {
                        // Blockquote each line of the thought.
                        for line in t.lines() {
                            out.push_str(&format!("> {line}\n"));
                        }
                    }
                }
                other => {
                    out.push_str(&format!(
                        "### [seq {seq}] {} · agent · {other}\n",
                        fmt_ts(ts)
                    ));
                    if let Some(t) = text {
                        out.push_str(t);
                        out.push('\n');
                    }
                }
            }
        }
        SessionTranscriptEntry::TerminalChunk {
            ts,
            seq,
            terminal_session_id,
            command,
            text,
            ..
        } => {
            out.push_str(&format!(
                "### [seq {seq}] {} · terminal · {terminal_session_id}\n",
                fmt_ts(ts)
            ));
            if let Some(c) = command {
                out.push_str(&format!("`$ {c}`\n"));
            }
            if let Some(t) = text {
                out.push_str("```\n");
                out.push_str(t);
                if !t.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```\n");
            }
        }
        SessionTranscriptEntry::FrEvent {
            ts,
            seq,
            event_type,
            fr_event,
            actor,
            payload,
            ..
        } => {
            let tag = fr_event.as_deref().unwrap_or(event_type);
            out.push_str(&format!(
                "### [seq {seq}] {} · fr · {tag} · {actor}\n",
                fmt_ts(ts)
            ));
            if !payload.is_null() {
                out.push_str(&json_block(payload));
                out.push('\n');
            }
        }
        SessionTranscriptEntry::Process {
            ts,
            seq,
            process_uuid,
            phase,
            payload,
            ..
        } => {
            let uuid = process_uuid.as_deref().unwrap_or("—");
            out.push_str(&format!(
                "### [seq {seq}] {} · process · {phase} · {uuid}\n",
                fmt_ts(ts)
            ));
            if !payload.is_null() {
                out.push_str(&json_block(payload));
                out.push('\n');
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON render
// ---------------------------------------------------------------------------

/// Render the export JSON document: a `schemaVersion` + a `summary` (the header
/// + redaction telemetry + export time) + the `transcript` shaped exactly like
/// the existing `SessionTranscriptResponse` (entries already redacted).
fn render_json(
    entries: &[SessionTranscriptEntry],
    header: &ExportHeader,
    exported_at: DateTime<Utc>,
    redacted_field_count: u64,
) -> String {
    let summary = json!({
        "sessionId": header.session_id,
        "kind": header.kind,
        "provider": header.provider,
        "modelId": header.model_id,
        "worktreeId": header.worktree_id,
        "startedAt": header.started_at,
        "lastActivityAt": header.last_activity_at,
        "counts": header.counts,
        "redactedFieldCount": redacted_field_count,
        "exportedAt": exported_at,
    });

    // The transcript object reuses the camelCase `SessionTranscriptEntry` serde
    // (tag = `kind`). Entries are the REDACTED clones.
    let transcript = json!({
        "sessionId": header.session_id,
        "entries": entries,
        "truncated": false,
    });

    let doc = json!({
        "schemaVersion": EXPORT_SCHEMA_VERSION,
        "summary": summary,
        "transcript": transcript,
    });

    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

// ---------------------------------------------------------------------------
// Safe filename helper (no blank spaces, no path separators) [GLOBAL-NAMING]
// ---------------------------------------------------------------------------

/// Maximum length of the sanitized session-id stem in a filename.
const MAX_STEM_LEN: usize = 96;

/// Sanitize a session id into a path-safe, space-free filename stem.
///
/// Composite ids contain `#` (`<model_id>#<n>`), chat ids are UUIDs. Every char
/// that is not `[A-Za-z0-9._-]` (which includes spaces, `#`, and every path
/// separator `/ \ :`) is replaced with `-`; runs of `-` are collapsed; leading
/// and trailing `-` are trimmed; the result is capped at [`MAX_STEM_LEN`] and
/// falls back to `"session"` if empty. Path-safe on Windows (msvc) and POSIX.
pub fn safe_session_stem(session_id: &str) -> String {
    let mapped: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Collapse runs of '-'.
    let mut collapsed = String::with_capacity(mapped.len());
    let mut prev_dash = false;
    for c in mapped.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push('-');
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }

    let trimmed = collapsed.trim_matches('-');
    let mut stem: String = trimmed.chars().take(MAX_STEM_LEN).collect();
    // A truncation could leave a trailing '-'; trim again.
    let stem_trimmed = stem.trim_end_matches('-').to_string();
    stem = stem_trimmed;

    if stem.is_empty() {
        stem.push_str("session");
    }
    stem
}

/// Build the UTC timestamp suffix used in export filenames: `yyyymmddThhmmssZ`
/// (space-free, lexically sortable).
pub fn export_timestamp_suffix(now: DateTime<Utc>) -> String {
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_transcript::SessionTranscriptEntry;
    use crate::terminal::redaction::{RedactionResult, SecretRedactor};
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    /// A spy redactor that delegates to a real `PatternRedactor` but counts EVERY
    /// `redact_command` call, so a test can prove every emitted text field was
    /// routed through the redactor (not only the matching ones).
    struct SpyRedactor {
        calls: AtomicU64,
        inner: crate::terminal::redaction::PatternRedactor,
    }
    impl SpyRedactor {
        fn new() -> Self {
            Self {
                calls: AtomicU64::new(0),
                inner: crate::terminal::redaction::PatternRedactor,
            }
        }
        fn call_count(&self) -> u64 {
            self.calls.load(Ordering::SeqCst)
        }
    }
    impl SecretRedactor for SpyRedactor {
        fn redact_command(&self, command: &str) -> RedactionResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inner.redact_command(command)
        }
        fn redact_output(&self, stdout: &[u8], stderr: &[u8]) -> RedactionResult {
            self.inner.redact_output(stdout, stderr)
        }
    }

    fn redactor() -> crate::terminal::redaction::PatternRedactor {
        crate::terminal::redaction::PatternRedactor
    }

    fn header() -> ExportHeader {
        ExportHeader {
            session_id: "claude-sonnet#0".to_string(),
            kind: "swarm".to_string(),
            provider: Some("cloud".to_string()),
            model_id: Some("claude-sonnet".to_string()),
            worktree_id: Some("wt-recovery-1".to_string()),
            started_at: Some(at(100)),
            last_activity_at: Some(at(360)),
            counts: ExportCounts {
                chat: 1,
                fr: 2,
                terminal: 1,
                process: 1,
            },
        }
    }

    fn sample_entries() -> Vec<SessionTranscriptEntry> {
        vec![
            SessionTranscriptEntry::ChatTurn {
                ts: at(100),
                seq: 0,
                role: "user".to_string(),
                model_role: None,
                content: "Build handshake_core and report the gates.".to_string(),
                message_id: "m1".to_string(),
            },
            SessionTranscriptEntry::AgentActivity {
                ts: at(120),
                seq: 1,
                activity_kind: "thinking".to_string(),
                name: None,
                detail: None,
                text: Some("I'll compile the crate first".to_string()),
                event_id: "ev-think".to_string(),
            },
            SessionTranscriptEntry::AgentActivity {
                ts: at(140),
                seq: 2,
                activity_kind: "tool_call".to_string(),
                name: Some("Bash".to_string()),
                detail: Some(json!({ "command": "cargo build --lib" })),
                text: None,
                event_id: "ev-tool".to_string(),
            },
            SessionTranscriptEntry::TerminalChunk {
                ts: at(160),
                seq: 3,
                terminal_session_id: "term-1".to_string(),
                fr_event: Some("FR-EVT-TERMINAL-COMMAND-EXEC".to_string()),
                command: Some("cargo build --lib".to_string()),
                text: Some("compiling handshake_core ... ok".to_string()),
            },
            SessionTranscriptEntry::FrEvent {
                ts: at(180),
                seq: 4,
                event_type: "system".to_string(),
                fr_event: Some("FR-EVT-LLM-INFER-END".to_string()),
                actor: "agent".to_string(),
                model_id: Some("claude-sonnet".to_string()),
                payload: json!({ "tokens": 184, "request_id": "req-7" }),
                event_id: "ev-fr".to_string(),
            },
            SessionTranscriptEntry::Process {
                ts: at(200),
                seq: 5,
                process_uuid: Some("9f2a".to_string()),
                phase: "completed".to_string(),
                model_id: Some("claude-sonnet".to_string()),
                payload: json!({ "exit_code": 0 }),
            },
        ]
    }

    #[test]
    fn render_markdown_has_header_and_typed_sections() {
        let r = render(&sample_entries(), &header(), ExportFormat::Markdown, &redactor());
        let md = r.markdown.expect("markdown");
        // Header table fields.
        assert!(md.contains("# Session Export — claude-sonnet#0"));
        assert!(md.contains("| Kind | swarm |"));
        assert!(md.contains("| Provider / Model | cloud · claude-sonnet |"));
        assert!(md.contains("| Worktree | wt-recovery-1 |"));
        assert!(md.contains("| Counts | chat 1 · fr 2 · terminal 1 · process 1 |"));
        // Timeline + each typed section, in order, with timestamps + seq.
        assert!(md.contains("## Timeline (6 entries)"));
        assert!(md.contains("· chat · user"));
        assert!(md.contains("· agent · thinking"));
        assert!(md.contains("· agent · tool_call · Bash"));
        assert!(md.contains("· terminal · term-1"));
        assert!(md.contains("· fr · FR-EVT-LLM-INFER-END · agent"));
        assert!(md.contains("· process · completed · 9f2a"));
        // Timestamps rendered rfc3339.
        assert!(md.contains(&at(100).to_rfc3339()));
        // Sections appear in seq order.
        let chat_pos = md.find("· chat · user").unwrap();
        let proc_pos = md.find("· process · completed").unwrap();
        assert!(chat_pos < proc_pos, "sections must be in timeline order");
    }

    #[test]
    fn render_json_is_redacted_transcript_response() {
        let r = render(&sample_entries(), &header(), ExportFormat::Json, &redactor());
        let js = r.json.expect("json");
        let v: Value = serde_json::from_str(&js).expect("valid json");
        assert_eq!(v["schemaVersion"], json!(EXPORT_SCHEMA_VERSION));
        // Summary is camelCase.
        assert_eq!(v["summary"]["sessionId"], json!("claude-sonnet#0"));
        assert_eq!(v["summary"]["modelId"], json!("claude-sonnet"));
        assert_eq!(v["summary"]["counts"]["fr"], json!(2));
        // Transcript entries round-trip with the camelCase `kind`-tagged contract.
        let entries = v["transcript"]["entries"].as_array().expect("entries array");
        assert_eq!(entries.len(), 6);
        assert_eq!(entries[0]["kind"], json!("chat_turn"));
        // camelCase field keys present, snake_case absent.
        assert!(entries[4]["eventType"].is_string());
        assert!(entries[4].get("event_type").is_none());
    }

    #[test]
    fn render_redacts_every_text_field() {
        // Each lane carries a secret in a text field.
        let secret = "supersecretvalue123";
        let entries = vec![
            SessionTranscriptEntry::ChatTurn {
                ts: at(1),
                seq: 0,
                role: "user".to_string(),
                model_role: None,
                content: format!("export API_KEY={secret} now"),
                message_id: "m".to_string(),
            },
            SessionTranscriptEntry::AgentActivity {
                ts: at(2),
                seq: 1,
                activity_kind: "text".to_string(),
                name: None,
                detail: None,
                text: Some(format!("token={secret}")),
                event_id: "e".to_string(),
            },
            SessionTranscriptEntry::TerminalChunk {
                ts: at(3),
                seq: 2,
                terminal_session_id: "t".to_string(),
                fr_event: None,
                command: Some(format!("run --password={secret}")),
                text: Some(format!("secret={secret}")),
            },
            SessionTranscriptEntry::FrEvent {
                ts: at(4),
                seq: 3,
                event_type: "system".to_string(),
                fr_event: None,
                actor: "agent".to_string(),
                model_id: None,
                payload: json!({ "env": format!("API_KEY={secret}") }),
                event_id: "e2".to_string(),
            },
            SessionTranscriptEntry::Process {
                ts: at(5),
                seq: 4,
                process_uuid: Some("p".to_string()),
                phase: "spawned".to_string(),
                model_id: None,
                payload: json!({ "cmd": format!("token={secret}") }),
            },
        ];

        let spy = SpyRedactor::new();
        let r = render(&entries, &header(), ExportFormat::Both, &spy);
        let md = r.markdown.expect("md");
        let js = r.json.expect("js");

        // The raw secret never appears in EITHER output.
        assert!(!md.contains(secret), "secret leaked into markdown");
        assert!(!js.contains(secret), "secret leaked into json");
        assert!(md.contains(REDACTION_TOKEN), "mask present in markdown");
        assert!(js.contains("REDACTED"), "mask present in json");

        // 6 matching text fields: content, text, command, terminal-text,
        // fr payload, process payload.
        assert_eq!(r.redacted_field_count, 6, "every secret-bearing field counted");

        // The spy proves EVERY emitted text field was routed through the redactor,
        // not only the matching ones. Fields routed (per redact_entry):
        //   chat: role, content                       = 2
        //   agent(text): text                         = 1
        //   terminal: command, text                   = 2
        //   fr: payload                               = 1
        //   process: payload                          = 1
        // model_role/name/detail are None -> not routed. Total = 7.
        assert_eq!(spy.call_count(), 7, "all present text fields routed through redactor");
    }

    #[test]
    fn payload_redaction_that_breaks_json_uses_envelope_not_raw() {
        // A secret whose masked form would BREAK json re-parse (an unquoted
        // numeric value redacted to a non-numeric token) must fall to the
        // `_redacted_text` envelope, NEVER the raw value.
        let entries = vec![SessionTranscriptEntry::FrEvent {
            ts: at(1),
            seq: 0,
            event_type: "system".to_string(),
            fr_event: None,
            actor: "agent".to_string(),
            model_id: None,
            // The `[A-Z0-9_]{3,}=value` pattern matches `TOKEN=abc...`; here the
            // value sits where the compact json is `{"TOKEN":"abc=def123"}`.
            payload: json!({ "TOKEN": "abc=supersecretdef123" }),
            event_id: "e".to_string(),
        }];
        let r = render(&entries, &header(), ExportFormat::Json, &redactor());
        let js = r.json.expect("js");
        assert!(!js.contains("supersecretdef123"), "secret must not leak");
        assert!(js.contains("REDACTED"));
    }

    #[test]
    fn render_empty_session_is_valid_not_error() {
        let r = render(&[], &header(), ExportFormat::Both, &redactor());
        let md = r.markdown.expect("md");
        let js = r.json.expect("js");
        assert!(md.contains("_No entries recorded for this session._"));
        assert!(md.contains("# Session Export — claude-sonnet#0"));
        let v: Value = serde_json::from_str(&js).unwrap();
        assert_eq!(
            v["transcript"]["entries"].as_array().unwrap().len(),
            0
        );
        assert_eq!(r.redacted_field_count, 0);
    }

    #[test]
    fn safe_session_stem_strips_separators_and_spaces() {
        assert_eq!(safe_session_stem("claude-sonnet#0"), "claude-sonnet-0");
        // Path separators + spaces all map to '-' and collapse.
        let s = safe_session_stem(r"a/b\c:d e");
        assert!(!s.contains(' '));
        assert!(!s.contains('/'));
        assert!(!s.contains('\\'));
        assert!(!s.contains(':'));
        assert_eq!(s, "a-b-c-d-e");
        // Empty / all-separator -> fallback.
        assert_eq!(safe_session_stem(""), "session");
        assert_eq!(safe_session_stem("#"), "session");
        assert_eq!(safe_session_stem("///"), "session");
        // A UUID is preserved (alphanumerics + '-').
        let uuid = "0192a000-0000-7000-8000-000000000001";
        assert_eq!(safe_session_stem(uuid), uuid);
    }

    #[test]
    fn safe_session_stem_caps_length() {
        let long = "x".repeat(300);
        let stem = safe_session_stem(&long);
        assert!(stem.len() <= MAX_STEM_LEN);
    }

    #[test]
    fn export_timestamp_suffix_is_space_free() {
        let s = export_timestamp_suffix(at(0));
        assert!(!s.contains(' '));
        assert!(s.ends_with('Z'));
        assert_eq!(s, "19700101T000000Z");
    }

    #[test]
    fn export_format_from_ipc_parses_known_rejects_unknown() {
        assert_eq!(ExportFormat::from_ipc("markdown"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::from_ipc("MD"), Some(ExportFormat::Markdown));
        assert_eq!(ExportFormat::from_ipc(" json "), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_ipc("both"), Some(ExportFormat::Both));
        assert_eq!(ExportFormat::from_ipc("bogus"), None);
    }
}
