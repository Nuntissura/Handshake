//! PII and secret redaction for Skill Bank entries (Master Spec Section 9, §1.1.3).
//!
//! Scans text content in chat snapshots for secrets (API keys, high-entropy
//! tokens, .env patterns) and PII (emails, phone numbers), replaces detected
//! spans with typed placeholders, and sets privacy flags.

use regex::Regex;

use crate::models::skill_bank::{ChatSnapshot, Content, SkillBankLogEntry};

/// Result of redacting a [`SkillBankLogEntry`].
#[derive(Debug, Clone)]
pub struct RedactionResult {
    /// The entry with sensitive spans replaced by placeholders.
    pub redacted_entry: SkillBankLogEntry,
    /// Whether any secret-like patterns were detected.
    pub secrets_found: bool,
    /// Whether any PII patterns were detected.
    pub pii_found: bool,
}

/// Placeholder tokens inserted in place of redacted content.
const SECRET_PLACEHOLDER: &str = "[REDACTED_SECRET]";
const EMAIL_PLACEHOLDER: &str = "[REDACTED_EMAIL]";
const PHONE_PLACEHOLDER: &str = "[REDACTED_PHONE]";
const IBAN_PLACEHOLDER: &str = "[REDACTED_IBAN]";
const ENV_PLACEHOLDER: &str = "[REDACTED_ENV]";

