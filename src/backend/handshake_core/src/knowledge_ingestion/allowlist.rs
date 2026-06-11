//! MT-081 ProjectRootAllowlist: typed runtime allowlist policy deciding which
//! repo-relative paths may be registered (and therefore indexed) as
//! knowledge source roots.
//!
//! Layering: this policy gates ROOT REGISTRATION. The per-root FILE allowlist
//! (`knowledge_source_roots.allowlist_policy`, migration 0131) gates which
//! files inside an approved root are eligible. Deny patterns always win;
//! empty allow list means "nothing is allowlisted" (fail closed); the
//! operator-approval flag forces explicit human waving-through.
//!
//! Persistence: `knowledge_ingestion_root_policies` (0160). The default
//! policy below is compiled into the product so a fresh workspace is
//! protected before any operator configuration exists.

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use super::{IngestionError, IngestionResult};
use crate::storage::knowledge::KnowledgeRootKind;

/// Root kinds that ALWAYS require operator approval regardless of the policy
/// flag: external imports and operator folders bring non-repo content into
/// the index, which the WP constraints gate behind explicit operator intent.
pub const OPERATOR_GATED_ROOT_KINDS: &[KnowledgeRootKind] = &[
    KnowledgeRootKind::ExternalImport,
    KnowledgeRootKind::OperatorFolder,
];

/// Deny patterns every workspace starts with (machine-local / secret-prone /
/// derived-output paths that must never become index roots).
pub const DEFAULT_DENY_PATTERNS: &[&str] = &[
    ".git",
    ".git/**",
    "**/.git/**",
    "**/.ssh/**",
    "**/secrets/**",
    "**/node_modules/**",
    "**/target/**",
    "Handshake_Artifacts/managed_pgdata/**",
    // MT-091 #10: secret-bearing file shapes that must never be index roots
    // even when nested under an otherwise-allowed tree (dotenv files, private
    // keys, SSH identities, cloud-credential and registry-auth stores).
    "**/.env*",
    "**/*.pem",
    "**/id_rsa*",
    "**/.aws/**",
    "**/.npmrc",
];

/// Typed workspace policy for root registration (MT-081).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootRegistrationPolicy {
    /// Globs over normalized repo-relative POSIX paths. A candidate must
    /// match at least one allow pattern. Empty list = fail closed.
    pub allow_patterns: Vec<String>,
    /// Deny globs; any match rejects the candidate (deny wins over allow).
    pub deny_patterns: Vec<String>,
    /// When true, every registration additionally requires
    /// `operator_approved = true` from the caller.
    pub require_operator_approval: bool,
}

impl Default for RootRegistrationPolicy {
    fn default() -> Self {
        Self {
            allow_patterns: vec!["**".to_string()],
            deny_patterns: DEFAULT_DENY_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            require_operator_approval: false,
        }
    }
}

/// Outcome kind of one policy evaluation. String forms match the
/// `knowledge_ingestion_policy_decisions.verdict` CHECK constraint.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyVerdictKind {
    Allowed,
    DeniedPattern,
    DeniedNotAllowlisted,
    DeniedRequiresApproval,
}

impl PolicyVerdictKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::DeniedPattern => "denied_pattern",
            Self::DeniedNotAllowlisted => "denied_not_allowlisted",
            Self::DeniedRequiresApproval => "denied_requires_approval",
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

impl std::fmt::Display for PolicyVerdictKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PolicyVerdictKind {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allowed" => Ok(Self::Allowed),
            "denied_pattern" => Ok(Self::DeniedPattern),
            "denied_not_allowlisted" => Ok(Self::DeniedNotAllowlisted),
            "denied_requires_approval" => Ok(Self::DeniedRequiresApproval),
            other => Err(IngestionError::Validation(format!(
                "invalid policy verdict: {other}"
            ))),
        }
    }
}

