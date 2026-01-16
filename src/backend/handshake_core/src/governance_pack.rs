use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::storage::EntityRef;

const TEMPLATE_VOLUME_BEGIN: &str = "<!-- GOV_PACK_TEMPLATE_VOLUME_BEGIN -->";
const TEMPLATE_VOLUME_END: &str = "<!-- GOV_PACK_TEMPLATE_VOLUME_END -->";

const SPEC_CURRENT_BOLD_PATTERN: &str = r"\*\*([^*\r\n]+)\*\*";
const PLACEHOLDER_PATTERN: &str = r"\{\{([A-Z0-9_]+)\}\}";

static SPEC_CURRENT_BOLD_RE: Lazy<Result<Regex, regex::Error>> =
    Lazy::new(|| Regex::new(SPEC_CURRENT_BOLD_PATTERN));
static PLACEHOLDER_RE: Lazy<Result<Regex, regex::Error>> =
    Lazy::new(|| Regex::new(PLACEHOLDER_PATTERN));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePackExportRequest {
    pub export_target: ExportTarget,
    #[serde(default)]
    pub overwrite: bool,
    pub invariants: GovernancePackInvariants,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePackInvariants {
    pub project_code: String,
    pub project_display_name: String,
    #[serde(default)]
    pub project_prefix: Option<String>,
    pub issue_prefix: String,
    pub language_layout_profile_id: String,
    pub frontend_root_dir: String,
    pub frontend_src_dir: String,
    pub backend_root_dir: String,
    pub backend_crate_name: String,
    #[serde(default)]
    pub codex_version: Option<String>,
    #[serde(default)]
    pub master_spec_filename: Option<String>,
    #[serde(default)]
    pub cargo_target_dir_name: Option<String>,
    #[serde(default)]
    pub node_package_manager: Option<String>,
    #[serde(default)]
    pub default_base_branch: Option<String>,
    #[serde(default)]
    pub additional_placeholders: HashMap<String, String>,
}

/// ExportTarget (Spec §2.3.2) - v1 supports LocalFile only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportTarget {
    LocalFile { path: PathBuf },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismLevel {
    Bitwise,
    Structural,
    BestEffort,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportActor {
    HumanDev,
    AiJob,
    PluginTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterInfo {
    pub engine_id: String,
    pub engine_version: String,
    pub config_hash: String,
}

/// ExportRecord (Spec §2.3.10.2 normative minimum).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub export_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub actor: ExportActor,
    #[serde(default)]
    pub job_id: Option<Uuid>,
    pub source_entity_refs: Vec<EntityRef>,
    pub source_hashes: Vec<String>,
    #[serde(default)]
    pub display_projection_ref: Option<serde_json::Value>,
    pub export_format: String,
    pub exporter: ExporterInfo,
    pub determinism_level: DeterminismLevel,
    pub export_target: ExportTarget,
    pub policy_id: String,
    pub redactions_applied: bool,
    pub output_artifact_handles: Vec<ArtifactHandle>,
    pub materialized_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GovernancePackTemplate {
    pub rel_path: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct GovernancePackExportOutcome {
    pub export_record: ExportRecord,
    pub templates_written: usize,
}

#[derive(Debug, Error)]
pub enum GovernancePackExportError {
    #[error("failed to read docs/SPEC_CURRENT.md: {0}")]
    SpecCurrentRead(#[source] io::Error),
    #[error("docs/SPEC_CURRENT.md did not contain a master spec filename")]
    SpecCurrentParse,
    #[error("regex compile failed for {pattern}: {message}")]
    RegexCompile {
        pattern: &'static str,
        message: String,
    },
    #[error("regex capture missing group {group} for {pattern}")]
    RegexCaptureMissing { pattern: &'static str, group: usize },
    #[error("failed to read master spec at {path}: {source}")]
    MasterSpecRead { path: PathBuf, source: io::Error },
    #[error("master spec is missing template volume markers")]
    TemplateVolumeMarkersMissing,
    #[error("template parse error: {0}")]
    TemplateParse(String),
    #[error("missing placeholder token {token} in template {template_file}")]
    MissingPlaceholder {
        token: String,
        template_file: String,
    },
    #[error("invalid template path {template_file}: {reason}")]
    InvalidTemplatePath {
        template_file: String,
        reason: String,
    },
    #[error("invalid placeholder value for {token}: {reason}")]
    InvalidPlaceholderValue { token: String, reason: String },
    #[error("export target must be an absolute directory path")]
    ExportTargetNotAbsolute,
    #[error("export target exists and is not a directory")]
    ExportTargetNotDirectory,
    #[error("export directory is non-empty (overwrite=false)")]
    ExportDirNotEmpty,
    #[error("path traversal blocked: {path}")]
    PathTraversalBlocked { path: String },
    #[error("write blocked: target escapes export root: {path}")]
    ExportRootEscape { path: String },
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub fn export_governance_pack(
    request: &GovernancePackExportRequest,
    job_id: Option<Uuid>,
) -> Result<GovernancePackExportOutcome, GovernancePackExportError> {
    let master_spec_filename = match request.invariants.master_spec_filename.clone() {
        Some(filename) => filename,
        None => resolve_master_spec_filename()?,
    };
    let codex_version = request
        .invariants
        .codex_version
        .clone()
        .unwrap_or_else(|| "1.4".to_string());

    let spec_path = PathBuf::from(master_spec_filename.trim());
    let spec_text = fs::read_to_string(&spec_path).map_err(|source| {
        GovernancePackExportError::MasterSpecRead {
            path: spec_path.clone(),
            source,
        }
    })?;

    let templates = extract_template_volume(&spec_text)?;

    let export_root = match &request.export_target {
        ExportTarget::LocalFile { path } => path.clone(),
    };
    if !export_root.is_absolute() {
        return Err(GovernancePackExportError::ExportTargetNotAbsolute);
    }

    prepare_export_dir(&export_root, request.overwrite)?;
    let export_root = fs::canonicalize(&export_root)?;

    let placeholders = build_placeholder_map(
        &request.invariants,
        &export_root,
        &codex_version,
        &master_spec_filename,
        &spec_path,
        &spec_text,
    )?;

    let mut rendered: Vec<(String, String)> = Vec::with_capacity(templates.len());
    for template in templates {
        let template_path =
            substitute_placeholders(&template.rel_path, &placeholders, &template.rel_path)?;
        let body = substitute_placeholders(&template.body, &placeholders, &template_path)?;
        ensure_no_placeholders_remain(&body, &template_path)?;
        ensure_safe_rel_path(&template_path)?;
        rendered.push((template_path, body));
    }

    // Deterministic write order: resolved rel_path lexicographic.
    rendered.sort_by(|a, b| a.0.cmp(&b.0));

    let mut materialized_paths: Vec<String> = Vec::with_capacity(rendered.len());
    for (rel_path, body) in &rendered {
        let target_path = export_root.join(Path::new(rel_path));
        write_template_file_atomic(
            &export_root,
            &target_path,
            body.as_bytes(),
            request.overwrite,
        )?;
        materialized_paths.push(normalize_rel_path(rel_path));
    }
    materialized_paths.sort();

    let export_id = Uuid::new_v4();
    let created_at = Utc::now();

    let (spec_volume_sha256, invariants_sha256, config_hash) =
        compute_hashes(&spec_text, &placeholders);

    let exporter = ExporterInfo {
        engine_id: "handshake.governance_pack_export".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        config_hash,
    };

    let export_record = ExportRecord {
        export_id,
        created_at,
        actor: ExportActor::AiJob,
        job_id,
        source_entity_refs: vec![EntityRef {
            entity_id: spec_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| spec_path.to_string_lossy().to_string()),
            entity_kind: "master_spec".to_string(),
        }],
        source_hashes: vec![spec_volume_sha256, invariants_sha256],
        display_projection_ref: None,
        export_format: "governance_pack_template_volume".to_string(),
        exporter,
        determinism_level: DeterminismLevel::Bitwise,
        export_target: request.export_target.clone(),
        policy_id: "SAFE_DEFAULT".to_string(),
        redactions_applied: false,
        output_artifact_handles: vec![ArtifactHandle::new(
            export_id,
            "gov_pack_template_volume".to_string(),
        )],
        materialized_paths,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    Ok(GovernancePackExportOutcome {
        export_record,
        templates_written: rendered.len(),
    })
}

fn resolve_master_spec_filename() -> Result<String, GovernancePackExportError> {
    let spec_current = fs::read_to_string("docs/SPEC_CURRENT.md")
        .map_err(GovernancePackExportError::SpecCurrentRead)?;
    let re = spec_current_bold_re()?;
    let mut matches = re.captures_iter(&spec_current);
    matches
        .next()
        .and_then(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|s| !s.is_empty())
        .ok_or(GovernancePackExportError::SpecCurrentParse)
}

fn extract_template_volume(
    spec_text: &str,
) -> Result<Vec<GovernancePackTemplate>, GovernancePackExportError> {
    let begin = spec_text
        .find(TEMPLATE_VOLUME_BEGIN)
        .ok_or(GovernancePackExportError::TemplateVolumeMarkersMissing)?;
    let end = spec_text
        .find(TEMPLATE_VOLUME_END)
        .ok_or(GovernancePackExportError::TemplateVolumeMarkersMissing)?;
    if end <= begin {
        return Err(GovernancePackExportError::TemplateVolumeMarkersMissing);
    }

    let volume = &spec_text[begin..end];
    let mut templates: Vec<GovernancePackTemplate> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let lines: Vec<&str> = volume.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if let Some(path) = parse_template_header_line(line) {
            // Find the next opening ```` fence.
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim_start().starts_with("````") {
                j += 1;
            }
            if j >= lines.len() {
                return Err(GovernancePackExportError::TemplateParse(format!(
                    "missing code fence for template {path}"
                )));
            }
            // Consume until closing fence line == ````.
            let mut k = j + 1;
            let mut body_lines: Vec<&str> = Vec::new();
            while k < lines.len() {
                if lines[k].trim() == "````" {
                    break;
                }
                body_lines.push(lines[k]);
                k += 1;
            }
            if k >= lines.len() {
                return Err(GovernancePackExportError::TemplateParse(format!(
                    "unterminated code fence for template {path}"
                )));
            }

            if !seen.insert(path.clone()) {
                return Err(GovernancePackExportError::TemplateParse(format!(
                    "duplicate template path: {path}"
                )));
            }

            let body = normalize_lf(&body_lines.join("\n"));
            let body = ensure_trailing_newline(&body);
            templates.push(GovernancePackTemplate {
                rel_path: path,
                body,
            });
            i = k + 1;
            continue;
        }
        i += 1;
    }

    Ok(templates)
}

fn parse_template_header_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = "###### Template File: `";
    if !trimmed.starts_with(prefix) || !trimmed.ends_with('`') {
        return None;
    }
    let inner = &trimmed[prefix.len()..trimmed.len() - 1];
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }
    Some(inner.to_string())
}

fn prepare_export_dir(path: &Path, overwrite: bool) -> Result<(), GovernancePackExportError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(GovernancePackExportError::ExportTargetNotDirectory);
        }
        if !overwrite && dir_is_non_empty(path)? {
            return Err(GovernancePackExportError::ExportDirNotEmpty);
        }
        return Ok(());
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn dir_is_non_empty(path: &Path) -> io::Result<bool> {
    let mut it = fs::read_dir(path)?;
    Ok(it.next().is_some())
}

fn build_placeholder_map(
    invariants: &GovernancePackInvariants,
    export_root: &Path,
    codex_version: &str,
    master_spec_filename: &str,
    spec_path: &Path,
    spec_text: &str,
) -> Result<BTreeMap<String, String>, GovernancePackExportError> {
    let project_code = invariants.project_code.trim().to_string();
    let project_display_name = invariants.project_display_name.trim().to_string();
    let issue_prefix = invariants.issue_prefix.trim().to_string();
    let language_layout_profile_id = invariants.language_layout_profile_id.trim().to_string();
    let project_prefix = invariants
        .project_prefix
        .as_deref()
        .unwrap_or(&project_code)
        .trim()
        .to_string();

    validate_placeholder_value("PROJECT_CODE", &project_code)?;
    validate_placeholder_value("PROJECT_DISPLAY_NAME", &project_display_name)?;
    validate_placeholder_value("ISSUE_PREFIX", &issue_prefix)?;
    validate_placeholder_value("LANGUAGE_LAYOUT_PROFILE_ID", &language_layout_profile_id)?;
    validate_placeholder_value("PROJECT_PREFIX", &project_prefix)?;

    let frontend_root_dir = normalize_rel_dir(&invariants.frontend_root_dir);
    let frontend_src_dir = normalize_rel_dir(&invariants.frontend_src_dir);
    let backend_root_dir = normalize_rel_dir(&invariants.backend_root_dir);
    let backend_crate_name = normalize_rel_dir(&invariants.backend_crate_name);

    validate_placeholder_value("FRONTEND_ROOT_DIR", &frontend_root_dir)?;
    validate_placeholder_value("FRONTEND_SRC_DIR", &frontend_src_dir)?;
    validate_placeholder_value("BACKEND_ROOT_DIR", &backend_root_dir)?;
    validate_placeholder_value("BACKEND_CRATE_NAME", &backend_crate_name)?;

    let backend_crate_dir = format!("{backend_root_dir}/{backend_crate_name}");
    let backend_src_dir = format!("{backend_crate_dir}/src");
    let backend_tests_dir = format!("{backend_crate_dir}/tests");
    let backend_migrations_dir = format!("{backend_crate_dir}/migrations");
    let backend_cargo_toml = format!("{backend_crate_dir}/Cargo.toml");
    let backend_jobs_dir = format!("{backend_src_dir}/jobs");
    let backend_llm_dir = format!("{backend_src_dir}/llm");
    let backend_storage_dir = format!("{backend_src_dir}/storage");
    let backend_observability_dir = format!("{backend_src_dir}/observability");
    let backend_api_dir = format!("{backend_src_dir}/api");
    let backend_local_models_dir = format!("{backend_src_dir}/local_models");
    let backend_pipeline_dir = format!("{backend_src_dir}/content_pipeline");
    let backend_util_dir = format!("{backend_src_dir}/util");

    let cargo_target_dir_name = invariants
        .cargo_target_dir_name
        .clone()
        .unwrap_or_else(|| format!("{project_prefix}-cargo-target"));
    let cargo_target_dir = format!("../{cargo_target_dir_name}");

    let postgres_test_db = format!("{project_prefix}_test");
    let codex_filename = format!("{project_prefix}_Codex_v{codex_version}.md");

    let default_base_branch = invariants
        .default_base_branch
        .clone()
        .unwrap_or_else(|| "main".to_string());

    let operator_worktree_dir = export_root.to_string_lossy().to_string();
    let operator_branch = default_base_branch.clone();

    let export_root_parent = export_root.parent().unwrap_or(export_root);
    let orchestrator_worktree_dir = export_root_parent
        .join("wt-orchestrator")
        .to_string_lossy()
        .to_string();
    let validator_worktree_dir = export_root_parent
        .join("wt-validator")
        .to_string_lossy()
        .to_string();
    let orchestrator_branch = "user_orchestrator".to_string();
    let validator_branch = "user_validator".to_string();

    let spec_target_resolved = format!("docs/SPEC_CURRENT.md -> {}", master_spec_filename.trim());
    let spec_target_sha1 = sha1_hex(spec_text.as_bytes());

    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("PROJECT_CODE".to_string(), project_code);
    map.insert("PROJECT_DISPLAY_NAME".to_string(), project_display_name);
    map.insert("PROJECT_PREFIX".to_string(), project_prefix);
    map.insert("CODEX_VERSION".to_string(), codex_version.to_string());
    map.insert("CODEX_FILENAME".to_string(), codex_filename);
    map.insert("ISSUE_PREFIX".to_string(), issue_prefix);
    map.insert(
        "MASTER_SPEC_FILENAME".to_string(),
        master_spec_filename.trim().to_string(),
    );
    map.insert(
        "LANGUAGE_LAYOUT_PROFILE_ID".to_string(),
        language_layout_profile_id,
    );
    map.insert("FRONTEND_ROOT_DIR".to_string(), frontend_root_dir);
    map.insert("FRONTEND_SRC_DIR".to_string(), frontend_src_dir);
    map.insert("BACKEND_ROOT_DIR".to_string(), backend_root_dir);
    map.insert("BACKEND_CRATE_NAME".to_string(), backend_crate_name);
    map.insert("BACKEND_CRATE_DIR".to_string(), backend_crate_dir);
    map.insert("BACKEND_SRC_DIR".to_string(), backend_src_dir);
    map.insert("BACKEND_TESTS_DIR".to_string(), backend_tests_dir);
    map.insert("BACKEND_MIGRATIONS_DIR".to_string(), backend_migrations_dir);
    map.insert("BACKEND_CARGO_TOML".to_string(), backend_cargo_toml);
    map.insert("BACKEND_JOBS_DIR".to_string(), backend_jobs_dir);
    map.insert("BACKEND_LLM_DIR".to_string(), backend_llm_dir);
    map.insert("BACKEND_STORAGE_DIR".to_string(), backend_storage_dir);
    map.insert(
        "BACKEND_OBSERVABILITY_DIR".to_string(),
        backend_observability_dir,
    );
    map.insert("BACKEND_API_DIR".to_string(), backend_api_dir);
    map.insert(
        "BACKEND_LOCAL_MODELS_DIR".to_string(),
        backend_local_models_dir,
    );
    map.insert("BACKEND_PIPELINE_DIR".to_string(), backend_pipeline_dir);
    map.insert("BACKEND_UTIL_DIR".to_string(), backend_util_dir);
    map.insert("CARGO_TARGET_DIR_NAME".to_string(), cargo_target_dir_name);
    map.insert("CARGO_TARGET_DIR".to_string(), cargo_target_dir);
    map.insert("POSTGRES_TEST_DB".to_string(), postgres_test_db);
    map.insert(
        "NODE_PACKAGE_MANAGER".to_string(),
        invariants
            .node_package_manager
            .clone()
            .unwrap_or_else(|| "pnpm".to_string()),
    );
    map.insert("DEFAULT_BASE_BRANCH".to_string(), default_base_branch);
    map.insert("OPERATOR_WORKTREE_DIR".to_string(), operator_worktree_dir);
    map.insert("OPERATOR_BRANCH".to_string(), operator_branch);
    map.insert(
        "ORCHESTRATOR_WORKTREE_DIR".to_string(),
        orchestrator_worktree_dir,
    );
    map.insert("ORCHESTRATOR_BRANCH".to_string(), orchestrator_branch);
    map.insert("VALIDATOR_WORKTREE_DIR".to_string(), validator_worktree_dir);
    map.insert("VALIDATOR_BRANCH".to_string(), validator_branch);

    // Template-volume placeholders used inside templates-as-templates.
    map.insert("WP_ID".to_string(), "WP-{ID}".to_string());
    map.insert("DATE_ISO".to_string(), "YYYY-MM-DDTHH:MM:SSZ".to_string());
    map.insert("REQUESTOR".to_string(), "Operator".to_string());
    map.insert("AGENT_ID".to_string(), "CodexCLI".to_string());
    map.insert("USER_SIGNATURE".to_string(), "<pending>".to_string());
    map.insert(
        "SPEC_BASELINE".to_string(),
        master_spec_filename.trim().to_string(),
    );
    map.insert("SPEC_TARGET_RESOLVED".to_string(), spec_target_resolved);
    map.insert("SPEC_TARGET_SHA1".to_string(), spec_target_sha1);
    map.insert("SPEC_ANCHOR".to_string(), "<fill>".to_string());
    map.insert("SPEC_ANCHOR_1".to_string(), "<fill>".to_string());
    map.insert("SPEC_ANCHOR_2".to_string(), "<fill>".to_string());
    map.insert(
        "ROADMAP_POINTER".to_string(),
        "<ROADMAP_POINTER>".to_string(),
    );

    // OSS register "example row" placeholders.
    map.insert(
        "FRONTEND_DEP_NAME".to_string(),
        "<FRONTEND_DEP_NAME>".to_string(),
    );
    map.insert(
        "BACKEND_DEP_NAME".to_string(),
        "<BACKEND_DEP_NAME>".to_string(),
    );
    map.insert(
        "FRONTEND_TRANSITIVE_DEP_NAME".to_string(),
        "<FRONTEND_TRANSITIVE_DEP_NAME>".to_string(),
    );
    map.insert(
        "BACKEND_TRANSITIVE_DEP_NAME".to_string(),
        "<BACKEND_TRANSITIVE_DEP_NAME>".to_string(),
    );
    map.insert("LICENSE".to_string(), "<LICENSE>".to_string());
    map.insert("PURPOSE".to_string(), "<PURPOSE>".to_string());

    // Allow explicit override/injection for any placeholder.
    for (k, v) in &invariants.additional_placeholders {
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        validate_placeholder_value(key, v)?;
        map.insert(key.to_string(), v.trim().to_string());
    }

    // Ensure all placeholder tokens present in template volume are resolved.
    let required_tokens = scan_template_volume_tokens(spec_text)?;
    for token in required_tokens {
        if !map.contains_key(&token) {
            return Err(GovernancePackExportError::MissingPlaceholder {
                token,
                template_file: spec_path.to_string_lossy().to_string(),
            });
        }
    }

    Ok(map)
}

fn scan_template_volume_tokens(
    spec_text: &str,
) -> Result<HashSet<String>, GovernancePackExportError> {
    let begin = match spec_text.find(TEMPLATE_VOLUME_BEGIN) {
        Some(pos) => pos,
        None => return Ok(HashSet::new()),
    };
    let end = match spec_text.find(TEMPLATE_VOLUME_END) {
        Some(pos) => pos,
        None => return Ok(HashSet::new()),
    };
    if end <= begin {
        return Ok(HashSet::new());
    }
    let volume = &spec_text[begin..end];
    let re = placeholder_re()?;
    Ok(re
        .captures_iter(volume)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn validate_placeholder_value(token: &str, value: &str) -> Result<(), GovernancePackExportError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GovernancePackExportError::InvalidPlaceholderValue {
            token: token.to_string(),
            reason: "value must be non-empty".to_string(),
        });
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(GovernancePackExportError::InvalidPlaceholderValue {
            token: token.to_string(),
            reason: "value must be single-line".to_string(),
        });
    }
    Ok(())
}

fn normalize_rel_dir(input: &str) -> String {
    input
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

fn normalize_lf(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

fn ensure_trailing_newline(input: &str) -> String {
    if input.ends_with('\n') {
        return input.to_string();
    }
    format!("{input}\n")
}

fn substitute_placeholders(
    text: &str,
    placeholders: &BTreeMap<String, String>,
    template_file: &str,
) -> Result<String, GovernancePackExportError> {
    let re = placeholder_re()?;
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for cap in re.captures_iter(text) {
        let m = cap
            .get(0)
            .ok_or(GovernancePackExportError::RegexCaptureMissing {
                pattern: PLACEHOLDER_PATTERN,
                group: 0,
            })?;
        let token = cap
            .get(1)
            .ok_or(GovernancePackExportError::RegexCaptureMissing {
                pattern: PLACEHOLDER_PATTERN,
                group: 1,
            })?
            .as_str();
        out.push_str(&text[last..m.start()]);
        let value = placeholders.get(token).ok_or_else(|| {
            GovernancePackExportError::MissingPlaceholder {
                token: token.to_string(),
                template_file: template_file.to_string(),
            }
        })?;
        out.push_str(value);
        last = m.end();
    }
    out.push_str(&text[last..]);
    Ok(normalize_lf(&out))
}

fn ensure_no_placeholders_remain(
    rendered_body: &str,
    template_file: &str,
) -> Result<(), GovernancePackExportError> {
    let re = placeholder_re()?;
    if let Some(cap) = re.captures(rendered_body) {
        let token = cap.get(1).map(|m| m.as_str()).unwrap_or("UNKNOWN");
        return Err(GovernancePackExportError::MissingPlaceholder {
            token: token.to_string(),
            template_file: template_file.to_string(),
        });
    }
    Ok(())
}

fn ensure_safe_rel_path(rel_path: &str) -> Result<(), GovernancePackExportError> {
    let path = Path::new(rel_path);
    if path.is_absolute() {
        return Err(GovernancePackExportError::PathTraversalBlocked {
            path: rel_path.to_string(),
        });
    }
    for component in path.components() {
        match component {
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(GovernancePackExportError::PathTraversalBlocked {
                    path: rel_path.to_string(),
                })
            }
            _ => {}
        }
    }
    Ok(())
}

fn normalize_rel_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn write_template_file_atomic(
    export_root: &Path,
    target_path: &Path,
    bytes: &[u8],
    overwrite: bool,
) -> Result<(), GovernancePackExportError> {
    let parent =
        target_path
            .parent()
            .ok_or_else(|| GovernancePackExportError::InvalidTemplatePath {
                template_file: target_path.to_string_lossy().to_string(),
                reason: "missing parent directory".to_string(),
            })?;

    fs::create_dir_all(parent)?;
    let export_root = fs::canonicalize(export_root)?;
    let parent_canon = fs::canonicalize(parent)?;
    if !parent_canon.starts_with(&export_root) {
        return Err(GovernancePackExportError::ExportRootEscape {
            path: target_path.to_string_lossy().to_string(),
        });
    }

    // Atomic materialize: temp + fsync + rename (+ best-effort dir fsync).
    let tmp_path = parent_canon.join(format!(".hsk_tmp_{}", Uuid::new_v4()));
    let mut tmp_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    tmp_file.write_all(bytes)?;
    tmp_file.sync_all()?;
    drop(tmp_file);

    if !overwrite {
        if target_path.exists() {
            let _ = fs::remove_file(&tmp_path);
            return Err(GovernancePackExportError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "target already exists (overwrite=false)",
            )));
        }
        if let Err(err) = fs::rename(&tmp_path, target_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(GovernancePackExportError::Io(err));
        }
    } else {
        if target_path.exists() && target_path.is_dir() {
            let _ = fs::remove_file(&tmp_path);
            return Err(GovernancePackExportError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "target path exists and is a directory",
            )));
        }
        if target_path.exists() {
            fs::remove_file(target_path)?;
        }
        if let Err(err) = fs::rename(&tmp_path, target_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(GovernancePackExportError::Io(err));
        }
    }

    if let Ok(dir_handle) = fs::File::open(&parent_canon) {
        let _ = dir_handle.sync_all();
    }

    Ok(())
}

