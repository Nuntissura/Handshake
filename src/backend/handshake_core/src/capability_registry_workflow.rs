use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use fs2::FileExt;
use std::path::{Path, PathBuf};

use handshake_capability_registry_document::{
    CapabilityKind, CapabilityRegistryDiff, CapabilityRegistryDocument,
    CapabilityRegistryDocumentError, CapabilityRegistryEntry, GovernanceMode, RiskClass,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::mex::registry::MexRegistry;

#[derive(Debug, Error)]
pub enum CapabilityRegistryWorkflowError {
    #[error("CAP_REG_REPO_ROOT_RESOLVE_FAILED")]
    RepoRootResolveFailed,
    #[error("CAP_REG_IO_READ: read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("CAP_REG_IO_CREATE_DIR: mkdir {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("CAP_REG_IO_WRITE: write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("CAP_REG_JSON_PARSE: parse {path}: {source}")]
    JsonParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("CAP_REG_JSON_SERIALIZE: json serialize: {source}")]
    JsonSerialize { source: serde_json::Error },
    #[error("CAP_REG_SCHEMA_PARSE: parse embedded schema: {source}")]
    SchemaParse { source: serde_json::Error },
    #[error("CAP_REG_SCHEMA_COMPILE: compile schema: {message}")]
    SchemaCompile { message: String },
    #[error("CAP_REG_SCHEMA_VALIDATE: {details}")]
    SchemaValidationFailed { details: String },
    #[error("CAP_REG_REGEX_COMPILE: section_ref regex compile failed: {message}")]
    RegexCompile { message: String },
    #[error("CAP_REG_INTEGRITY_DUPLICATE_CAPABILITY_ID: duplicate capability_id {capability_id}")]
    DuplicateCapabilityId { capability_id: String },
    #[error("CAP_REG_INTEGRITY_INVALID_SECTION_REF: invalid section_ref for {capability_id}: {section_ref}")]
    InvalidSectionRef {
        capability_id: String,
        section_ref: String,
    },
    #[error(
        "CAP_REG_INTEGRITY_EMPTY_DISPLAY_NAME: display_name must be non-empty for {capability_id}"
    )]
    EmptyDisplayName { capability_id: String },
    #[error("CAP_REG_MEX_LOAD: load mechanical engines {path}: {message}")]
    MexLoad { path: PathBuf, message: String },
    #[error("CAP_REG_FR_RECORD_EVENT: {message}")]
    FlightRecorderRecordEvent { message: String },
    #[error("CAP_REG_FR_INIT: {message}")]
    FlightRecorderInit { message: String },
    #[error("CAP_REG_POLICY_DECISION_ID_REQUIRED")]
    PolicyDecisionIdRequired,
    #[error("CAP_REG_REVIEWER_ID_REQUIRED")]
    ReviewerIdRequired,
    #[error("CAP_REG_PUBLISH_DIFF_SHA_MISMATCH")]
    PublishDiffShaMismatch,
    #[error(transparent)]
    Storage(#[from] crate::storage::surreal::SurrealStorageError),
    #[error(transparent)]
    Authority(#[from] crate::storage::surreal::resource_authority::ResourceAuthorityError),
    #[error("CAP_REG_SERVER_APPROVAL_CONFIGURATION_REQUIRED")]
    ServerApprovalConfigurationRequired,
    #[error("CAP_REG_APPROVED_INPUT_REQUIRED")]
    ApprovedInputRequired,
    #[error("CAP_REG_APPROVED_INPUT_MISMATCH: {category}")]
    ApprovedInputMismatch { category: &'static str },
    #[error("CAP_REG_PUBLICATION_RECONCILIATION_REQUIRED: original_operation_id={operation_id}, reason={reason}")]
    PublicationReconciliationRequired { operation_id: Uuid, reason: &'static str },
    #[error("CAP_REG_PUBLICATION_UNCONFIRMED: previous_asset_restored={previous_asset_restored}, receipt_outcome={receipt_outcome}, operation_id={operation_id}, settlement_pending={settlement_pending}, observed_asset_sha256={observed_asset_sha256:?}, cause={cause}")]
    PublicationUnconfirmed {
        previous_asset_restored: bool,
        receipt_outcome: &'static str,
        operation_id: Uuid,
        settlement_pending: bool,
        observed_asset_sha256: Option<String>,
        #[source]
        cause: Box<CapabilityRegistryWorkflowError>,
    },
    #[error("CAP_REG_USAGE: {usage}")]
    Usage { usage: String },
    #[error("CAP_REG_UNSUPPORTED_COMMAND: {command}")]
    UnsupportedCommand { command: String },
    #[error("CAP_REG_UNKNOWN_FLAG: {flag}")]
    UnknownFlag { flag: String },
    #[error("CAP_REG_MISSING_FLAG_VALUE: {flag}")]
    MissingFlagValue { flag: String },
}

pub type CapabilityRegistryWorkflowResult<T> =
    std::result::Result<T, CapabilityRegistryWorkflowError>;

const REGISTRY_WORKFLOW_ACTOR_ID: &str = "capability_registry_build";

#[derive(Debug, Clone)]
pub struct CapabilityRegistryWorkflowParams {
    pub trace_id: Uuid,
    pub policy_decision_id: String,
    pub model_id: String,
    pub reviewer_id: Option<String>,
    pub approve: bool,
    pub job_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct CapabilityRegistryWorkflowArtifacts {
    pub draft_path: PathBuf,
    pub diff_path: PathBuf,
    pub review_path: PathBuf,
    pub published_path: PathBuf,
    pub draft_sha256: String,
    pub diff_sha256: String,
    pub capability_registry_version: String,
    pub published: bool,
    pub publication_receipt_confirmed: bool,
}

#[derive(Deserialize)]
struct ApprovedRegistryDependency {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct ApprovedRegistryBaseline {
    registry_path: String,
    sha256: String,
    entry_count: usize,
}

#[derive(Deserialize)]
struct ApprovedRegistryDraft {
    path: String,
    sha256: String,
    entry_count: usize,
}

#[derive(Deserialize)]
struct ApprovedRegistryDiff {
    schema: String,
    baseline: ApprovedRegistryBaseline,
    draft: ApprovedRegistryDraft,
    mechanical_engines: ApprovedRegistryDependency,
    registry_schema: ApprovedRegistryDependency,
    added: Vec<CapabilityRegistryEntry>,
    removed: Vec<Value>,
    changed: Vec<Value>,
}

// Supplied only by the authenticated owning workflow; the retained CLI cannot
// publish with an approve flag. These paths select inputs, never confer approval.
#[derive(Clone)]
pub(crate) struct ApprovedRegistryInput {
    normative_sources: BTreeMap<String, ApprovedRegistryExport>,
    source_exports_path: PathBuf,
    source_exports_sha256: String,
    pub(crate) proposal_directory: PathBuf,
    pub(crate) output_directory: PathBuf,
    pub(crate) approval_record_path: PathBuf,
    pub(crate) approval_record_pointer: String,
    pub(crate) approval_record_sha256: String,
    pub(crate) expected_draft_sha256: String,
    pub(crate) expected_diff_sha256: String,
    pub(crate) attribution: Value,
    pub(crate) storage: crate::storage::surreal::SurrealStorage,
    pub(crate) flight_recorder: std::sync::Arc<dyn FlightRecorder>,
    server_configuration_path: PathBuf,
    server_configuration_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerRegistryApprovalConfiguration {
    proposal_directory: PathBuf,
    output_directory: PathBuf,
    source_exports_path: PathBuf,
    source_exports_sha256: String,
    approval_source_id: String,
    approval_record_pointer: String,
    approval_record_sha256: String,
    expected_draft_sha256: String,
    expected_diff_sha256: String,
}

// Exported original bytes are read-only data. Source IDs are provenance keys,
// never filesystem paths or a second issuer of human approval.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovedRegistryExport {
    path: PathBuf,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrySourceExports {
    source_authority: String,
    normative_sources: BTreeMap<String, ApprovedRegistryExport>,
    approval_record_pointer: String,
    approval_record_sha256: String,
}

fn registry_external_path(parent: &Path, selected: &Path) -> CapabilityRegistryWorkflowResult<PathBuf> {
    let required = || CapabilityRegistryWorkflowError::ServerApprovalConfigurationRequired;
    let forbidden = |path: &Path| path.components().any(|component|
        component.as_os_str().to_string_lossy().eq_ignore_ascii_case(".GOV"));
    if selected.as_os_str().is_empty() || forbidden(selected)
        || (!selected.is_absolute() && (selected.has_root()
            || selected.components().any(|component| matches!(component, std::path::Component::Prefix(_))))) {
        return Err(required());
    }
    let path = fs::canonicalize(if selected.is_absolute() { selected.to_owned() }
        else { parent.join(selected) }).map_err(|_| required())?;
    if !path.is_absolute() || forbidden(&path) { return Err(required()); }
    Ok(path)
}

impl ApprovedRegistryInput {
    pub(crate) fn from_server_configuration(
        storage: crate::storage::surreal::SurrealStorage,
        attribution: Value,
        job_id: Uuid,
        flight_recorder: std::sync::Arc<dyn FlightRecorder>,
    ) -> CapabilityRegistryWorkflowResult<Self> {
        let required = || CapabilityRegistryWorkflowError::ServerApprovalConfigurationRequired;
        // Only server configuration selects immutable external inputs. Jobs cannot
        // select approval authority, and runtime never opens live .GOV or aliases.
        let selected = PathBuf::from(std::env::var_os("HANDSHAKE_CAPABILITY_REGISTRY_APPROVAL_CONFIG")
            .ok_or_else(required)?);
        if !selected.is_absolute() { return Err(required()); }
        let server_configuration_path = registry_external_path(Path::new(""), &selected)?;
        let parent = server_configuration_path.parent().ok_or_else(required)?;
        let bytes = fs::read(&server_configuration_path).map_err(|source|
            CapabilityRegistryWorkflowError::ReadFile { path: server_configuration_path.clone(), source })?;
        let configuration: ServerRegistryApprovalConfiguration = serde_json::from_slice(&bytes)
            .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: server_configuration_path.clone(), source })?;
        let source_exports_path = registry_external_path(parent, &configuration.source_exports_path)?;
        let export_bytes = fs::read(&source_exports_path).map_err(|source|
            CapabilityRegistryWorkflowError::ReadFile { path: source_exports_path.clone(), source })?;
        if sha256_hex(&export_bytes) != configuration.source_exports_sha256 { return Err(required()); }
        let mut exports: RegistrySourceExports = serde_json::from_slice(&export_bytes)
            .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: source_exports_path.clone(), source })?;
        if exports.source_authority.trim().is_empty() || exports.normative_sources.is_empty()
            || exports.approval_record_pointer != configuration.approval_record_pointer
            || exports.approval_record_sha256 != configuration.approval_record_sha256 {
            return Err(required());
        }
        let export_parent = source_exports_path.parent().ok_or_else(required)?;
        for (identity, source) in &mut exports.normative_sources {
            if identity.is_empty() || source.sha256.len() != 64
                || !source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) { return Err(required()); }
            source.path = registry_external_path(export_parent, &source.path)?;
        }
        let approval = exports.normative_sources.get(&configuration.approval_source_id).ok_or_else(required)?;
        if approval.sha256 != configuration.approval_record_sha256 { return Err(required()); }
        let approval_record_path = approval.path.clone();
        let proposal_directory = registry_external_path(parent, &configuration.proposal_directory)?;
        let output_directory = registry_external_path(parent, &configuration.output_directory)?.join(job_id.to_string());
        Ok(Self {
            normative_sources: exports.normative_sources, source_exports_path,
            source_exports_sha256: configuration.source_exports_sha256,
            proposal_directory, output_directory,
            approval_record_path, approval_record_pointer: configuration.approval_record_pointer,
            approval_record_sha256: configuration.approval_record_sha256,
            expected_draft_sha256: configuration.expected_draft_sha256,
            expected_diff_sha256: configuration.expected_diff_sha256,
            attribution, storage, flight_recorder, server_configuration_path,
            server_configuration_sha256: sha256_hex(&bytes),
        })
    }
}

tokio::task_local! {
    static APPROVED_REGISTRY_INPUT: ApprovedRegistryInput;
}

pub(crate) async fn run_approved_capability_registry_workflow(
    repo_root: &Path,
    registry: &CapabilityRegistry,
    flight_recorder: &dyn FlightRecorder,
    params: CapabilityRegistryWorkflowParams,
    input: ApprovedRegistryInput,
) -> CapabilityRegistryWorkflowResult<CapabilityRegistryWorkflowArtifacts> {
    APPROVED_REGISTRY_INPUT.scope(input,
        run_capability_registry_workflow(repo_root, registry, flight_recorder, params)).await
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CapabilityRegistryReview {
    approved: bool,
    reviewer_id: Option<String>,
    diff_sha256: String,
    capability_registry_version: String,
}

use crate::storage::artifacts::sha256_hex;

pub fn repo_root_from_manifest_dir() -> CapabilityRegistryWorkflowResult<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or(CapabilityRegistryWorkflowError::RepoRootResolveFailed)
}

fn load_optional_registry(
    path: &Path,
) -> CapabilityRegistryWorkflowResult<Option<CapabilityRegistryDocument>> {
    if !path.exists() {
        return Ok(None);
    }
    let data =
        fs::read_to_string(path).map_err(|source| CapabilityRegistryWorkflowError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
    let doc: CapabilityRegistryDocument = serde_json::from_str(&data).map_err(|source| {
        CapabilityRegistryWorkflowError::JsonParse {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(Some(doc))
}

fn load_schema() -> CapabilityRegistryWorkflowResult<Value> {
    let raw = include_str!("../schemas/capability_registry.schema.json");
    serde_json::from_str(raw)
        .map_err(|source| CapabilityRegistryWorkflowError::SchemaParse { source })
}

fn document_error_into_workflow(error: CapabilityRegistryDocumentError) -> CapabilityRegistryWorkflowError {
    match error {
        CapabilityRegistryDocumentError::SchemaCompile { message } =>
            CapabilityRegistryWorkflowError::SchemaCompile { message },
        CapabilityRegistryDocumentError::SchemaValidationFailed { details } =>
            CapabilityRegistryWorkflowError::SchemaValidationFailed { details },
        CapabilityRegistryDocumentError::RegexCompile { message } =>
            CapabilityRegistryWorkflowError::RegexCompile { message },
        CapabilityRegistryDocumentError::DuplicateCapabilityId { capability_id } =>
            CapabilityRegistryWorkflowError::DuplicateCapabilityId { capability_id },
        CapabilityRegistryDocumentError::InvalidSectionRef { capability_id, section_ref } =>
            CapabilityRegistryWorkflowError::InvalidSectionRef { capability_id, section_ref },
        CapabilityRegistryDocumentError::EmptyDisplayName { capability_id } =>
            CapabilityRegistryWorkflowError::EmptyDisplayName { capability_id },
    }
}

fn validate_schema(schema: &Value, instance: &Value) -> CapabilityRegistryWorkflowResult<()> {
    handshake_capability_registry_document::validate_schema(schema, instance)
        .map_err(document_error_into_workflow)
}

fn validate_integrity(doc: &CapabilityRegistryDocument) -> CapabilityRegistryWorkflowResult<()> {
    handshake_capability_registry_document::validate_integrity(doc)
        .map_err(document_error_into_workflow)
}

fn display_name_for(capability_id: &str) -> String {
    let mut words = Vec::new();
    let mut current = String::new();

    let push_word = |words: &mut Vec<String>, current: &mut String| {
        if current.is_empty() {
            return;
        }
        if current.chars().all(|c| c.is_ascii_uppercase() || c == '-') {
            words.push(current.to_ascii_lowercase());
        } else {
            words.push(std::mem::take(current));
        }
        current.clear();
    };

    for ch in capability_id.chars() {
        let normalized = match ch {
            '.' | ':' | '_' => ' ',
            other => other,
        };
        if normalized.is_whitespace() {
            push_word(&mut words, &mut current);
        } else {
            current.push(normalized);
        }
    }
    push_word(&mut words, &mut current);

    let mut out = String::new();
    for (idx, word) in words.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push_str(&first.to_uppercase().collect::<String>());
            out.push_str(chars.as_str());
        }
    }

    out
}

fn classify(capability_id: &str) -> (CapabilityKind, RiskClass, String, BTreeSet<String>) {
    let mut tags = BTreeSet::new();
    tags.insert("capability".to_string());

    let (kind, risk, section_ref) = if capability_id.starts_with("engine.") {
        tags.insert("engine".to_string());
        (
            CapabilityKind::Engine,
            RiskClass::Medium,
            "11.8".to_string(),
        )
    } else if capability_id.starts_with("terminal.") || capability_id == "terminal.exec" {
        tags.insert("terminal".to_string());
        (
            CapabilityKind::Surface,
            RiskClass::High,
            "11.7.1".to_string(),
        )
    } else if capability_id.starts_with("export.") {
        tags.insert("workflow".to_string());
        (
            CapabilityKind::Workflow,
            RiskClass::Medium,
            "11.5".to_string(),
        )
    } else if capability_id.starts_with("doc.") {
        tags.insert("docs".to_string());
        (
            CapabilityKind::Surface,
            RiskClass::Medium,
            "7.1.1".to_string(),
        )
    } else if capability_id.starts_with("fr.") || capability_id.starts_with("diagnostics.") {
        tags.insert("observability".to_string());
        (
            CapabilityKind::Runtime,
            RiskClass::Medium,
            "11.5".to_string(),
        )
    } else if capability_id.starts_with("jobs.") {
        tags.insert("jobs".to_string());
        (
            CapabilityKind::Runtime,
            RiskClass::Medium,
            "2.6".to_string(),
        )
    } else if capability_id.starts_with("CALENDAR_") {
        tags.insert("calendar".to_string());
        let risk = if capability_id.contains("DELETE") {
            RiskClass::High
        } else {
            RiskClass::Medium
        };
        (CapabilityKind::Surface, risk, "11.9".to_string())
    } else {
        match capability_id {
            "fs.read" => (
                CapabilityKind::Runtime,
                RiskClass::Medium,
                "11.1".to_string(),
            ),
            "fs.write" => (CapabilityKind::Runtime, RiskClass::High, "11.1".to_string()),
            "proc.exec" => (CapabilityKind::Runtime, RiskClass::High, "11.1".to_string()),
            "net.http" => (CapabilityKind::Runtime, RiskClass::High, "11.1".to_string()),
            "device" => (
                CapabilityKind::Runtime,
                RiskClass::Medium,
                "11.1".to_string(),
            ),
            "secrets.use" => (
                CapabilityKind::Runtime,
                RiskClass::Critical,
                "11.1".to_string(),
            ),
            "creative" => (CapabilityKind::Runtime, RiskClass::Low, "11.1".to_string()),
            _ => (
                CapabilityKind::Integration,
                RiskClass::Medium,
                "11.1".to_string(),
            ),
        }
    };

    (kind, risk, section_ref, tags)
}

fn default_mode_for(risk: &RiskClass) -> GovernanceMode {
    match risk {
        RiskClass::Low => GovernanceMode::GovLight,
        RiskClass::Medium => GovernanceMode::GovStandard,
        RiskClass::High => GovernanceMode::GovStandard,
        RiskClass::Critical => GovernanceMode::GovStrict,
    }
}

fn build_registry_document(
    registry: &CapabilityRegistry,
    repo_root: &Path,
) -> CapabilityRegistryWorkflowResult<CapabilityRegistryDocument> {
    let mut by_id: BTreeMap<String, CapabilityRegistryEntry> = BTreeMap::new();

    // Axes
    for axis in registry.axes().iter() {
        let (kind, risk, section_ref, tags) = classify(axis.as_str());
        by_id.insert(
            axis.clone(),
            CapabilityRegistryEntry {
                capability_id: axis.clone(),
                kind,
                display_name: display_name_for(axis.as_str()),
                section_ref,
                required_capabilities: Vec::new(),
                default_governance_mode: default_mode_for(&risk),
                risk_class: risk,
                tags: tags.into_iter().collect(),
            },
        );
    }

    // Full IDs
    for full in registry.ids().iter() {
        let (kind, risk, section_ref, tags) = classify(full.as_str());
        by_id.insert(
            full.clone(),
            CapabilityRegistryEntry {
                capability_id: full.clone(),
                kind,
                display_name: display_name_for(full.as_str()),
                section_ref,
                required_capabilities: Vec::new(),
                default_governance_mode: default_mode_for(&risk),
                risk_class: risk,
                tags: tags.into_iter().collect(),
            },
        );
    }

    // Mechanical engines
    let engines_path = repo_root.join("src/backend/handshake_core/mechanical_engines.json");
    let mex = MexRegistry::load_from_path(&engines_path).map_err(|source| {
        CapabilityRegistryWorkflowError::MexLoad {
            path: engines_path.clone(),
            message: source.to_string(),
        }
    })?;
    for engine in mex.engines() {
        let mut required = engine.required_caps.clone();
        for op in &engine.ops {
            required.extend(op.capabilities.iter().cloned());
        }
        required.sort();
        required.dedup();

        let (kind, risk, section_ref, mut tags) = classify(engine.engine_id.as_str());
        tags.insert("mechanical_engine".to_string());

        by_id.insert(
            engine.engine_id.clone(),
            CapabilityRegistryEntry {
                capability_id: engine.engine_id.clone(),
                kind,
                display_name: display_name_for(engine.engine_id.as_str()),
                section_ref,
                required_capabilities: required,
                default_governance_mode: default_mode_for(&risk),
                risk_class: risk,
                tags: tags.into_iter().collect(),
            },
        );
    }

    Ok(CapabilityRegistryDocument {
        entries: by_id.into_values().collect(),
    })
}

fn write_json_file(path: &Path, value: &Value) -> CapabilityRegistryWorkflowResult<Vec<u8>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            CapabilityRegistryWorkflowError::CreateDir {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|source| CapabilityRegistryWorkflowError::JsonSerialize { source })?;
    fs::write(path, &bytes).map_err(|source| CapabilityRegistryWorkflowError::WriteFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(bytes)
}

fn write_struct_file<T: Serialize>(
    path: &Path,
    value: &T,
) -> CapabilityRegistryWorkflowResult<Vec<u8>> {
    let json = serde_json::to_value(value)
        .map_err(|source| CapabilityRegistryWorkflowError::JsonSerialize { source })?;
    write_json_file(path, &json)
}

// A canonical-path OS lock serializes processes; the synced journal retains
// ownership after process loss or ambiguous settlement. It is never grant proof.
// Missing/invisible committed receipts require reconciliation, not a timeout,
// rollback assumption or automatic retry; failures expose the original UUID.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryPublicationAttempt {
    operation_id: Uuid,
    fr_event_id: Uuid,
    receipt: Value,
    previous_bytes: Vec<u8>,
}

struct RegistryPublicationJournal {
    file: fs::File,
    path: PathBuf,
    attempt: Option<RegistryPublicationAttempt>,
}

impl RegistryPublicationJournal {
    fn acquire(asset: &Path) -> CapabilityRegistryWorkflowResult<Self> {
        let path = asset.with_extension("json.publication-owner");
        let mut file = fs::OpenOptions::new().read(true).write(true).create(true).open(&path)
            .map_err(|source| CapabilityRegistryWorkflowError::WriteFile { path: path.clone(), source })?;
        file.try_lock_exclusive().map_err(|_| CapabilityRegistryWorkflowError::ApprovedInputMismatch {
            category: "canonical_publication_path_owned",
        })?;
        let mut bytes = Vec::new();
        (&mut file).take(1024 * 1024 + 1).read_to_end(&mut bytes)
            .map_err(|source| CapabilityRegistryWorkflowError::ReadFile { path: path.clone(), source })?;
        if bytes.len() > 1024 * 1024 {
            return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "publication_journal_size" });
        }
        let attempt = if bytes.is_empty() { None } else {
            Some(serde_json::from_slice(&bytes).map_err(|source|
                CapabilityRegistryWorkflowError::JsonParse { path: path.clone(), source })?)
        };
        Ok(Self { file, path, attempt })
    }

    fn retain(&mut self, attempt: RegistryPublicationAttempt) -> CapabilityRegistryWorkflowResult<()> {
        // Write once before mutation. Never truncate an existing pending owner
        // while trying to mark terminal state: a crash could erase its identity.
        if self.attempt.is_some() {
            return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "publication_owner_already_retained" });
        }
        let bytes = serde_json::to_vec(&attempt)
            .map_err(|source| CapabilityRegistryWorkflowError::JsonSerialize { source })?;
        if bytes.len() > 1024 * 1024 {
            return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "publication_journal_size" });
        }
        self.file.seek(SeekFrom::Start(0)).and_then(|_| self.file.set_len(0))
            .and_then(|_| self.file.write_all(&bytes)).and_then(|_| self.file.sync_all())
            .map_err(|source| CapabilityRegistryWorkflowError::WriteFile { path: self.path.clone(), source })?;
        self.attempt = Some(attempt);
        Ok(())
    }
}

