//! WP-KERNEL-004 (ROI follow-up): structured agent-activity parser for the
//! official-CLI bridge.
//!
//! GOAL: capture the operator's "all toolcalls, visible thought processes" as
//! STRUCTURED typed records — not just raw stdout text. When the bridged CLI is
//! run in a JSON-stream mode (`claude --output-format stream-json`,
//! `codex exec --json`), each stdout line is a self-contained JSON event. This
//! module turns ONE such line (+ the [`CliKind`] dialect) into 0..N typed
//! [`AgentActivity`] records that the CLI runtime emits as `FR-EVT-AGENT-*`
//! Flight-Recorder events, which the session-transcript aggregator then surfaces
//! as `SessionTranscriptEntry::AgentActivity` rows.
//!
//! ## Defensive contract (the load-bearing rule)
//!
//! Parsing is BEST-EFFORT and LOSSLESS:
//!
//! - [`parse_line`] NEVER returns `Err` and NEVER panics.
//! - A malformed-JSON line or a non-JSON line becomes exactly one
//!   [`AgentActivity::Other`] carrying the raw line verbatim — never dropped.
//! - An empty / whitespace-only line yields zero activities (nothing to capture).
//! - Pure CLI lifecycle envelopes (claude `system`/`result`, codex
//!   `thread.*`/`turn.*`) are SKIPPED — they are already represented by the
//!   `FR-EVT-LLM-INFER-{START,END}` lifecycle events, so re-emitting them would
//!   double the timeline. This is the ONLY case where a non-empty line yields no
//!   activity.
//! - A recognised event with an unexpected shape degrades to `Other` rather than
//!   guessing — honest, never fabricated.
//!
//! The raw `GeneratedToken` stdout path is unaffected: structured parsing is
//! purely additive (the runtime continues to stream the raw bytes to the
//! terminal/capture sink exactly as before; see `cli_bridge_runtime.rs`).
//!
//! ## Research basis (CLI JSON event shapes — current as of 2026-05-31)
//!
//! Claude Code `claude -p --output-format stream-json` (newline-delimited JSON):
//!   - Top-level `type`: `system`(subtype `init`), `assistant`, `user`, `result`.
//!   - `assistant` carries `message.content[]` blocks:
//!       text:      `{"type":"text","text":"…"}`
//!       tool use:  `{"type":"tool_use","id":"toolu_…","name":"Bash","input":{…}}`
//!       thinking:  `{"type":"thinking","thinking":"…","signature":"…"}`
//!         (the text field is `thinking`, NOT `text`).
//!   - `user` carries `message.content[]` with
//!       `{"type":"tool_result","tool_use_id":"toolu_…","content":…}`.
//!   - Honesty caveat (claude-code issue #20127): since v2.1.8 stream-json may
//!     OMIT `thinking` blocks. The parser simply emits fewer Thinking rows then —
//!     never fabricated. The raw-fallback path stays intact.
//!
//! Codex CLI `codex exec --json` (JSONL):
//!   - Envelopes: `thread.started`, `turn.started`, `turn.completed`, and item
//!     lifecycle `item.started|item.updated|item.completed` carrying `item`.
//!   - `item.type`: `agent_message`(`text`), `reasoning`(`text`),
//!     `command_execution`(`command`,`status`), `file_change`, `mcp_tool_call`,
//!     `web_search`.
//!   - We emit on `item.completed` (and `item.started` for `command_execution` so
//!     a long-running command shows immediately) to avoid double rows from
//!     `item.updated`.

use serde_json::Value;

use super::official_cli_bridge::CliKind;

