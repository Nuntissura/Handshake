//! MT-088 MediaTranscriptIngestion: parse operator-provided transcript
//! artifacts (SRT, WebVTT, JSON) into time-coded cues.
//!
//! NO ASR: Handshake never runs speech recognition. "Media transcript
//! ingestion" means parsing transcript ARTIFACTS the operator provides
//! (caption files exported from their tooling). Cues become
//! `media_time`-anchored spans (spec 2.3.13.11 span anchor vocabulary).
//!
//! Malformed-cue policy: a malformed cue (bad timing line, negative range,
//! missing text) is recorded as a typed [`MalformedCue`] and parsing
//! CONTINUES — the engine then writes a `partial` receipt naming every lost
//! cue, never a silent success (WP constraint). A file that is not the
//! claimed format at all (e.g. VTT without the WEBVTT header) is a
//! `PARSE_ERROR` failure.
//!
//! Supported JSON shapes (documented for fixture authors):
//! * canonical: `{"cues": [{"start_ms": 0, "end_ms": 1000, "text": "..."}]}`
//! * whisper-style: `{"segments": [{"start": 0.0, "end": 1.0, "text": "..."}]}`

use serde::{Deserialize, Serialize};

use super::receipts::IngestionErrorClass;

/// Transcript artifact format.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptFormat {
    Srt,
    Vtt,
    Json,
}

impl TranscriptFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Json => "json",
        }
    }
}

/// One well-formed cue.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptCue {
    /// 0-based cue index in document order (well-formed cues only).
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// One cue the parser had to drop, with a typed reason.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MalformedCue {
    /// 1-based line where the cue block starts.
    pub line: u32,
    pub reason: String,
}

/// Parse outcome: cues + losses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptParse {
    pub format: TranscriptFormat,
    pub cues: Vec<TranscriptCue>,
    pub malformed: Vec<MalformedCue>,
}

/// Typed whole-file parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptParseError {
    pub class: IngestionErrorClass,
    pub detail: String,
}

/// Parse `HH:MM:SS,mmm` (SRT) / `[HH:]MM:SS.mmm` (VTT) into milliseconds.
fn parse_timestamp(raw: &str, comma: bool) -> Result<u64, String> {
    let raw = raw.trim();
    let (time, millis) = if comma {
        raw.split_once(',')
            .ok_or("missing ',' millisecond separator")?
    } else {
        raw.split_once('.')
            .ok_or("missing '.' millisecond separator")?
    };
    if millis.len() != 3 || !millis.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("milliseconds must be 3 digits, got '{millis}'"));
    }
    let millis: u64 = millis.parse().map_err(|_| "bad milliseconds")?;

    let parts: Vec<&str> = time.split(':').collect();
    let (h, m, s) = match parts.as_slice() {
        [h, m, s] => (
            h.parse::<u64>().map_err(|_| "bad hours")?,
            m.parse::<u64>().map_err(|_| "bad minutes")?,
            s.parse::<u64>().map_err(|_| "bad seconds")?,
        ),
        // VTT allows MM:SS.mmm.
        [m, s] if !comma => (
            0,
            m.parse::<u64>().map_err(|_| "bad minutes")?,
            s.parse::<u64>().map_err(|_| "bad seconds")?,
        ),
        _ => return Err(format!("bad time shape '{time}'")),
    };
    if m >= 60 || s >= 60 {
        return Err(format!("minutes/seconds out of range in '{time}'"));
    }
    Ok(((h * 60 + m) * 60 + s) * 1000 + millis)
}

