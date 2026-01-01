use std::cmp::Ordering;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

/// Canonical severity values (DIAG-SCHEMA-001).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Fatal,
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Fatal => "fatal",
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

impl FromStr for DiagnosticSeverity {
    type Err = DiagnosticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fatal" => Ok(DiagnosticSeverity::Fatal),
            "error" => Ok(DiagnosticSeverity::Error),
            "warning" => Ok(DiagnosticSeverity::Warning),
            "info" => Ok(DiagnosticSeverity::Info),
            "hint" => Ok(DiagnosticSeverity::Hint),
            other => Err(DiagnosticError::Serialization(format!(
                "unknown severity: {}",
                other
            ))),
        }
    }
}

/// Sources include parametrised plugin/matcher values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSource {
    Lsp,
    Terminal,
    Validator,
    Engine,
    Connector,
    System,
    Plugin(String),
    Matcher(String),
}

impl Serialize for DiagnosticSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

impl DiagnosticSource {
    pub fn as_str(&self) -> String {
        match self {
            DiagnosticSource::Lsp => "lsp".to_string(),
            DiagnosticSource::Terminal => "terminal".to_string(),
            DiagnosticSource::Validator => "validator".to_string(),
            DiagnosticSource::Engine => "engine".to_string(),
            DiagnosticSource::Connector => "connector".to_string(),
            DiagnosticSource::System => "system".to_string(),
            DiagnosticSource::Plugin(name) => format!("plugin:{name}"),
            DiagnosticSource::Matcher(name) => format!("matcher:{name}"),
        }
    }
}