/// One structured agent-activity record parsed from a single CLI JSONL line.
///
/// An unrecognised / malformed line never panics and is never dropped — it
/// becomes [`AgentActivity::Other`] (carrying the raw line) so capture is
/// lossless.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentActivity {
    /// A tool / command invocation. `input` is the (later redacted) argument
    /// object; `call_id` is the CLI's correlation id when present.
    ToolCall {
        name: String,
        input: Value,
        call_id: Option<String>,
    },
    /// A visible thought process (claude `thinking`, codex `reasoning`).
    Thinking { text: String },
    /// Operator-visible assistant text (claude `text`, codex `agent_message`,
    /// and claude `tool_result` bodies rendered as `[tool_result] …`).
    Text { text: String },
    /// An unknown JSON line OR a non-JSON line, kept verbatim. The defensive
    /// fallback that guarantees nothing is ever lost.
    Other { raw: String },
}

/// The coarse kind of an [`AgentActivity`], used to pick the `FR-EVT-AGENT-*`
/// event id and the transcript classify label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentActivityKind {
    ToolCall,
    Thinking,
    Text,
    Other,
}

impl AgentActivity {
    /// The coarse kind discriminant.
    pub fn kind(&self) -> AgentActivityKind {
        match self {
            AgentActivity::ToolCall { .. } => AgentActivityKind::ToolCall,
            AgentActivity::Thinking { .. } => AgentActivityKind::Thinking,
            AgentActivity::Text { .. } => AgentActivityKind::Text,
            AgentActivity::Other { .. } => AgentActivityKind::Other,
        }
    }
}

impl AgentActivityKind {
    /// The transcript `activity_kind` string the aggregator stamps on the row
    /// (and the frontend keys on).
    pub fn label(self) -> &'static str {
        match self {
            AgentActivityKind::ToolCall => "tool_call",
            AgentActivityKind::Thinking => "thinking",
            AgentActivityKind::Text => "text",
            AgentActivityKind::Other => "other",
        }
    }
}

/// Parse ONE stdout line (already newline-split, no trailing `\n`) for the given
/// dialect. Returns 0..N activities. NEVER returns `Err`; a parse miss yields a
/// single [`AgentActivity::Other`]. An empty / whitespace-only line yields an
/// empty vec.
pub fn parse_line(kind: CliKind, line: &str) -> Vec<AgentActivity> {
    if line.trim().is_empty() {
        return Vec::new();
    }
    match kind {
        CliKind::ClaudeCode => parse_claude_line(line),
        CliKind::CodexCli => parse_codex_line(line),
        CliKind::GeminiCli | CliKind::Other => parse_generic_line(line),
    }
}

/// Decode a line to JSON; on failure return the lossless `Other` fallback so the
/// caller can early-return.
fn json_or_other(line: &str) -> Result<Value, Vec<AgentActivity>> {
    match serde_json::from_str::<Value>(line) {
        Ok(v) => Ok(v),
        Err(_) => Err(vec![AgentActivity::Other {
            raw: line.to_string(),
        }]),
    }
}

/// Claude Code `--output-format stream-json` dialect.
fn parse_claude_line(line: &str) -> Vec<AgentActivity> {
    let value = match json_or_other(line) {
        Ok(v) => v,
        Err(fallback) => return fallback,
    };
    let ty = value.get("type").and_then(Value::as_str).unwrap_or("");
    match ty {
        "assistant" => claude_message_blocks(&value),
        "user" => claude_message_blocks(&value),
        // `system`(init) and `result` are pure lifecycle — already covered by the
        // FR-INFER START/END events. Skip to avoid doubling the timeline.
        "system" | "result" => Vec::new(),
        // `stream_event` partial-message deltas: coalesce the delta text into the
        // matching kind when present (best-effort; absent in non-partial mode).
        "stream_event" => claude_stream_event(&value),
        // Unknown top-level type with no recognised shape: keep verbatim.
        _ => vec![AgentActivity::Other {
            raw: line.to_string(),
        }],
    }
}