fn registry_receipt_key(operation_id: Uuid) -> String {
    format!("capability-registry-publication:{operation_id}")
}

async fn observe_registry_commit(
    storage: &crate::storage::surreal::SurrealStorage,
    attempt: &RegistryPublicationAttempt,
) -> CapabilityRegistryWorkflowResult<()> {
    use crate::kernel::KernelEventType;
    let event = crate::storage::surreal::event_ledger::get_by_idempotency(
        storage, &registry_receipt_key(attempt.operation_id)).await
        .map_err(|_| CapabilityRegistryWorkflowError::PublicationReconciliationRequired {
            operation_id: attempt.operation_id, reason: "authenticated_receipt_unavailable",
        })?
        .ok_or(CapabilityRegistryWorkflowError::PublicationReconciliationRequired {
            operation_id: attempt.operation_id, reason: "receipt_absence_does_not_prove_rollback",
        })?;
    // Registry payload has no preference-event normalization branch. Compare the
    // complete decoded payload and its original whole canonical hash, never a subset.
    if event.event_type != KernelEventType::FlightRecorderMirrorRecorded
        || event.source_component != "capability_registry_workflow"
        || event.aggregate_type != "capability_registry_publication"
        || event.aggregate_id != attempt.operation_id.to_string()
        || event.event_version != "kernel_event_v1"
        || event.idempotency_key != registry_receipt_key(attempt.operation_id)
        || Some(event.kernel_task_run_id.as_str()) != attempt.receipt["binding"]["job_id"].as_str()
        || Some(event.session_run_id.as_str()) != attempt.receipt["envelope"]["authenticated_session_id"].as_str()
        || event.correlation_id.as_deref() != attempt.receipt["binding"]["trace_id"].as_str()
        || Some(event.actor.actor_kind()) != attempt.receipt["envelope"]["actor_kind"].as_str()
        || Some(event.actor.actor_id()) != attempt.receipt["envelope"]["actor_id"].as_str()
        || event.payload != attempt.receipt
        || event.payload_hash != sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(&attempt.receipt)) {
        return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "publication_receipt_binding" });
    }
    Ok(())
}

