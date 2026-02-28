use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ace::SourceRef;
use crate::llm::{canonical_json_bytes_nfc, sha256_hex};
use crate::tokenization::{TokenizationService, TokenizerError};

use super::spec_prompt_pack::SpecPromptPackV1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingContextV1 {
    pub blocks: Vec<ContextBlockV1>,
    pub token_budget: u32,
    pub token_estimate: u32,
    pub build_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlockV1 {
    pub kind: String,
    pub content: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    pub sensitivity: String,
    pub projection: String,
    pub order_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEnvelopeTruncationV1 {
    #[serde(default)]
    pub per_placeholder_truncated: BTreeMap<String, bool>,
    pub variable_suffix_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEnvelopeV1 {
    pub stable_prefix: WorkingContextV1,
    pub variable_suffix: WorkingContextV1,
    pub stable_prefix_hash: String,
    pub variable_suffix_hash: String,
    pub full_prompt_hash: String,
    pub stable_prefix_tokens: u32,
    pub variable_suffix_tokens: u32,
    pub total_tokens: u32,
    pub truncation: PromptEnvelopeTruncationV1,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecPromptCompilerError {
    #[error("missing required placeholder: {name}")]
    MissingRequiredPlaceholder { name: String },
    #[error("duplicate placeholder name: {name}")]
    DuplicatePlaceholderName { name: String },
    #[error("tokenization error: {0}")]
    Tokenization(#[from] TokenizerError),
    #[error("budget exceeded: stable_prefix_tokens={stable_prefix_tokens} max_total_tokens={max_total_tokens}")]
    BudgetExceeded {
        stable_prefix_tokens: u32,
        max_total_tokens: u32,
    },
    #[error("model_id must be non-empty")]
    EmptyModelId,
}

pub fn render_working_context_text(blocks: &[ContextBlockV1]) -> String {
    blocks
        .iter()
        .map(|b| b.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn render_prompt_envelope_text(envelope: &PromptEnvelopeV1) -> String {
    let stable_prefix_text = render_working_context_text(&envelope.stable_prefix.blocks);
    let variable_suffix_text = render_working_context_text(&envelope.variable_suffix.blocks);
    if stable_prefix_text.is_empty() {
        return variable_suffix_text;
    }
    format!("{stable_prefix_text}\n\n{variable_suffix_text}")
}

fn stable_prefix_kind_for_section(section_id: &str) -> String {
    match section_id {
        "SYSTEM_RULES" => "rules".to_string(),
        "OUTPUT_CONTRACT" => "constraints".to_string(),
        _ => "rules".to_string(),
    }
}

fn blocks_sha256_hex(blocks: &[ContextBlockV1]) -> Result<String, SpecPromptCompilerError> {
    let value = serde_json::to_value(blocks)
        .map_err(|e| TokenizerError::TokenizationFailed(e.to_string()))?;
    Ok(sha256_hex(&canonical_json_bytes_nfc(&value)))
}

pub fn compile_spec_router_envelope(
    pack: &SpecPromptPackV1,
    values: &BTreeMap<String, String>,
    tokenization: &dyn TokenizationService,
    model_id: &str,
) -> Result<PromptEnvelopeV1, SpecPromptCompilerError> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err(SpecPromptCompilerError::EmptyModelId);
    }

    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut per_placeholder_truncated: BTreeMap<String, bool> = BTreeMap::new();

    // Step 1: require presence + per-placeholder truncation (deterministic order: pack.placeholders order).
    let mut truncated_values: BTreeMap<String, String> = BTreeMap::new();
    for p in &pack.placeholders {
        if !seen.insert(p.name.clone()) {
            return Err(SpecPromptCompilerError::DuplicatePlaceholderName {
                name: p.name.clone(),
            });
        }

        let raw = values.get(&p.name).cloned().unwrap_or_default();
        if p.required && raw.trim().is_empty() {
            return Err(SpecPromptCompilerError::MissingRequiredPlaceholder {
                name: p.name.clone(),
            });
        }

        let count = tokenization.count_tokens(&raw, model_id)?;
        if count > p.max_tokens {
            let truncated = tokenization.truncate(&raw, p.max_tokens, model_id);
            per_placeholder_truncated.insert(p.name.clone(), true);
            truncated_values.insert(p.name.clone(), truncated);
        } else {
            per_placeholder_truncated.insert(p.name.clone(), false);
            truncated_values.insert(p.name.clone(), raw);
        }
    }

    // Step 2: build stable_prefix blocks from stable_prefix_sections in order.
    let stable_prefix_blocks: Vec<ContextBlockV1> = pack
        .stable_prefix_sections
        .iter()
        .enumerate()
        .map(|(i, s)| ContextBlockV1 {
            kind: stable_prefix_kind_for_section(&s.section_id),
            content: s.content_md.clone(),
            source_refs: Vec::new(),
            sensitivity: "low".to_string(),
            projection: "full".to_string(),
            order_key: format!("{:04}_{}", i, s.section_id),
        })
        .collect();

    // Step 3: deterministic template expansion.
    let mut expanded = pack.variable_suffix_template_md.clone();
    for (name, value) in &truncated_values {
        let needle = format!("{{{{{name}}}}}");
        expanded = expanded.replace(&needle, value);
    }

    let mut variable_suffix_blocks = vec![ContextBlockV1 {
        kind: "user_input".to_string(),
        content: expanded,
        source_refs: Vec::new(),
        sensitivity: "medium".to_string(),
        projection: "full".to_string(),
        order_key: "0000_VARIABLE_SUFFIX".to_string(),
    }];

    // Step 4: enforce total budget by truncating variable suffix to remaining tokens.
    let stable_prefix_text = render_working_context_text(&stable_prefix_blocks);
    let stable_prefix_tokens = tokenization.count_tokens(&stable_prefix_text, model_id)?;

    let max_total_tokens = pack.budgets.max_total_tokens;
    if stable_prefix_tokens > max_total_tokens {
        return Err(SpecPromptCompilerError::BudgetExceeded {
            stable_prefix_tokens,
            max_total_tokens,
        });
    }

    let separator = if stable_prefix_text.is_empty() {
        ""
    } else {
        "\n\n"
    };
    let prefix_plus_sep_text = format!("{stable_prefix_text}{separator}");
    let prefix_plus_sep_tokens = tokenization.count_tokens(&prefix_plus_sep_text, model_id)?;
    if prefix_plus_sep_tokens > max_total_tokens {
        return Err(SpecPromptCompilerError::BudgetExceeded {
            stable_prefix_tokens: prefix_plus_sep_tokens,
            max_total_tokens,
        });
    }

    let mut variable_suffix_truncated = false;
    let variable_suffix_budget = max_total_tokens.saturating_sub(prefix_plus_sep_tokens);
    let variable_suffix_text = render_working_context_text(&variable_suffix_blocks);
    let variable_suffix_tokens = tokenization.count_tokens(&variable_suffix_text, model_id)?;
    if variable_suffix_tokens > variable_suffix_budget {
        let truncated =
            tokenization.truncate(&variable_suffix_text, variable_suffix_budget, model_id);
        variable_suffix_blocks[0].content = truncated;
        variable_suffix_truncated = true;
    }

    let variable_suffix_text_final = render_working_context_text(&variable_suffix_blocks);
    let variable_suffix_tokens_final =
        tokenization.count_tokens(&variable_suffix_text_final, model_id)?;
    let full_prompt_text = format!("{stable_prefix_text}{separator}{variable_suffix_text_final}");
    let total_tokens = tokenization.count_tokens(&full_prompt_text, model_id)?;

    let stable_prefix_hash = blocks_sha256_hex(&stable_prefix_blocks)?;
    let variable_suffix_hash = blocks_sha256_hex(&variable_suffix_blocks)?;
    let full_prompt_hash = sha256_hex(&canonical_json_bytes_nfc(&json!({
        "stable_prefix_blocks": &stable_prefix_blocks,
        "variable_suffix_blocks": &variable_suffix_blocks,
    })));

    let stable_prefix = WorkingContextV1 {
        blocks: stable_prefix_blocks,
        token_budget: max_total_tokens,
        token_estimate: stable_prefix_tokens,
        build_id: format!("spec_prompt_pack:{}:stable_prefix", pack.pack_id),
    };
    let variable_suffix = WorkingContextV1 {
        blocks: variable_suffix_blocks,
        token_budget: variable_suffix_budget,
        token_estimate: variable_suffix_tokens_final,
        build_id: format!("spec_prompt_pack:{}:variable_suffix", pack.pack_id),
    };

    Ok(PromptEnvelopeV1 {
        stable_prefix,
        variable_suffix,
        stable_prefix_hash,
        variable_suffix_hash,
        full_prompt_hash,
        stable_prefix_tokens,
        variable_suffix_tokens: variable_suffix_tokens_final,
        total_tokens,
        truncation: PromptEnvelopeTruncationV1 {
            per_placeholder_truncated,
            variable_suffix_truncated,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{canonical_json_bytes_nfc, sha256_hex};
    use super::spec_prompt_pack::{
        BudgetsV1, PlaceholderSourceV1, PlaceholderV1, SpecPromptPackV1, StablePrefixSectionV1,
    };

    #[derive(Debug, Default)]
    struct CharTokenization;

    impl TokenizationService for CharTokenization {
        fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, TokenizerError> {
            Ok(text.chars().count() as u32)
        }

        fn truncate(&self, text: &str, limit: u32, _model: &str) -> String {
            text.chars().take(limit as usize).collect()
        }
    }

    fn minimal_pack() -> SpecPromptPackV1 {
        SpecPromptPackV1 {
            schema_version: "hsk.spec_prompt_pack@1".to_string(),
            pack_id: "spec_router_pack@1".to_string(),
            description: "test pack".to_string(),
            target_job_kind: "spec_router".to_string(),
            stable_prefix_sections: vec![StablePrefixSectionV1 {
                section_id: "SYSTEM_RULES".to_string(),
                content_md: "AAAA".to_string(),
            }],
            variable_suffix_template_md: "Hello".to_string(),
            placeholders: Vec::new(),
            required_outputs: Vec::new(),
            budgets: BudgetsV1 {
                max_total_tokens: 100,
                max_prompt_excerpt_tokens: 10,
                max_capsule_tokens: 10,
                max_capability_table_tokens: 10,
            },
        }
    }

    #[test]
    fn hashes_are_deterministic() {
        let mut pack = minimal_pack();
        pack.placeholders = vec![PlaceholderV1 {
            name: "name".to_string(),
            source: PlaceholderSourceV1::PromptRef,
            max_tokens: 50,
            required: true,
        }];
        pack.variable_suffix_template_md = "Hi {{name}}".to_string();

        let mut values: BTreeMap<String, String> = BTreeMap::new();
        values.insert("name".to_string(), "World".to_string());

        let tokenization = CharTokenization;
        let env1 =
            compile_spec_router_envelope(&pack, &values, &tokenization, "test-model").unwrap();
        let env2 =
            compile_spec_router_envelope(&pack, &values, &tokenization, "test-model").unwrap();

        assert_eq!(env1.stable_prefix_hash, env2.stable_prefix_hash);
        assert_eq!(env1.variable_suffix_hash, env2.variable_suffix_hash);
        assert_eq!(env1.full_prompt_hash, env2.full_prompt_hash);
    }

    #[test]
    fn placeholder_truncation_sets_flags() {
        let mut pack = minimal_pack();
        pack.placeholders = vec![PlaceholderV1 {
            name: "p".to_string(),
            source: PlaceholderSourceV1::PromptRef,
            max_tokens: 3,
            required: true,
        }];
        pack.variable_suffix_template_md = "X={{p}}".to_string();

        let mut values: BTreeMap<String, String> = BTreeMap::new();
        values.insert("p".to_string(), "ABCDE".to_string());

        let tokenization = CharTokenization;
        let env =
            compile_spec_router_envelope(&pack, &values, &tokenization, "test-model").unwrap();

        assert_eq!(
            env.truncation.per_placeholder_truncated.get("p"),
            Some(&true)
        );
        assert_eq!(env.variable_suffix.blocks[0].content, "X=ABC");
    }

    #[test]
    fn budget_truncation_sets_variable_suffix_flag_and_counts() {
        let mut pack = minimal_pack();
        pack.placeholders = Vec::new();
        pack.stable_prefix_sections[0].content_md = "AAAA".to_string();
        pack.variable_suffix_template_md = "BBBBBB".to_string();
        pack.budgets.max_total_tokens = 10;

        let values: BTreeMap<String, String> = BTreeMap::new();
        let tokenization = CharTokenization;
        let env =
            compile_spec_router_envelope(&pack, &values, &tokenization, "test-model").unwrap();

        assert!(env.truncation.variable_suffix_truncated);
        assert_eq!(env.stable_prefix_tokens, 4);
        assert_eq!(env.variable_suffix_tokens, 4);
        assert_eq!(env.total_tokens, 10);
        assert_eq!(env.variable_suffix.blocks[0].content, "BBBB");
    }

    #[test]
    fn full_prompt_hash_is_over_boundary_object() {
        let pack = minimal_pack();
        let values: BTreeMap<String, String> = BTreeMap::new();
        let tokenization = CharTokenization;
        let env =
            compile_spec_router_envelope(&pack, &values, &tokenization, "test-model").unwrap();

        let expected = sha256_hex(&canonical_json_bytes_nfc(&serde_json::json!({
            "stable_prefix_blocks": &env.stable_prefix.blocks,
            "variable_suffix_blocks": &env.variable_suffix.blocks,
        })));

        assert_eq!(env.full_prompt_hash, expected);
    }
}
