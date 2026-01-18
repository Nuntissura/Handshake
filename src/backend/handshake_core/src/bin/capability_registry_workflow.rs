use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{env, fs};

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::{
    duckdb::DuckDbFlightRecorder, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType,
};
use handshake_core::mex::registry::MexRegistry;
use jsonschema::{Draft, JSONSchema};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

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

fn repo_root_from_manifest_dir() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve repo root".to_string())
}

fn load_optional_registry(path: &Path) -> Result<Option<CapabilityRegistryDocument>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let doc: CapabilityRegistryDocument =
        serde_json::from_str(&data).map_err(|e| format!("parse {path:?}: {e}"))?;
    Ok(Some(doc))
}

fn load_schema() -> Result<Value, String> {
    let raw = include_str!("../../schemas/capability_registry.schema.json");
    serde_json::from_str(raw).map_err(|e| format!("parse embedded schema: {e}"))
}

fn validate_schema(schema: &Value, instance: &Value) -> Result<(), String> {
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|e| format!("compile schema: {e}"))?;

    let result = compiled.validate(instance);
    if let Err(errors) = result {
        let mut messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        messages.sort();
        return Err(format!(
            "schema validation errors:\n- {}",
            messages.join("\n- ")
        ));
    }
    Ok(())
}

fn validate_integrity(doc: &CapabilityRegistryDocument) -> Result<(), String> {
    let section_re = Regex::new(r"^\d+(\.\d+)*$")
        .map_err(|e| format!("section_ref regex compile failed: {e}"))?;

    let mut seen = HashSet::new();
    for entry in &doc.entries {
        if !seen.insert(entry.capability_id.as_str()) {
            return Err(format!("duplicate capability_id: {}", entry.capability_id));
        }
        if !section_re.is_match(entry.section_ref.as_str()) {
            return Err(format!(
                "invalid section_ref for {}: {}",
                entry.capability_id, entry.section_ref
            ));
        }
        if entry.display_name.trim().is_empty() {
            return Err(format!(
                "display_name must be non-empty for {}",
                entry.capability_id
            ));
        }
    }

    Ok(())
}