/// Walk a claude `message.content[]` array into typed activities. A single line
/// with multiple content blocks yields multiple activities (in block order).
fn claude_message_blocks(value: &Value) -> Vec<AgentActivity> {
    let blocks = value
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array);
    let blocks = match blocks {
        Some(b) => b,
        None => return Vec::new(),
    };
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        let bty = block.get("type").and_then(Value::as_str).unwrap_or("");
        match bty {
            "text" => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    out.push(AgentActivity::Text {
                        text: text.to_string(),
                    });
                }
            }
            "thinking" => {
                // The field holding the thought text is `thinking`, NOT `text`.
                if let Some(text) = block.get("thinking").and_then(Value::as_str) {
                    out.push(AgentActivity::Thinking {
                        text: text.to_string(),
                    });
                }
            }
            "tool_use" => {
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("tool")
                    .to_string();
                let input = block.get("input").cloned().unwrap_or(Value::Null);
                let call_id = block
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                out.push(AgentActivity::ToolCall {
                    name,
                    input,
                    call_id,
                });
            }
            "tool_result" => {
                // Render the result body as visible Text (prefixed) so results
                // are surfaced without a 5th variant. `content` is either a
                // string or an array of `{type:text,text}` blocks.
                let body = claude_tool_result_text(block);
                out.push(AgentActivity::Text {
                    text: format!("[tool_result] {body}"),
                });
            }
            // Unknown block type: do not guess — but do not drop either; keep a
            // verbatim Other of the block so the timeline still shows it.
            _ => out.push(AgentActivity::Other {
                raw: block.to_string(),
            }),
        }
    }
    out
}