impl FromStr for DiagnosticSource {
    type Err = DiagnosticError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = raw.strip_prefix("plugin:") {
            return Ok(DiagnosticSource::Plugin(rest.to_string()));
        }
        if let Some(rest) = raw.strip_prefix("matcher:") {
            return Ok(DiagnosticSource::Matcher(rest.to_string()));
        }
        match raw {
            "lsp" => Ok(DiagnosticSource::Lsp),
            "terminal" => Ok(DiagnosticSource::Terminal),
            "validator" => Ok(DiagnosticSource::Validator),
            "engine" => Ok(DiagnosticSource::Engine),
            "connector" => Ok(DiagnosticSource::Connector),
            "system" => Ok(DiagnosticSource::System),
            other => Err(DiagnosticError::InvalidSource(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSurface {
    Monaco,
    Canvas,
    Sheet,
    Terminal,
    Connector,
    System,
}

impl DiagnosticSurface {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSurface::Monaco => "monaco",
            DiagnosticSurface::Canvas => "canvas",
            DiagnosticSurface::Sheet => "sheet",
            DiagnosticSurface::Terminal => "terminal",
            DiagnosticSurface::Connector => "connector",
            DiagnosticSurface::System => "system",
        }
    }
}

impl FromStr for DiagnosticSurface {
    type Err = DiagnosticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "monaco" => Ok(DiagnosticSurface::Monaco),
            "canvas" => Ok(DiagnosticSurface::Canvas),
            "sheet" => Ok(DiagnosticSurface::Sheet),
            "terminal" => Ok(DiagnosticSurface::Terminal),
            "connector" => Ok(DiagnosticSurface::Connector),
            "system" => Ok(DiagnosticSurface::System),
            other => Err(DiagnosticError::Serialization(format!(
                "unknown surface: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinkConfidence {
    Direct,
    Inferred,
    Ambiguous,
    #[default]
    Unlinked,
}

impl LinkConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkConfidence::Direct => "direct",
            LinkConfidence::Inferred => "inferred",
            LinkConfidence::Ambiguous => "ambiguous",
            LinkConfidence::Unlinked => "unlinked",
        }
    }
}

impl FromStr for LinkConfidence {
    type Err = DiagnosticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "direct" => Ok(LinkConfidence::Direct),
            "inferred" => Ok(LinkConfidence::Inferred),
            "ambiguous" => Ok(LinkConfidence::Ambiguous),
            "unlinked" => Ok(LinkConfidence::Unlinked),
            other => Err(DiagnosticError::Serialization(format!(
                "unknown link_confidence: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Open,
    Acknowledged,
    Muted,
    Resolved,
}

impl DiagnosticStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticStatus::Open => "open",
            DiagnosticStatus::Acknowledged => "acknowledged",
            DiagnosticStatus::Muted => "muted",
            DiagnosticStatus::Resolved => "resolved",
        }
    }
}

impl FromStr for DiagnosticStatus {
    type Err = DiagnosticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(DiagnosticStatus::Open),
            "acknowledged" => Ok(DiagnosticStatus::Acknowledged),
            "muted" => Ok(DiagnosticStatus::Muted),
            "resolved" => Ok(DiagnosticStatus::Resolved),
            other => Err(DiagnosticError::Serialization(format!(
                "unknown status: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticActor {
    Human,
    Agent,
    System,
}

impl DiagnosticActor {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticActor::Human => "human",
            DiagnosticActor::Agent => "agent",
            DiagnosticActor::System => "system",
        }
    }
}

impl FromStr for DiagnosticActor {
    type Err = DiagnosticError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(DiagnosticActor::Human),
            "agent" => Ok(DiagnosticActor::Agent),
            "system" => Ok(DiagnosticActor::System),
            other => Err(DiagnosticError::Serialization(format!(
                "unknown actor: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticRange {
    #[serde(rename = "startLine", alias = "start_line")]
    pub start_line: i32,
    #[serde(rename = "startColumn", alias = "start_column")]
    pub start_column: i32,
    #[serde(rename = "endLine", alias = "end_line")]
    pub end_line: i32,
    #[serde(rename = "endColumn", alias = "end_column")]
    pub end_column: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticLocation {
    /// Local path where applicable (normalized to '/')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// URI (file:// or internal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Workspace surface id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    /// KG / RawContent entity id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<DiagnosticRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fr_event_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_job_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_activity_span_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_session_span_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_hashes: Option<ArtifactHashes>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub id: Uuid,
    pub fingerprint: String,
    pub title: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub source: DiagnosticSource,
    pub surface: DiagnosticSurface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<DiagnosticActor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_decision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<DiagnosticLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<EvidenceRefs>,
    pub link_confidence: LinkConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DiagnosticStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_seen: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<DateTime<Utc>>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Builder input without derived fields (id/fingerprint/timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticInput {
    pub title: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub source: DiagnosticSource,
    pub surface: DiagnosticSurface,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub wsid: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub actor: Option<DiagnosticActor>,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub policy_decision_id: Option<String>,
    #[serde(default)]
    pub locations: Option<Vec<DiagnosticLocation>>,
    #[serde(default)]
    pub evidence_refs: Option<EvidenceRefs>,
    #[serde(default)]
    pub link_confidence: LinkConfidence,
    #[serde(default)]
    pub status: Option<DiagnosticStatus>,
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub first_seen: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl DiagnosticInput {
    pub fn into_diagnostic(self) -> Result<Diagnostic, DiagnosticError> {
        let now = self.timestamp.unwrap_or_else(Utc::now);
        let normalized_title = normalize_text(&self.title);
        let normalized_message = normalize_text(&self.message);
        let fingerprint = compute_fingerprint(
            &self.source,
            &self.surface,
            self.tool.as_deref(),
            self.code.as_deref(),
            &self.severity,
            &normalized_title,
            self.locations.as_ref(),
            self.capability_id.as_deref(),
            self.policy_decision_id.as_deref(),
        )?;

        Ok(Diagnostic {
            id: Uuid::new_v4(),
            fingerprint,
            title: normalized_title,
            message: normalized_message,
            severity: self.severity,
            source: self.source,
            surface: self.surface,
            tool: self.tool,
            code: self.code,
            tags: self.tags.map(normalize_tags),
            wsid: self.wsid,
            job_id: self.job_id,
            model_id: self.model_id,
            actor: self.actor,
            capability_id: self.capability_id,
            policy_decision_id: self.policy_decision_id,
            locations: self.locations.map(normalize_locations),
            evidence_refs: self.evidence_refs,
            link_confidence: self.link_confidence,
            status: self.status,
            count: self.count,
            first_seen: self.first_seen,
            last_seen: self.last_seen,
            timestamp: now,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagFilter {
    pub severity: Option<DiagnosticSeverity>,
    pub source: Option<String>,
    pub surface: Option<DiagnosticSurface>,
    pub wsid: Option<String>,
    pub job_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub fingerprint: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Error)]
pub enum DiagnosticError {
    #[error("invalid diagnostic source: {0}")]
    InvalidSource(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for DiagnosticError {
    fn from(err: serde_json::Error) -> Self {
        DiagnosticError::Serialization(err.to_string())
    }
}

fn normalize_text(input: &str) -> String {
    let collapsed_newlines = input.replace("\r\n", "\n");
    collapsed_newlines.trim().nfc().collect()
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").nfc().collect()
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut deduped = tags
        .into_iter()
        .map(|t| normalize_text(&t))
        .collect::<Vec<_>>();
    deduped.sort();
    deduped.dedup();
    deduped
}

fn normalize_locations(locations: Vec<DiagnosticLocation>) -> Vec<DiagnosticLocation> {
    let mut normalized: Vec<DiagnosticLocation> = locations
        .into_iter()
        .map(|loc| DiagnosticLocation {
            path: loc.path.as_ref().map(|p| normalize_path(p)),
            uri: loc.uri.map(|u| u.nfc().collect()),
            wsid: loc.wsid.map(|v| v.nfc().collect()),
            entity_id: loc.entity_id.map(|v| v.nfc().collect()),
            range: loc.range,
        })
        .collect();

    normalized.sort_by(|a, b| {
        let a_key = format!(
            "{}|{}|{}|{}",
            a.path.as_deref().unwrap_or(""),
            a.uri.as_deref().unwrap_or(""),
            a.entity_id.as_deref().unwrap_or(""),
            a.wsid.as_deref().unwrap_or("")
        );
        let b_key = format!(
            "{}|{}|{}|{}",
            b.path.as_deref().unwrap_or(""),
            b.uri.as_deref().unwrap_or(""),
            b.entity_id.as_deref().unwrap_or(""),
            b.wsid.as_deref().unwrap_or("")
        );
        a_key.cmp(&b_key)
    });

    normalized
}

fn canonicalize_locations(locations: Option<&Vec<DiagnosticLocation>>) -> Value {
    let Some(list) = locations else {
        return Value::Null;
    };

    if list.is_empty() {
        return Value::Array(Vec::new());
    }

    let mut sortable: Vec<(String, Value)> = list
        .iter()
        .map(|loc| {
            let normalized_path = loc.path.as_ref().map(|p| normalize_path(p));
            let normalized_uri = loc.uri.as_ref().map(|u| u.nfc().collect::<String>());
            let normalized_entity_id = loc.entity_id.as_ref().map(|e| e.nfc().collect::<String>());
            let normalized_wsid = loc.wsid.as_ref().map(|w| w.nfc().collect::<String>());

            let mut map = Map::new();
            map.insert(
                "path".to_string(),
                normalized_path
                    .as_ref()
                    .map(|p| Value::String(p.clone()))
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "uri".to_string(),
                normalized_uri
                    .as_ref()
                    .map(|u| Value::String(u.clone()))
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "entity_id".to_string(),
                normalized_entity_id
                    .as_ref()
                    .map(|e| Value::String(e.clone()))
                    .unwrap_or(Value::Null),
            );
            map.insert(
                "wsid".to_string(),
                normalized_wsid
                    .as_ref()
                    .map(|w| Value::String(w.clone()))
                    .unwrap_or(Value::Null),
            );

            let range_key = if let Some(range) = loc.range.as_ref() {
                let mut range_map = Map::new();
                range_map.insert("startLine".to_string(), Value::from(range.start_line));
                range_map.insert("startColumn".to_string(), Value::from(range.start_column));
                range_map.insert("endLine".to_string(), Value::from(range.end_line));
                range_map.insert("endColumn".to_string(), Value::from(range.end_column));
                map.insert("range".to_string(), Value::Object(range_map));
                format!(
                    "{}:{}:{}:{}",
                    range.start_line, range.start_column, range.end_line, range.end_column
                )
            } else {
                map.insert("range".to_string(), Value::Null);
                String::new()
            };

            let sort_key = format!(
                "{}|{}|{}|{}|{}",
                normalized_path.as_deref().unwrap_or(""),
                normalized_uri.as_deref().unwrap_or(""),
                normalized_entity_id.as_deref().unwrap_or(""),
                normalized_wsid.as_deref().unwrap_or(""),
                range_key
            );

            (sort_key, Value::Object(map))
        })
        .collect();

    sortable.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Array(sortable.into_iter().map(|(_, v)| v).collect())
}

#[allow(clippy::too_many_arguments)]
fn canonical_tuple(
    source: &DiagnosticSource,
    surface: &DiagnosticSurface,
    tool: Option<&str>,
    code: Option<&str>,
    severity: &DiagnosticSeverity,
    title: &str,
    locations: Option<&Vec<DiagnosticLocation>>,
    capability_id: Option<&str>,
    policy_decision_id: Option<&str>,
) -> Value {
    let mut map = Map::new();
    map.insert("source".to_string(), Value::String(source.as_str()));
    map.insert(
        "surface".to_string(),
        Value::String(surface.as_str().to_string()),
    );
    map.insert(
        "tool".to_string(),
        tool.map(|t| Value::String(t.nfc().collect()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "code".to_string(),
        code.map(|c| Value::String(c.nfc().collect()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "severity".to_string(),
        Value::String(severity.as_str().to_string()),
    );
    map.insert("title".to_string(), Value::String(title.to_string()));
    map.insert("locations".to_string(), canonicalize_locations(locations));
    map.insert(
        "capability_id".to_string(),
        capability_id
            .map(|c| Value::String(c.nfc().collect()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "policy_decision_id".to_string(),
        policy_decision_id
            .map(|p| Value::String(p.nfc().collect()))
            .unwrap_or(Value::Null),
    );

    Value::Object(map)
}

/// Deterministic fingerprint per DIAG-SCHEMA-003.
#[allow(clippy::too_many_arguments)]
pub fn compute_fingerprint(
    source: &DiagnosticSource,
    surface: &DiagnosticSurface,
    tool: Option<&str>,
    code: Option<&str>,
    severity: &DiagnosticSeverity,
    title: &str,
    locations: Option<&Vec<DiagnosticLocation>>,
    capability_id: Option<&str>,
    policy_decision_id: Option<&str>,
) -> Result<String, DiagnosticError> {
    let tuple = canonical_tuple(
        source,
        surface,
        tool,
        code,
        severity,
        title,
        locations,
        capability_id,
        policy_decision_id,
    );
    let serialized = serde_json::to_string(&tuple)?;
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

/// Aggregates diagnostics by fingerprint and assigns count/first/last.
pub fn aggregate_grouped(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by(|a, b| {
        a.fingerprint
            .cmp(&b.fingerprint)
            .then_with(|| a.timestamp.cmp(&b.timestamp))
    });

    let mut grouped: Vec<Diagnostic> = Vec::new();
    let mut iter = diagnostics.into_iter().peekable();
    while let Some(mut current) = iter.next() {
        let mut count = 1u64;
        let mut first_seen = current.timestamp;
        let mut last_seen = current.timestamp;

        while let Some(next_item) = iter.next_if(|n| n.fingerprint == current.fingerprint) {
            count += 1;
            if next_item.timestamp < first_seen {
                first_seen = next_item.timestamp;
            }
            if next_item.timestamp >= last_seen {
                last_seen = next_item.timestamp;
                current = next_item;
            }
        }

        current.count = Some(count);
        current.first_seen = Some(first_seen);
        current.last_seen = Some(last_seen);
        grouped.push(current);
    }

    grouped.sort_by(|a, b| match (a.last_seen, b.last_seen) {
        (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts),
        _ => Ordering::Equal,
    });

    grouped
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemGroup {
    pub fingerprint: String,
    pub count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub sample: Diagnostic,
}

pub fn diagnostic_metadata(diagnostic: &Diagnostic) -> Value {
    let mut map = Map::new();
    map.insert(
        "tags".to_string(),
        diagnostic
            .tags
            .as_ref()
            .map(|t| Value::Array(t.iter().map(|v| Value::String(v.clone())).collect()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "model_id".to_string(),
        diagnostic
            .model_id
            .as_ref()
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "actor".to_string(),
        diagnostic
            .actor
            .as_ref()
            .map(|v| Value::String(v.as_str().to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "capability_id".to_string(),
        diagnostic
            .capability_id
            .as_ref()
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "policy_decision_id".to_string(),
        diagnostic
            .policy_decision_id
            .as_ref()
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "locations".to_string(),
        diagnostic
            .locations
            .as_ref()
            .map(|loc| serde_json::to_value(loc).unwrap_or(Value::Null))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "evidence_refs".to_string(),
        diagnostic
            .evidence_refs
            .as_ref()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "status".to_string(),
        diagnostic
            .status
            .as_ref()
            .map(|s| Value::String(s.as_str().to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "count".to_string(),
        diagnostic.count.map(Value::from).unwrap_or(Value::Null),
    );
    map.insert(
        "first_seen".to_string(),
        diagnostic
            .first_seen
            .as_ref()
            .map(|v| Value::String(v.to_rfc3339()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "last_seen".to_string(),
        diagnostic
            .last_seen
            .as_ref()
            .map(|v| Value::String(v.to_rfc3339()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "updated_at".to_string(),
        diagnostic
            .updated_at
            .as_ref()
            .map(|v| Value::String(v.to_rfc3339()))
            .unwrap_or(Value::Null),
    );
    Value::Object(map)
}

pub fn apply_metadata(diagnostic: &mut Diagnostic, meta: &Value) {
    let Some(map) = meta.as_object() else { return };
    diagnostic.tags = map
        .get("tags")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok());
    diagnostic.model_id = map
        .get("model_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    diagnostic.actor = map
        .get("actor")
        .and_then(|v| v.as_str())
        .and_then(|s| DiagnosticActor::from_str(s).ok());
    diagnostic.capability_id = map
        .get("capability_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    diagnostic.policy_decision_id = map
        .get("policy_decision_id")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    diagnostic.locations = map
        .get("locations")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    diagnostic.evidence_refs = map
        .get("evidence_refs")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    diagnostic.status = map
        .get("status")
        .and_then(|v| v.as_str())
        .and_then(|s| DiagnosticStatus::from_str(s).ok());
    diagnostic.count = map.get("count").and_then(|v| v.as_u64());
    diagnostic.first_seen = map
        .get("first_seen")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    diagnostic.last_seen = map
        .get("last_seen")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
    diagnostic.updated_at = map
        .get("updated_at")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
}

#[async_trait::async_trait]
pub trait DiagnosticsStore: Send + Sync {
    async fn record_diagnostic(&self, diag: Diagnostic)
        -> Result<(), crate::storage::StorageError>;
    async fn list_problems(
        &self,
        filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, crate::storage::StorageError>;
    async fn get_diagnostic(&self, id: Uuid) -> Result<Diagnostic, crate::storage::StorageError>;
    async fn list_diagnostics(
        &self,
        filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, crate::storage::StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_normalizes_paths_and_newlines() -> Result<(), DiagnosticError> {
        let base_locations = vec![DiagnosticLocation {
            path: Some("C:\\path\\to\\file.rs".to_string()),
            uri: None,
            wsid: Some("ws-123".to_string()),
            entity_id: None,
            range: Some(DiagnosticRange {
                start_line: 1,
                start_column: 1,
                end_line: 2,
                end_column: 4,
            }),
        }];

        let input_a = DiagnosticInput {
            title: "Line endings\r\n normalized".to_string(),
            message: "Test message".to_string(),
            severity: DiagnosticSeverity::Error,
            source: DiagnosticSource::Validator,
            surface: DiagnosticSurface::Monaco,
            tool: Some("linter".to_string()),
            code: Some("E100".to_string()),
            tags: None,
            wsid: Some("ws-123".to_string()),
            job_id: Some("job-1".to_string()),
            model_id: None,
            actor: Some(DiagnosticActor::System),
            capability_id: None,
            policy_decision_id: None,
            locations: Some(base_locations.clone()),
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        };

        let mut input_b = input_a.clone();
        input_b.title = "Line endings\n normalized ".to_string();
        input_b.locations = Some(vec![DiagnosticLocation {
            path: Some("C:/path/to/file.rs".to_string()),
            ..base_locations[0].clone()
        }]);

        let diag_a = input_a.into_diagnostic()?;
        let diag_b = input_b.into_diagnostic()?;

        assert_eq!(diag_a.fingerprint, diag_b.fingerprint);
        Ok(())
    }

    #[test]
    fn fingerprint_is_location_order_independent() -> Result<(), DiagnosticError> {
        let loc_a = DiagnosticLocation {
            path: Some("C:\\path\\to\\a.rs".to_string()),
            uri: None,
            wsid: Some("ws-123".to_string()),
            entity_id: None,
            range: None,
        };
        let loc_b = DiagnosticLocation {
            path: Some("C:\\path\\to\\b.rs".to_string()),
            uri: None,
            wsid: Some("ws-123".to_string()),
            entity_id: None,
            range: Some(DiagnosticRange {
                start_line: 10,
                start_column: 1,
                end_line: 10,
                end_column: 5,
            }),
        };

        let base = DiagnosticInput {
            title: "Title".to_string(),
            message: "Message".to_string(),
            severity: DiagnosticSeverity::Error,
            source: DiagnosticSource::Validator,
            surface: DiagnosticSurface::Monaco,
            tool: None,
            code: None,
            tags: None,
            wsid: Some("ws-123".to_string()),
            job_id: None,
            model_id: None,
            actor: None,
            capability_id: None,
            policy_decision_id: None,
            locations: Some(vec![loc_a.clone(), loc_b.clone()]),
            evidence_refs: None,
            link_confidence: LinkConfidence::Unlinked,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        };

        let mut swapped = base.clone();
        swapped.locations = Some(vec![loc_b, loc_a]);

        let diag_a = base.into_diagnostic()?;
        let diag_b = swapped.into_diagnostic()?;
        assert_eq!(diag_a.fingerprint, diag_b.fingerprint);
        Ok(())
    }

    #[test]
    fn aggregate_sets_counts_and_timestamps() {
        let now = Utc::now();
        let d1 = Diagnostic {
            id: Uuid::new_v4(),
            fingerprint: "f1".to_string(),
            title: "a".to_string(),
            message: "m".to_string(),
            severity: DiagnosticSeverity::Error,
            source: DiagnosticSource::System,
            surface: DiagnosticSurface::Monaco,
            tool: None,
            code: None,
            tags: None,
            wsid: None,
            job_id: None,
            model_id: None,
            actor: None,
            capability_id: None,
            policy_decision_id: None,
            locations: None,
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: now,
            updated_at: None,
        };
        let mut d2 = d1.clone();
        d2.timestamp = now + chrono::Duration::seconds(10);
        let grouped = aggregate_grouped(vec![d1.clone(), d2.clone()]);
        assert_eq!(grouped.len(), 1);
        let g = &grouped[0];
        assert_eq!(g.count, Some(2));
        assert_eq!(g.first_seen, Some(now));
        assert_eq!(g.last_seen, Some(now + chrono::Duration::seconds(10)));
        assert_eq!(g.timestamp, d2.timestamp, "latest diagnostic chosen");
    }
}
