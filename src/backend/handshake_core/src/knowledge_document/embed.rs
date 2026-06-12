//! MT-152 EmbedReferenceModel + MT-153 BrokenEmbedRepairState.
//!
//! Embeds (images, videos, albums, slideshows, and file references) are stored
//! as TYPED references — an artifact / media / source id, or a typed URL —
//! NEVER a random absolute filesystem path (MT-152). A path-shaped target is
//! rejected at construction. When an embed's target cannot be resolved it is
//! represented as a repairable typed broken node carrying the repair actions a
//! backend can take (MT-153); the broken state is data, not a dead link, so the
//! editor renders a repairable placeholder instead of a blank box.

use serde::{Deserialize, Serialize};

/// The kind of a typed embed reference (MT-152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedRefKind {
    /// A Handshake artifact id (e.g. an exported render or generated file).
    Artifact,
    /// A media id (image/video) in the media store.
    Media,
    /// A knowledge source id (`KSRC-...`).
    Source,
    /// A typed, scheme-qualified URL (http/https only; no `file:` paths).
    Url,
}

impl EmbedRefKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Media => "media",
            Self::Source => "source",
            Self::Url => "url",
        }
    }
}

/// Errors constructing an embed target (MT-152: never a random absolute path).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbedTargetError {
    #[error("embed target value must be non-empty")]
    Empty,
    #[error("embed target looks like an absolute filesystem path (`{0}`); embeds must be artifact/media/source ids or typed http(s) URLs, never absolute paths")]
    AbsolutePath(String),
    #[error("embed url target must be an http(s) URL, got `{0}`")]
    NonHttpUrl(String),
    #[error("embed id target carries a URL scheme (`{0}`); artifact/media/source ids are bare ids — scheme-bearing values must use the url kind (http/https only)")]
    SchemeNotAllowedForId(String),
}

/// A typed embed reference target (MT-152).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedTarget {
    pub kind: EmbedRefKind,
    /// The id (artifact/media/source) or the typed URL. Guaranteed by
    /// construction to never be an absolute filesystem path.
    pub value: String,
}

impl EmbedTarget {
    /// Construct a typed embed target, rejecting absolute filesystem paths,
    /// non-http URLs, and scheme-bearing "ids" (MT-152, hardened by
    /// adversarial-v2 MT-150/152). This is the only public constructor, so a
    /// path or a `javascript:`/`data:` URI can never become an embed target:
    ///
    /// * `url` kind: must start with `http://` / `https://` exactly (an
    ///   obfuscated scheme — case tricks, embedded tab/newline — fails the
    ///   strict prefix and is rejected).
    /// * id kinds (`artifact`/`media`/`source`): must be bare ids; any value
    ///   that parses as carrying a URL scheme (after stripping the tab/LF/CR
    ///   obfuscation characters browsers ignore) is rejected, so
    ///   `javascript:...`/`data:...` can never hide inside an id field.
    pub fn new(kind: EmbedRefKind, value: impl Into<String>) -> Result<Self, EmbedTargetError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(EmbedTargetError::Empty);
        }
        if is_absolute_path(trimmed) {
            return Err(EmbedTargetError::AbsolutePath(trimmed.to_string()));
        }
        match kind {
            EmbedRefKind::Url => {
                if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
                    return Err(EmbedTargetError::NonHttpUrl(trimmed.to_string()));
                }
            }
            EmbedRefKind::Artifact | EmbedRefKind::Media | EmbedRefKind::Source => {
                if url_scheme(trimmed).is_some() {
                    return Err(EmbedTargetError::SchemeNotAllowedForId(trimmed.to_string()));
                }
            }
        }
        Ok(Self {
            kind,
            value: trimmed.to_string(),
        })
    }

    /// Validate a raw embed target string from document content (MT-150/152:
    /// the projection + save paths route raw `attrs.target`/`attrs.src` values
    /// through THIS law instead of trusting them). The kind is inferred:
    /// a scheme-bearing value must be a typed http(s) URL; everything else is
    /// treated as a bare media/artifact/source id.
    pub fn parse_raw(value: &str) -> Result<Self, EmbedTargetError> {
        let kind = if url_scheme(value.trim()).is_some() {
            EmbedRefKind::Url
        } else {
            EmbedRefKind::Media
        };
        Self::new(kind, value)
    }
}

