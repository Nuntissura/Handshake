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

        // Email addresses
        let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
        if email_re.is_match(&result) {
            *pii = true;
            result = email_re
                .replace_all(&result, EMAIL_PLACEHOLDER)
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
                }
            }
        }
    };

    let redact_snapshot = |snapshot: &mut ChatSnapshot, secrets: &mut bool, pii: &mut bool| {
        for msg in snapshot.messages.iter_mut() {
            redact_content(&mut msg.content, secrets, pii);
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

    // Also scan request_summary
    if let Some(ref summary) = entry.task.request_summary {
        let redacted = redact_text(summary, &mut secrets_found, &mut pii_found);
        entry.task.request_summary = Some(redacted);
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
}
