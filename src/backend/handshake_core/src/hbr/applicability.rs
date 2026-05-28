use std::path::{Path, PathBuf};

use globset::Glob;
use serde::Deserialize;

use super::registry::HbrRule;

// Path-derivable tag coverage is intentionally limited to a few high-signal
// tags whose touched-path globs are unambiguous. Other tags
// (shared_state, concurrency_primitive, operator_surface, inter_agent, ...)
// become Applicable via explicitly declared tags — the hydrator's
// HYDRATOR_TAG_EXPANSIONS derives those declared tags — not via path globs.
// A packet that needs one of those rules must declare the tag; relying on
// path inference alone would risk NA-by-omission. Extend this table only
// when a tag has a reliable, unambiguous path signature.
const TAG_GLOBS: &[(&str, &[&str])] = &[
    ("model_invocation", &["**/model_runtime/**"]),
    ("crdt", &["**/kernel/crdt/**"]),
    ("process_lifecycle", &["**/process_ledger/**"]),
    (
        "automation_surface",
        &["app/src-tauri/**", "tests/visual/**"],
    ),
];

#[derive(Debug, Clone, Deserialize)]
pub struct HbrApplicability {
    pub predicate_text: String,
    pub tags: Vec<String>,
}

impl HbrApplicability {
    pub fn evaluate(rule: &HbrRule, ctx: &PacketContext) -> Applicability {
        if let Some(override_entry) = ctx
            .not_applicable_overrides
            .iter()
            .find(|entry| entry.hbr_id == rule.id)
        {
            let reason = if override_entry.reason.trim().is_empty() {
                format!("not_applicable override matched for {}", rule.id)
            } else {
                override_entry.reason.trim().to_string()
            };
            return Applicability::NotApplicable { reason };
        }

        if rule_tag_declared(rule, ctx) || rule_tag_path_matched(rule, ctx) {
            return Applicability::Applicable;
        }

        Applicability::NotApplicable {
            reason: format!(
                "No declared tags or touched paths matched {} for {}.",
                rule.id, ctx.wp_id
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Applicability {
    Applicable,
    NotApplicable { reason: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PacketContext {
    pub wp_id: String,
    pub touched_paths: Vec<PathBuf>,
    pub tags_declared: Vec<String>,
    pub not_applicable_overrides: Vec<HbrNaOverride>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HbrNaOverride {
    pub hbr_id: String,
    pub reason: String,
}

fn rule_tag_declared(rule: &HbrRule, ctx: &PacketContext) -> bool {
    rule.applicability.tags.iter().any(|rule_tag| {
        ctx.tags_declared
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(rule_tag))
    })
}

fn rule_tag_path_matched(rule: &HbrRule, ctx: &PacketContext) -> bool {
    rule.applicability.tags.iter().any(|rule_tag| {
        TAG_GLOBS
            .iter()
            .filter(|(tag, _)| tag.eq_ignore_ascii_case(rule_tag))
            .flat_map(|(_, patterns)| *patterns)
            .any(|pattern| {
                ctx.touched_paths
                    .iter()
                    .any(|path| path_matches_glob(path, pattern))
            })
    })
}

fn path_matches_glob(path: &Path, pattern: &str) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    Glob::new(pattern)
        .expect("HBR starter tag glob is valid")
        .compile_matcher()
        .is_match(normalized)
}
