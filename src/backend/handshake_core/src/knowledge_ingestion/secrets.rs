//! MT-091 SecretRedactionPreflight: scan candidate source text for secrets
//! BEFORE any span content is stored; block or redact per severity.
//!
//! Field alignment: the pattern-set + Shannon-entropy approach mirrors the
//! established secret scanners (gitleaks / trufflehog / detect-secrets):
//! high-signal literal patterns for well-known credential shapes, plus an
//! entropy gate on generic `key = value` assignments to keep documentation
//! placeholders out of the findings.
//!
//! Outcome policy (documented decision):
//! * HIGH severity (private key blocks, AWS keys, credential-bearing
//!   connection URLs) -> BLOCK the file: receipt status `blocked` /
//!   `SECRET_BLOCKED`, source `redaction_state = redacted`, NO span content
//!   stored at all.
//! * MEDIUM severity (vendor tokens, JWTs, high-entropy generic
//!   assignments) -> REDACT: extraction runs over the redacted text, span
//!   content carries `[REDACTED:<kind>]` markers, the source records
//!   `redaction_state = partial`, receipts count the redactions.
//!
//! Raw secret bytes never reach a durable row in either mode. Findings
//! deliberately carry NO excerpt of the matched value — only kind, location,
//! and length.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Kinds of secrets the preflight detects.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    PrivateKeyBlock,
    AwsAccessKeyId,
    AwsSecretAssignment,
    ConnectionStringCredentials,
    GithubToken,
    SlackToken,
    GoogleApiKey,
    JsonWebToken,
    GenericHighEntropyAssignment,
}

impl SecretKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrivateKeyBlock => "private_key_block",
            Self::AwsAccessKeyId => "aws_access_key_id",
            Self::AwsSecretAssignment => "aws_secret_assignment",
            Self::ConnectionStringCredentials => "connection_string_credentials",
            Self::GithubToken => "github_token",
            Self::SlackToken => "slack_token",
            Self::GoogleApiKey => "google_api_key",
            Self::JsonWebToken => "json_web_token",
            Self::GenericHighEntropyAssignment => "generic_high_entropy_assignment",
        }
    }

    /// HIGH severity blocks the whole file; MEDIUM redacts regions.
    pub fn severity(&self) -> SecretSeverity {
        match self {
            Self::PrivateKeyBlock
            | Self::AwsAccessKeyId
            | Self::AwsSecretAssignment
            | Self::ConnectionStringCredentials => SecretSeverity::High,
            _ => SecretSeverity::Medium,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SecretSeverity {
    Medium,
    High,
}

/// One detected secret region. Carries NO content of the match.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretFinding {
    pub kind: SecretKind,
    /// 1-based line of the match start.
    pub line: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub matched_len: usize,
}

/// Scan outcome: findings plus the blocking decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretScanReport {
    pub findings: Vec<SecretFinding>,
}

impl SecretScanReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Any HIGH-severity finding blocks the file outright.
    pub fn must_block(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.kind.severity() == SecretSeverity::High)
    }

    pub fn redaction_count(&self) -> usize {
        self.findings.len()
    }
}

struct PatternSpec {
    kind: SecretKind,
    regex: &'static Lazy<Regex>,
}

static PRIVATE_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY(?: BLOCK)?-----(?s:.*?)(?:-----END (?:RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY(?: BLOCK)?-----|\z)",
    )
    .expect("private key regex")
});
static AWS_ACCESS_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16}\b")
        .expect("aws access key regex")
});
static AWS_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\baws_?secret_?(?:access_?)?key\b\s*[:=]\s*["']?[A-Za-z0-9/+=]{30,}"#)
        .expect("aws secret regex")
});
static CONNECTION_STRING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[^/\s:@'\x22]+:[^@\s/'\x22]{6,}@")
        .expect("conn string regex")
});
static GITHUB_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bgh[pousr]_[A-Za-z0-9]{36,255}\b").expect("github token regex"));
static SLACK_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bxox[baprs]-[0-9A-Za-z-]{10,}\b").expect("slack token regex"));
static GOOGLE_API_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bAIza[0-9A-Za-z_-]{35}\b").expect("google api key regex"));
static JWT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b")
        .expect("jwt regex")
});
static GENERIC_ASSIGNMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)\b(?:api[_-]?key|api[_-]?token|auth[_-]?token|access[_-]?token|secret|password|passwd|pwd)\b\s*[:=]\s*["']?(?P<value>[^\s"']{8,})"#,
    )
    .expect("generic assignment regex")
});

