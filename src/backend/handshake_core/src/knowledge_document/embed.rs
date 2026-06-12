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
    /// Construct a typed embed target, rejecting absolute filesystem paths and
    /// non-http URLs (MT-152). This is the only public constructor, so a path
    /// can never become an embed target.
    pub fn new(kind: EmbedRefKind, value: impl Into<String>) -> Result<Self, EmbedTargetError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(EmbedTargetError::Empty);
        }
        if is_absolute_path(trimmed) {
            return Err(EmbedTargetError::AbsolutePath(trimmed.to_string()));
        }
        if kind == EmbedRefKind::Url
            && !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        {
            return Err(EmbedTargetError::NonHttpUrl(trimmed.to_string()));
        }
        Ok(Self {
            kind,
            value: trimmed.to_string(),
        })
    }
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