async fn record_build_event(
    flight_recorder: &dyn FlightRecorder,
    params: &CapabilityRegistryWorkflowParams,
    mut payload: Value,
    model_id: Option<&str>,
) -> CapabilityRegistryWorkflowResult<()> {
    let input = APPROVED_REGISTRY_INPUT.try_with(Clone::clone)
        .map_err(|_| CapabilityRegistryWorkflowError::ApprovedInputRequired)?;
    payload["attribution"] = input.attribution.clone();
    for (target, source) in [("owner_account_id", "account_id"), ("actor_principal_id", "principal_id"),
        ("authenticated_session_id", "session_id"), ("access_space_id", "access_space_id"), ("workspace_id", "workspace_id")] {
        payload[target] = input.attribution[source].clone();
    }
    let stable_event_id = payload["publication"]["fr_event_id"].as_str()
        .map(Uuid::parse_str).transpose()
        .map_err(|_| CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "publication_fr_identity" })?;

    let workspace_id = input.attribution["workspace_id"].as_str()
        .ok_or(CapabilityRegistryWorkflowError::ApprovedInputRequired)?;
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        params.trace_id,
        payload,
    )
    .with_actor_id(REGISTRY_WORKFLOW_ACTOR_ID)
    .with_wsids(vec![workspace_id.to_owned()]);

    if let Some(event_id) = stable_event_id { event.event_id = event_id; }

    if let Some(job_id) = params.job_id {
        event = event.with_job_id(job_id.to_string());
    }
    if let Some(workflow_id) = params.workflow_id {
        event = event.with_workflow_id(workflow_id.to_string());
    }
    if let Some(model_id) = model_id {
        event = event.with_model_id(model_id.to_string());
    }
    event = event.with_policy_decision(params.policy_decision_id.clone());

    flight_recorder.record_event(event).await.map_err(|source| {
        CapabilityRegistryWorkflowError::FlightRecorderRecordEvent {
            message: source.to_string(),
        }
    })
}