const PATTERNS: &[PatternSpec] = &[
    PatternSpec {
        kind: SecretKind::PrivateKeyBlock,
        regex: &PRIVATE_KEY,
    },
    PatternSpec {
        kind: SecretKind::AwsAccessKeyId,
        regex: &AWS_ACCESS_KEY,
    },
    PatternSpec {
        kind: SecretKind::AwsSecretAssignment,
        regex: &AWS_SECRET,
    },
    PatternSpec {
        kind: SecretKind::ConnectionStringCredentials,
        regex: &CONNECTION_STRING,
    },
    PatternSpec {
        kind: SecretKind::GithubToken,
        regex: &GITHUB_TOKEN,
    },
    PatternSpec {
        kind: SecretKind::SlackToken,
        regex: &SLACK_TOKEN,
    },
    PatternSpec {
        kind: SecretKind::GoogleApiKey,
        regex: &GOOGLE_API_KEY,
    },
    PatternSpec {
        kind: SecretKind::JsonWebToken,
        regex: &JWT,
    },
];

/// Shannon entropy in bits per character.
pub fn shannon_entropy(value: &str) -> f64 {
    if value.is_empty() {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for c in value.chars() {
        *counts.entry(c).or_insert(0usize) += 1;
    }
    let len = value.chars().count() as f64;
    counts
        .values()
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Placeholder shapes that documentation legitimately contains.
fn looks_like_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains('<')
        || value.contains('>')
        || value.contains("${")
        || value.contains("{{")
        || lower.contains("example")
        || lower.contains("changeme")
        || lower.contains("change-me")
        || lower.contains("placeholder")
        || lower.contains("dummy")
        || lower.contains("your_")
        || lower.contains("your-")
        || lower.contains("xxxx")
        || lower.contains("redacted")
}

/// Entropy gate for generic assignments: random credentials sit well above
/// 3.5 bits/char; English words and placeholders sit below.
const GENERIC_ENTROPY_THRESHOLD: f64 = 3.5;

fn line_of(text: &str, byte_offset: usize) -> u32 {
    (text[..byte_offset].bytes().filter(|b| *b == b'\n').count() + 1) as u32
}

/// Scan text for secrets. Overlapping findings are merged (first kind wins)
/// so redaction never double-splices a region.
pub fn scan_text(text: &str) -> SecretScanReport {
    let mut findings: Vec<SecretFinding> = Vec::new();

    for spec in PATTERNS {
        for m in spec.regex.find_iter(text) {
            findings.push(SecretFinding {
                kind: spec.kind,
                line: line_of(text, m.start()),
                byte_start: m.start(),
                byte_end: m.end(),
                matched_len: m.end() - m.start(),
            });
        }
    }

    for captures in GENERIC_ASSIGNMENT.captures_iter(text) {
        let Some(value) = captures.name("value") else {
            continue;
        };
        let full = captures.get(0).expect("regex match 0");
        let candidate = value.as_str();
        if looks_like_placeholder(candidate) {
            continue;
        }
        if shannon_entropy(candidate) < GENERIC_ENTROPY_THRESHOLD {
            continue;
        }
        findings.push(SecretFinding {
            kind: SecretKind::GenericHighEntropyAssignment,
            line: line_of(text, full.start()),
            byte_start: full.start(),
            byte_end: full.end(),
            matched_len: full.end() - full.start(),
        });
    }

    // Sort + merge overlaps so redaction is single-pass safe.
    findings.sort_by_key(|f| (f.byte_start, std::cmp::Reverse(f.byte_end)));
    let mut merged: Vec<SecretFinding> = Vec::with_capacity(findings.len());
    for finding in findings {
        match merged.last_mut() {
            Some(last) if finding.byte_start < last.byte_end => {
                if finding.byte_end > last.byte_end {
                    last.byte_end = finding.byte_end;
                    last.matched_len = last.byte_end - last.byte_start;
                }
            }
            _ => merged.push(finding),
        }
    }

    SecretScanReport { findings: merged }
}

/// Replace every finding region with `[REDACTED:<kind>]`. The returned text
/// contains no byte of any matched region.
pub fn redact_text(text: &str, report: &SecretScanReport) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for finding in &report.findings {
        if finding.byte_start < cursor {
            continue; // defensive: merged findings should not overlap
        }
        out.push_str(&text[cursor..finding.byte_start]);
        out.push_str(&format!("[REDACTED:{}]", finding.kind.as_str()));
        cursor = finding.byte_end;
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: all values below are FAKE fixtures shaped like real credentials.

    #[test]
    fn detects_fake_aws_key_and_private_key_block_as_blocking() {
        let text = "config:\n  aws_key = AKIAIOSFODNN7EXAMPLE\n-----BEGIN RSA PRIVATE KEY-----\nMIIfake+keymaterial/lines\n-----END RSA PRIVATE KEY-----\n";
        let report = scan_text(text);
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::AwsAccessKeyId));
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::PrivateKeyBlock));
        assert!(report.must_block(), "high severity must block the file");

        let redacted = redact_text(text, &report);
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!redacted.contains("keymaterial"));
        assert!(redacted.contains("[REDACTED:aws_access_key_id]"));
        assert!(redacted.contains("[REDACTED:private_key_block]"));
    }

    #[test]
    fn detects_tokens_and_connection_strings() {
        // Fake values shaped exactly like the real credentials: GitHub PATs
        // carry 36+ chars after the prefix, Google API keys exactly 35 after
        // `AIza` ending on a word boundary.
        let text = concat!(
            "github: ghp_aB3dE6gH9jK2mN5pQ8sT1vW4yZ7cF0eH3iZ9X\n",
            "slack: xoxb-1234567890-abcdefghij\n",
            "google: AIzaSyA1234567890abcdefghijklmnopqrstuv\n",
            "db: postgres://svc_user:sup3rs3cretpw@db.internal:5432/app\n",
        );
        let report = scan_text(text);
        let kinds: Vec<_> = report.findings.iter().map(|f| f.kind).collect();
        assert!(kinds.contains(&SecretKind::GithubToken));
        assert!(kinds.contains(&SecretKind::SlackToken));
        assert!(kinds.contains(&SecretKind::GoogleApiKey));
        assert!(kinds.contains(&SecretKind::ConnectionStringCredentials));
        assert!(report.must_block(), "credential URL is high severity");
    }

    #[test]
    fn entropy_gate_keeps_placeholders_out_and_random_values_in() {
        let placeholders = concat!(
            "password = <your-password>\n",
            "api_key = ${API_KEY}\n",
            "token: changeme123\n",
            "secret = example_secret_value\n",
        );
        let report = scan_text(placeholders);
        assert!(
            report.is_clean(),
            "placeholders must not be findings: {:?}",
            report.findings
        );

        let real = "api_key = q7Xz2pLm9KvR4tNw8YbD3cFgH6sJaUeP\n";
        let report = scan_text(real);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].kind,
            SecretKind::GenericHighEntropyAssignment
        );
        assert!(
            !report.must_block(),
            "generic assignment redacts, not blocks"
        );
        let redacted = redact_text(real, &report);
        assert!(!redacted.contains("q7Xz2pLm9KvR4tNw8YbD3cFgH6sJaUeP"));
    }

    #[test]
    fn jwt_is_detected_as_medium_severity() {
        let text = "bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\n";
        let report = scan_text(text);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, SecretKind::JsonWebToken);
        assert!(!report.must_block());
    }

    #[test]
    fn findings_carry_locations_but_no_content() {
        let text = "line1\nkey = AKIAIOSFODNN7EXAMPLE\n";
        let report = scan_text(text);
        let finding = &report.findings[0];
        assert_eq!(finding.line, 2);
        assert_eq!(
            &text[finding.byte_start..finding.byte_end],
            "AKIAIOSFODNN7EXAMPLE"
        );
        let serialized = serde_json::to_string(&report.findings).expect("serialize");
        assert!(
            !serialized.contains("AKIA"),
            "serialized findings must not leak the secret"
        );
    }

    #[test]
    fn truncated_private_key_block_still_blocks_to_end_of_text() {
        let text = "-----BEGIN PRIVATE KEY-----\nMIIfakekeymaterial\n(no end marker)";
        let report = scan_text(text);
        assert!(report.must_block());
        let redacted = redact_text(text, &report);
        assert!(!redacted.contains("fakekeymaterial"));
    }

    #[test]
    fn shannon_entropy_behaves() {
        assert!(shannon_entropy("aaaaaaaa") < 0.1);
        assert!(shannon_entropy("q7Xz2pLm9KvR4tNw") > 3.5);
        assert_eq!(shannon_entropy(""), 0.0);
    }
}