fn compute_hashes(
    spec_text: &str,
    placeholders: &BTreeMap<String, String>,
) -> (String, String, String) {
    let volume_sha256 = sha256_hex(template_volume_slice(spec_text).as_bytes());
    let mut invariants_hasher = Sha256::new();
    for (k, v) in placeholders {
        invariants_hasher.update(k.as_bytes());
        invariants_hasher.update(b"=");
        invariants_hasher.update(v.as_bytes());
        invariants_hasher.update(b"\n");
    }
    let invariants_sha256 = hex::encode(invariants_hasher.finalize());

    let mut config_hasher = Sha256::new();
    config_hasher.update(b"handshake.governance_pack_export.v1\n");
    config_hasher.update(volume_sha256.as_bytes());
    config_hasher.update(b"\n");
    config_hasher.update(invariants_sha256.as_bytes());
    let config_hash = hex::encode(config_hasher.finalize());

    (volume_sha256, invariants_sha256, config_hash)
}

fn template_volume_slice(spec_text: &str) -> String {
    let begin = spec_text.find(TEMPLATE_VOLUME_BEGIN).unwrap_or(0);
    let end = spec_text
        .find(TEMPLATE_VOLUME_END)
        .unwrap_or(spec_text.len());
    if end <= begin {
        return String::new();
    }
    spec_text[begin..end].to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn spec_current_bold_re() -> Result<&'static Regex, GovernancePackExportError> {
    match SPEC_CURRENT_BOLD_RE.as_ref() {
        Ok(re) => Ok(re),
        Err(err) => Err(GovernancePackExportError::RegexCompile {
            pattern: SPEC_CURRENT_BOLD_PATTERN,
            message: err.to_string(),
        }),
    }
}

fn placeholder_re() -> Result<&'static Regex, GovernancePackExportError> {
    match PLACEHOLDER_RE.as_ref() {
        Ok(re) => Ok(re),
        Err(err) => Err(GovernancePackExportError::RegexCompile {
            pattern: PLACEHOLDER_PATTERN,
            message: err.to_string(),
        }),
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut msg = Vec::with_capacity(bytes.len() + 64);
    msg.extend_from_slice(bytes);
    msg.push(0x80);

    while (msg.len() % 64) != 56 {
        msg.push(0);
    }

    let bit_len = (bytes.len() as u64) * 8;
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let start = i * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    hex::encode(out)
}

#[cfg(test)]
mod tests {
    use super::sha1_hex;

    #[test]
    fn sha1_vectors() {
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1_hex(b"The quick brown fox jumps over the lazy dog"),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
    }
}