pub async fn run_capability_registry_workflow(
    repo_root: &Path,
    registry: &CapabilityRegistry,
    flight_recorder: &dyn FlightRecorder,
    params: CapabilityRegistryWorkflowParams,
) -> CapabilityRegistryWorkflowResult<CapabilityRegistryWorkflowArtifacts> {
    if params.policy_decision_id.trim().is_empty() {
        return Err(CapabilityRegistryWorkflowError::PolicyDecisionIdRequired);
    }

    let input = APPROVED_REGISTRY_INPUT.try_with(Clone::clone)
        .map_err(|_| CapabilityRegistryWorkflowError::ApprovedInputRequired)?;
    let mismatch = |category| CapabilityRegistryWorkflowError::ApprovedInputMismatch { category };
    let read = |path: &Path| -> CapabilityRegistryWorkflowResult<Vec<u8>> {
        let resolved = registry_external_path(repo_root, path)?;
        fs::read(&resolved).map_err(|source|
            CapabilityRegistryWorkflowError::ReadFile { path: resolved, source })
    };
    let parse = |path: &Path, bytes: &[u8]| serde_json::from_slice::<Value>(bytes)
        .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: path.to_owned(), source });
    for path in [&input.proposal_directory, &input.output_directory] {
        if !path.is_absolute() { return Err(mismatch("absolute_input_paths")); }
    }
    if params.job_id.is_none() || params.workflow_id.is_none()
        || ["account_id", "principal_id", "session_id", "access_space_id", "workspace_id"]
            .iter().any(|key| input.attribution[*key].as_str().map_or(true, str::is_empty)) {
        return Err(mismatch("actual_job_and_native_attribution"));
    }
    let draft_sha = input.expected_draft_sha256.as_str();
    let diff_sha = input.expected_diff_sha256.as_str();
    for hash in [draft_sha, diff_sha, input.approval_record_sha256.as_str()] {
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(mismatch("expected_sha256"));
        }
    }
    let draft_path = input.proposal_directory.join("capability_registry_draft.json");
    let diff_path = input.proposal_directory.join("capability_registry_diff.json");
    let draft_bytes = read(&draft_path)?;
    let diff_bytes = read(&diff_path)?;
    if sha256_hex(&draft_bytes) != draft_sha || sha256_hex(&diff_bytes) != diff_sha {
        return Err(mismatch("approved_artifact_bytes"));
    }
    if !input.approval_record_path.is_absolute()
        || !input.approval_record_pointer.starts_with('/') {
        return Err(mismatch("approval_record_selector"));
    }
    let approval_bytes = read(&input.approval_record_path)?;
    if sha256_hex(&approval_bytes) != input.approval_record_sha256 {
        return Err(mismatch("approval_record_bytes"));
    }
    let approval_record = parse(&input.approval_record_path, &approval_bytes)?;
    let approval = approval_record.pointer(&input.approval_record_pointer)
        .ok_or_else(|| mismatch("approval_record_pointer"))?;
    // The governed invocation selects an existing human decision and its raw
    // binding. An approve execution flag or newly authored review is not authority.
    if approval["human_actual_diff_approval"] != true
        || approval["signature_invented"] != false
        || approval["registry_operator_quote"].as_str().map_or(true, str::is_empty)
        || approval["source_thread_id"].as_str().map_or(true, str::is_empty)
        || approval["registry_draft_sha256"] != draft_sha
        || approval["registry_diff_sha256"] != diff_sha {
        return Err(mismatch("recorded_human_approval"));
    }
    let diff: ApprovedRegistryDiff = serde_json::from_slice(&diff_bytes)
        .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: diff_path.clone(), source })?;
    if diff.schema != "wp012.auth_mapping_registry_proposal_diff@1"
        || diff.baseline.registry_path != "assets/capability_registry.json"
        || diff.draft.path != "capability_registry_draft.json" || diff.draft.sha256 != draft_sha
        || !diff.removed.is_empty() || !diff.changed.is_empty()  {
        return Err(mismatch("approved_diff_contract"));
    }
    let published_path = fs::canonicalize(repo_root.join("assets/capability_registry.json"))
        .map_err(|source| CapabilityRegistryWorkflowError::ReadFile {
            path: repo_root.join("assets/capability_registry.json"), source,
        })?;
    // Acquire before any baseline observation, not just before mutation.
    let journal = if params.approve {
        Some(std::sync::Arc::new(std::sync::Mutex::new(RegistryPublicationJournal::acquire(&published_path)?)))
    } else { None };
    let retained = journal.as_ref().and_then(|journal|
        journal.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).attempt.clone());
    let previous_bytes = match &retained {
        Some(attempt) => attempt.previous_bytes.clone(),
        None => read(&published_path)?,
    };
    if sha256_hex(&previous_bytes) != diff.baseline.sha256.as_str() {
        return Err(mismatch("baseline_bytes"));
    }
    let previous = parse(&published_path, &previous_bytes)?;
    let instance = parse(&draft_path, &draft_bytes)?;
    let doc: CapabilityRegistryDocument = serde_json::from_value(instance.clone())
        .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: draft_path.clone(), source })?;
    validate_schema(&load_schema()?, &instance)?;
    validate_integrity(&doc)?;
    let baseline = previous["entries"].as_array().ok_or_else(|| mismatch("baseline_entries"))?;
    let next = instance["entries"].as_array().ok_or_else(|| mismatch("draft_entries"))?;
    if baseline.len() != diff.baseline.entry_count || next.len() != diff.draft.entry_count || baseline.iter().any(|entry| !next.contains(entry)) {
        return Err(mismatch("baseline_preservation"));
    }
    let previous_doc: CapabilityRegistryDocument = serde_json::from_value(previous.clone())
        .map_err(|source| CapabilityRegistryWorkflowError::JsonParse { path: published_path.clone(), source })?;
    let additions: Vec<_> = doc.entries.iter().filter(|entry| !previous_doc.entries.contains(entry)).collect();
    if additions.len() != diff.added.len() || diff.added.iter().any(|entry| !additions.contains(&entry)) {
        return Err(mismatch("approved_additions"));
    }
    let mut dependencies = BTreeMap::new();
    let mut dependency_paths = BTreeMap::new();
    for dependency in [&diff.mechanical_engines, &diff.registry_schema] {
        let relative = dependency.path.as_str();
        let bytes = read(&repo_root.join(relative))?;
        let hash = sha256_hex(&bytes);
        if hash != dependency.sha256 { return Err(mismatch("normative_dependency_drift")); }
        dependency_paths.insert(relative.to_owned(), registry_external_path(repo_root, Path::new(relative))?);
        dependencies.insert(relative.to_owned(), hash);
    }
    // Original source identities resolve only through the pinned external map.
    let exported = |identity: &str| -> CapabilityRegistryWorkflowResult<(PathBuf, Vec<u8>)> {
        let source = input.normative_sources.get(identity).ok_or_else(|| mismatch("exported_source_missing"))?;
        if registry_external_path(Path::new(""), &source.path)? != source.path {
            return Err(mismatch("exported_source_alias_drift"));
        }
        let bytes = read(&source.path)?;
        if sha256_hex(&bytes) != source.sha256 { return Err(mismatch("exported_source_drift")); }
        Ok((source.path.clone(), bytes))
    };
    let pointer_id = ".GOV/spec/SPEC_CURRENT.md";
    let (pointer_path, pointer_bytes) = exported(pointer_id)?;
    let pointer = parse(&pointer_path, &pointer_bytes)?;
    let manifest_id = pointer["current_spec"]["entrypoint_path"].as_str().ok_or_else(|| mismatch("spec_pointer"))?;
    let (manifest_path, manifest_bytes) = exported(manifest_id)?;
    let manifest = parse(&manifest_path, &manifest_bytes)?;
    dependency_paths.insert(pointer_id.to_owned(), pointer_path);
    dependencies.insert(pointer_id.to_owned(), sha256_hex(&pointer_bytes));
    dependency_paths.insert(manifest_id.to_owned(), manifest_path);
    dependencies.insert(manifest_id.to_owned(), sha256_hex(&manifest_bytes));
    let manifest_parent = manifest_id.rsplit_once('/').ok_or_else(|| mismatch("spec_manifest_parent"))?.0;
    for module in manifest["modules"].as_array().ok_or_else(|| mismatch("spec_modules"))? {
        let relative = module["path"].as_str().ok_or_else(|| mismatch("spec_module_path"))?;
        let identity = format!("{manifest_parent}/{relative}");
        let (path, bytes) = exported(&identity)?;
        let hash = sha256_hex(&bytes);
        if Some(hash.as_str()) != module["sha256"].as_str() { return Err(mismatch("spec_module_drift")); }
        if dependency_paths.insert(identity.clone(), path).is_some() { return Err(mismatch("spec_module_duplicate")); }
        dependencies.insert(identity, hash);
    }
    // Recheck every supplied export, including the original human approval record.
    for identity in input.normative_sources.keys() {
        let (path, bytes) = exported(identity)?;
        dependency_paths.insert(identity.clone(), path);
        dependencies.insert(identity.clone(), sha256_hex(&bytes));
    }
    dependency_paths.insert(input.source_exports_path.to_string_lossy().into_owned(), input.source_exports_path.clone());
    dependencies.insert(input.source_exports_path.to_string_lossy().into_owned(), input.source_exports_sha256.clone());
    dependency_paths.insert(input.approval_record_path.to_string_lossy().into_owned(), input.approval_record_path.clone());
    dependency_paths.insert(input.server_configuration_path.to_string_lossy().into_owned(), input.server_configuration_path.clone());
    dependencies.insert(input.approval_record_path.to_string_lossy().into_owned(), sha256_hex(&approval_bytes));
    dependencies.insert(input.server_configuration_path.to_string_lossy().into_owned(), input.server_configuration_sha256.clone());
    if input.output_directory.starts_with(repo_root)
        || input.output_directory == input.proposal_directory {
        return Err(mismatch("external_scoped_output_directory"));
    }
    let review_path = input.output_directory.join("capability_registry_review.json");
    let output_parent = input.output_directory.parent().ok_or_else(|| mismatch("output_parent_missing"))?;
    if registry_external_path(Path::new(""), output_parent)? != output_parent {
        return Err(mismatch("output_parent_alias_drift"));
    }
    if input.output_directory.exists()
        && registry_external_path(Path::new(""), &input.output_directory)? != input.output_directory {
        return Err(mismatch("output_alias_drift"));
    }
    fs::create_dir_all(&input.output_directory).map_err(|source|
        CapabilityRegistryWorkflowError::CreateDir { path: input.output_directory.clone(), source })?;
    write_json_file(&review_path, &json!({
        "approval": approval, "draft_sha256": draft_sha, "diff_sha256": diff_sha,
        "normative_dependencies": dependencies, "publication_requested": params.approve,
        "publication_confirmed": false, "attribution": input.attribution,
    }))?;
    record_build_event(flight_recorder, &params, json!({
        "event": "capability_registry_review", "approval": approval,
        "draft_sha256": draft_sha, "diff_sha256": diff_sha,
        "normative_dependencies": dependencies, "publication_requested": params.approve,
        "publication_confirmed": false,
    }), None).await?;
    let mut artifacts = CapabilityRegistryWorkflowArtifacts {
        draft_path, diff_path, review_path, published_path: published_path.clone(),
        draft_sha256: draft_sha.into(), diff_sha256: diff_sha.into(),
        capability_registry_version: draft_sha.into(), published: false,
        publication_receipt_confirmed: false,
    };
    if !params.approve { return Ok(artifacts); }
    let journal = journal.ok_or_else(|| mismatch("publication_path_owner_missing"))?;
    let publication_binding = json!({
        "path_sha256": sha256_hex(published_path.to_string_lossy().as_bytes()),
        "baseline_sha256": diff.baseline.sha256,
        "draft_sha256": draft_sha, "diff_sha256": diff_sha,
        "approval_sha256": input.approval_record_sha256,
        "configuration_sha256": input.server_configuration_sha256,
        "normative_sha256": sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(&json!(dependencies))),
        "job_id": params.job_id, "workflow_id": params.workflow_id, "trace_id": params.trace_id,
    });
    if let Some(attempt) = retained {
        let storage = input.storage.clone();
        let read_storage = storage.clone();
        let expected_binding = publication_binding.clone();
        let retained_path = published_path.clone();
        let original_operation_id = attempt.operation_id;
        let unresolved_path = published_path.clone();
        // A read-only reconciliation transaction is not a new publication UUID.
        // Receipt absence/invisibility never authorizes restoration or a retry.
        storage.with_registry_publication_authority(Uuid::now_v7(),
            crate::storage::surreal::ContextOperationCancellation::new(), move |envelope| async move {
                if attempt.receipt["binding"] != expected_binding
                    || attempt.receipt["envelope"] != envelope
                    || attempt.receipt["operation_id"] != attempt.operation_id.to_string()
                    || attempt.receipt["fr_event_id"] != attempt.fr_event_id.to_string() {
                    return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "original_publication_envelope" });
                }
                observe_registry_commit(&read_storage, &attempt).await?;
                let bytes = fs::read(&retained_path).map_err(|source|
                    CapabilityRegistryWorkflowError::ReadFile { path: retained_path.clone(), source })?;
                if sha256_hex(&bytes) != expected_binding["draft_sha256"].as_str().unwrap_or("") {
                    return Err(CapabilityRegistryWorkflowError::ApprovedInputMismatch { category: "committed_publication_asset_drift" });
                }
                Ok::<_, CapabilityRegistryWorkflowError>(())
            }).await.map_err(|cause| CapabilityRegistryWorkflowError::PublicationUnconfirmed {
                previous_asset_restored: false, operation_id: original_operation_id,
                settlement_pending: true, receipt_outcome: "reconciliation_not_confirmed",
                observed_asset_sha256: fs::read(&unresolved_path).ok().map(|bytes| sha256_hex(&bytes)),
                cause: Box::new(cause),
            })?;
        artifacts.published = true;
        artifacts.publication_receipt_confirmed = true;
        return Ok(artifacts);
    }
    // Recheck captured input bytes immediately before the file mutation. File and
    // recorder writes are separate; a failed receipt can have an unknown outcome.
    for (path, expected) in [(&artifacts.draft_path, draft_sha), (&artifacts.diff_path, diff_sha),
        (&published_path, diff.baseline.sha256.as_str())] {
        if sha256_hex(&read(path)?) != expected { return Err(mismatch("prepublication_input_drift")); }
    }
    for (relative, expected) in &dependencies {
        let path = dependency_paths.get(relative).ok_or_else(|| mismatch("normative_source_path_missing"))?;
        if registry_external_path(Path::new(""), path)? != *path
            || sha256_hex(&read(path)?) != *expected { return Err(mismatch("prepublication_normative_drift")); }
    }
    let operation_id = Uuid::now_v7();
    let cancellation = crate::storage::surreal::ContextOperationCancellation::new();
    struct CallerLifetime(std::sync::Arc<crate::storage::surreal::ContextOperationCancellation>);
    impl Drop for CallerLifetime { fn drop(&mut self) { self.0.cancel(); } }
    let caller = CallerLifetime(cancellation.clone());
    let completion_storage = input.storage.clone();
    let authority_storage = completion_storage.clone();
    let completion_authority = crate::storage::surreal::event_ledger::capture_account_authority();
    let completion_input = input.clone();
    let publication_path = published_path.clone();
    let publication_input = input.clone();
    let publication_params = params.clone();
    let publication_approval = approval.clone();
    let publication_dependencies = dependencies.clone();
    let publication_dependency_paths = dependency_paths;
    let publication_draft_path = artifacts.draft_path.clone();
    let publication_diff_path = artifacts.diff_path.clone();
    let publication_baseline_sha = diff.baseline.sha256.clone();
    let publication_draft_sha = draft_sha.to_owned();
    let publication_diff_sha = diff_sha.to_owned();
    let publication_root = repo_root.to_owned();
    // The callback must own the recorder handle across the authenticated worker.
    // The workflow supplies the same process-owned Arc; no secondary recorder exists.
    let recorder = input.flight_recorder.clone();
    let completion_published_path = published_path.clone();
    let completion_journal = journal.clone();
    let worker_journal = journal.clone();
    let publication_binding = publication_binding.clone();
    let receipt_storage = input.storage.clone();
    let fr_event_id = Uuid::now_v7();

    let completion = tokio::spawn(async move {
        let work = async move {
            let write_attempted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let receipt_attempted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let receipt_recorded = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let publication_previous_bytes = previous_bytes.clone();
            let worker_write_attempted = write_attempted.clone();
            let worker_receipt_attempted = receipt_attempted.clone();
            let worker_receipt_recorded = receipt_recorded.clone();
            let result: CapabilityRegistryWorkflowResult<()> = authority_storage
                .with_registry_publication_authority(operation_id, cancellation, move |envelope| async move {
                    APPROVED_REGISTRY_INPUT.scope(publication_input.clone(), async move {
                        let mismatch = |category| CapabilityRegistryWorkflowError::ApprovedInputMismatch { category };
                        for (path, expected) in [(&publication_draft_path, &publication_draft_sha),
                            (&publication_diff_path, &publication_diff_sha), (&publication_path, &publication_baseline_sha)] {
                            let resolved = registry_external_path(Path::new(""), path)?;
                            let bytes = fs::read(&resolved).map_err(|source|
                                CapabilityRegistryWorkflowError::ReadFile { path: resolved, source })?;
                            if sha256_hex(&bytes) != *expected { return Err(mismatch("publication_input_drift")); }
                        }
                        for (relative, expected) in &publication_dependencies {
                            let path = publication_dependency_paths.get(relative).ok_or_else(|| mismatch("normative_source_path_missing"))?;
                            if registry_external_path(Path::new(""), path)? != *path { return Err(mismatch("publication_source_alias_drift")); }
                            let bytes = fs::read(path).map_err(|source|
                                CapabilityRegistryWorkflowError::ReadFile { path: path.clone(), source })?;
                            if sha256_hex(&bytes) != *expected { return Err(mismatch("publication_normative_drift")); }
                        }
                        if fs::canonicalize(publication_root.join("assets/capability_registry.json")).ok().as_ref() != Some(&publication_path)
                            || fs::canonicalize(&publication_path).ok().as_ref() != Some(&publication_path) {
                            return Err(mismatch("canonical_publication_path_drift"));
                        }
                        let receipt = json!({
                            "schema": "capability_registry_publication@1",
                            "receipt_kind": "capability_registry_publication_committed",
                            "operation_id": operation_id, "fr_event_id": fr_event_id,
                            "workspace_id": envelope["workspace_id"],
                            "owner_account_id": envelope["owner_account_id"],
                            "actor_principal_id": envelope["actor_principal_id"],
                            "authenticated_session_id": envelope["authenticated_session_id"],
                            "access_space_id": envelope["access_space_id"],
                            "binding": publication_binding,
                            "envelope_sha256": sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(&envelope)),
                            "envelope": envelope,
                        });
                        worker_journal.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).retain(
                            RegistryPublicationAttempt { operation_id, fr_event_id, receipt: receipt.clone(),
                                previous_bytes: publication_previous_bytes })?;
                        worker_write_attempted.store(true, std::sync::atomic::Ordering::SeqCst);
                        fs::write(&publication_path, &draft_bytes).map_err(|source|
                            CapabilityRegistryWorkflowError::WriteFile { path: publication_path.clone(), source })?;
                        let observed = fs::read(&publication_path).map_err(|source|
                            CapabilityRegistryWorkflowError::ReadFile { path: publication_path.clone(), source })?;
                        if observed != draft_bytes { return Err(mismatch("asset_write_not_observed")); }
                        worker_receipt_attempted.store(true, std::sync::atomic::Ordering::SeqCst);
                        record_build_event(recorder.as_ref(), &publication_params, json!({
                            "event": "capability_registry_publish", "published_path": publication_path,
                            "capability_registry_version": publication_draft_sha, "diff_sha256": publication_diff_sha,
                            "approval": publication_approval, "normative_dependencies": publication_dependencies,
                            "asset_write_observed": true, "publication": receipt,
                        }), None).await?;
                        use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
                        let actor_id = receipt["envelope"]["actor_id"].as_str()
                            .ok_or_else(|| mismatch("publication_actor"))?.to_owned();
                        let actor = match receipt["envelope"]["actor_kind"].as_str() {
                            Some("operator") => KernelActor::Operator(actor_id),
                            Some("system") => KernelActor::System(actor_id),
                            Some("session_broker") => KernelActor::SessionBroker(actor_id),
                            Some("model_adapter") => KernelActor::ModelAdapter(actor_id),
                            Some("toolgate") => KernelActor::ToolGate(actor_id),
                            Some("validation_runner") => KernelActor::ValidationRunner(actor_id),
                            Some("promotion_gate") => KernelActor::PromotionGate(actor_id),
                            _ => return Err(mismatch("publication_actor")),
                        };
                        let event = NewKernelEvent::builder(publication_params.job_id.unwrap().to_string(),
                            receipt["envelope"]["authenticated_session_id"].as_str().unwrap().to_owned(),
                            KernelEventType::FlightRecorderMirrorRecorded,
                            actor)
                            .aggregate("capability_registry_publication", operation_id.to_string())
                            .idempotency_key(registry_receipt_key(operation_id))
                            .source_component("capability_registry_workflow")
                            .correlation_id(publication_params.trace_id.to_string())
                            .payload(receipt).build()
                            .map_err(|_| mismatch("publication_commit_witness"))?;
                        crate::storage::surreal::event_ledger::append(&receipt_storage, event).await
                            .map_err(|_| mismatch("publication_commit_witness"))?;
                        worker_receipt_recorded.store(true, std::sync::atomic::Ordering::SeqCst);
                        Ok(())
                    }).await
                }).await;
            if let Err(cause) = result {
                if !write_attempted.load(std::sync::atomic::Ordering::SeqCst) { return Err(cause); }
                // Callback/cancellation/failed commit cannot confirm publication. Once the
                // owner observes commit, later revocation stops access without undo.
                let settlement_pending = authority_storage.transaction_settlement_pending(operation_id);
                // Ambiguous commit/cancel observation cannot certify rejection. Leave the
                // asset and actual receipt intact for canonical resolution, never undo it.
                let owner = completion_journal.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                let matching_owner = owner.attempt.as_ref().is_some_and(|attempt| attempt.operation_id == operation_id);
                let owned_asset = fs::read(&completion_published_path).ok().map(|bytes| sha256_hex(&bytes));
                let expected_draft = owner.attempt.as_ref().and_then(|attempt| attempt.receipt["binding"]["draft_sha256"].as_str());
                let restored = !settlement_pending && matching_owner
                    && fs::canonicalize(&completion_published_path).ok().as_ref() == Some(&completion_published_path)
                    && owned_asset.as_deref() == expected_draft
                    && fs::write(&completion_published_path, &previous_bytes).is_ok()
                    && fs::read(&completion_published_path).ok().as_deref() == Some(previous_bytes.as_slice());
                return Err(CapabilityRegistryWorkflowError::PublicationUnconfirmed {
                    previous_asset_restored: restored,
                    operation_id, settlement_pending,
                    receipt_outcome: if receipt_recorded.load(std::sync::atomic::Ordering::SeqCst) { "record_call_succeeded" }
                        else if receipt_attempted.load(std::sync::atomic::Ordering::SeqCst) { "unknown" }
                        else { "not_attempted" },
                    observed_asset_sha256: fs::read(&completion_published_path).ok().map(|bytes| sha256_hex(&bytes)),
                    cause: Box::new(cause),
                });
            }
            Ok::<_, CapabilityRegistryWorkflowError>(())
        };
        let outcome = crate::storage::surreal::event_ledger::with_captured_account_authority(
            &completion_storage, completion_authority,
            APPROVED_REGISTRY_INPUT.scope(completion_input, work)).await;
        if let Err(error) = &outcome {
            tracing::error!(%operation_id, error = %error, "registry publication completion rejected or retained for canonical settlement");
        }
        outcome
    });
    let outcome = completion.await.map_err(|error| CapabilityRegistryWorkflowError::Storage(
        crate::storage::surreal::SurrealStorageError::TransactionWorker(error.to_string())));
    drop(caller);
    outcome??;
    artifacts.published = true;
    artifacts.publication_receipt_confirmed = true;
    Ok(artifacts)
}