/// Parse the URL scheme of a value, defending against the obfuscation
/// browsers tolerate (adversarial-v2 MT-150): ASCII tab/LF/CR are stripped
/// anywhere (HTML URL parsing ignores them, so `jav\tascript:` IS
/// `javascript:` to a browser) and leading/trailing C0 controls + space are
/// trimmed. Returns the lowercased scheme when the cleaned value starts with
/// `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"`, else `None` (a relative
/// or scheme-less value).
pub(crate) fn url_scheme(value: &str) -> Option<String> {
    let cleaned: String = value
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c <= ' ' || c == '\u{7f}');
    let mut scheme = String::new();
    for (index, ch) in cleaned.chars().enumerate() {
        if ch == ':' {
            return if scheme.is_empty() {
                None
            } else {
                Some(scheme)
            };
        }
        let valid = if index == 0 {
            ch.is_ascii_alphabetic()
        } else {
            ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')
        };
        if !valid {
            // A non-scheme character before any ':' means the value has no
            // scheme (it is a relative path / bare id / title).
            return None;
        }
        scheme.push(ch.to_ascii_lowercase());
    }
    None
}

/// A typed embed reference attached to an embed block (MT-152).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedRef {
    /// Stable block id of the embed block this reference belongs to (MT-148).
    pub block_id: String,
    pub target: EmbedTarget,
    /// Optional human caption / alt text (regenerable display content).
    pub caption: Option<String>,
}

/// A repair action available for a broken embed (MT-153).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedRepairAction {
    /// Re-point the embed at a new typed target.
    Relink,
    /// Re-resolve the existing target (the backend retries resolution).
    Reresolve,
    /// Remove the broken embed block from the document.
    Remove,
}

impl EmbedRepairAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relink => "relink",
            Self::Reresolve => "reresolve",
            Self::Remove => "remove",
        }
    }

    /// The repair actions offered for any broken embed.
    pub fn all() -> [EmbedRepairAction; 3] {
        [Self::Relink, Self::Reresolve, Self::Remove]
    }
}

/// The repairable broken-embed state for an embed whose target did not resolve
/// (MT-153). This is DATA the editor renders as a repairable placeholder, with
/// the typed actions a backend can take to fix it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenEmbedRepair {
    /// Stable block id of the broken embed block (MT-148).
    pub block_id: String,
    /// The target that failed to resolve (preserved so a relink can show the
    /// old value).
    pub target: EmbedTarget,
    /// Why the target is considered broken (e.g. "media id not found").
    pub reason: String,
    /// The repair actions available to the operator/model/backend.
    pub available_actions: Vec<EmbedRepairAction>,
}

impl BrokenEmbedRepair {
    /// Build a broken-embed repair record offering all standard actions.
    pub fn new(
        block_id: impl Into<String>,
        target: EmbedTarget,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            target,
            reason: reason.into(),
            available_actions: EmbedRepairAction::all().to_vec(),
        }
    }
}