/// Parse a cue timing line `<start> --> <end>[ settings]`.
fn parse_timing_line(line: &str, comma: bool) -> Result<(u64, u64), String> {
    let (start_raw, rest) = line.split_once("-->").ok_or("missing '-->'")?;
    // VTT allows cue settings after the end timestamp.
    let end_raw = rest
        .split_whitespace()
        .next()
        .ok_or("missing end timestamp")?;
    let start_ms = parse_timestamp(start_raw, comma).map_err(|e| format!("start: {e}"))?;
    let end_ms = parse_timestamp(end_raw, comma).map_err(|e| format!("end: {e}"))?;
    if end_ms < start_ms {
        return Err(format!("negative range: {start_ms}ms --> {end_ms}ms"));
    }
    Ok((start_ms, end_ms))
}

/// Strip `<...>` markup tags (VTT voice/styling) from cue text.
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Split text into blank-line-separated blocks, tracking start lines.
fn blocks_of(text: &str) -> Vec<(u32, Vec<&str>)> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut current_start = 1u32;
    for (idx, line) in text.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push((current_start, std::mem::take(&mut current)));
            }
        } else {
            if current.is_empty() {
                current_start = line_no;
            }
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push((current_start, current));
    }
    blocks
}

/// Parse an SRT artifact.
pub fn parse_srt(text: &str) -> Result<TranscriptParse, TranscriptParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let blocks = blocks_of(text);
    if blocks.is_empty() {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: "empty SRT artifact: no cue blocks".to_string(),
        });
    }

    let mut cues = Vec::new();
    let mut malformed = Vec::new();
    for (start_line, lines) in blocks {
        // Optional numeric index line, then the timing line.
        let mut iter = lines.iter();
        let first = *iter.next().expect("non-empty block");
        let timing_line = if first.trim().bytes().all(|b| b.is_ascii_digit()) {
            match iter.next() {
                Some(line) => *line,
                None => {
                    malformed.push(MalformedCue {
                        line: start_line,
                        reason: "cue block has an index but no timing line".to_string(),
                    });
                    continue;
                }
            }
        } else {
            first
        };
        let (start_ms, end_ms) = match parse_timing_line(timing_line, true) {
            Ok(range) => range,
            Err(reason) => {
                malformed.push(MalformedCue {
                    line: start_line,
                    reason: format!("bad timing line: {reason}"),
                });
                continue;
            }
        };
        let body: Vec<&str> = iter.copied().collect();
        let cue_text = strip_tags(&body.join("\n")).trim().to_string();
        if cue_text.is_empty() {
            malformed.push(MalformedCue {
                line: start_line,
                reason: "cue has no text".to_string(),
            });
            continue;
        }
        cues.push(TranscriptCue {
            index: cues.len() as u32,
            start_ms,
            end_ms,
            text: cue_text,
        });
    }

    if cues.is_empty() && !malformed.is_empty() {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: format!(
                "no well-formed cue in SRT artifact ({} malformed)",
                malformed.len()
            ),
        });
    }
    Ok(TranscriptParse {
        format: TranscriptFormat::Srt,
        cues,
        malformed,
    })
}

/// Parse a WebVTT artifact.
pub fn parse_vtt(text: &str) -> Result<TranscriptParse, TranscriptParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    if !header.trim_end().starts_with("WEBVTT") {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: "not a WebVTT artifact: missing WEBVTT header".to_string(),
        });
    }
    let body: String = lines.collect::<Vec<_>>().join("\n");

    let mut cues = Vec::new();
    let mut malformed = Vec::new();
    for (start_line, block_lines) in blocks_of(&body) {
        let first = block_lines[0].trim();
        // Skip metadata blocks.
        if first.starts_with("NOTE") || first.starts_with("STYLE") || first.starts_with("REGION") {
            continue;
        }
        // Optional cue identifier line before the timing line.
        let mut iter = block_lines.iter();
        let mut timing_line = *iter.next().expect("non-empty block");
        if !timing_line.contains("-->") {
            match iter.next() {
                Some(line) if line.contains("-->") => timing_line = *line,
                _ => {
                    malformed.push(MalformedCue {
                        // +1: the WEBVTT header line was consumed above.
                        line: start_line + 1,
                        reason: "cue block has no timing line".to_string(),
                    });
                    continue;
                }
            }
        }
        let (start_ms, end_ms) = match parse_timing_line(timing_line, false) {
            Ok(range) => range,
            Err(reason) => {
                malformed.push(MalformedCue {
                    line: start_line + 1,
                    reason: format!("bad timing line: {reason}"),
                });
                continue;
            }
        };
        let body_lines: Vec<&str> = iter.copied().collect();
        let cue_text = strip_tags(&body_lines.join("\n")).trim().to_string();
        if cue_text.is_empty() {
            malformed.push(MalformedCue {
                line: start_line + 1,
                reason: "cue has no text".to_string(),
            });
            continue;
        }
        cues.push(TranscriptCue {
            index: cues.len() as u32,
            start_ms,
            end_ms,
            text: cue_text,
        });
    }

    if cues.is_empty() && !malformed.is_empty() {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: format!(
                "no well-formed cue in VTT artifact ({} malformed)",
                malformed.len()
            ),
        });
    }
    Ok(TranscriptParse {
        format: TranscriptFormat::Vtt,
        cues,
        malformed,
    })
}

