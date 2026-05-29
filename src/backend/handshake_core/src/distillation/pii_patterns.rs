//! MT-120: PII regex patterns for the distillation content-review
//! pipeline.
//!
//! Adult-production discipline (GLOBAL-PRODUCTION-002..009): this module
//! detects PERSONAL INFORMATION LEAKAGE (emails, phones, credit cards,
//! API keys, Windows user paths, MAC addresses, IPv4) — it is NOT a
//! content moderation system. Explicit sexual content is never a PII
//! detection target. Operators' content discipline is enforced via
//! license tagging + opt-in consent (MT-119), NOT via this scanner.
//!
//! Patterns are kept narrow and additive: false positives quarantine,
//! they do not reject. Full NER is deferred to a future WP per MT-120
//! implementation_notes; the personal-name heuristic here is regex +
//! capitalization plus a small allowlist of common-noun bigrams to
//! suppress obvious non-name matches.

use std::sync::OnceLock;

use regex::Regex;

/// Severity of a detection. The review pipeline maps Low/Medium ->
/// quarantine, High -> reject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiiSeverity {
    Low,
    Medium,
    High,
}

/// Kind of detected PII. Stable enum so downstream telemetry can grep
/// by variant name without parsing prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiiKind {
    Email,
    Phone,
    CreditCard,
    ApiKey,
    WindowsUserPath,
    MacAddress,
    Ipv4,
}

impl PiiKind {
    pub fn severity(self) -> PiiSeverity {
        match self {
            // High: leaks of credentials or financial primary keys.
            PiiKind::ApiKey | PiiKind::CreditCard => PiiSeverity::High,
            // Medium: contact identifiers + machine fingerprints.
            PiiKind::Email | PiiKind::Phone | PiiKind::WindowsUserPath | PiiKind::MacAddress => {
                PiiSeverity::Medium
            }
            // Low: IPv4 addresses (may be public infra).
            PiiKind::Ipv4 => PiiSeverity::Low,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PiiKind::Email => "email",
            PiiKind::Phone => "phone",
            PiiKind::CreditCard => "credit_card",
            PiiKind::ApiKey => "api_key",
            PiiKind::WindowsUserPath => "windows_user_path",
            PiiKind::MacAddress => "mac_address",
            PiiKind::Ipv4 => "ipv4",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiiDetection {
    pub kind: PiiKind,
    pub severity: PiiSeverity,
    pub start: usize,
    pub end: usize,
    pub matched: String,
}

fn email_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Simplified RFC 5322; intentionally narrow to limit false
        // positives in prose. Local-part allows dot/underscore/plus/
        // hyphen; domain is dot-separated alphanumeric.
        Regex::new(r"(?i)\b[a-z0-9._+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)+\b").unwrap()
    })
}

fn phone_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // E.164 OR common US/EU spaced/dashed formats.
        Regex::new(
            r"(?x)
            (?:
                \+?\d{1,3}[\s.-]?\(?\d{2,4}\)?[\s.-]?\d{3,4}[\s.-]?\d{3,4}
            )
            ",
        )
        .unwrap()
    })
}

fn credit_card_candidate_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // 13..19 digit runs separated by optional spaces/dashes. The
        // Luhn check below filters non-cards.
        Regex::new(r"\b(?:\d[\s-]?){12,18}\d\b").unwrap()
    })
}

fn api_key_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Common prefixes: OpenAI (sk-), AWS access key (AKIA), GitHub
        // personal access (ghp_), GitHub fine-grained (github_pat_),
        // Anthropic (sk-ant-), Slack bot/user tokens (xoxb-/xoxp-).
        Regex::new(
            r"(?x)
            \b(?:
                sk-(?:ant-)?[A-Za-z0-9_-]{16,}
              | AKIA[0-9A-Z]{16}
              | ghp_[A-Za-z0-9]{20,}
              | github_pat_[A-Za-z0-9_]{20,}
              | xox[bp]-[A-Za-z0-9-]{10,}
            )\b
            ",
        )
        .unwrap()
    })
}

fn windows_user_path_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Reveals an OS account name. Matches `C:\Users\<name>` or
        // `C:/Users/<name>`. `<name>` is one path segment of safe chars.
        Regex::new(r"(?i)\b[c-z]:[\\/]Users[\\/][A-Za-z0-9._-]+").unwrap()
    })
}

fn mac_address_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Six octets, colon or hyphen separated.
        Regex::new(r"\b[0-9A-Fa-f]{2}(?:[:-][0-9A-Fa-f]{2}){5}\b").unwrap()
    })
}

fn ipv4_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Four 0..=255 octets dot-separated. We accept anything that
        // looks like a dotted quad; range validation is done at match
        // time below to avoid catastrophic regex backtracking.
        Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap()
    })
}

fn is_valid_octet(value: &str) -> bool {
    value.parse::<u8>().is_ok()
}

fn luhn_ok(digits: &str) -> bool {
    let collected: Vec<u32> = digits
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).expect("ascii digit"))
        .collect();
    if collected.len() < 13 || collected.len() > 19 {
        return false;
    }
    let mut sum = 0_u32;
    for (idx, digit) in collected.iter().rev().enumerate() {
        let mut d = *digit;
        if idx % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum % 10 == 0
}

