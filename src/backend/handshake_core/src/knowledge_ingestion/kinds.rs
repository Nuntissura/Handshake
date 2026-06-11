//! MT-082 SourceKindRegistry: the typed registry of ingestion source kinds
//! with per-kind capabilities and extension/MIME mapping.
//!
//! Authority decision: the registry is CODE (this enum + const table) because
//! extractors are code — a DB row cannot grant a parsing capability the
//! binary does not ship. The DB projection
//! (`knowledge_ingestion_kind_registry`, migration 0161, authority_class
//! `projection`) exists so validators, the API, and no-context models can
//! read the active capability matrix without spelunking source; it is synced
//! by [`sync_kind_projection`] and never hand-edited.
//!
//! Two levels:
//! * [`IngestionSourceKind`]: the eight primary kinds the ingestion engine
//!   dispatches on.
//! * [`GovernanceSubKind`]: governance artifacts subdivide into the WP-009
//!   contract vocabulary (spec module, WP contract, MT contract, taskboard,
//!   changelog, workflow JSON, receipt, UserManual) recorded in provenance.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use super::{IngestionError, IngestionResult};
use crate::storage::knowledge::KnowledgeRootKind;

/// Version token stored on every projected registry row; bump when the
/// registry shape or semantics change so stale projections are detectable.
pub const KIND_REGISTRY_VERSION: &str = "ingestion_kind_registry_v1";

/// Primary ingestion source kinds (MT-082).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IngestionSourceKind {
    CodeFile,
    MarkdownText,
    RichDocument,
    Pdf,
    MediaTranscript,
    GovernanceArtifact,
    OperatorResearchNote,
    ExternalImport,
}

impl IngestionSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CodeFile => "code_file",
            Self::MarkdownText => "markdown_text",
            Self::RichDocument => "rich_document",
            Self::Pdf => "pdf",
            Self::MediaTranscript => "media_transcript",
            Self::GovernanceArtifact => "governance_artifact",
            Self::OperatorResearchNote => "operator_research_note",
            Self::ExternalImport => "external_import",
        }
    }
}

impl std::str::FromStr for IngestionSourceKind {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        registry()
            .iter()
            .find(|spec| spec.kind_key == value)
            .map(|spec| spec.kind)
            .ok_or_else(|| {
                IngestionError::Validation(format!("invalid ingestion source kind: {value}"))
            })
    }
}

/// Governance artifact sub-kinds (MT-082 contract vocabulary).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceSubKind {
    SpecModule,
    WpContract,
    MtContract,
    Taskboard,
    Changelog,
    WorkflowJson,
    Receipt,
    UserManual,
}

impl GovernanceSubKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SpecModule => "spec_module",
            Self::WpContract => "wp_contract",
            Self::MtContract => "mt_contract",
            Self::Taskboard => "taskboard",
            Self::Changelog => "changelog",
            Self::WorkflowJson => "workflow_json",
            Self::Receipt => "receipt",
            Self::UserManual => "user_manual",
        }
    }
}

/// What the shipped extractors can do for a kind.
// NOTE (cross-lane unblock by PostgresEventLedgerCore agent): `Deserialize`
// removed from this derive — `&'static [&'static str]` fields cannot
// implement `Deserialize`, so the derive could never compile and no consumer
// can exist. Serialize is untouched.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct IngestionCapabilities {
    /// The kind produces ingestion spans (citable evidence units).
    pub span_extraction: bool,
    /// Anchor kinds the extractor emits (`knowledge_ingestion_spans.anchor_kind`).
    pub anchor_kinds: &'static [&'static str],
    /// Extraction may legitimately succeed for part of the source (and must
    /// then produce a `partial` receipt, never silent success).
    pub supports_partial_extraction: bool,
    /// The source must pass text-layer detection before extraction (PDF).
    pub requires_text_layer_detection: bool,
    /// Content is text-decodable and must pass the secret preflight scan.
    pub secret_scan: bool,
}

/// One registry entry: kind + identity + file mapping + capabilities.
#[derive(Clone, Copy, Debug)]
pub struct IngestionKindSpec {
    pub kind: IngestionSourceKind,
    pub kind_key: &'static str,
    pub display_name: &'static str,
    pub extensions: &'static [&'static str],
    pub mime_types: &'static [&'static str],
    pub capabilities: IngestionCapabilities,
}