/// Parse a JSON transcript artifact (canonical `cues` or whisper-style
/// `segments` shape).
pub fn parse_json_transcript(text: &str) -> Result<TranscriptParse, TranscriptParseError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|err| TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: format!("invalid JSON transcript: {err}"),
        })?;

    let (entries, whisper_style) = if let Some(cues) = value.get("cues").and_then(|v| v.as_array())
    {
        (cues, false)
    } else if let Some(segments) = value.get("segments").and_then(|v| v.as_array()) {
        (segments, true)
    } else {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: "JSON transcript must carry a 'cues' or 'segments' array".to_string(),
        });
    };

    let mut cues = Vec::new();
    let mut malformed = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let text_field = entry.get("text").and_then(|v| v.as_str()).map(str::trim);
        let range = if whisper_style {
            let start = entry.get("start").and_then(|v| v.as_f64());
            let end = entry.get("end").and_then(|v| v.as_f64());
            match (start, end) {
                (Some(s), Some(e)) if e >= s && s >= 0.0 => {
                    Some(((s * 1000.0).round() as u64, (e * 1000.0).round() as u64))
                }
                _ => None,
            }
        } else {
            let start = entry.get("start_ms").and_then(|v| v.as_u64());
            let end = entry.get("end_ms").and_then(|v| v.as_u64());
            match (start, end) {
                (Some(s), Some(e)) if e >= s => Some((s, e)),
                _ => None,
            }
        };
        match (range, text_field) {
            (Some((start_ms, end_ms)), Some(text)) if !text.is_empty() => {
                cues.push(TranscriptCue {
                    index: cues.len() as u32,
                    start_ms,
                    end_ms,
                    text: text.to_string(),
                });
            }
            _ => malformed.push(MalformedCue {
                line: (idx + 1) as u32,
                reason: format!("entry {idx} lacks a valid time range or text"),
            }),
        }
    }

    if cues.is_empty() && !malformed.is_empty() {
        return Err(TranscriptParseError {
            class: IngestionErrorClass::ParseError,
            detail: format!(
                "no well-formed cue in JSON transcript ({} malformed)",
                malformed.len()
            ),
        });
    }
    Ok(TranscriptParse {
        format: TranscriptFormat::Json,
        cues,
        malformed,
    })
}