/// Scan and redact a [`SkillBankLogEntry`] before persistence or training.
///
/// Applies a chain of cheap regex detectors:
/// - API key / secret patterns (Bearer tokens, AWS keys, generic API keys)
/// - `.env`-style `KEY=value` patterns for known secret variable names
/// - High-entropy hex/base64 tokens (>= 32 chars)
/// - Email addresses
/// - Phone numbers (international and US formats)
///
/// Detected spans are replaced with typed placeholders and the entry's
/// privacy flags (`contains_secrets`, `pii_present`, `redaction_applied`)
/// are set accordingly.
pub fn redact_entry(raw_entry: &SkillBankLogEntry) -> RedactionResult {
    let mut entry = raw_entry.clone();
    let mut secrets_found = false;
    let mut pii_found = false;

    let redact_text = |text: &str, secrets: &mut bool, pii: &mut bool| -> String {
        let mut result = text.to_string();

        // .env-style patterns for known secret variable names (most specific, run first)
        let env_re = Regex::new(
            r"(?i)(DATABASE_URL|DB_PASSWORD|SECRET_KEY|PRIVATE_KEY|AWS_SECRET_ACCESS_KEY|OPENAI_API_KEY|ANTHROPIC_API_KEY)\s*=\s*\S+",
        )
        .unwrap();
        if env_re.is_match(&result) {
            *secrets = true;
            result = env_re.replace_all(&result, ENV_PLACEHOLDER).to_string();
        }

        // API key / Bearer token patterns
        let bearer_re = Regex::new(r"(?i)Bearer\s+[A-Za-z0-9\-._~+/]+=*").unwrap();
        if bearer_re.is_match(&result) {
            *secrets = true;
            result = bearer_re.replace_all(&result, SECRET_PLACEHOLDER).to_string();
        }

        // AWS-style access keys (AKIA...)
        let aws_re = Regex::new(r"AKIA[0-9A-Z]{16}").unwrap();
        if aws_re.is_match(&result) {
            *secrets = true;
            result = aws_re.replace_all(&result, SECRET_PLACEHOLDER).to_string();
        }

        // Generic API key patterns (api_key=..., apikey:..., etc.)
        let api_key_re =
            Regex::new(r"(?i)(api[_-]?key|api[_-]?secret|token|password|secret)\s*[=:]\s*\S+")
                .unwrap();
        if api_key_re.is_match(&result) {
            *secrets = true;
            result = api_key_re
                .replace_all(&result, SECRET_PLACEHOLDER)
                .to_string();
        }

        // High-entropy hex strings (>= 32 hex chars)
        let hex_re = Regex::new(r"\b[0-9a-fA-F]{32,}\b").unwrap();
        if hex_re.is_match(&result) {
            *secrets = true;
            result = hex_re.replace_all(&result, SECRET_PLACEHOLDER).to_string();
        }

        // High-entropy base64 tokens (>= 32 base64 chars, optionally padded).
        // Guard rejects natural-language identifiers via two paths:
        //   A. If the match contains a digit, +, or / → structurally non-prose.
        //   B. If pure-alpha, require BOTH:
        //      - Case balance: uppercase ratio 0.35–0.65 (random base64 ≈ 0.50,
        //        CamelCase ≈ 0.20–0.25, acronym-heavy ≈ 0.35–0.40).
        //      - Case transition density ≥ 0.40: the fraction of adjacent pairs
        //        that switch case (U→L or L→U). Random base64 ≈ 0.50, CamelCase
        //        ≈ 0.25–0.35 because words and acronyms form same-case runs.
        // Then: distinct-character ratio ≥ 0.55 filters low-diversity strings.
        let base64_re =
            Regex::new(r"\b[A-Za-z0-9+/]{32,}={0,2}").unwrap();
        {
            let mut new_result = String::new();
            let mut last_end = 0;
            let mut any_replaced = false;
            for m in base64_re.find_iter(&result) {
                new_result.push_str(&result[last_end..m.start()]);
                let stripped = m.as_str().trim_end_matches('=');
                let has_non_alpha = stripped
                    .bytes()
                    .any(|b| b.is_ascii_digit() || b == b'+' || b == b'/');
                let looks_random = if has_non_alpha {
                    true
                } else {
                    let upper = stripped.bytes().filter(|b| b.is_ascii_uppercase()).count();
                    let alpha = stripped.bytes().filter(|b| b.is_ascii_alphabetic()).count();
                    let upper_ratio = upper as f64 / alpha as f64;
                    let bytes: Vec<u8> = stripped.bytes().collect();
                    let transitions = bytes
                        .windows(2)
                        .filter(|w| {
                            w[0].is_ascii_uppercase() != w[1].is_ascii_uppercase()
                        })
                        .count();
                    let transition_density =
                        transitions as f64 / (bytes.len() - 1) as f64;
                    (0.35..=0.65).contains(&upper_ratio)
                        && transition_density >= 0.40
                };
                let len = stripped.len() as f64;
                let mut seen = [false; 256];
                for b in stripped.bytes() {
                    seen[b as usize] = true;
                }
                let distinct = seen.iter().filter(|&&v| v).count() as f64;
                let ratio = distinct / len;
                if looks_random && ratio >= 0.55 {
                    new_result.push_str(SECRET_PLACEHOLDER);
                    any_replaced = true;
                } else {
                    new_result.push_str(m.as_str());
                }
                last_end = m.end();
            }
            if any_replaced {
                new_result.push_str(&result[last_end..]);
                *secrets = true;
                result = new_result;
            }
        }

        // Email addresses
        let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
        if email_re.is_match(&result) {
            *pii = true;
            result = email_re
                .replace_all(&result, EMAIL_PLACEHOLDER)
                .to_string();
        }

        // IBAN (International Bank Account Number) — must run before phone
        // to prevent phone regex from consuming digit sequences within IBANs
        let iban_re =
            Regex::new(r"(?i)\b[A-Z]{2}\d{2}[\s]?[A-Z0-9]{4}[\s]?(?:[A-Z0-9]{4}[\s]?){2,7}[A-Z0-9]{1,4}\b")
                .unwrap();
        if iban_re.is_match(&result) {
            *pii = true;
            result = iban_re
                .replace_all(&result, IBAN_PLACEHOLDER)
                .to_string();
        }

        // Phone numbers (international and US formats)
        let phone_re = Regex::new(r"(\+\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}")
            .unwrap();
        if phone_re.is_match(&result) {
            *pii = true;
            result = phone_re
                .replace_all(&result, PHONE_PLACEHOLDER)
                .to_string();
        }

        result
    };

    let redact_content = |content: &mut Content, secrets: &mut bool, pii: &mut bool| {
        match content {
            Content::Plain(ref mut text) => {
                *text = redact_text(text, secrets, pii);
            }
            Content::Segments(ref mut segments) => {
                for seg in segments.iter_mut() {
                    seg.text = redact_text(&seg.text, secrets, pii);
                    if let Some(ref fp) = seg.file_path {
                        let redacted = redact_text(fp, secrets, pii);
                        seg.file_path = Some(redacted);
                    }
                }
            }
        }
    };

    // Recursively redact string values inside serde_json::Value trees.
    fn redact_json_value(
        val: &serde_json::Value,
        redact_text: &dyn Fn(&str, &mut bool, &mut bool) -> String,
        secrets: &mut bool,
        pii: &mut bool,
    ) -> serde_json::Value {
        match val {
            serde_json::Value::String(s) => {
                serde_json::Value::String(redact_text(s, secrets, pii))
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| redact_json_value(v, redact_text, secrets, pii))
                    .collect(),
            ),
            serde_json::Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, v) in map {
                    out.insert(
                        k.clone(),
                        redact_json_value(v, redact_text, secrets, pii),
                    );
                }
                serde_json::Value::Object(out)
            }
            other => other.clone(),
        }
    }

    let redact_snapshot = |snapshot: &mut ChatSnapshot, secrets: &mut bool, pii: &mut bool| {
        for msg in snapshot.messages.iter_mut() {
            redact_content(&mut msg.content, secrets, pii);
            let redacted_meta: std::collections::HashMap<String, serde_json::Value> = msg
                .metadata
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        redact_json_value(v, &redact_text, secrets, pii),
                    )
                })
                .collect();
            msg.metadata = redacted_meta;
        }
    };

    redact_snapshot(&mut entry.snapshots_input, &mut secrets_found, &mut pii_found);
    redact_snapshot(
        &mut entry.snapshots_output_raw,
        &mut secrets_found,
        &mut pii_found,
    );
    if let Some(ref mut final_snap) = entry.snapshots_output_final {
        redact_snapshot(final_snap, &mut secrets_found, &mut pii_found);
    }

    // Scan context_refs.files[].path for PII leakage in provenance paths
    for file_ref in entry.context_refs.files.iter_mut() {
        file_ref.path = redact_text(&file_ref.path, &mut secrets_found, &mut pii_found);
    }

    // Also scan request_summary
    if let Some(ref summary) = entry.task.request_summary {
        let redacted = redact_text(summary, &mut secrets_found, &mut pii_found);
        entry.task.request_summary = Some(redacted);
    }

    // Scan edit_summary in UserEditStats
    if let Some(ref summary) = entry.quality.user_edit_stats.edit_summary {
        let redacted = redact_text(summary, &mut secrets_found, &mut pii_found);
        entry.quality.user_edit_stats.edit_summary = Some(redacted);
    }

    // Set privacy flags
    entry.privacy.contains_secrets = secrets_found;
    entry.privacy.pii_present = pii_found;
    entry.privacy.redaction_applied = secrets_found || pii_found;

    RedactionResult {
        redacted_entry: entry,
        secrets_found,
        pii_found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill_bank::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn entry_with_input(text: &str) -> SkillBankLogEntry {
        let msg_id = Uuid::new_v4();
        SkillBankLogEntry {
            version: "1.0.0".to_string(),
            log_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            session: SessionMeta {
                session_id: Uuid::new_v4(),
                turn_index: 0,
                task_id: None,
                user_id_hash: None,
                workspace_id: None,
            },
            task: TaskMeta {
                r#type: "code_generation".to_string(),
                subtype: None,
                language: Some("rust".to_string()),
                tags: vec![],
                request_summary: None,
            },
            engine: EngineMeta {
                actor_role: ActorRole::Student,
                model_name: "test-model".to_string(),
                model_family: None,
                model_revision: None,
                provider: None,
                tokenizer_id: None,
                tokenizer_family: None,
                context_window_tokens: None,
                precision: None,
                inference_params: HashMap::new(),
            },
            context_refs: ContextRefs::default(),
            snapshots_input: ChatSnapshot {
                format: SnapshotFormat::Chatml,
                messages: vec![ChatMessage {
                    id: msg_id,
                    parent_id: None,
                    role: Role::User,
                    content: Content::Plain(text.to_string()),
                    metadata: HashMap::new(),
                }],
                focus_message_id: None,
            },
            snapshots_output_raw: ChatSnapshot {
                format: SnapshotFormat::Chatml,
                messages: vec![],
                focus_message_id: None,
            },
            snapshots_output_final: None,
            quality: QualityMeta {
                quality_tag: QualityTag::Good,
                thumb: ThumbValue::Up,
                score: None,
                source: None,
                labels: vec![],
                auto_eval: AutoEvalMeta::default(),
                user_edit_stats: UserEditStats::default(),
                data_trust_score: None,
                reward_features: HashMap::new(),
            },
            telemetry: TelemetryMeta::default(),
            environment: EnvironmentMeta::default(),
            privacy: PrivacyMeta::default(),
        }
    }

    fn input_text(entry: &SkillBankLogEntry) -> &str {
        match &entry.snapshots_input.messages[0].content {
            Content::Plain(t) => t.as_str(),
            Content::Segments(segs) => segs[0].text.as_str(),
        }
    }

    #[test]
    fn redaction_clean_text_unchanged() {
        let entry = entry_with_input("Hello, write me a function.");
        let result = redact_entry(&entry);

        assert!(!result.secrets_found);
        assert!(!result.pii_found);
        assert_eq!(
            input_text(&result.redacted_entry),
            "Hello, write me a function."
        );
        assert!(!result.redacted_entry.privacy.redaction_applied);
    }

    #[test]
    fn redaction_detects_bearer_token() {
        let entry = entry_with_input("Use Bearer sk-abc123def456 to authenticate.");
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(input_text(&result.redacted_entry).contains(SECRET_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("sk-abc123def456"));
        assert!(result.redacted_entry.privacy.contains_secrets);
    }

    #[test]
    fn redaction_detects_aws_key() {
        let entry = entry_with_input("My access key is AKIAIOSFODNN7EXAMPLE.");
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(input_text(&result.redacted_entry).contains(SECRET_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn redaction_detects_api_key_pattern() {
        let entry = entry_with_input("Set api_key=sk-live-12345abcdef in config.");
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(!input_text(&result.redacted_entry).contains("sk-live-12345abcdef"));
    }

    #[test]
    fn redaction_detects_env_style_secret() {
        let entry = entry_with_input("OPENAI_API_KEY=sk-supersecret123");
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(input_text(&result.redacted_entry).contains(ENV_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("sk-supersecret123"));
    }

    #[test]
    fn redaction_detects_high_entropy_hex() {
        let hex = "a".repeat(64); // 64 hex chars
        let text = format!("Hash: {hex}");
        let entry = entry_with_input(&text);
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(!input_text(&result.redacted_entry).contains(&hex));
    }

    #[test]
    fn redaction_detects_email() {
        let entry = entry_with_input("Contact user@example.com for details.");
        let result = redact_entry(&entry);

        assert!(result.pii_found);
        assert!(input_text(&result.redacted_entry).contains(EMAIL_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("user@example.com"));
        assert!(result.redacted_entry.privacy.pii_present);
    }

    #[test]
    fn redaction_detects_phone_number() {
        let entry = entry_with_input("Call +1-555-123-4567 for support.");
        let result = redact_entry(&entry);

        assert!(result.pii_found);
        assert!(input_text(&result.redacted_entry).contains(PHONE_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("555-123-4567"));
    }

    #[test]
    fn redaction_handles_segmented_content() {
        let mut entry = entry_with_input("placeholder");
        entry.snapshots_input.messages[0].content = Content::Segments(vec![
            ContentSegment {
                r#type: ContentSegmentType::Code,
                text: "api_key = sk-secret123".to_string(),
                language: Some("python".to_string()),
                file_path: None,
            },
            ContentSegment {
                r#type: ContentSegmentType::Text,
                text: "Send to user@test.org".to_string(),
                language: None,
                file_path: None,
            },
        ]);

        let result = redact_entry(&entry);
        assert!(result.secrets_found);
        assert!(result.pii_found);

        if let Content::Segments(segs) = &result.redacted_entry.snapshots_input.messages[0].content
        {
            assert!(!segs[0].text.contains("sk-secret123"));
            assert!(!segs[1].text.contains("user@test.org"));
        } else {
            panic!("expected Segments content");
        }
    }

    #[test]
    fn redaction_scans_output_snapshots() {
        let mut entry = entry_with_input("clean input");
        let msg_id = Uuid::new_v4();
        entry.snapshots_output_raw.messages.push(ChatMessage {
            id: msg_id,
            parent_id: None,
            role: Role::Assistant,
            content: Content::Plain("Use Bearer sk-output-token here".to_string()),
            metadata: HashMap::new(),
        });

        let result = redact_entry(&entry);
        assert!(result.secrets_found);

        let output_text = match &result.redacted_entry.snapshots_output_raw.messages[0].content {
            Content::Plain(t) => t.as_str(),
            _ => panic!("expected plain content"),
        };
        assert!(!output_text.contains("sk-output-token"));
    }

    #[test]
    fn redaction_scans_request_summary() {
        let mut entry = entry_with_input("clean input");
        entry.task.request_summary = Some("Email admin@corp.com for access".to_string());

        let result = redact_entry(&entry);
        assert!(result.pii_found);
        assert!(result
            .redacted_entry
            .task
            .request_summary
            .as_ref()
            .unwrap()
            .contains(EMAIL_PLACEHOLDER));
    }

    #[test]
    fn redaction_sets_privacy_flags() {
        let entry = entry_with_input("OPENAI_API_KEY=secret user@test.com");
        let result = redact_entry(&entry);

        assert!(result.redacted_entry.privacy.contains_secrets);
        assert!(result.redacted_entry.privacy.pii_present);
        assert!(result.redacted_entry.privacy.redaction_applied);
    }

    #[test]
    fn redaction_no_false_positive_on_short_hex() {
        // 16 hex chars is too short to flag as high-entropy secret
        let entry = entry_with_input("hash: abcdef0123456789");
        let result = redact_entry(&entry);

        assert!(!result.secrets_found, "16-char hex should not trigger secret detection");
    }

    #[test]
    fn redaction_scans_content_segment_file_path() {
        let mut entry = entry_with_input("placeholder");
        entry.snapshots_input.messages[0].content = Content::Segments(vec![ContentSegment {
            r#type: ContentSegmentType::Code,
            text: "clean code".to_string(),
            language: Some("rust".to_string()),
            file_path: Some("/home/user@example.com/project/main.rs".to_string()),
        }]);

        let result = redact_entry(&entry);
        assert!(result.pii_found);

        if let Content::Segments(segs) = &result.redacted_entry.snapshots_input.messages[0].content
        {
            let fp = segs[0].file_path.as_ref().unwrap();
            assert!(fp.contains(EMAIL_PLACEHOLDER));
            assert!(!fp.contains("user@example.com"));
        } else {
            panic!("expected Segments content");
        }
    }

    #[test]
    fn redaction_scans_chat_message_metadata() {
        let mut entry = entry_with_input("clean input");
        entry.snapshots_input.messages[0]
            .metadata
            .insert("note".to_string(), serde_json::json!("Contact admin@corp.com"));

        let result = redact_entry(&entry);
        assert!(result.pii_found);

        let meta_val = result.redacted_entry.snapshots_input.messages[0]
            .metadata
            .get("note")
            .unwrap();
        let s = meta_val.as_str().unwrap();
        assert!(s.contains(EMAIL_PLACEHOLDER));
        assert!(!s.contains("admin@corp.com"));
    }

    #[test]
    fn redaction_scans_nested_metadata_values() {
        let mut entry = entry_with_input("clean input");
        entry.snapshots_input.messages[0].metadata.insert(
            "config".to_string(),
            serde_json::json!({"key": "OPENAI_API_KEY=sk-secret999"}),
        );

        let result = redact_entry(&entry);
        assert!(result.secrets_found);

        let meta_val = &result.redacted_entry.snapshots_input.messages[0].metadata["config"];
        let inner = meta_val.get("key").unwrap().as_str().unwrap();
        assert!(inner.contains(ENV_PLACEHOLDER));
        assert!(!inner.contains("sk-secret999"));
    }

    #[test]
    fn redaction_scans_edit_summary() {
        let mut entry = entry_with_input("clean input");
        entry.quality.user_edit_stats.edit_summary =
            Some("Fixed api_key=leaked-secret in handler".to_string());

        let result = redact_entry(&entry);
        assert!(result.secrets_found);

        let summary = result
            .redacted_entry
            .quality
            .user_edit_stats
            .edit_summary
            .as_ref()
            .unwrap();
        assert!(summary.contains(SECRET_PLACEHOLDER));
        assert!(!summary.contains("leaked-secret"));
    }

    #[test]
    fn redaction_scans_context_refs_file_paths() {
        let mut entry = entry_with_input("clean input");
        entry.context_refs.files.push(FileContextRef {
            path: "/home/user@example.com/project/src/main.rs".to_string(),
            hash: None,
            selection_ranges: vec![],
        });

        let result = redact_entry(&entry);
        assert!(result.pii_found);

        let path = &result.redacted_entry.context_refs.files[0].path;
        assert!(path.contains(EMAIL_PLACEHOLDER));
        assert!(!path.contains("user@example.com"));
    }

    #[test]
    fn redaction_detects_iban() {
        let entry = entry_with_input("Transfer to DE89370400440532013000 please.");
        let result = redact_entry(&entry);

        assert!(result.pii_found);
        assert!(input_text(&result.redacted_entry).contains(IBAN_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("DE89370400440532013000"));
        assert!(result.redacted_entry.privacy.pii_present);
    }

    #[test]
    fn redaction_detects_iban_with_spaces() {
        let entry = entry_with_input("IBAN: GB29 NWBK 6016 1331 9268 19");
        let result = redact_entry(&entry);

        assert!(result.pii_found);
        assert!(input_text(&result.redacted_entry).contains(IBAN_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("NWBK"));
    }

    #[test]
    fn redaction_detects_iban_mixed_case() {
        let entry = entry_with_input("Pay to de89370400440532013000 now");
        let result = redact_entry(&entry);

        assert!(result.pii_found);
        assert!(input_text(&result.redacted_entry).contains(IBAN_PLACEHOLDER));
        assert!(!input_text(&result.redacted_entry).contains("de89"));
    }

    #[test]
    fn redaction_detects_base64_token() {
        let b64 = "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo0MTIzNDU2Nzg5MA==";
        let text = format!("Secret: {b64}");
        let entry = entry_with_input(&text);
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(!input_text(&result.redacted_entry).contains(b64));
        assert!(input_text(&result.redacted_entry).contains(SECRET_PLACEHOLDER));
        // No trailing padding remnant survives
        assert!(
            !input_text(&result.redacted_entry).contains("=="),
            "base64 padding must not leak: got {:?}",
            input_text(&result.redacted_entry)
        );
    }

    #[test]
    fn redaction_no_false_positive_on_short_base64() {
        // 16 base64 chars should not trigger
        let entry = entry_with_input("id: QWJjRGVmR2hJams=");
        let result = redact_entry(&entry);

        assert!(!result.secrets_found, "short base64 should not trigger secret detection");
    }

    #[test]
    fn redaction_no_false_positive_on_long_alpha_string() {
        // Pure-alpha CamelCase identifiers must not trigger base64 detection
        let entry = entry_with_input(
            "ThisSentenceHasManyLettersButNoPaddingAndIsProbablyNotBase64Token",
        );
        let result = redact_entry(&entry);

        assert!(
            !result.secrets_found,
            "long alpha-only string should not trigger secret detection"
        );
    }

    #[test]
    fn redaction_detects_base64_with_few_digits() {
        // Real base64 tokens with <4 digits but containing + or /
        let b64 = "xFb/ltoDgyIvIzUTyFmO+ZSCgHy4guJj+1iNW7yAlLw=";
        let entry = entry_with_input(&format!("key: {b64}"));
        let result = redact_entry(&entry);

        assert!(result.secrets_found);
        assert!(!input_text(&result.redacted_entry).contains("xFb"));
    }

    #[test]
    fn redaction_detects_base64_no_special_low_digit() {
        // Real base64 token with neither +/ nor >= 4 digits — caught by
        // has_digit + distinct-character ratio guard.
        let b64 = "lWmAhErUCMa4bWKSEtCVHpdNwJi4lsvAwmdcHtgP5zA=";
        let entry = entry_with_input(&format!("token: {b64}"));
        let result = redact_entry(&entry);

        assert!(result.secrets_found, "high-entropy base64 without +/ or many digits must be redacted");
        assert!(!input_text(&result.redacted_entry).contains("lWmAhEr"));
    }

    #[test]
    fn redaction_no_false_positive_on_high_diversity_identifier() {
        // Pure-alpha CamelCase with high character diversity (ratio 0.74)
        // must NOT be redacted — uppercase ratio 0.24 is outside 0.35–0.65.
        let ident = "SphinxOfBlackQuartzJudgeMyVowAlphaBeta";
        let entry = entry_with_input(ident);
        let result = redact_entry(&entry);

        assert!(
            !result.secrets_found,
            "high-diversity pure-alpha identifier should not trigger secret detection"
        );
    }

    #[test]
    fn redaction_no_false_positive_on_acronym_heavy_identifier() {
        // Acronym-heavy CamelCase (upper ratio 0.37, diversity 0.71) must NOT
        // be redacted — case transition density 0.32 is below 0.45 threshold.
        let ident = "OAuthJWTHttpXMLApiTokenParserConfig";
        let entry = entry_with_input(ident);
        let result = redact_entry(&entry);

        assert!(
            !result.secrets_found,
            "acronym-heavy CamelCase identifier should not trigger secret detection"
        );
    }

    #[test]
    fn redaction_detects_base64_zero_digits_with_special() {
        // Real base64 token with +/ but zero digits — caught by
        // has_non_alpha (+ and / count) + ratio guard.
        let b64 = "aCZZVEZhGX/DAWor+ayXVmLRjeeRCdIVGbuqZJTJNjc=";
        let entry = entry_with_input(&format!("secret: {b64}"));
        let result = redact_entry(&entry);

        assert!(result.secrets_found, "base64 with +/ but no digits must be redacted");
        assert!(!input_text(&result.redacted_entry).contains("aCZZVEZh"));
    }

    #[test]
    fn redaction_detects_base64_pure_alpha() {
        // Pure-alpha base64 token (no digits, no +/) — caught by case-balance
        // (upper ratio 0.51) + transition density (0.52 ≥ 0.40).
        let b64 = "pgRrGbitwZCCAyRlOGuDrRcTMYaqEaaEPDPGacpTFwc";
        let entry = entry_with_input(&format!("key: {b64}"));
        let result = redact_entry(&entry);

        assert!(result.secrets_found, "pure-alpha base64 with balanced case must be redacted");
        assert!(!input_text(&result.redacted_entry).contains("pgRrGbitw"));
    }

    #[test]
    fn redaction_detects_base64_pure_alpha_low_transition() {
        // Pure-alpha base64 with lower transition density (0.43) — still caught
        // because density ≥ 0.40 and case balance 0.42 is in range.
        let b64 = "QAHrVTlmQfgfOxmGZIwrMSQFgqjjrpmhsyCpPolnRzQ";
        let entry = entry_with_input(&format!("token: {b64}"));
        let result = redact_entry(&entry);

        assert!(result.secrets_found, "low-transition pure-alpha base64 must be redacted");
        assert!(!input_text(&result.redacted_entry).contains("QAHrVTlm"));
    }
}
