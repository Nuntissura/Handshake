//! Dataset assembly for adapter training (Master Spec Section 9.1.3.2).
//!
//! Filters Skill Bank entries, scores them, and splits into new + replay
//! batches for distillation jobs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::skill_bank::{ActorRole, QualityTag, SkillBankLogEntry};

use super::scoring::compute_data_trust_score;

/// Configuration for dataset assembly, stored in `distill_job.config_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Fraction of the budget allocated to new (recent) entries, e.g. 0.7.
    pub new_ratio: f64,
    /// Number of days defining the "recent" time window.
    pub recent_window_days: u32,
    /// Maximum total examples to include in the dataset.
    pub max_examples: usize,
    /// Minimum data_trust_score to be eligible.
    pub min_trust_score: f64,
    /// Task types considered code-related for candidate filtering.
    pub code_task_types: Vec<String>,
}

/// A row destined for the `distill_example` table.
#[derive(Debug, Clone, PartialEq)]
pub struct DistillExample {
    pub job_id: Uuid,
    pub log_entry_id: Uuid,
    pub role: ActorRole,
    pub is_replay: bool,
    pub sample_weight: f64,
}

/// Build a distill dataset from candidate Skill Bank entries.
///
/// Implements Section 9.1.3.2:
/// 1. Filter candidates: `quality_tag` in (Good, NeedsEdit), no secrets/PII,
///    task type in `code_task_types`.
/// 2. Compute `data_trust_score` for each; exclude below `min_trust_score`.
/// 3. Split into new batch (within `recent_window_days` of `now`) and replay
///    batch (older entries).
/// 4. Allocate budget per `new_ratio`, select highest-scoring entries from
///    each batch.
///
/// Returns `DistillExample` rows with `sample_weight = data_trust_score`.
pub fn build_distill_dataset(
    job_id: Uuid,
    target_role: ActorRole,
    config: &DatasetConfig,
    candidates: &[SkillBankLogEntry],
    now: DateTime<Utc>,
) -> Vec<DistillExample> {
    // 1) Filter: quality, privacy, task type
    let filtered = candidates.iter().filter(|e| {
        matches!(
            e.quality.quality_tag,
            QualityTag::Good | QualityTag::NeedsEdit
        ) && !e.privacy.contains_secrets
            && !e.privacy.pii_present
            && config.code_task_types.contains(&e.task.r#type)
    });

    // 2) Score and apply minimum threshold
    let mut scored: Vec<(&SkillBankLogEntry, f64)> = filtered
        .map(|e| (e, compute_data_trust_score(e)))
        .filter(|(_, score)| *score >= config.min_trust_score)
        .collect();

    // Stable sort by score descending for deterministic selection
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 3) Split into new_batch (recent) and replay_batch (older)
    let cutoff = now - chrono::Duration::days(config.recent_window_days as i64);
    let (mut new_batch, mut replay_batch): (Vec<_>, Vec<_>) =
        scored.into_iter().partition(|(e, _)| e.timestamp >= cutoff);

    // Re-sort after partition (partition does not preserve order)
    new_batch.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    replay_batch.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 4) Allocate budget
    let max = config.max_examples;
    let new_budget = ((max as f64) * config.new_ratio).ceil() as usize;
    let replay_budget = max.saturating_sub(new_budget);

    new_batch.truncate(new_budget);
    replay_batch.truncate(replay_budget);

    // Build output examples
    let mut examples = Vec::with_capacity(new_batch.len() + replay_batch.len());

    for (entry, score) in &new_batch {
        examples.push(DistillExample {
            job_id,
            log_entry_id: entry.log_id,
            role: target_role,
            is_replay: false,
            sample_weight: *score,
        });
    }
    for (entry, score) in &replay_batch {
        examples.push(DistillExample {
            job_id,
            log_entry_id: entry.log_id,
            role: target_role,
            is_replay: true,
            sample_weight: *score,
        });
    }

    examples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill_bank::*;
    use std::collections::HashMap;

    fn default_config() -> DatasetConfig {
        DatasetConfig {
            new_ratio: 0.7,
            recent_window_days: 7,
            max_examples: 100,
            min_trust_score: 0.1,
            code_task_types: vec!["code_generation".to_string(), "code_review".to_string()],
        }
    }

    fn make_entry(
        quality_tag: QualityTag,
        thumb: ThumbValue,
        task_type: &str,
        timestamp: DateTime<Utc>,
    ) -> SkillBankLogEntry {
        let msg_id = Uuid::new_v4();
        SkillBankLogEntry {
            version: "1.0.0".to_string(),
            log_id: Uuid::new_v4(),
            timestamp,
            session: SessionMeta {
                session_id: Uuid::new_v4(),
                turn_index: 0,
                task_id: None,
                user_id_hash: None,
                workspace_id: None,
            },
            task: TaskMeta {
                r#type: task_type.to_string(),
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
                quality_tag,
                thumb,
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
    fn distill_dataset_empty_candidates_returns_empty() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[], Utc::now());
        assert!(result.is_empty());
    }

    #[test]
    fn distill_dataset_excludes_secrets() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let mut entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);
        entry.privacy.contains_secrets = true;

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(result.is_empty(), "entries with secrets must be excluded");
    }

    #[test]
    fn distill_dataset_excludes_pii() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let mut entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);
        entry.privacy.pii_present = true;

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(result.is_empty(), "entries with PII must be excluded");
    }

    #[test]
    fn distill_dataset_excludes_bad_quality() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Bad, ThumbValue::Neutral, "code_generation", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(result.is_empty(), "Bad quality entries must be excluded");
    }

    #[test]
    fn distill_dataset_excludes_unrated_quality() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Unrated, ThumbValue::Neutral, "code_generation", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(result.is_empty(), "Unrated quality entries must be excluded");
    }

    #[test]
    fn distill_dataset_excludes_non_code_task_type() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Good, ThumbValue::Up, "chat", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(result.is_empty(), "non-code task types must be excluded");
    }

    #[test]
    fn distill_dataset_includes_needs_edit() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::NeedsEdit, ThumbValue::Neutral, "code_generation", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert_eq!(result.len(), 1, "NeedsEdit entries should be included");
    }

    #[test]
    fn distill_dataset_respects_min_trust_score() {
        let mut config = default_config();
        config.min_trust_score = 0.95;
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        // base_entry with Good + Up + 5/5 tests + no flags = 0.9 score
        let entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert!(
            result.is_empty(),
            "entries below min_trust_score must be excluded"
        );
    }

    #[test]
    fn distill_dataset_splits_new_and_replay() {
        let config = default_config(); // 7-day window, 0.7 new ratio
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let recent = now - chrono::Duration::days(1);
        let old = now - chrono::Duration::days(30);

        let new_entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", recent);
        let replay_entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", old);

        let result = build_distill_dataset(
            job_id,
            ActorRole::Student,
            &config,
            &[new_entry, replay_entry],
            now,
        );
        assert_eq!(result.len(), 2);

        let new_count = result.iter().filter(|e| !e.is_replay).count();
        let replay_count = result.iter().filter(|e| e.is_replay).count();
        assert_eq!(new_count, 1, "recent entry should be in new batch");
        assert_eq!(replay_count, 1, "old entry should be in replay batch");
    }

    #[test]
    fn distill_dataset_weight_equals_trust_score() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);
        let expected_score = compute_data_trust_score(&entry);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert_eq!(result.len(), 1);
        assert!(
            (result[0].sample_weight - expected_score).abs() < 1e-10,
            "sample_weight should equal data_trust_score"
        );
    }

    #[test]
    fn distill_dataset_respects_max_examples() {
        let mut config = default_config();
        config.max_examples = 2;
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entries: Vec<_> = (0..5)
            .map(|_| make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now))
            .collect();

        let result =
            build_distill_dataset(job_id, ActorRole::Student, &config, &entries, now);
        assert!(
            result.len() <= 2,
            "should not exceed max_examples, got {}",
            result.len()
        );
    }

    #[test]
    fn distill_dataset_all_recent_no_replay() {
        let mut config = default_config();
        config.new_ratio = 1.0;
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entries: Vec<_> = (0..3)
            .map(|_| make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now))
            .collect();

        let result =
            build_distill_dataset(job_id, ActorRole::Student, &config, &entries, now);
        assert!(
            result.iter().all(|e| !e.is_replay),
            "with new_ratio=1.0 all entries should be new"
        );
    }

    #[test]
    fn distill_dataset_assigns_target_role() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);

        let result =
            build_distill_dataset(job_id, ActorRole::Teacher, &config, &[entry], now);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, ActorRole::Teacher, "role must match target_role");
    }

    #[test]
    fn distill_dataset_assigns_job_id() {
        let config = default_config();
        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = make_entry(QualityTag::Good, ThumbValue::Up, "code_generation", now);

        let result = build_distill_dataset(job_id, ActorRole::Student, &config, &[entry], now);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].job_id, job_id);
    }

    #[test]
    fn distill_dataset_config_round_trip() {
        let config = default_config();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: DatasetConfig = serde_json::from_str(&json).unwrap();
        assert!((parsed.new_ratio - config.new_ratio).abs() < 1e-10);
        assert_eq!(parsed.recent_window_days, config.recent_window_days);
        assert_eq!(parsed.max_examples, config.max_examples);
        assert_eq!(parsed.code_task_types, config.code_task_types);
    }
}