/// Full evaluation result: verdict + the pattern that decided it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyVerdict {
    pub kind: PolicyVerdictKind,
    pub matched_pattern: Option<String>,
}

/// Compiled policy: glob sets are built once and reused per evaluation.
pub struct CompiledRootPolicy {
    policy: RootRegistrationPolicy,
    allow: GlobSet,
    allow_sources: Vec<String>,
    deny: GlobSet,
    deny_sources: Vec<String>,
}

impl std::fmt::Debug for CompiledRootPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledRootPolicy")
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

fn build_globset(patterns: &[String]) -> IngestionResult<(GlobSet, Vec<String>)> {
    let mut builder = GlobSetBuilder::new();
    let mut sources = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|err| {
            IngestionError::Validation(format!("invalid allowlist glob '{pattern}': {err}"))
        })?;
        builder.add(glob);
        sources.push(pattern.clone());
    }
    let set = builder
        .build()
        .map_err(|err| IngestionError::Validation(format!("allowlist globset build: {err}")))?;
    Ok((set, sources))
}

impl CompiledRootPolicy {
    /// Compile a typed policy; malformed globs are a typed validation error
    /// (a policy that cannot be compiled never silently allows anything).
    pub fn compile(policy: RootRegistrationPolicy) -> IngestionResult<Self> {
        let (allow, allow_sources) = build_globset(&policy.allow_patterns)?;
        let (deny, deny_sources) = build_globset(&policy.deny_patterns)?;
        Ok(Self {
            policy,
            allow,
            allow_sources,
            deny,
            deny_sources,
        })
    }

    pub fn policy(&self) -> &RootRegistrationPolicy {
        &self.policy
    }