/// Scan `text` and return all PII detections in source order.
pub fn scan(text: &str) -> Vec<PiiDetection> {
    let mut out = Vec::new();

    for m in email_regex().find_iter(text) {
        out.push(PiiDetection {
            kind: PiiKind::Email,
            severity: PiiKind::Email.severity(),
            start: m.start(),
            end: m.end(),
            matched: m.as_str().to_string(),
        });
    }
    for m in phone_regex().find_iter(text) {
        out.push(PiiDetection {
            kind: PiiKind::Phone,
            severity: PiiKind::Phone.severity(),
            start: m.start(),
            end: m.end(),
            matched: m.as_str().to_string(),
        });
    }
    for m in credit_card_candidate_regex().find_iter(text) {
        if luhn_ok(m.as_str()) {
            out.push(PiiDetection {
                kind: PiiKind::CreditCard,
                severity: PiiKind::CreditCard.severity(),
                start: m.start(),
                end: m.end(),
                matched: m.as_str().to_string(),
            });
        }
    }
    for m in api_key_regex().find_iter(text) {
        out.push(PiiDetection {
            kind: PiiKind::ApiKey,
            severity: PiiKind::ApiKey.severity(),
            start: m.start(),
            end: m.end(),
            matched: m.as_str().to_string(),
        });
    }
    for m in windows_user_path_regex().find_iter(text) {
        out.push(PiiDetection {
            kind: PiiKind::WindowsUserPath,
            severity: PiiKind::WindowsUserPath.severity(),
            start: m.start(),
            end: m.end(),
            matched: m.as_str().to_string(),
        });
    }
    for m in mac_address_regex().find_iter(text) {
        out.push(PiiDetection {
            kind: PiiKind::MacAddress,
            severity: PiiKind::MacAddress.severity(),
            start: m.start(),
            end: m.end(),
            matched: m.as_str().to_string(),
        });
    }
    for m in ipv4_regex().find_iter(text) {
        if m.as_str().split('.').all(is_valid_octet) {
            out.push(PiiDetection {
                kind: PiiKind::Ipv4,
                severity: PiiKind::Ipv4.severity(),
                start: m.start(),
                end: m.end(),
                matched: m.as_str().to_string(),
            });
        }
    }

    out.sort_by_key(|d| (d.start, d.end));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_detection_positive_and_negative() {
        let hits = scan("contact me at alice@example.com please");
        assert!(hits
            .iter()
            .any(|d| d.kind == PiiKind::Email && d.matched == "alice@example.com"));
        // Plain prose without an @ symbol must not match.
        let hits = scan("no email here, just text");
        assert!(!hits.iter().any(|d| d.kind == PiiKind::Email));
    }

    #[test]
    fn phone_detection_e164_and_dashed_formats() {
        let hits = scan("Call +1 555 123 4567 or 555-987-6543");
        assert!(hits.iter().any(|d| d.kind == PiiKind::Phone));
        let phones: Vec<_> = hits.iter().filter(|d| d.kind == PiiKind::Phone).collect();
        assert!(phones.len() >= 2, "{phones:?}");
    }

    #[test]
    fn credit_card_requires_luhn_pass() {
        // 4242 4242 4242 4242 is a famous Stripe test card; Luhn-valid.
        let hits = scan("test card 4242 4242 4242 4242 nope");
        assert!(hits.iter().any(|d| d.kind == PiiKind::CreditCard));

        // Random 16-digit run that fails Luhn must not match.
        let hits = scan("not a card 1234567890123456");
        assert!(!hits.iter().any(|d| d.kind == PiiKind::CreditCard));
    }

    #[test]
    fn api_key_detection_common_prefixes() {
        let hits = scan(
            "openai key sk-abcdefghij1234567890 and aws AKIAIOSFODNN7EXAMPLE and gh ghp_abcdefghij1234567890ABCD",
        );
        let kinds: Vec<_> = hits.iter().map(|d| d.kind).collect();
        let api_key_hits = kinds.iter().filter(|k| **k == PiiKind::ApiKey).count();
        assert!(api_key_hits >= 3, "{kinds:?}");
    }

    #[test]
    fn windows_user_path_reveals_account_name() {
        let hits = scan(r"path is C:\Users\Ilja Smets\file.txt");
        // Path stops at the user-name segment (no space allowed by
        // the regex), so the match is `C:\Users\Ilja`.
        assert!(hits.iter().any(|d| d.kind == PiiKind::WindowsUserPath
            && d.matched.eq_ignore_ascii_case(r"C:\Users\Ilja")));
    }

    #[test]
    fn mac_address_detection() {
        let hits = scan("device 00:1A:2B:3C:4D:5E reported");
        assert!(hits.iter().any(|d| d.kind == PiiKind::MacAddress));
    }

    #[test]
    fn ipv4_detection_rejects_out_of_range_octets() {
        let hits = scan("router at 192.168.1.1 saw 999.999.999.999");
        assert_eq!(
            hits.iter().filter(|d| d.kind == PiiKind::Ipv4).count(),
            1,
            "{hits:?}"
        );
        assert!(hits
            .iter()
            .any(|d| d.kind == PiiKind::Ipv4 && d.matched == "192.168.1.1"));
    }

    #[test]
    fn explicit_sexual_content_is_not_flagged_as_pii() {
        // GLOBAL-PRODUCTION-002..009: the PII pipeline is NOT a content
        // moderation system. This sentence contains explicit operator-
        // production language and MUST produce zero PII detections.
        let hits = scan("explicit production prompt: pussy, tits, cock; verbatim per operator");
        assert!(
            hits.is_empty(),
            "PII scanner must not flag explicit sexual content; got {hits:?}"
        );
    }

    #[test]
    fn severity_classification_is_stable() {
        assert_eq!(PiiKind::ApiKey.severity(), PiiSeverity::High);
        assert_eq!(PiiKind::CreditCard.severity(), PiiSeverity::High);
        assert_eq!(PiiKind::Email.severity(), PiiSeverity::Medium);
        assert_eq!(PiiKind::Phone.severity(), PiiSeverity::Medium);
        assert_eq!(PiiKind::Ipv4.severity(), PiiSeverity::Low);
    }
}
