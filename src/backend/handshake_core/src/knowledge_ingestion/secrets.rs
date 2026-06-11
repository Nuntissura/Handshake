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
    /// A PEM-shaped key block whose label is not a standard PRIVATE KEY
    /// label (e.g. `-----BEGIN OPENSSH KEY-----`, vendor key blocks).
    NonPemLabelKeyBlock,
    /// A standalone high-entropy base64 blob with no PEM header — the body of
    /// a private key pasted without its armor (heuristic: length + charset +
    /// entropy on a standalone line).
    HeaderlessKeyBlob,
    AwsAccessKeyId,
    AwsSecretAssignment,
    ConnectionStringCredentials,
    GithubToken,
    /// Fine-grained GitHub personal access token (`github_pat_...`).
    GithubFineGrainedPat,
    SlackToken,
    /// Slack app-level token (`xapp-...`).
    SlackAppToken,
    GoogleApiKey,
    JsonWebToken,
    GenericHighEntropyAssignment,
}

impl SecretKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrivateKeyBlock => "private_key_block",
            Self::NonPemLabelKeyBlock => "non_pem_label_key_block",
            Self::HeaderlessKeyBlob => "headerless_key_blob",
            Self::AwsAccessKeyId => "aws_access_key_id",
            Self::AwsSecretAssignment => "aws_secret_assignment",
            Self::ConnectionStringCredentials => "connection_string_credentials",
            Self::GithubToken => "github_token",
            Self::GithubFineGrainedPat => "github_fine_grained_pat",
            Self::SlackToken => "slack_token",
            Self::SlackAppToken => "slack_app_token",
            Self::GoogleApiKey => "google_api_key",
            Self::JsonWebToken => "json_web_token",
            Self::GenericHighEntropyAssignment => "generic_high_entropy_assignment",
        }
    }

    /// HIGH severity blocks the whole file; MEDIUM redacts regions.
    pub fn severity(&self) -> SecretSeverity {
        match self {
            // Key material in any shape (armored, alternately-labelled, or a
            // bare high-entropy blob) is the highest-impact leak: block.
            Self::PrivateKeyBlock
            | Self::NonPemLabelKeyBlock
            | Self::HeaderlessKeyBlob
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
// Fine-grained PAT: `github_pat_` + 22 base62 + `_` + 59 base62 (GitHub's
// documented shape). Matched before the classic `ghp_` rule so the longer,
// more specific token wins.
static GITHUB_FINE_GRAINED_PAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\bgithub_pat_[A-Za-z0-9]{22}_[A-Za-z0-9]{59}\b")
        .expect("github fine-grained pat regex")
});
static SLACK_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bxox[baprs]-[0-9A-Za-z-]{10,}\b").expect("slack token regex"));
// Slack app-level token: `xapp-` + version digit + `-` + alnum/`-` body.
static SLACK_APP_TOKEN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\bxapp-[0-9]-[0-9A-Za-z-]{10,}\b").expect("slack app token regex")
});
// Alternately-labelled key block: any PEM-armored `BEGIN ... KEY` block whose
// label is NOT a standard PRIVATE KEY label already covered by PRIVATE_KEY
// (e.g. `OPENSSH PRIVATE KEY` without the explicit prefix-form, vendor key
// labels). The PRIVATE_KEY rule runs first; merge logic drops the overlap so
// a single block is never double-counted.
static NON_PEM_LABEL_KEY_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"-----BEGIN [A-Z0-9 ]*KEY(?: BLOCK)?-----(?s:.*?)(?:-----END [A-Z0-9 ]*KEY(?: BLOCK)?-----|\z)",
    )
    .expect("non-pem-label key block regex")
});
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
    // PRIVATE_KEY before NON_PEM_LABEL_KEY_BLOCK: on a standard private-key
    // block both match the same region; scan_text's overlap merge keeps the
    // FIRST-inserted kind, so the specific `private_key_block` wins.
    PatternSpec {
        kind: SecretKind::PrivateKeyBlock,
        regex: &PRIVATE_KEY,
    },
    PatternSpec {
        kind: SecretKind::NonPemLabelKeyBlock,
        regex: &NON_PEM_LABEL_KEY_BLOCK,
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
    // Fine-grained PAT before the classic token: distinct prefixes, but keep
    // the more specific rule earlier for clarity.
    PatternSpec {
        kind: SecretKind::GithubFineGrainedPat,
        regex: &GITHUB_FINE_GRAINED_PAT,
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
        kind: SecretKind::SlackAppToken,
        regex: &SLACK_APP_TOKEN,
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

/// Headerless key-blob heuristic. A standalone base64 line this long with
/// near-maximal entropy is overwhelmingly key material pasted without its PEM
/// armor (base64 of random bytes approaches ~6 bits/char; English prose and
/// source identifiers sit far below 4.8). Bounded above so an enormous data
/// URI does not anchor a pathological match.
const BLOB_MIN_LEN: usize = 64;
const BLOB_MAX_LEN: usize = 8192;
const BLOB_ENTROPY_THRESHOLD: f64 = 4.8;

fn is_base64_line(line: &str) -> bool {
    !line.is_empty()
        && line
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

/// Detect standalone headerless base64 key blobs. Each whole standalone line
/// (after trimming surrounding whitespace) that is pure base64, within the
/// length window, and high-entropy is reported over its ORIGINAL byte span.
fn scan_headerless_key_blobs(text: &str, out: &mut Vec<SecretFinding>) {
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let lead_ws = trimmed.len() - trimmed.trim_start().len();
        let body = trimmed.trim();
        if body.len() < BLOB_MIN_LEN || body.len() > BLOB_MAX_LEN {
            continue;
        }
        if !is_base64_line(body) {
            continue;
        }
        if looks_like_placeholder(body) {
            continue;
        }
        if shannon_entropy(body) < BLOB_ENTROPY_THRESHOLD {
            continue;
        }
        let start = line_start + lead_ws;
        let end = start + body.len();
        out.push(SecretFinding {
            kind: SecretKind::HeaderlessKeyBlob,
            line: line_of(text, start),
            byte_start: start,
            byte_end: end,
            matched_len: body.len(),
        });
    }
}

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

    // Headerless key blobs (standalone high-entropy base64 lines).
    scan_headerless_key_blobs(text, &mut findings);

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

/// Outcome of redacting one span against the whole-file findings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanRedactionOutcome {
    /// Span content after redaction (no raw secret byte survives).
    pub content: String,
    /// How many whole-file findings touched this span (including findings
    /// that only partially overlap the span boundary).
    pub redactions: usize,
}

/// Map the largest byte offset `<= limit` that is a valid char boundary of `s`.
fn clamp_to_boundary(s: &str, mut offset: usize) -> usize {
    if offset >= s.len() {
        return s.len();
    }
    while offset > 0 && !s.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// Redact one span using WHOLE-FILE findings (MT-091 #1 cross-span boundary
/// fix). `span_byte_start` is the span's start offset in the ORIGINAL scanned
/// text; `content` is the span's stored text. Any whole-file finding that
/// overlaps `[span_byte_start, span_byte_start + content.len())` — even a
/// secret that STRADDLES the span boundary and is only partially inside this
/// span — has its in-span portion replaced with a `[REDACTED:<kind>]` marker.
///
/// This catches secrets that a per-span re-scan would miss because each
/// window holds only a fragment that matches no pattern on its own.
pub fn redact_span_with_whole_file_findings(
    content: &str,
    span_byte_start: usize,
    report: &SecretScanReport,
) -> SpanRedactionOutcome {
    // Span byte range in original-text coordinates. The stored content can be
    // SHORTER than the source slice (e.g. a windowed code span drops the
    // trailing newline), so map by content length, not by a recomputed end.
    let span_end = span_byte_start.saturating_add(content.len());
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0usize; // span-relative
    let mut redactions = 0usize;
    for finding in &report.findings {
        // Overlap test in original-text coordinates.
        if finding.byte_end <= span_byte_start || finding.byte_start >= span_end {
            continue;
        }
        // Clip the finding to the span, then translate to span-relative bytes.
        let clip_start = finding.byte_start.max(span_byte_start) - span_byte_start;
        let clip_end = finding.byte_end.min(span_end) - span_byte_start;
        let rel_start = clamp_to_boundary(content, clip_start);
        let rel_end = clamp_to_boundary(content, clip_end);
        if rel_end <= cursor {
            // Already covered by a previous (merged/overlapping) finding.
            continue;
        }
        let splice_start = rel_start.max(cursor);
        if splice_start > cursor {
            out.push_str(&content[cursor..splice_start]);
        }
        out.push_str(&format!("[REDACTED:{}]", finding.kind.as_str()));
        cursor = rel_end;
        redactions += 1;
    }
    if cursor < content.len() {
        out.push_str(&content[cursor..]);
    }
    SpanRedactionOutcome {
        content: out,
        redactions,
    }
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

    // -- MT-091 #2 new patterns ------------------------------------------------

    #[test]
    fn detects_github_fine_grained_pat() {
        // github_pat_ + 22 + _ + 59 base62 chars (FAKE shape).
        let body22 = "A1b2C3d4E5f6G7h8I9j0K1";
        let body59 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456";
        assert_eq!(body22.len(), 22);
        assert_eq!(body59.len(), 59);
        let text = format!("token = github_pat_{body22}_{body59}\n");
        let report = scan_text(&text);
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::GithubFineGrainedPat));
        // A token redacts (MEDIUM), consistent with the classic GitHub token.
        assert!(!report.must_block(), "fine-grained PAT redacts, not blocks");
        let redacted = redact_text(&text, &report);
        assert!(!redacted.contains(body59));
        assert!(redacted.contains("[REDACTED:github_fine_grained_pat]"));
    }

    #[test]
    fn detects_slack_app_token() {
        let text = "slack_app = xapp-1-A012BCDEFGH-1234567890-abcdef0123456789\n";
        let report = scan_text(text);
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::SlackAppToken));
        let redacted = redact_text(text, &report);
        assert!(!redacted.contains("A012BCDEFGH"));
    }

    #[test]
    fn detects_headerless_base64_key_blob() {
        // A standalone high-entropy base64 line with NO PEM armor.
        let blob = "MIIBVAIBADANBgkqhkiG9w0BAQEFAASCAT4wggE6AgEAAkEA3Tn7HkQxZpLm9KvR4tNw8YbD3cFgH6sJaUePq7Xz2pLm9KvR4tNwQ==";
        assert!(blob.len() >= BLOB_MIN_LEN);
        let text = format!("key:\n{blob}\nend\n");
        let report = scan_text(&text);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == SecretKind::HeaderlessKeyBlob),
            "headerless blob must be detected: {:?}",
            report.findings
        );
        assert!(report.must_block(), "key material blocks the file");
        let redacted = redact_text(&text, &report);
        assert!(!redacted.contains(blob));
    }

    #[test]
    fn headerless_blob_heuristic_ignores_prose_and_short_lines() {
        // Long English prose line: low entropy, not base64 charset.
        let prose = "The quick brown fox jumps over the lazy dog while the cat sleeps soundly in the warm afternoon sun today.";
        // Short base64 identifier: under the length floor.
        let short = "aGVsbG8gd29ybGQ=";
        let text = format!("{prose}\n{short}\n");
        let report = scan_text(&text);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.kind == SecretKind::HeaderlessKeyBlob),
            "prose / short base64 must not be a key blob: {:?}",
            report.findings
        );
    }

    #[test]
    fn detects_non_pem_label_key_block() {
        // OPENSSH key armor — labelled block the standard PRIVATE_KEY prefix
        // form does not literally enumerate; the alt-label rule catches it.
        let text = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEFAKE\n-----END OPENSSH PRIVATE KEY-----\n";
        let report = scan_text(text);
        assert!(report.must_block());
        // The standard rule already enumerates `OPENSSH PRIVATE KEY`; assert
        // at least one key-block kind fires and the body is redacted.
        assert!(report.findings.iter().any(|f| matches!(
            f.kind,
            SecretKind::PrivateKeyBlock | SecretKind::NonPemLabelKeyBlock
        )));
        let redacted = redact_text(text, &report);
        assert!(!redacted.contains("b3BlbnNzaC1rZXktdjEFAKE"));
    }

    #[test]
    fn non_standard_label_key_block_is_caught_by_alt_rule() {
        // A vendor key label the PRIVATE_KEY rule does NOT enumerate.
        let text =
            "-----BEGIN VENDOR SIGNING KEY-----\nQUJDREVGRkFLRQ==\n-----END VENDOR SIGNING KEY-----\n";
        let report = scan_text(text);
        assert!(report
            .findings
            .iter()
            .any(|f| f.kind == SecretKind::NonPemLabelKeyBlock));
        assert!(report.must_block());
    }

    // -- MT-091 #1 cross-span boundary redaction ------------------------------

    #[test]
    fn whole_file_redaction_catches_boundary_split_secret() {
        // Build text where a MEDIUM secret (JWT) sits at a known byte offset,
        // then split it into two "spans" at a byte INSIDE the secret. Neither
        // half re-scanned alone would match, but whole-file findings redact
        // both partial overlaps.
        let prefix = "line one\nbearer ";
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let suffix = "\ntrailing line\n";
        let text = format!("{prefix}{jwt}{suffix}");
        let report = scan_text(&text);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == SecretKind::JsonWebToken),
            "whole-file scan must see the JWT"
        );

        // Cut point: 10 bytes into the JWT.
        let jwt_start = prefix.len();
        let cut = jwt_start + 10;
        let span_a = &text[..cut];
        let span_b = &text[cut..];

        // Per-span re-scan (the OLD behaviour) misses both fragments.
        assert!(
            scan_text(span_a).is_clean(),
            "fragment A alone should not match"
        );
        assert!(
            scan_text(span_b).is_clean(),
            "fragment B alone should not match"
        );

        // Whole-file redaction over each span removes every secret byte.
        let out_a = redact_span_with_whole_file_findings(span_a, 0, &report);
        let out_b = redact_span_with_whole_file_findings(span_b, cut, &report);
        assert!(out_a.redactions >= 1);
        assert!(out_b.redactions >= 1);
        let combined = format!("{}{}", out_a.content, out_b.content);
        // No contiguous run of the original JWT survives across the join.
        assert!(
            !combined.contains(&jwt[..20]),
            "boundary-split secret leaked: {combined}"
        );
        assert!(!combined.contains(&jwt[10..30]));
        assert!(combined.contains("[REDACTED:json_web_token]"));
    }

    #[test]
    fn whole_file_redaction_preserves_clean_span_unchanged() {
        let text = "alpha\nbeta\ngamma\n";
        let report = scan_text(text);
        assert!(report.is_clean());
        let out = redact_span_with_whole_file_findings("beta", 6, &report);
        assert_eq!(out.content, "beta");
        assert_eq!(out.redactions, 0);
    }

    #[test]
    fn whole_file_redaction_handles_multibyte_content() {
        // A unicode prefix shifts byte offsets; redaction must stay on char
        // boundaries and still excise the secret.
        let pre = "café ☕ note\nkey = ";
        let secret = "AKIAIOSFODNN7EXAMPLE";
        let text = format!("{pre}{secret}\n");
        let report = scan_text(&text);
        let out = redact_span_with_whole_file_findings(&text, 0, &report);
        assert!(!out.content.contains(secret));
        assert!(out.content.contains("café ☕ note"));
        assert!(out.redactions >= 1);
    }
}