    /// Evaluate a candidate repo-relative POSIX path.
    ///
    /// Order: deny patterns -> allow patterns (empty = fail closed) ->
    /// operator approval (policy flag OR an operator-gated root kind).
    pub fn evaluate(
        &self,
        candidate_path: &str,
        root_kind: KnowledgeRootKind,
        operator_approved: bool,
    ) -> PolicyVerdict {
        // The empty path addresses the repo root itself; match it as ".".
        let match_path = if candidate_path.is_empty() {
            "."
        } else {
            candidate_path
        };

        let deny_hits = self.deny.matches(match_path);
        if let Some(first) = deny_hits.first() {
            return PolicyVerdict {
                kind: PolicyVerdictKind::DeniedPattern,
                matched_pattern: self.deny_sources.get(*first).cloned(),
            };
        }

        let allow_hits = self.allow.matches(match_path);
        let Some(first_allow) = allow_hits.first() else {
            return PolicyVerdict {
                kind: PolicyVerdictKind::DeniedNotAllowlisted,
                matched_pattern: None,
            };
        };
        let allow_pattern = self.allow_sources.get(*first_allow).cloned();

        let needs_approval =
            self.policy.require_operator_approval || OPERATOR_GATED_ROOT_KINDS.contains(&root_kind);
        if needs_approval && !operator_approved {
            return PolicyVerdict {
                kind: PolicyVerdictKind::DeniedRequiresApproval,
                matched_pattern: allow_pattern,
            };
        }

        PolicyVerdict {
            kind: PolicyVerdictKind::Allowed,
            matched_pattern: allow_pattern,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(policy: RootRegistrationPolicy) -> CompiledRootPolicy {
        CompiledRootPolicy::compile(policy).expect("compile policy")
    }

    #[test]
    fn default_policy_denies_machine_local_and_secret_paths() {
        let policy = compiled(RootRegistrationPolicy::default());
        for denied in [
            ".git",
            "src/.git/hooks",
            "ops/secrets/prod",
            "app/node_modules/x",
            "src/backend/target/debug",
            "Handshake_Artifacts/managed_pgdata/base",
            // MT-091 #10 secret-bearing file shapes.
            ".env",
            "app/.env.local",
            "deploy/.env.production",
            "certs/server.pem",
            "keys/id_rsa",
            "keys/id_rsa.pub",
            "home/.aws/credentials",
            "project/.npmrc",
        ] {
            let verdict = policy.evaluate(denied, KnowledgeRootKind::ProjectRepo, false);
            assert_eq!(
                verdict.kind,
                PolicyVerdictKind::DeniedPattern,
                "expected deny for {denied}"
            );
            assert!(verdict.matched_pattern.is_some());
        }
        let allowed = policy.evaluate("src/backend", KnowledgeRootKind::ProjectRepo, false);
        assert!(allowed.kind.is_allowed());
        // A file legitimately named `environment.rs` must NOT be caught by the
        // `.env*` shape (deny is anchored at the path-segment dot).
        assert!(policy
            .evaluate("src/environment.rs", KnowledgeRootKind::ProjectRepo, false)
            .kind
            .is_allowed());
    }

    #[test]
    fn empty_allow_list_fails_closed() {
        let policy = compiled(RootRegistrationPolicy {
            allow_patterns: vec![],
            deny_patterns: vec![],
            require_operator_approval: false,
        });
        let verdict = policy.evaluate("docs", KnowledgeRootKind::ProjectRepo, false);
        assert_eq!(verdict.kind, PolicyVerdictKind::DeniedNotAllowlisted);
    }

    #[test]
    fn deny_wins_over_allow() {
        let policy = compiled(RootRegistrationPolicy {
            allow_patterns: vec!["docs/**".to_string()],
            deny_patterns: vec!["docs/private/**".to_string()],
            require_operator_approval: false,
        });
        assert!(policy
            .evaluate("docs/public", KnowledgeRootKind::ProjectRepo, false)
            .kind
            .is_allowed());
        assert_eq!(
            policy
                .evaluate("docs/private/keys", KnowledgeRootKind::ProjectRepo, false)
                .kind,
            PolicyVerdictKind::DeniedPattern
        );
    }

    #[test]
    fn operator_gated_kinds_always_require_approval() {
        let policy = compiled(RootRegistrationPolicy {
            allow_patterns: vec!["**".to_string()],
            deny_patterns: vec![],
            require_operator_approval: false,
        });
        let denied = policy.evaluate("imports/papers", KnowledgeRootKind::ExternalImport, false);
        assert_eq!(denied.kind, PolicyVerdictKind::DeniedRequiresApproval);
        let approved = policy.evaluate("imports/papers", KnowledgeRootKind::ExternalImport, true);
        assert!(approved.kind.is_allowed());
    }

    #[test]
    fn approval_flag_applies_to_every_kind() {
        let policy = compiled(RootRegistrationPolicy {
            allow_patterns: vec!["**".to_string()],
            deny_patterns: vec![],
            require_operator_approval: true,
        });
        let denied = policy.evaluate("src", KnowledgeRootKind::ProjectRepo, false);
        assert_eq!(denied.kind, PolicyVerdictKind::DeniedRequiresApproval);
        assert!(policy
            .evaluate("src", KnowledgeRootKind::ProjectRepo, true)
            .kind
            .is_allowed());
    }

    #[test]
    fn malformed_glob_is_a_typed_error() {
        let err = CompiledRootPolicy::compile(RootRegistrationPolicy {
            allow_patterns: vec!["a{".to_string()],
            deny_patterns: vec![],
            require_operator_approval: false,
        })
        .expect_err("malformed glob must not compile");
        assert!(matches!(err, IngestionError::Validation(_)));
    }

    #[test]
    fn repo_root_itself_is_evaluated_as_dot() {
        let policy = compiled(RootRegistrationPolicy {
            allow_patterns: vec!["*".to_string()],
            deny_patterns: vec![],
            require_operator_approval: false,
        });
        assert!(policy
            .evaluate("", KnowledgeRootKind::ProjectRepo, false)
            .kind
            .is_allowed());
    }
}