/// Extract the text of a claude `tool_result` block, which may be a plain string
/// or an array of `{type:"text","text":"…"}` blocks.
fn claude_tool_result_text(block: &Value) -> String {
    match block.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Best-effort coalesce of a claude `stream_event` partial-message delta. Maps a
/// `content_block_delta` carrying `text_delta`/`thinking_delta` to Text/Thinking.
fn claude_stream_event(value: &Value) -> Vec<AgentActivity> {
    let delta = value
        .get("event")
        .and_then(|e| e.get("delta"))
        .or_else(|| value.get("delta"));
    let delta = match delta {
        Some(d) => d,
        None => return Vec::new(),
    };
    let dty = delta.get("type").and_then(Value::as_str).unwrap_or("");
    match dty {
        "text_delta" => delta
            .get("text")
            .and_then(Value::as_str)
            .map(|t| {
                vec![AgentActivity::Text {
                    text: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        "thinking_delta" => delta
            .get("thinking")
            .and_then(Value::as_str)
            .map(|t| {
                vec![AgentActivity::Thinking {
                    text: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Codex CLI `codex exec --json` dialect.
fn parse_codex_line(line: &str) -> Vec<AgentActivity> {
    let value = match json_or_other(line) {
        Ok(v) => v,
        Err(fallback) => return fallback,
    };
    let ty = value.get("type").and_then(Value::as_str).unwrap_or("");
    // Only item lifecycle events carry agent activity. Envelope events
    // (thread.*, turn.*) are lifecycle — skip (FR-INFER covers them).
    if !ty.starts_with("item.") {
        return Vec::new();
    }
    let item = match value.get("item") {
        Some(item) => item,
        None => return Vec::new(),
    };
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");

    // Emit on item.completed; additionally emit on item.started ONLY for
    // command_execution so a long-running command appears immediately. This
    // avoids double rows from item.updated.
    let emit = ty == "item.completed"
        || (ty == "item.started" && item_type == "command_execution");
    if !emit {
        return Vec::new();
    }

    match item_type {
        "agent_message" => item
            .get("text")
            .and_then(Value::as_str)
            .map(|t| {
                vec![AgentActivity::Text {
                    text: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        "reasoning" => item
            .get("text")
            .and_then(Value::as_str)
            .map(|t| {
                vec![AgentActivity::Thinking {
                    text: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        "command_execution" => {
            let mut input = serde_json::Map::new();
            if let Some(cmd) = item.get("command") {
                input.insert("command".to_string(), cmd.clone());
            }
            if let Some(status) = item.get("status") {
                input.insert("status".to_string(), status.clone());
            }
            vec![AgentActivity::ToolCall {
                name: "command_execution".to_string(),
                input: Value::Object(input),
                call_id: item.get("id").and_then(Value::as_str).map(str::to_string),
            }]
        }
        // mcp_tool_call / web_search / file_change: carry the whole item as input.
        "mcp_tool_call" | "web_search" | "file_change" => {
            vec![AgentActivity::ToolCall {
                name: item_type.to_string(),
                input: item.clone(),
                call_id: item.get("id").and_then(Value::as_str).map(str::to_string),
            }]
        }
        // Unknown item type with a recognised lifecycle envelope: keep verbatim.
        _ => vec![AgentActivity::Other {
            raw: line.to_string(),
        }],
    }
}

/// Generic best-effort dialect for Gemini / Other CLIs (and any unknown future
/// CLI). Probes for common keys; anything unrecognised → `Other{raw}`.
fn parse_generic_line(line: &str) -> Vec<AgentActivity> {
    let value = match json_or_other(line) {
        Ok(v) => v,
        Err(fallback) => return fallback,
    };

    // A tool-ish object: `name`/`tool` + `input`/`arguments`.
    let tool_name = value
        .get("name")
        .or_else(|| value.get("tool"))
        .and_then(Value::as_str);
    let tool_input = value.get("input").or_else(|| value.get("arguments"));
    if let (Some(name), Some(input)) = (tool_name, tool_input) {
        return vec![AgentActivity::ToolCall {
            name: name.to_string(),
            input: input.clone(),
            call_id: value.get("id").and_then(Value::as_str).map(str::to_string),
        }];
    }

    // A thinking-ish object: `thinking`/`reasoning` text.
    if let Some(text) = value
        .get("thinking")
        .or_else(|| value.get("reasoning"))
        .and_then(Value::as_str)
    {
        return vec![AgentActivity::Thinking {
            text: text.to_string(),
        }];
    }

    // A text-ish object: a `text` field.
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        return vec![AgentActivity::Text {
            text: text.to_string(),
        }];
    }

    // Recognised JSON but no known shape: keep verbatim (never dropped).
    vec![AgentActivity::Other {
        raw: line.to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_line_yields_no_activity() {
        assert!(parse_line(CliKind::ClaudeCode, "").is_empty());
        assert!(parse_line(CliKind::ClaudeCode, "   \t ").is_empty());
        assert!(parse_line(CliKind::CodexCli, "").is_empty());
    }

    #[test]
    fn claude_assistant_text_thinking_tooluse_yields_three_typed() {
        let line = json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "hello operator"},
                    {"type": "thinking", "thinking": "let me reason", "signature": "sig"},
                    {"type": "tool_use", "id": "toolu_1", "name": "Bash",
                     "input": {"command": "ls -la"}}
                ]
            }
        })
        .to_string();
        let acts = parse_line(CliKind::ClaudeCode, &line);
        assert_eq!(acts.len(), 3, "three blocks -> three activities: {acts:?}");
        assert_eq!(
            acts[0],
            AgentActivity::Text {
                text: "hello operator".to_string()
            }
        );
        assert_eq!(
            acts[1],
            AgentActivity::Thinking {
                text: "let me reason".to_string()
            }
        );
        match &acts[2] {
            AgentActivity::ToolCall {
                name,
                input,
                call_id,
            } => {
                assert_eq!(name, "Bash");
                assert_eq!(input.get("command").unwrap(), "ls -la");
                assert_eq!(call_id.as_deref(), Some("toolu_1"));
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn claude_user_tool_result_becomes_text() {
        let line = json!({
            "type": "user",
            "message": {
                "content": [
                    {"type": "tool_result", "tool_use_id": "toolu_1",
                     "content": "total 8\ndrwxr-xr-x"}
                ]
            }
        })
        .to_string();
        let acts = parse_line(CliKind::ClaudeCode, &line);
        assert_eq!(acts.len(), 1);
        assert_eq!(
            acts[0],
            AgentActivity::Text {
                text: "[tool_result] total 8\ndrwxr-xr-x".to_string()
            }
        );
    }

    #[test]
    fn claude_tool_result_content_array_is_joined() {
        let line = json!({
            "type": "user",
            "message": {
                "content": [
                    {"type": "tool_result", "tool_use_id": "t",
                     "content": [{"type":"text","text":"a"},{"type":"text","text":"b"}]}
                ]
            }
        })
        .to_string();
        let acts = parse_line(CliKind::ClaudeCode, &line);
        assert_eq!(
            acts[0],
            AgentActivity::Text {
                text: "[tool_result] ab".to_string()
            }
        );
    }

    #[test]
    fn claude_system_and_result_are_skipped_lifecycle() {
        let sys = json!({"type":"system","subtype":"init","session_id":"s"}).to_string();
        let res = json!({"type":"result","subtype":"success","result":"done"}).to_string();
        assert!(parse_line(CliKind::ClaudeCode, &sys).is_empty());
        assert!(parse_line(CliKind::ClaudeCode, &res).is_empty());
    }

    #[test]
    fn claude_stream_event_delta_coalesces() {
        let td = json!({
            "type":"stream_event",
            "event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}
        })
        .to_string();
        assert_eq!(
            parse_line(CliKind::ClaudeCode, &td),
            vec![AgentActivity::Text { text: "hi".into() }]
        );
        let thd = json!({
            "type":"stream_event",
            "event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"hm"}}
        })
        .to_string();
        assert_eq!(
            parse_line(CliKind::ClaudeCode, &thd),
            vec![AgentActivity::Thinking { text: "hm".into() }]
        );
    }

    #[test]
    fn codex_agent_message_reasoning_command_yield_typed() {
        let msg = json!({
            "type":"item.completed",
            "item":{"id":"item_1","type":"agent_message","text":"done thinking"}
        })
        .to_string();
        assert_eq!(
            parse_line(CliKind::CodexCli, &msg),
            vec![AgentActivity::Text {
                text: "done thinking".into()
            }]
        );

        let reasoning = json!({
            "type":"item.completed",
            "item":{"id":"item_2","type":"reasoning","text":"step by step"}
        })
        .to_string();
        assert_eq!(
            parse_line(CliKind::CodexCli, &reasoning),
            vec![AgentActivity::Thinking {
                text: "step by step".into()
            }]
        );

        let cmd = json!({
            "type":"item.completed",
            "item":{"id":"item_3","type":"command_execution",
                    "command":"cargo build","status":"completed"}
        })
        .to_string();
        let acts = parse_line(CliKind::CodexCli, &cmd);
        assert_eq!(acts.len(), 1);
        match &acts[0] {
            AgentActivity::ToolCall { name, input, .. } => {
                assert_eq!(name, "command_execution");
                assert_eq!(input.get("command").unwrap(), "cargo build");
                assert_eq!(input.get("status").unwrap(), "completed");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn codex_command_started_emits_started_others_do_not() {
        // command_execution started -> emitted (long command shows immediately).
        let started = json!({
            "type":"item.started",
            "item":{"id":"i","type":"command_execution","command":"sleep 10","status":"running"}
        })
        .to_string();
        assert_eq!(parse_line(CliKind::CodexCli, &started).len(), 1);

        // agent_message started -> NOT emitted (only completed), avoids double row.
        let started_msg = json!({
            "type":"item.started",
            "item":{"id":"i","type":"agent_message","text":"partial"}
        })
        .to_string();
        assert!(parse_line(CliKind::CodexCli, &started_msg).is_empty());

        // item.updated never emits.
        let updated = json!({
            "type":"item.updated",
            "item":{"id":"i","type":"agent_message","text":"more"}
        })
        .to_string();
        assert!(parse_line(CliKind::CodexCli, &updated).is_empty());
    }

    #[test]
    fn codex_envelopes_are_skipped() {
        for env in [
            json!({"type":"thread.started","thread_id":"t"}).to_string(),
            json!({"type":"turn.started"}).to_string(),
            json!({"type":"turn.completed","usage":{"input_tokens":1}}).to_string(),
        ] {
            assert!(parse_line(CliKind::CodexCli, &env).is_empty(), "{env}");
        }
    }

    #[test]
    fn codex_mcp_and_web_search_become_toolcalls() {
        let mcp = json!({
            "type":"item.completed",
            "item":{"id":"i","type":"mcp_tool_call","server":"fs","tool":"read"}
        })
        .to_string();
        match &parse_line(CliKind::CodexCli, &mcp)[0] {
            AgentActivity::ToolCall { name, .. } => assert_eq!(name, "mcp_tool_call"),
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_line_becomes_other_never_dropped() {
        let raw = "{not valid json at all";
        for kind in [CliKind::ClaudeCode, CliKind::CodexCli, CliKind::GeminiCli, CliKind::Other] {
            let acts = parse_line(kind, raw);
            assert_eq!(acts.len(), 1, "{kind:?} must keep the line");
            assert_eq!(
                acts[0],
                AgentActivity::Other {
                    raw: raw.to_string()
                },
                "{kind:?} malformed -> verbatim Other"
            );
        }
    }

    #[test]
    fn non_json_line_becomes_other_never_dropped() {
        let raw = "plain stdout banner line from the CLI";
        let acts = parse_line(CliKind::ClaudeCode, raw);
        assert_eq!(
            acts,
            vec![AgentActivity::Other {
                raw: raw.to_string()
            }]
        );
    }

    #[test]
    fn generic_dialect_best_effort_probes() {
        // tool-ish
        let tool = json!({"name":"search","input":{"q":"rust"}}).to_string();
        match &parse_line(CliKind::GeminiCli, &tool)[0] {
            AgentActivity::ToolCall { name, .. } => assert_eq!(name, "search"),
            other => panic!("expected ToolCall, got {other:?}"),
        }
        // tool-ish with `arguments`
        let tool2 = json!({"tool":"calc","arguments":{"x":1}}).to_string();
        match &parse_line(CliKind::Other, &tool2)[0] {
            AgentActivity::ToolCall { name, .. } => assert_eq!(name, "calc"),
            other => panic!("expected ToolCall, got {other:?}"),
        }
        // thinking-ish
        let think = json!({"reasoning":"because"}).to_string();
        assert_eq!(
            parse_line(CliKind::GeminiCli, &think),
            vec![AgentActivity::Thinking {
                text: "because".into()
            }]
        );
        // text-ish
        let text = json!({"text":"hi there"}).to_string();
        assert_eq!(
            parse_line(CliKind::GeminiCli, &text),
            vec![AgentActivity::Text {
                text: "hi there".into()
            }]
        );
        // unknown shape but valid JSON -> Other
        let unknown = json!({"foo":"bar"}).to_string();
        assert_eq!(
            parse_line(CliKind::GeminiCli, &unknown),
            vec![AgentActivity::Other { raw: unknown }]
        );
    }

    #[test]
    fn thinkingless_claude_stream_still_yields_text_lossless() {
        // Mirrors claude-code #20127: a stream with NO thinking blocks must still
        // produce the Text rows and lose nothing.
        let line = json!({
            "type":"assistant",
            "message":{"content":[{"type":"text","text":"answer only"}]}
        })
        .to_string();
        assert_eq!(
            parse_line(CliKind::ClaudeCode, &line),
            vec![AgentActivity::Text {
                text: "answer only".into()
            }]
        );
    }

    #[test]
    fn activity_kind_labels_are_stable() {
        assert_eq!(AgentActivityKind::ToolCall.label(), "tool_call");
        assert_eq!(AgentActivityKind::Thinking.label(), "thinking");
        assert_eq!(AgentActivityKind::Text.label(), "text");
        assert_eq!(AgentActivityKind::Other.label(), "other");
    }
}