fn build_diff(
    previous: Option<&CapabilityRegistryDocument>,
    next: &CapabilityRegistryDocument,
    next_bytes: &[u8],
    previous_sha256: Option<String>,
) -> CapabilityRegistryDiff {
    let next_sha256 = sha256_hex(next_bytes);
    let mut prev_ids = BTreeSet::new();
    let mut next_ids = BTreeSet::new();
    let mut prev_map: HashMap<&str, &CapabilityRegistryEntry> = HashMap::new();
    let mut next_map: HashMap<&str, &CapabilityRegistryEntry> = HashMap::new();

    if let Some(prev) = previous {
        for entry in &prev.entries {
            prev_ids.insert(entry.capability_id.clone());
            prev_map.insert(entry.capability_id.as_str(), entry);
        }
    }
    for entry in &next.entries {
        next_ids.insert(entry.capability_id.clone());
        next_map.insert(entry.capability_id.as_str(), entry);
    }

    let added: Vec<String> = next_ids.difference(&prev_ids).cloned().collect();
    let removed: Vec<String> = prev_ids.difference(&next_ids).cloned().collect();

    let mut changed = Vec::new();
    for id in next_ids.intersection(&prev_ids) {
        let prev = prev_map.get(id.as_str());
        let next = next_map.get(id.as_str());
        if prev != next {
            changed.push(id.clone());
        }
    }

    CapabilityRegistryDiff {
        previous_registry_sha256: previous_sha256,
        next_registry_sha256: next_sha256,
        added,
        removed,
        changed,
    }
}