/// Dispatch by detected format: `.srt` / `.vtt` / `.json`-suffixed paths.
pub fn parse_transcript_artifact(
    relative_path: &str,
    text: &str,
) -> Result<TranscriptParse, TranscriptParseError> {
    let lower = relative_path.to_ascii_lowercase();
    if lower.ends_with(".srt") {
        parse_srt(text)
    } else if lower.ends_with(".vtt") {
        parse_vtt(text)
    } else if lower.ends_with(".json") {
        parse_json_transcript(text)
    } else {
        Err(TranscriptParseError {
            class: IngestionErrorClass::UnsupportedFormat,
            detail: format!("unknown transcript artifact extension: {relative_path}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRT: &str = "1\n00:00:01,000 --> 00:00:04,000\nFirst cue line one\nline two\n\n2\n00:00:04,500 --> 00:00:06,000\nSecond cue\n";

    #[test]
    fn srt_parses_cues_with_milliseconds() {
        let parse = parse_srt(SRT).expect("well-formed SRT");
        assert_eq!(parse.cues.len(), 2);
        assert!(parse.malformed.is_empty());
        assert_eq!(parse.cues[0].start_ms, 1000);
        assert_eq!(parse.cues[0].end_ms, 4000);
        assert_eq!(parse.cues[0].text, "First cue line one\nline two");
        assert_eq!(parse.cues[1].index, 1);
    }

    #[test]
    fn srt_malformed_cues_are_recorded_and_parsing_continues() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nGood cue\n\n2\nnot a timing line\nLost cue\n\n3\n00:00:09,000 --> 00:00:08,000\nNegative range\n\n4\n00:00:10,000 --> 00:00:11,000\nLast good cue\n";
        let parse = parse_srt(srt).expect("partial SRT");
        assert_eq!(parse.cues.len(), 2);
        assert_eq!(parse.malformed.len(), 2);
        assert!(parse.malformed[0].reason.contains("bad timing line"));
        assert!(parse.malformed[1].reason.contains("negative range"));
        assert_eq!(parse.cues[1].text, "Last good cue");
    }

    #[test]
    fn vtt_requires_header_and_supports_short_timestamps_and_tags() {
        let vtt = "WEBVTT\n\nNOTE this is a comment\n\nintro\n00:01.000 --> 00:04.000 position:10%\n<v Speaker>Hello <b>world</b>\n\n00:00:05.000 --> 00:00:06.000\nSecond\n";
        let parse = parse_vtt(vtt).expect("well-formed VTT");
        assert_eq!(parse.cues.len(), 2);
        assert_eq!(parse.cues[0].start_ms, 1000);
        assert_eq!(parse.cues[0].text, "Hello world");

        let err = parse_vtt("1\n00:00:01.000 --> 00:00:02.000\nNo header\n")
            .expect_err("missing WEBVTT header must fail");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
    }

    #[test]
    fn json_transcripts_accept_canonical_and_whisper_shapes() {
        let canonical = r#"{"cues": [{"start_ms": 0, "end_ms": 1500, "text": "hi"}]}"#;
        let parse = parse_json_transcript(canonical).expect("canonical shape");
        assert_eq!(parse.cues[0].end_ms, 1500);

        let whisper = r#"{"segments": [{"start": 0.5, "end": 2.25, "text": "hello"}]}"#;
        let parse = parse_json_transcript(whisper).expect("whisper shape");
        assert_eq!(parse.cues[0].start_ms, 500);
        assert_eq!(parse.cues[0].end_ms, 2250);

        let mixed = r#"{"cues": [{"start_ms": 0, "end_ms": 1, "text": "ok"}, {"start_ms": 5}, {"end_ms": 3, "start_ms": 9, "text": "bad range"}]}"#;
        let parse = parse_json_transcript(mixed).expect("partial JSON");
        assert_eq!(parse.cues.len(), 1);
        assert_eq!(parse.malformed.len(), 2);

        let err = parse_json_transcript(r#"{"foo": 1}"#).expect_err("unknown shape");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
        let err = parse_json_transcript("not json").expect_err("invalid json");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
    }

    #[test]
    fn fully_malformed_artifacts_fail_typed() {
        let err = parse_srt("garbage\nwithout any timing\n").expect_err("no cues at all");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
        let err = parse_transcript_artifact("media/audio.xyz", "x").expect_err("unknown extension");
        assert_eq!(err.class, IngestionErrorClass::UnsupportedFormat);
    }
}
