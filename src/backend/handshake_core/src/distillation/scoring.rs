//! Data trust scoring for Skill Bank entries (Master Spec Section 9.1.3.1).
//!
//! Computes a [0.0, 1.0] suitability score for training data selection.

use crate::models::skill_bank::{QualityTag, SkillBankLogEntry, ThumbValue};

/// Compute the data trust score for a Skill Bank log entry.
///
/// Returns a score in `[0.0, 1.0]` indicating suitability for training.
///
/// Hard excludes (returns 0.0):
/// - `contains_secrets` or `pii_present`
/// - `quality_tag == Bad`
/// - `compile_success` is explicitly `false`
/// - `tests_failed > 0` where tests exist
///
/// Soft scoring components:
/// - +0.4 if `quality_tag == Good`
/// - +0.2 if `thumb == Up`
/// - +0.2 * test_pass_ratio (when tests exist)
/// - +0.1 if no security flags
/// - +0.2 * (reasoning_score - 0.5) if present
/// - +0.2 * (factuality_score - 0.5) if present
/// - +0.05 * (style_score - 0.5) if present
/// - -0.1 if output < 128 chars but `thumb == Up`
///
/// Final value is clamped to `[0.0, 1.0]`.
pub fn compute_data_trust_score(entry: &SkillBankLogEntry) -> f64 {
    let privacy = &entry.privacy;
    let quality = &entry.quality;
    let auto_eval = &quality.auto_eval;
    let telemetry = &entry.telemetry;

    // Hard excludes
    if privacy.contains_secrets || privacy.pii_present {
        return 0.0;
    }
    if quality.quality_tag == QualityTag::Bad {
        return 0.0;
    }
    if auto_eval.compile_success == Some(false) {
        return 0.0;
    }
    let total_tests = auto_eval.tests_passed + auto_eval.tests_failed;
    if auto_eval.tests_failed > 0 && total_tests > 0 {
        return 0.0;
    }

    // Soft scoring
    let mut score = 0.0_f64;

    if quality.quality_tag == QualityTag::Good {
        score += 0.4;
    }
    if quality.thumb == ThumbValue::Up {
        score += 0.2;
    }

    if total_tests > 0 {
        let test_ratio = auto_eval.tests_passed as f64 / total_tests as f64;
        score += 0.2 * test_ratio;
    }

    if auto_eval.security_flags.is_empty() {
        score += 0.1;
    }

    if let Some(reasoning) = auto_eval.reasoning_score {
        score += 0.2 * (reasoning - 0.5);
    }

    if let Some(factuality) = auto_eval.factuality_score {
        score += 0.2 * (factuality - 0.5);
    }

    if let Some(style) = auto_eval.style_score {
        score += 0.05 * (style - 0.5);
    }

    let output_len = telemetry.output_char_len.unwrap_or(0);
    if output_len < 128 && quality.thumb == ThumbValue::Up {
        score -= 0.1;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill_bank::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn base_entry() -> SkillBankLogEntry {
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
                    content: Content::Plain("test".to_string()),
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
                auto_eval: AutoEvalMeta {
                    tests_passed: 5,
                    tests_failed: 0,
                    compile_success: Some(true),
                    security_flags: vec![],
                    toxicity_scores: HashMap::new(),
                    style_score: None,
                    reasoning_score: None,
                    factuality_score: None,
                },
                user_edit_stats: UserEditStats::default(),
                data_trust_score: None,
                reward_features: HashMap::new(),
            },
            telemetry: TelemetryMeta {
                output_char_len: Some(500),
                ..TelemetryMeta::default()
            },
            environment: EnvironmentMeta::default(),
            privacy: PrivacyMeta::default(),
        }
    }

    #[test]
    fn data_trust_score_hard_exclude_secrets() {
        let mut entry = base_entry();
        entry.privacy.contains_secrets = true;
        assert_eq!(compute_data_trust_score(&entry), 0.0);
    }

    #[test]
    fn data_trust_score_hard_exclude_pii() {
        let mut entry = base_entry();
        entry.privacy.pii_present = true;
        assert_eq!(compute_data_trust_score(&entry), 0.0);
    }

    #[test]
    fn data_trust_score_hard_exclude_bad_quality() {
        let mut entry = base_entry();
        entry.quality.quality_tag = QualityTag::Bad;
        assert_eq!(compute_data_trust_score(&entry), 0.0);
    }

    #[test]
    fn data_trust_score_hard_exclude_compile_failure() {
        let mut entry = base_entry();
        entry.quality.auto_eval.compile_success = Some(false);
        assert_eq!(compute_data_trust_score(&entry), 0.0);
    }

    #[test]
    fn data_trust_score_hard_exclude_test_failures() {
        let mut entry = base_entry();
        entry.quality.auto_eval.tests_passed = 3;
        entry.quality.auto_eval.tests_failed = 1;
        assert_eq!(compute_data_trust_score(&entry), 0.0);
    }

    #[test]
    fn data_trust_score_good_quality_up_thumb_all_tests_pass_no_flags() {
        // +0.4 (good) +0.2 (up) +0.2 (5/5 tests) +0.1 (no flags) = 0.9
        let entry = base_entry();
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.9).abs() < 1e-10, "expected 0.9, got {score}");
    }

    #[test]
    fn data_trust_score_unrated_neutral_no_tests_no_flags() {
        let mut entry = base_entry();
        entry.quality.quality_tag = QualityTag::Unrated;
        entry.quality.thumb = ThumbValue::Neutral;
        entry.quality.auto_eval.tests_passed = 0;
        entry.quality.auto_eval.tests_failed = 0;
        // +0.0 (unrated) +0.0 (neutral) +0.0 (no tests) +0.1 (no flags) = 0.1
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.1).abs() < 1e-10, "expected 0.1, got {score}");
    }

    #[test]
    fn data_trust_score_short_output_penalty() {
        let mut entry = base_entry();
        entry.telemetry.output_char_len = Some(50); // < 128 and thumb == Up
        // 0.9 - 0.1 = 0.8
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.8).abs() < 1e-10, "expected 0.8, got {score}");
    }

    #[test]
    fn data_trust_score_reasoning_and_factuality_boost() {
        let mut entry = base_entry();
        entry.quality.auto_eval.reasoning_score = Some(0.9); // +0.2 * 0.4 = +0.08
        entry.quality.auto_eval.factuality_score = Some(0.8); // +0.2 * 0.3 = +0.06
        // base 0.9 + 0.08 + 0.06 = 1.04 -> clamped to 1.0
        let score = compute_data_trust_score(&entry);
        assert!((score - 1.0).abs() < 1e-10, "expected 1.0 (clamped), got {score}");
    }

    #[test]
    fn data_trust_score_low_reasoning_reduces_score() {
        let mut entry = base_entry();
        entry.quality.auto_eval.reasoning_score = Some(0.1); // +0.2 * -0.4 = -0.08
        // base 0.9 - 0.08 = 0.82
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.82).abs() < 1e-10, "expected 0.82, got {score}");
    }

    #[test]
    fn data_trust_score_security_flags_remove_bonus() {
        let mut entry = base_entry();
        entry.quality.auto_eval.security_flags = vec!["sql_injection".to_string()];
        // 0.9 - 0.1 (no flag bonus) = 0.8
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.8).abs() < 1e-10, "expected 0.8, got {score}");
    }

    #[test]
    fn data_trust_score_clamps_to_zero_not_negative() {
        let mut entry = base_entry();
        entry.quality.quality_tag = QualityTag::NeedsEdit;
        entry.quality.thumb = ThumbValue::Down;
        entry.quality.auto_eval.tests_passed = 0;
        entry.quality.auto_eval.tests_failed = 0;
        entry.quality.auto_eval.security_flags = vec!["xss".to_string()];
        entry.quality.auto_eval.reasoning_score = Some(0.0); // -0.1
        entry.quality.auto_eval.factuality_score = Some(0.0); // -0.1
        // +0.0 +0.0 +0.0 +0.0 -0.1 -0.1 = -0.2 -> clamped to 0.0
        let score = compute_data_trust_score(&entry);
        assert!((score - 0.0).abs() < 1e-10, "expected 0.0 (clamped), got {score}");
    }

    #[test]
    fn data_trust_score_compile_success_none_does_not_exclude() {
        let mut entry = base_entry();
        entry.quality.auto_eval.compile_success = None;
        let score = compute_data_trust_score(&entry);
        assert!(score > 0.0, "compile_success=None should not hard-exclude");
    }
}
