use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::{Draft, JSONSchema};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CapabilityKind {
    Surface,
    Engine,
    Runtime,
    Integration,
    Model,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GovernanceMode {
    GovStrict,
    GovStandard,
    GovLight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CapabilityRegistryEntry {
    capability_id: String,
    kind: CapabilityKind,
    display_name: String,
    section_ref: String,
    required_capabilities: Vec<String>,
    default_governance_mode: GovernanceMode,
    risk_class: RiskClass,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CapabilityRegistryDocument {
    entries: Vec<CapabilityRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CapabilityRegistryDiff {
    previous_registry_sha256: Option<String>,
    next_registry_sha256: String,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CapabilityRegistryReview {
    approved: bool,
    reviewer_id: Option<String>,
    diff_sha256: String,
    capability_registry_version: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

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

fn validate_schema(schema: &Value, instance: &Value) -> CapabilityRegistryWorkflowResult<()> {
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|source| CapabilityRegistryWorkflowError::SchemaCompile {
            message: source.to_string(),
        })?;

    let result = compiled.validate(instance);
    if let Err(errors) = result {
        let mut messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        messages.sort();
        let details = messages
            .into_iter()
            .map(|m| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(CapabilityRegistryWorkflowError::SchemaValidationFailed { details });
    }
    Ok(())
}

fn validate_integrity(doc: &CapabilityRegistryDocument) -> CapabilityRegistryWorkflowResult<()> {
    let section_re = Regex::new(r"^\d+(\.\d+)*$").map_err(|source| {
        CapabilityRegistryWorkflowError::RegexCompile {
            message: source.to_string(),
        }
    })?;

    let mut seen = HashSet::new();
    for entry in &doc.entries {
        if !seen.insert(entry.capability_id.as_str()) {
            return Err(CapabilityRegistryWorkflowError::DuplicateCapabilityId {
                capability_id: entry.capability_id.clone(),
            });
        }
        if !section_re.is_match(entry.section_ref.as_str()) {
            return Err(CapabilityRegistryWorkflowError::InvalidSectionRef {
                capability_id: entry.capability_id.clone(),
                section_ref: entry.section_ref.clone(),
            });
        }
        if entry.display_name.trim().is_empty() {
            return Err(CapabilityRegistryWorkflowError::EmptyDisplayName {
                capability_id: entry.capability_id.clone(),
            });
        }
    }

    Ok(())
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

async fn record_build_event(
    flight_recorder: &dyn FlightRecorder,
    params: &CapabilityRegistryWorkflowParams,
    payload: Value,
    model_id: Option<&str>,
) -> CapabilityRegistryWorkflowResult<()> {
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        params.trace_id,
        payload,
    )
    .with_actor_id(REGISTRY_WORKFLOW_ACTOR_ID);

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

    let schema = load_schema()?;

    // Extract
    let doc = build_registry_document(registry, repo_root)?;
    let draft_path = repo_root.join("capability_registry_draft.json");
    let draft_bytes = write_struct_file(&draft_path, &doc)?;
    let draft_sha256 = sha256_hex(&draft_bytes);

    let prompt = "TASK: Extract CapabilityRegistry entries into JSON matching capability_registry.schema.json.\nINPUTS: spec=Handshake_Master_Spec_v02.113.md, mechanical_engines.json, prior_registry=assets/capability_registry.json.\nOUTPUT: JSON object with 'entries' only.";
    let prompt_hashes = vec![sha256_hex(prompt.as_bytes())];

    record_build_event(
        flight_recorder,
        &params,
        json!({
            "event": "capability_registry_extract",
            "draft_path": draft_path.to_string_lossy(),
            "draft_sha256": draft_sha256,
            "model_id": params.model_id.as_str(),
            "policy_decision_id": params.policy_decision_id.as_str(),
            "prompt_hashes": prompt_hashes,
        }),
        Some(params.model_id.as_str()),
    )
    .await?;

    // Validate (schema + integrity)
    let instance = serde_json::to_value(&doc)
        .map_err(|source| CapabilityRegistryWorkflowError::JsonSerialize { source })?;
    validate_schema(&schema, &instance)?;
    validate_integrity(&doc)?;
    record_build_event(
        flight_recorder,
        &params,
        json!({
            "event": "capability_registry_validate",
            "draft_sha256": draft_sha256,
            "schema": "src/backend/handshake_core/schemas/capability_registry.schema.json",
        }),
        None,
    )
    .await?;

    // Diff against previous published registry (optional)
    let published_path = repo_root.join("assets/capability_registry.json");
    let previous = load_optional_registry(&published_path)?;
    let previous_sha = match previous.as_ref() {
        Some(prev) => {
            let bytes = serde_json::to_vec_pretty(prev)
                .map_err(|source| CapabilityRegistryWorkflowError::JsonSerialize { source })?;
            Some(sha256_hex(&bytes))
        }
        None => None,
    };
    let diff = build_diff(previous.as_ref(), &doc, &draft_bytes, previous_sha.clone());
    let diff_path = repo_root.join("capability_registry_diff.json");
    let diff_bytes = write_struct_file(&diff_path, &diff)?;
    let diff_sha256 = sha256_hex(&diff_bytes);
    record_build_event(
        flight_recorder,
        &params,
        json!({
            "event": "capability_registry_diff",
            "diff_path": diff_path.to_string_lossy(),
            "diff_sha256": diff_sha256,
            "previous_registry_sha256": previous_sha,
            "next_registry_sha256": diff.next_registry_sha256,
        }),
        None,
    )
    .await?;

    // Review gate
    let capability_registry_version = sha256_hex(&draft_bytes);
    let review = CapabilityRegistryReview {
        approved: params.approve,
        reviewer_id: params.reviewer_id.clone(),
        diff_sha256: diff_sha256.clone(),
        capability_registry_version: capability_registry_version.clone(),
    };
    let review_path = repo_root.join("capability_registry_review.json");
    write_struct_file(&review_path, &review)?;
    record_build_event(
        flight_recorder,
        &params,
        json!({
            "event": "capability_registry_review",
            "review_path": review_path.to_string_lossy(),
            "approved": params.approve,
            "reviewer_id": params.reviewer_id,
            "diff_sha256": diff_sha256,
        }),
        None,
    )
    .await?;

    let artifacts = CapabilityRegistryWorkflowArtifacts {
        draft_path,
        diff_path: diff_path.clone(),
        review_path,
        published_path: published_path.clone(),
        draft_sha256,
        diff_sha256: diff_sha256.clone(),
        capability_registry_version: capability_registry_version.clone(),
    };

    if !review.approved {
        return Ok(artifacts);
    }
    let reviewer_id = review.reviewer_id.as_deref().unwrap_or("");
    if reviewer_id.trim().is_empty() {
        return Err(CapabilityRegistryWorkflowError::ReviewerIdRequired);
    }

    // Publish (requires review approval + diff hash match)
    let current_diff_bytes =
        fs::read(&diff_path).map_err(|source| CapabilityRegistryWorkflowError::ReadFile {
            path: diff_path.clone(),
            source,
        })?;
    if review.diff_sha256 != sha256_hex(&current_diff_bytes) {
        return Err(CapabilityRegistryWorkflowError::PublishDiffShaMismatch);
    }

    let assets_dir = repo_root.join("assets");
    fs::create_dir_all(&assets_dir).map_err(|source| {
        CapabilityRegistryWorkflowError::CreateDir {
            path: assets_dir.clone(),
            source,
        }
    })?;
    fs::write(&published_path, &draft_bytes).map_err(|source| {
        CapabilityRegistryWorkflowError::WriteFile {
            path: published_path.clone(),
            source,
        }
    })?;

    record_build_event(
        flight_recorder,
        &params,
        json!({
            "event": "capability_registry_publish",
            "published_path": published_path.to_string_lossy(),
            "capability_registry_version": capability_registry_version,
            "diff_sha256": review.diff_sha256,
            "reviewer_id": review.reviewer_id,
        }),
        None,
    )
    .await?;

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
