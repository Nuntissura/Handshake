//! MT-025 Environment and Secret Redaction.
//!
//! Acceptance (MT-025.json): "prevent env/secret leakage. Acceptance:
//! secret-looking values are not emitted in stored logs or reports."
//!
//! Two surfaces:
//!   * `Redactor::redact_text(...)` walks a free-text string and replaces
//!     secret-looking substrings with the fixed token `[REDACTED:KIND]`.
//!   * `Redactor::redact_env(...)` walks a key-value env map and replaces the
//!     value of any key matching the secret heuristic with `[REDACTED:ENV]`.
//!
//! Detection is intentionally regex-light (no `regex` crate dependency) so we
//! never miss a leak because of a malformed pattern. The token `[REDACTED:*]`
//! is stable; downstream log/report sinks search for it to assert no leak.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::policy_default_deny::EnvRedactionV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecretKind {
    Env,
    BearerToken,
    AwsKey,
    GenericApiKey,
    PrivateKeyBlock,
    UrlCredential,
    HighEntropy,
    Custom,
}

impl SecretKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Env => "ENV",
            Self::BearerToken => "BEARER",
            Self::AwsKey => "AWS",
            Self::GenericApiKey => "API_KEY",
            Self::PrivateKeyBlock => "PRIVATE_KEY",
            Self::UrlCredential => "URL_CRED",
            Self::HighEntropy => "ENTROPY",
            Self::Custom => "CUSTOM",
        }
    }
}

pub struct Redactor {
    enabled: bool,
    extra_patterns: Vec<String>,
}