fn display_name_for(capability_id: &str) -> String {
    let mut s: String = capability_id
        .chars()
        .map(|c| match c {
            '.' | ':' | '_' => ' ',
            other => other,
        })
        .collect();
    s = s
        .split_whitespace()
        .map(|w| {
            if w.chars().all(|c| c.is_ascii_uppercase() || c == '-') {
                w.to_ascii_lowercase()
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut out = String::new();
    for (idx, word) in s.split_whitespace().enumerate() {
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

fn build_registry_document(repo_root: &Path) -> Result<CapabilityRegistryDocument, String> {
    let registry = CapabilityRegistry::new();
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
    let mex = MexRegistry::load_from_path(&engines_path)
        .map_err(|e| format!("load mechanical engines {:?}: {e}", engines_path))?;
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

fn write_json_file(path: &Path, value: &Value) -> Result<Vec<u8>, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| format!("json serialize: {e}"))?;
    fs::write(path, &bytes).map_err(|e| format!("write {path:?}: {e}"))?;
    Ok(bytes)
}

fn write_struct_file<T: Serialize>(path: &Path, value: &T) -> Result<Vec<u8>, String> {
    let json = serde_json::to_value(value).map_err(|e| format!("json serialize: {e}"))?;
    write_json_file(path, &json)
}

async fn init_flight_recorder(repo_root: &Path) -> Result<DuckDbFlightRecorder, String> {
    let data_dir = repo_root.join("data");
    fs::create_dir_all(&data_dir).map_err(|e| format!("mkdir {data_dir:?}: {e}"))?;
    let fr_db_path = data_dir.join("flight_recorder.db");
    DuckDbFlightRecorder::new_on_path(&fr_db_path, 7).map_err(|e| e.to_string())
}

async fn record_build_event(
    flight_recorder: &dyn FlightRecorder,
    trace_id: Uuid,
    payload: Value,
    model_id: Option<&str>,
    policy_decision_id: Option<&str>,
) -> Result<(), String> {
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        trace_id,
        payload,
    )
    .with_actor_id("capability_registry_build");

    if let Some(model_id) = model_id {
        event = event.with_model_id(model_id.to_string());
    }
    if let Some(policy_decision_id) = policy_decision_id {
        event = event.with_policy_decision(policy_decision_id.to_string());
    }

    flight_recorder
        .record_event(event)
        .await
        .map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    run().await
}

async fn run() -> Result<(), String> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("usage: capability_registry_workflow run --policy-decision-id <id> --reviewer-id <id> --approve".to_string());
    }
    let cmd = args.remove(0);
    if cmd != "run" {
        return Err(format!("unsupported command: {cmd} (supported: run)"));
    }

    let mut policy_decision_id: Option<String> = None;
    let mut reviewer_id: Option<String> = None;
    let mut model_id: Option<String> = None;
    let mut approve = false;
    while let Some(flag) = args.first().cloned() {
        args.remove(0);
        match flag.as_str() {
            "--policy-decision-id" => {
                policy_decision_id = args.get(0).cloned();
                if policy_decision_id.is_none() {
                    return Err("missing value for --policy-decision-id".to_string());
                }
                args.remove(0);
            }
            "--reviewer-id" => {
                reviewer_id = args.get(0).cloned();
                if reviewer_id.is_none() {
                    return Err("missing value for --reviewer-id".to_string());
                }
                args.remove(0);
            }
            "--model-id" => {
                model_id = args.get(0).cloned();
                if model_id.is_none() {
                    return Err("missing value for --model-id".to_string());
                }
                args.remove(0);
            }
            "--approve" => approve = true,
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let policy_decision_id = policy_decision_id.ok_or_else(|| {
        "missing required --policy-decision-id (spec 11.1.6 extract step)".to_string()
    })?;
    let model_id = model_id
        .or_else(|| env::var("OLLAMA_MODEL").ok())
        .unwrap_or_else(|| "llama3".to_string());

    let repo_root = repo_root_from_manifest_dir()?;
    let schema = load_schema()?;
    let flight_recorder = init_flight_recorder(&repo_root).await?;
    let trace_id = Uuid::new_v4();

    // Extract
    let doc = build_registry_document(&repo_root)?;
    let draft_path = repo_root.join("capability_registry_draft.json");
    let draft_bytes = write_struct_file(&draft_path, &doc)?;
    let draft_sha256 = sha256_hex(&draft_bytes);

    let prompt = format!(
        "TASK: Extract CapabilityRegistry entries into JSON matching capability_registry.schema.json.\nINPUTS: spec=Handshake_Master_Spec_v02.113.md, mechanical_engines.json, prior_registry=assets/capability_registry.json.\nOUTPUT: JSON object with 'entries' only."
    );
    let prompt_hashes = vec![sha256_hex(prompt.as_bytes())];

    record_build_event(
        &flight_recorder,
        trace_id,
        json!({
            "event": "capability_registry_extract",
            "draft_path": draft_path.to_string_lossy(),
            "draft_sha256": draft_sha256,
            "model_id": model_id.as_str(),
            "policy_decision_id": policy_decision_id.as_str(),
            "prompt_hashes": prompt_hashes,
        }),
        Some(model_id.as_str()),
        Some(&policy_decision_id),
    )
    .await?;

    // Validate (schema + integrity)
    let instance = serde_json::to_value(&doc).map_err(|e| format!("serialize draft: {e}"))?;
    validate_schema(&schema, &instance)?;
    validate_integrity(&doc)?;
    record_build_event(
        &flight_recorder,
        trace_id,
        json!({
            "event": "capability_registry_validate",
            "draft_sha256": draft_sha256,
            "schema": "src/backend/handshake_core/schemas/capability_registry.schema.json",
        }),
        None,
        Some(&policy_decision_id),
    )
    .await?;

    // Diff against previous published registry (optional)
    let published_path = repo_root.join("assets/capability_registry.json");
    let previous = load_optional_registry(&published_path)?;
    let previous_sha = previous.as_ref().map(|prev| {
        let bytes = serde_json::to_vec_pretty(prev).unwrap_or_default();
        sha256_hex(&bytes)
    });
    let diff = build_diff(previous.as_ref(), &doc, &draft_bytes, previous_sha.clone())?;
    let diff_path = repo_root.join("capability_registry_diff.json");
    let diff_bytes = write_struct_file(&diff_path, &diff)?;
    let diff_sha256 = sha256_hex(&diff_bytes);
    record_build_event(
        &flight_recorder,
        trace_id,
        json!({
            "event": "capability_registry_diff",
            "diff_path": diff_path.to_string_lossy(),
            "diff_sha256": diff_sha256,
            "previous_registry_sha256": previous_sha,
            "next_registry_sha256": diff.next_registry_sha256,
        }),
        None,
        Some(&policy_decision_id),
    )
    .await?;

    // Review gate
    let capability_registry_version = sha256_hex(&draft_bytes);
    let review = CapabilityRegistryReview {
        approved: approve,
        reviewer_id: reviewer_id.clone(),
        diff_sha256: diff_sha256.clone(),
        capability_registry_version: capability_registry_version.clone(),
    };
    let review_path = repo_root.join("capability_registry_review.json");
    let _review_bytes = write_struct_file(&review_path, &review)?;
    record_build_event(
        &flight_recorder,
        trace_id,
        json!({
            "event": "capability_registry_review",
            "review_path": review_path.to_string_lossy(),
            "approved": approve,
            "reviewer_id": reviewer_id,
            "diff_sha256": diff_sha256,
        }),
        None,
        Some(&policy_decision_id),
    )
    .await?;

    if !review.approved {
        return Ok(());
    }
    let reviewer_id = review.reviewer_id.as_deref().unwrap_or("");
    if reviewer_id.trim().is_empty() {
        return Err("review gate: approved requires reviewer_id".to_string());
    }

    // Publish (requires review approval + diff hash match)
    if review.diff_sha256
        != sha256_hex(&fs::read(&diff_path).map_err(|e| format!("read diff: {e}"))?)
    {
        return Err(
            "publish gate: diff_sha256 mismatch vs current capability_registry_diff.json"
                .to_string(),
        );
    }

    fs::create_dir_all(repo_root.join("assets")).map_err(|e| format!("mkdir assets: {e}"))?;
    fs::write(&published_path, &draft_bytes).map_err(|e| format!("write publish: {e}"))?;

    record_build_event(
        &flight_recorder,
        trace_id,
        json!({
            "event": "capability_registry_publish",
            "published_path": published_path.to_string_lossy(),
            "capability_registry_version": capability_registry_version,
            "diff_sha256": review.diff_sha256,
            "reviewer_id": review.reviewer_id,
        }),
        None,
        Some(&policy_decision_id),
    )
    .await?;

    Ok(())
}

fn build_diff(
    previous: Option<&CapabilityRegistryDocument>,
    next: &CapabilityRegistryDocument,
    next_bytes: &[u8],
    previous_sha256: Option<String>,
) -> Result<CapabilityRegistryDiff, String> {
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

    Ok(CapabilityRegistryDiff {
        previous_registry_sha256: previous_sha256,
        next_registry_sha256: next_sha256,
        added,
        removed,
        changed,
    })
}