/// Whether a string looks like an absolute filesystem path that must never be
/// an embed target (MT-152). Catches POSIX absolute (`/...`), Windows
/// drive-letter (`C:\...` / `C:/...`), UNC (`\\host\share`), and `file:` URLs.
fn is_absolute_path(value: &str) -> bool {
    if value.starts_with('/') || value.starts_with("\\\\") {
        return true;
    }
    if value.to_ascii_lowercase().starts_with("file:") {
        return true;
    }
    let bytes = value.as_bytes();
    // Drive-letter form: `X:\` or `X:/`.
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ids_and_http_urls_are_accepted() {
        assert!(EmbedTarget::new(EmbedRefKind::Artifact, "KART-1").is_ok());
        assert!(EmbedTarget::new(EmbedRefKind::Media, "KMED-1").is_ok());
        assert!(EmbedTarget::new(EmbedRefKind::Source, "KSRC-1").is_ok());
        assert!(EmbedTarget::new(EmbedRefKind::Url, "https://x/y.png").is_ok());
        assert!(EmbedTarget::new(EmbedRefKind::Url, "http://x/y.png").is_ok());
    }

    #[test]
    fn absolute_paths_are_rejected_as_embed_targets() {
        // MT-152: never a random absolute path.
        for bad in [
            "/var/x.png",
            "C:\\x.png",
            "D:/x.png",
            "\\\\host\\share\\x",
            "file:///etc/passwd",
        ] {
            assert!(
                matches!(
                    EmbedTarget::new(EmbedRefKind::Media, bad),
                    Err(EmbedTargetError::AbsolutePath(_))
                ),
                "`{bad}` must be rejected as an absolute path"
            );
        }
    }

    #[test]
    fn url_kind_requires_http_scheme() {
        assert!(matches!(
            EmbedTarget::new(EmbedRefKind::Url, "ftp://x"),
            Err(EmbedTargetError::NonHttpUrl(_))
        ));
        assert!(matches!(
            EmbedTarget::new(EmbedRefKind::Media, "  "),
            Err(EmbedTargetError::Empty)
        ));
    }

    #[test]
    fn id_kinds_reject_scheme_bearing_values() {
        // Adversarial-v2 MT-150/152: a javascript:/data: URI can never hide
        // inside an artifact/media/source id field, including the obfuscations
        // browsers tolerate (case, embedded tab/newline).
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "jav\tascript:alert(1)",
            "jav\nascript:alert(1)",
            "data:text/html,<script>",
            "vbscript:msgbox(1)",
            "ftp://host/x",
        ] {
            for kind in [EmbedRefKind::Artifact, EmbedRefKind::Media, EmbedRefKind::Source] {
                assert!(
                    matches!(
                        EmbedTarget::new(kind, bad),
                        Err(EmbedTargetError::SchemeNotAllowedForId(_))
                    ),
                    "`{bad}` must be rejected as a {} id",
                    kind.as_str()
                );
            }
        }
        // Bare ids remain accepted.
        assert!(EmbedTarget::new(EmbedRefKind::Media, "KMED-1").is_ok());
        assert!(EmbedTarget::new(EmbedRefKind::Artifact, "KART-2026").is_ok());
    }

    #[test]
    fn parse_raw_infers_kind_and_applies_the_same_law() {
        // Bare id -> media id target.
        let id = EmbedTarget::parse_raw("KMED-1").expect("bare id");
        assert_eq!(id.kind, EmbedRefKind::Media);
        // http(s) -> typed url target.
        let url = EmbedTarget::parse_raw("https://cdn.example/x.png").expect("https url");
        assert_eq!(url.kind, EmbedRefKind::Url);
        // Every dangerous shape fails closed.
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "jav\tascript:alert(1)",
            "data:image/svg+xml,<svg>",
            "file:///etc/passwd",
            "/abs/path.png",
            "C:\\x.png",
            "\\\\host\\share\\x",
            "ftp://host/x",
        ] {
            assert!(
                EmbedTarget::parse_raw(bad).is_err(),
                "`{bad}` must fail parse_raw"
            );
        }
    }

    #[test]
    fn url_scheme_detection_defeats_obfuscation() {
        assert_eq!(url_scheme("javascript:x"), Some("javascript".to_string()));
        assert_eq!(url_scheme("JaVaScRiPt:x"), Some("javascript".to_string()));
        assert_eq!(
            url_scheme("jav\tascri\npt:x"),
            Some("javascript".to_string())
        );
        assert_eq!(url_scheme("  \u{1}data:text/html"), Some("data".to_string()));
        assert_eq!(url_scheme("https://x"), Some("https".to_string()));
        // Scheme-less values: bare ids, relative paths, wiki titles.
        assert_eq!(url_scheme("KMED-1"), None);
        assert_eq!(url_scheme("docs/page.md"), None);
        assert_eq!(url_scheme("Deploy Guide"), None);
        assert_eq!(url_scheme("&#106;avascript:x"), None, "entity payloads have no scheme");
        assert_eq!(url_scheme(":nope"), None);
    }

    #[test]
    fn broken_embed_offers_all_repair_actions() {
        let target = EmbedTarget::new(EmbedRefKind::Media, "KMED-missing").unwrap();
        let broken = BrokenEmbedRepair::new("KBL-1", target, "media not found");
        assert_eq!(broken.available_actions.len(), 3);
        assert!(broken
            .available_actions
            .contains(&EmbedRepairAction::Relink));
        assert!(broken
            .available_actions
            .contains(&EmbedRepairAction::Reresolve));
        assert!(broken
            .available_actions
            .contains(&EmbedRepairAction::Remove));
    }
}