impl Redactor {
    pub fn from_policy(policy: &EnvRedactionV1) -> Self {
        Self {
            enabled: policy.enabled,
            extra_patterns: policy.extra_patterns.clone(),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            extra_patterns: Vec::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Redact secret-looking substrings in `s`. Returns `s` unchanged when the
    /// redactor is disabled.
    pub fn redact_text(&self, s: &str) -> String {
        if !self.enabled {
            return s.to_string();
        }
        let mut out = s.to_string();

        // Each pass is a literal scan; we never use regex backtracking.
        out = redact_private_key_blocks(&out);
        out = redact_bearer_tokens(&out);
        out = redact_aws_keys(&out);
        out = redact_url_credentials(&out);
        out = redact_kv_secrets(&out);
        out = redact_high_entropy_tokens(&out);

        for pattern in &self.extra_patterns {
            if !pattern.is_empty() {
                out = out.replace(pattern, "[REDACTED:CUSTOM]");
            }
        }
        out
    }

    /// Redact secret-named env vars. Caller passes the env map; returned map has
    /// the same keys with secret values replaced by `[REDACTED:ENV]`.
    pub fn redact_env(&self, env: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        if !self.enabled {
            return env.clone();
        }
        env.iter()
            .map(|(k, v)| {
                if env_key_is_sensitive(k) {
                    (k.clone(), "[REDACTED:ENV]".to_string())
                } else {
                    (k.clone(), self.redact_text(v))
                }
            })
            .collect()
    }
}

fn env_key_is_sensitive(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    const NEEDLES: &[&str] = &[
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASSWD",
        "PWD",
        "API_KEY",
        "APIKEY",
        "ACCESS_KEY",
        "PRIVATE_KEY",
        "AUTH",
        "CREDENTIAL",
        "SESSION",
    ];
    NEEDLES.iter().any(|n| upper.contains(n))
}

fn redact_private_key_blocks(s: &str) -> String {
    let begin = "-----BEGIN ";
    let end = "-----END ";
    let mut out = String::with_capacity(s.len());
    let mut cursor = 0;
    let bytes = s.as_bytes();
    while cursor < s.len() {
        if let Some(i) = s[cursor..].find(begin) {
            let abs = cursor + i;
            out.push_str(&s[cursor..abs]);
            if let Some(j) = s[abs..].find(end) {
                // end_marker_start points at the first byte of `-----END `.
                let end_marker_start = abs + j;
                // The closing 5-dash run must be searched AFTER the END
                // header text, not at its leading dashes.
                let after_end_header = end_marker_start + end.len();
                if let Some(k) = s[after_end_header..].find("-----") {
                    let close = after_end_header + k + 5; // length of trailing "-----"
                    out.push_str("[REDACTED:PRIVATE_KEY]");
                    cursor = close;
                    continue;
                }
            }
            // No end marker; redact to end-of-string defensively.
            out.push_str("[REDACTED:PRIVATE_KEY]");
            cursor = bytes.len();
        } else {
            out.push_str(&s[cursor..]);
            break;
        }
    }
    out
}

fn redact_bearer_tokens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cursor = 0;
    let lower = s.to_ascii_lowercase();
    let needle = "bearer ";
    while cursor < s.len() {
        if let Some(rel) = lower[cursor..].find(needle) {
            let start = cursor + rel;
            out.push_str(&s[cursor..start + needle.len()]);
            let value_start = start + needle.len();
            let value_end = s[value_start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|d| value_start + d)
                .unwrap_or(s.len());
            if value_end > value_start {
                out.push_str("[REDACTED:BEARER]");
            }
            cursor = value_end;
        } else {
            out.push_str(&s[cursor..]);
            break;
        }
    }
    out
}

fn redact_aws_keys(s: &str) -> String {
    // AKIA / ASIA + 16 uppercase alphanum. UTF-8 safe: never slice into a
    // multibyte char; only attempt a match when the next 20 bytes are ASCII.
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let remaining = bytes.len() - i;
        if remaining >= 20 && bytes[i..i + 20].iter().all(|b| b.is_ascii()) {
            let head4 = &bytes[i..i + 4];
            if (head4 == b"AKIA" || head4 == b"ASIA")
                && bytes[i + 4..i + 20]
                    .iter()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            {
                out.push_str("[REDACTED:AWS]");
                i += 20;
                continue;
            }
        }
        let c = s[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

fn redact_url_credentials(s: &str) -> String {
    // `scheme://user:pass@host` -> `scheme://[REDACTED:URL_CRED]@host`
    let mut out = String::with_capacity(s.len());
    let mut cursor = 0;
    while cursor < s.len() {
        if let Some(scheme_rel) = s[cursor..].find("://") {
            let scheme_abs = cursor + scheme_rel + 3;
            out.push_str(&s[cursor..scheme_abs]);
            // From scheme_abs find next whitespace or end.
            let segment_end = s[scheme_abs..]
                .find(|c: char| c.is_whitespace())
                .map(|d| scheme_abs + d)
                .unwrap_or(s.len());
            let segment = &s[scheme_abs..segment_end];
            if let Some(at_rel) = segment.find('@') {
                // Make sure there is at least one `:` before `@` (looks like user:pass).
                if segment[..at_rel].contains(':') {
                    out.push_str("[REDACTED:URL_CRED]");
                    out.push_str(&segment[at_rel..]);
                    cursor = segment_end;
                    continue;
                }
            }
            out.push_str(segment);
            cursor = segment_end;
        } else {
            out.push_str(&s[cursor..]);
            break;
        }
    }
    out
}

fn redact_kv_secrets(s: &str) -> String {
    // Heuristic: replace `<key>=<value>` and `<key>: <value>` where key matches
    // env_key_is_sensitive.
    let mut out = String::with_capacity(s.len());
    for line in s.split_inclusive('\n') {
        let trimmed_newline = line.trim_end_matches('\n');
        let suffix_nl = if line.ends_with('\n') { "\n" } else { "" };
        let mut handled = false;
        for sep in ['=', ':'] {
            if let Some(pos) = trimmed_newline.find(sep) {
                let key = trimmed_newline[..pos].trim();
                if !key.is_empty() && env_key_is_sensitive(key) {
                    let val = trimmed_newline[pos + 1..].trim_start();
                    if !val.is_empty() {
                        let kind = if sep == '=' {
                            SecretKind::Env
                        } else {
                            SecretKind::GenericApiKey
                        };
                        out.push_str(&trimmed_newline[..pos + 1]);
                        if sep == ':' {
                            out.push(' ');
                        }
                        out.push_str(&format!("[REDACTED:{}]", kind.as_str()));
                        out.push_str(suffix_nl);
                        handled = true;
                        break;
                    }
                }
            }
        }
        if !handled {
            out.push_str(line);
        }
    }
    out
}

fn redact_high_entropy_tokens(s: &str) -> String {
    // Replace runs of >=32 chars made entirely of [A-Za-z0-9_-+/=] that mix
    // case + digits. This is the catch-all for opaque secrets.
    let mut out = String::with_capacity(s.len());
    let mut buf = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if is_entropy_char(c) {
            buf.push(c);
        } else {
            flush_entropy(&mut out, &mut buf);
            out.push(c);
        }
        if chars.peek().is_none() {
            flush_entropy(&mut out, &mut buf);
        }
    }
    out
}

fn is_entropy_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '+' || c == '/' || c == '='
}

fn flush_entropy(out: &mut String, buf: &mut String) {
    if buf.len() >= 32
        && buf.chars().any(|c| c.is_ascii_lowercase())
        && buf.chars().any(|c| c.is_ascii_uppercase())
        && buf.chars().any(|c| c.is_ascii_digit())
    {
        out.push_str("[REDACTED:ENTROPY]");
    } else {
        out.push_str(buf);
    }
    buf.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Redactor {
        Redactor::from_policy(&EnvRedactionV1::default())
    }

    #[test]
    fn redactor_default_is_enabled() {
        assert!(r().is_enabled());
    }

    #[test]
    fn disabled_redactor_passes_text_through() {
        let r = Redactor::disabled();
        assert_eq!(
            r.redact_text("AWS_SECRET_KEY=AKIAABCDEFGHIJKLMNOP"),
            "AWS_SECRET_KEY=AKIAABCDEFGHIJKLMNOP"
        );
    }

    #[test]
    fn secret_env_key_value_is_redacted() {
        let mut env = BTreeMap::new();
        env.insert("AWS_SECRET_ACCESS_KEY".into(), "lol/notReal/value+12345".into());
        env.insert("PATH".into(), "/usr/bin".into());
        let out = r().redact_env(&env);
        assert_eq!(out["AWS_SECRET_ACCESS_KEY"], "[REDACTED:ENV]");
        assert_eq!(out["PATH"], "/usr/bin");
    }

    #[test]
    fn bearer_token_in_log_is_redacted() {
        let log = "Authorization: Bearer abcDEF123ghiJKL456mnoPQR789";
        let out = r().redact_text(log);
        assert!(out.contains("[REDACTED:BEARER]"));
        assert!(!out.contains("abcDEF123"));
    }

    #[test]
    fn aws_key_in_log_is_redacted() {
        let log = "key=AKIAIOSFODNN7EXAMPLE rest";
        let out = r().redact_text(log);
        assert!(out.contains("[REDACTED:AWS]"));
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn private_key_block_is_redacted() {
        let pem = "header\n-----BEGIN RSA PRIVATE KEY-----\nMIIBlah==\n-----END RSA PRIVATE KEY-----\ntail";
        let out = r().redact_text(pem);
        assert!(out.contains("[REDACTED:PRIVATE_KEY]"));
        assert!(!out.contains("MIIBlah"));
        assert!(out.contains("header"));
        assert!(out.contains("tail"));
    }

    #[test]
    fn url_credentials_are_redacted() {
        let log = "Cloning https://user:hunter2@github.com/repo.git ...";
        let out = r().redact_text(log);
        assert!(out.contains("[REDACTED:URL_CRED]"));
        assert!(!out.contains("hunter2"));
        assert!(out.contains("github.com"));
    }

    #[test]
    fn kv_secret_inline_is_redacted() {
        let log = "PASSWORD=supersecret\nfoo=bar";
        let out = r().redact_text(log);
        assert!(out.contains("PASSWORD=[REDACTED:ENV]"));
        assert!(out.contains("foo=bar"));
    }

    #[test]
    fn high_entropy_blob_is_redacted() {
        let blob = "stuff aBcD1234EfGh5678IjKl9012MnOp3456 more";
        let out = r().redact_text(blob);
        assert!(out.contains("[REDACTED:ENTROPY]"));
        assert!(!out.contains("aBcD1234EfGh5678IjKl9012MnOp3456"));
    }

    #[test]
    fn extra_pattern_redacts_custom_strings() {
        let pol = EnvRedactionV1 {
            enabled: true,
            extra_patterns: vec!["MY-CUSTOM-LICENSE-KEY".into()],
        };
        let r = Redactor::from_policy(&pol);
        let out = r.redact_text("config: MY-CUSTOM-LICENSE-KEY found");
        assert!(out.contains("[REDACTED:CUSTOM]"));
        assert!(!out.contains("MY-CUSTOM-LICENSE-KEY"));
    }

    #[test]
    fn env_key_sensitivity_is_case_insensitive() {
        assert!(env_key_is_sensitive("MY_SECRET"));
        assert!(env_key_is_sensitive("my_token"));
        assert!(env_key_is_sensitive("Database_Password"));
        assert!(!env_key_is_sensitive("PATH"));
        assert!(!env_key_is_sensitive("USER"));
    }
}