const REGISTRY: &[IngestionKindSpec] = &[
    IngestionKindSpec {
        kind: IngestionSourceKind::CodeFile,
        kind_key: "code_file",
        display_name: "Code file",
        extensions: &[
            "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "sql", "toml", "yaml", "yml", "sh",
            "ps1", "css", "html",
        ],
        mime_types: &[
            "text/x-rust",
            "text/typescript",
            "text/javascript",
            "text/x-python",
        ],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["byte_range", "line_range"],
            supports_partial_extraction: false,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::MarkdownText,
        kind_key: "markdown_text",
        display_name: "Markdown / plain text",
        extensions: &["md", "markdown", "txt", "text"],
        mime_types: &["text/markdown", "text/plain"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["byte_range", "line_range", "heading_path"],
            supports_partial_extraction: false,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::RichDocument,
        kind_key: "rich_document",
        display_name: "Rich document (ProseMirror/Tiptap authority record)",
        // Product-native: rich documents are ingested from the `documents`
        // authority table, not from files on disk.
        extensions: &[],
        mime_types: &["application/x-handshake-richdoc+json"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["json_pointer"],
            supports_partial_extraction: false,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::Pdf,
        kind_key: "pdf",
        display_name: "PDF document",
        extensions: &["pdf"],
        mime_types: &["application/pdf"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["pdf_page"],
            supports_partial_extraction: true,
            requires_text_layer_detection: true,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::MediaTranscript,
        kind_key: "media_transcript",
        display_name: "Media transcript artifact (SRT/VTT/JSON; operator-provided, no ASR)",
        extensions: &["srt", "vtt"],
        mime_types: &["application/x-subrip", "text/vtt"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["media_time"],
            supports_partial_extraction: true,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::GovernanceArtifact,
        kind_key: "governance_artifact",
        display_name: "Governance artifact (JSON/JSONL packet, receipt, registry)",
        extensions: &["json", "jsonl"],
        mime_types: &["application/json", "application/jsonl"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["json_pointer"],
            supports_partial_extraction: true,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::OperatorResearchNote,
        kind_key: "operator_research_note",
        display_name: "Operator research note (non-normative context)",
        extensions: &["md", "markdown", "txt"],
        mime_types: &["text/markdown", "text/plain"],
        capabilities: IngestionCapabilities {
            span_extraction: true,
            anchor_kinds: &["byte_range", "line_range", "heading_path"],
            supports_partial_extraction: false,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
    IngestionKindSpec {
        kind: IngestionSourceKind::ExternalImport,
        kind_key: "external_import",
        display_name: "External import (operator-gated)",
        extensions: &[],
        mime_types: &["application/octet-stream"],
        capabilities: IngestionCapabilities {
            span_extraction: false,
            anchor_kinds: &[],
            supports_partial_extraction: false,
            requires_text_layer_detection: false,
            secret_scan: true,
        },
    },
];

/// The full kind registry (code authority).
pub fn registry() -> &'static [IngestionKindSpec] {
    REGISTRY
}

/// Spec lookup by kind.
pub fn spec_for(kind: IngestionSourceKind) -> &'static IngestionKindSpec {
    REGISTRY
        .iter()
        .find(|spec| spec.kind == kind)
        .expect("every IngestionSourceKind has a registry entry")
}

fn extension_of(path: &str) -> Option<String> {
    let file_name = path.rsplit('/').next()?;
    let (stem, ext) = file_name.rsplit_once('.')?;
    if stem.is_empty() {
        // Dotfiles like `.gitignore` have no extension in this model.
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

/// Detect the ingestion kind for a relative path under a root of the given
/// kind. Root kind disambiguates shared extensions:
/// * `governance` roots claim `json`/`jsonl`/`md` as governance artifacts /
///   research notes feed,
/// * `operator_folder` / `external_import` roots claim `md`/`txt` as operator
///   research notes and unknown files as external imports,
/// * everywhere else falls back to the primary extension table.
///
/// `None` = unsupported format: the engine must record a typed
/// `unsupported_format` receipt, never guess.
pub fn detect_kind(
    root_kind: KnowledgeRootKind,
    relative_path: &str,
) -> Option<IngestionSourceKind> {
    let ext = extension_of(relative_path);
    let ext = ext.as_deref();

    // Transcript JSON artifacts are media transcripts wherever they live.
    if relative_path
        .to_ascii_lowercase()
        .ends_with(".transcript.json")
    {
        return Some(IngestionSourceKind::MediaTranscript);
    }

    match root_kind {
        KnowledgeRootKind::Governance => match ext {
            Some("json") | Some("jsonl") => Some(IngestionSourceKind::GovernanceArtifact),
            Some("md") | Some("markdown") | Some("txt") => Some(IngestionSourceKind::MarkdownText),
            _ => detect_by_extension(ext),
        },
        KnowledgeRootKind::OperatorFolder | KnowledgeRootKind::ExternalImport => match ext {
            Some("md") | Some("markdown") | Some("txt") => {
                Some(IngestionSourceKind::OperatorResearchNote)
            }
            _ => detect_by_extension(ext).or(Some(IngestionSourceKind::ExternalImport)),
        },
        _ => detect_by_extension(ext),
    }
}

fn detect_by_extension(ext: Option<&str>) -> Option<IngestionSourceKind> {
    let ext = ext?;
    // First registry entry claiming the extension wins; CodeFile precedes
    // MarkdownText precedes the rest, and OperatorResearchNote never claims
    // by bare extension (root kind decides it above).
    REGISTRY
        .iter()
        .filter(|spec| spec.kind != IngestionSourceKind::OperatorResearchNote)
        .find(|spec| spec.extensions.contains(&ext))
        .map(|spec| spec.kind)
}

/// Classify a governance artifact path into the contract vocabulary.
pub fn detect_governance_sub_kind(relative_path: &str) -> Option<GovernanceSubKind> {
    let lower = relative_path.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(&lower);
    if file_name.starts_with("mt-") {
        return Some(GovernanceSubKind::MtContract);
    }
    if file_name.starts_with("wp-") || file_name == "packet.json" || file_name == "refinement.json"
    {
        return Some(GovernanceSubKind::WpContract);
    }
    if file_name.contains("taskboard") || file_name.contains("task_board") {
        return Some(GovernanceSubKind::Taskboard);
    }
    if file_name.contains("changelog") {
        return Some(GovernanceSubKind::Changelog);
    }
    if file_name.contains("workflow") {
        return Some(GovernanceSubKind::WorkflowJson);
    }
    if file_name.contains("receipt") {
        return Some(GovernanceSubKind::Receipt);
    }
    if file_name.contains("manual") {
        return Some(GovernanceSubKind::UserManual);
    }
    if lower.contains("spec") {
        return Some(GovernanceSubKind::SpecModule);
    }
    None
}

/// Sync the DB projection (`knowledge_ingestion_kind_registry`, 0161) to the
/// code registry: upsert every entry, delete rows the code no longer ships.
/// Returns the number of projected rows.
pub async fn sync_kind_projection(pool: &PgPool) -> IngestionResult<usize> {
    let mut tx = pool
        .begin()
        .await
        .map_err(crate::storage::StorageError::from)?;

    let keys: Vec<String> = REGISTRY.iter().map(|s| s.kind_key.to_string()).collect();
    sqlx::query("DELETE FROM knowledge_ingestion_kind_registry WHERE kind_key <> ALL($1)")
        .bind(&keys)
        .execute(&mut *tx)
        .await
        .map_err(crate::storage::StorageError::from)?;

    for spec in REGISTRY {
        sqlx::query(
            "INSERT INTO knowledge_ingestion_kind_registry
                 (kind_key, display_name, capabilities, extensions, mime_types,
                  registry_version, projected_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW())
             ON CONFLICT (kind_key) DO UPDATE SET
                 display_name = EXCLUDED.display_name,
                 capabilities = EXCLUDED.capabilities,
                 extensions = EXCLUDED.extensions,
                 mime_types = EXCLUDED.mime_types,
                 registry_version = EXCLUDED.registry_version,
                 projected_at = NOW()",
        )
        .bind(spec.kind_key)
        .bind(spec.display_name)
        .bind(json!({
            "span_extraction": spec.capabilities.span_extraction,
            "anchor_kinds": spec.capabilities.anchor_kinds,
            "supports_partial_extraction": spec.capabilities.supports_partial_extraction,
            "requires_text_layer_detection": spec.capabilities.requires_text_layer_detection,
            "secret_scan": spec.capabilities.secret_scan,
        }))
        .bind(json!(spec.extensions))
        .bind(json!(spec.mime_types))
        .bind(KIND_REGISTRY_VERSION)
        .execute(&mut *tx)
        .await
        .map_err(crate::storage::StorageError::from)?;
    }

    tx.commit()
        .await
        .map_err(crate::storage::StorageError::from)?;
    Ok(REGISTRY.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_exactly_one_registry_entry() {
        for kind in [
            IngestionSourceKind::CodeFile,
            IngestionSourceKind::MarkdownText,
            IngestionSourceKind::RichDocument,
            IngestionSourceKind::Pdf,
            IngestionSourceKind::MediaTranscript,
            IngestionSourceKind::GovernanceArtifact,
            IngestionSourceKind::OperatorResearchNote,
            IngestionSourceKind::ExternalImport,
        ] {
            let matches: Vec<_> = registry().iter().filter(|s| s.kind == kind).collect();
            assert_eq!(matches.len(), 1, "{kind:?} must have exactly one entry");
            assert_eq!(matches[0].kind_key, kind.as_str());
        }
    }

    #[test]
    fn kind_keys_round_trip_through_from_str() {
        for spec in registry() {
            let parsed: IngestionSourceKind = spec.kind_key.parse().expect("parse kind key");
            assert_eq!(parsed, spec.kind);
        }
        assert!("not_a_kind".parse::<IngestionSourceKind>().is_err());
    }

    #[test]
    fn detection_uses_root_kind_context() {
        use KnowledgeRootKind::*;
        assert_eq!(
            detect_kind(ProjectRepo, "src/kernel/mod.rs"),
            Some(IngestionSourceKind::CodeFile)
        );
        assert_eq!(
            detect_kind(ProjectRepo, "README.md"),
            Some(IngestionSourceKind::MarkdownText)
        );
        assert_eq!(
            detect_kind(Governance, "task_packets/MT-081.json"),
            Some(IngestionSourceKind::GovernanceArtifact)
        );
        assert_eq!(
            detect_kind(OperatorFolder, "notes/research.md"),
            Some(IngestionSourceKind::OperatorResearchNote)
        );
        assert_eq!(
            detect_kind(MediaLibrary, "episode1.srt"),
            Some(IngestionSourceKind::MediaTranscript)
        );
        assert_eq!(
            detect_kind(MediaLibrary, "episode1.transcript.json"),
            Some(IngestionSourceKind::MediaTranscript)
        );
        assert_eq!(
            detect_kind(ProjectRepo, "docs/spec.pdf"),
            Some(IngestionSourceKind::Pdf)
        );
        // Unknown binary under a repo root: unsupported, never guessed.
        assert_eq!(detect_kind(ProjectRepo, "build/output.bin"), None);
        // Unknown under external import: operator-gated import bucket.
        assert_eq!(
            detect_kind(ExternalImport, "papers/scan.tiff"),
            Some(IngestionSourceKind::ExternalImport)
        );
        // Dotfiles carry no extension.
        assert_eq!(detect_kind(ProjectRepo, ".gitignore"), None);
    }

    #[test]
    fn governance_sub_kinds_classify_contract_vocabulary() {
        use GovernanceSubKind::*;
        let cases = [
            ("task_packets/WP-X/MT-081.json", MtContract),
            ("task_packets/WP-X/packet.json", WpContract),
            ("records/TASKBOARD.json", Taskboard),
            ("records/CHANGELOG.json", Changelog),
            ("workflows/build_workflow.json", WorkflowJson),
            ("receipts/validation_receipt_77.json", Receipt),
            ("manuals/user_manual_index.json", UserManual),
            ("spec/master-spec/spec-modules/02-arch.json", SpecModule),
        ];
        for (path, expected) in cases {
            assert_eq!(
                detect_governance_sub_kind(path),
                Some(expected),
                "path {path}"
            );
        }
        assert_eq!(detect_governance_sub_kind("random/data.json"), None);
    }
}
