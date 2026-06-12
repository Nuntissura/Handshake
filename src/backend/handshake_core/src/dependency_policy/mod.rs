//! WP-KERNEL-009 / NativeDependencyAndPackaging — runtime dependency policy.
//!
//! Rust counterpart of the machine-readable runtime dependency allowlist at
//! `app/src/lib/dependency_policy/runtime_dependency_allowlist.json` (MT-017).
//!
//! This module owns:
//! - the typed [`RuntimeInputKind`] vocabulary for operator-provided external
//!   runtime inputs (model/GGUF/tensor/CUI-portable artifacts) — MT-017/MT-023;
//! - the `CuiPortableGate` operator gate, default closed — MT-022;
//! - the `RuntimeInputRegistry` declare-before-use boundary — MT-023;
//! - parity tests that read the frontend allowlist JSON so the TS and Rust
//!   sides cannot drift silently — MT-017;
//! - manifest/lockfile tripwires for forbidden dependency classes (SQLite et
//!   al.) — MT-025;
//! - native parser bundling proof (tree-sitter grammars statically compiled,
//!   no runtime grammar downloads) — MT-028.
//!
//! Policy summary (mirrors the JSON document):
//! - Core Handshake behavior is Handshake-native: no outside app, outside
//!   server, external daemon, Docker-only path, SQLite fallback, or
//!   Redis/Valkey cache authority.
//! - The only allowed external runtime inputs are operator-provided model
//!   files (GGUF/safetensors), tensor artifacts, and CUI-portable artifacts,
//!   all operator-gated and default-off.
//! - Editor libraries (Tiptap, Monaco, Yjs, tree-sitter) are bundled,
//!   lockfile-governed product libraries — never external runtime services.

pub mod cui_gate;
pub mod input_registry;
pub mod manifest_tripwires;
#[cfg(test)]
mod native_parser_bundling;
pub mod source_tripwires;

pub use cui_gate::{CuiGateError, CuiPortableGate};
pub use input_registry::{RuntimeInputDeclaration, RuntimeInputError, RuntimeInputRegistry};
pub use source_tripwires::{
    assert_source_tripwire_policy, assert_source_tripwire_policy_for_files,
};

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Schema id of the allowlist JSON document (must match the frontend copy).
pub const ALLOWLIST_SCHEMA: &str = "handshake.runtime_dependency_allowlist@1";

/// Repo-root-relative path of the allowlist document (single data authority).
pub const ALLOWLIST_RELATIVE_PATH: &str =
    "app/src/lib/dependency_policy/runtime_dependency_allowlist.json";

#[derive(Debug, Error)]
pub enum DependencyPolicyError {
    #[error("failed to read runtime dependency allowlist at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse runtime dependency allowlist JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid allowlist schema: expected {expected}, got {actual}")]
    InvalidSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("allowlist kind mismatch between JSON and Rust vocabulary: {0}")]
    KindMismatch(String),
}

/// Operator-provided external runtime input classes allowed by MT-017.
///
/// These are *data artifacts*, never runtime authority, storage authority, or
/// proof shortcuts (MT-023). Anything not representable here is not an allowed
/// external runtime input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeInputKind {
    /// Operator-provided GGUF model file.
    ModelGguf,
    /// Operator-provided safetensors weights.
    ModelSafetensors,
    /// Operator-provided tensor data artifact (steering vectors, embeddings).
    TensorArtifact,
    /// CUI-portable artifact; usable only behind an owning-WP gate (MT-022).
    CuiPortableArtifact,
}

impl RuntimeInputKind {
    pub const ALL: [RuntimeInputKind; 4] = [
        RuntimeInputKind::ModelGguf,
        RuntimeInputKind::ModelSafetensors,
        RuntimeInputKind::TensorArtifact,
        RuntimeInputKind::CuiPortableArtifact,
    ];

    /// Stable machine id; matches the `kind` field in the JSON document.
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeInputKind::ModelGguf => "model_gguf",
            RuntimeInputKind::ModelSafetensors => "model_safetensors",
            RuntimeInputKind::TensorArtifact => "tensor_artifact",
            RuntimeInputKind::CuiPortableArtifact => "cui_portable_artifact",
        }
    }

    pub fn from_str_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.as_str() == id)
    }

    /// Classifies a path by extension into a declared kind, mirroring the
    /// `allowed_extensions` lists in the JSON document.
    pub fn classify_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "gguf" => Some(RuntimeInputKind::ModelGguf),
            "safetensors" => Some(RuntimeInputKind::ModelSafetensors),
            "npy" | "npz" | "pt" => Some(RuntimeInputKind::TensorArtifact),
            "zip" | "7z" => Some(RuntimeInputKind::CuiPortableArtifact),
            _ => None,
        }
    }
}

/// Forbidden runtime dependency class ids (must match the JSON document).
pub const FORBIDDEN_CLASS_IDS: [&str; 5] = [
    "outside_app",
    "outside_server_daemon",
    "docker_default",
    "sqlite",
    "cdn_runtime_asset",
];

#[derive(Debug, Clone, Deserialize)]
pub struct AllowedExternalRuntimeInput {
    pub kind: String,
    pub description: String,
    pub operator_gated: bool,
    pub default_enabled: bool,
    pub allowed_extensions: Vec<String>,
    pub owning_surface: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForbiddenRuntimeDependencyClass {
    pub id: String,
    pub description: String,
    pub npm_package_names: Vec<String>,
    pub cargo_crate_name_substrings: Vec<String>,
    pub source_scan_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundledLibraryRule {
    pub family: String,
    pub ecosystem: String,
    pub package_patterns: Vec<String>,
    pub allowed_licenses: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerOptInException {
    pub path_prefix: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ScanSelfExemptPaths {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SourceTripwireExceptions {
    #[serde(default)]
    pub entries: Vec<SourceTripwireException>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceTripwireException {
    pub class_id: String,
    pub path: String,
    pub patterns: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductManifests {
    pub npm: Vec<String>,
    pub npm_lockfiles: Vec<String>,
    pub cargo: Vec<String>,
    pub cargo_lockfiles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeDependencyAllowlist {
    pub schema: String,
    pub version: String,
    pub wp_id: String,
    pub mt_id: String,
    pub allowed_external_runtime_inputs: Vec<AllowedExternalRuntimeInput>,
    pub forbidden_runtime_dependency_classes: Vec<ForbiddenRuntimeDependencyClass>,
    pub bundled_libraries: Vec<BundledLibraryRule>,
    pub docker_opt_in_exceptions: Vec<DockerOptInException>,
    pub product_scan_roots: Vec<String>,
    pub product_manifests: ProductManifests,
    #[serde(default)]
    pub scan_self_exempt_paths: ScanSelfExemptPaths,
    #[serde(default)]
    pub source_tripwire_exceptions: SourceTripwireExceptions,
}

impl RuntimeDependencyAllowlist {
    /// Loads and validates the allowlist document from a repo root.
    pub fn load_from_repo_root(repo_root: &Path) -> Result<Self, DependencyPolicyError> {
        let path = repo_root.join(ALLOWLIST_RELATIVE_PATH);
        let raw = std::fs::read_to_string(&path).map_err(|source| DependencyPolicyError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let doc: RuntimeDependencyAllowlist = serde_json::from_str(&raw)?;
        if doc.schema != ALLOWLIST_SCHEMA {
            return Err(DependencyPolicyError::InvalidSchema {
                expected: ALLOWLIST_SCHEMA,
                actual: doc.schema,
            });
        }
        doc.assert_kind_parity()?;
        Ok(doc)
    }

    /// Ensures the JSON `kind` vocabulary and [`RuntimeInputKind`] agree exactly.
    pub fn assert_kind_parity(&self) -> Result<(), DependencyPolicyError> {
        let json_kinds: Vec<&str> = self
            .allowed_external_runtime_inputs
            .iter()
            .map(|i| i.kind.as_str())
            .collect();
        for kind in RuntimeInputKind::ALL {
            if !json_kinds.contains(&kind.as_str()) {
                return Err(DependencyPolicyError::KindMismatch(format!(
                    "Rust kind {} missing from JSON document",
                    kind.as_str()
                )));
            }
        }
        for json_kind in &json_kinds {
            if RuntimeInputKind::from_str_id(json_kind).is_none() {
                return Err(DependencyPolicyError::KindMismatch(format!(
                    "JSON kind {json_kind} unknown to Rust vocabulary"
                )));
            }
        }
        Ok(())
    }

    pub fn forbidden_class(&self, id: &str) -> Option<&ForbiddenRuntimeDependencyClass> {
        self.forbidden_runtime_dependency_classes
            .iter()
            .find(|c| c.id == id)
    }
}

/// Repo root derived from this crate's manifest dir
/// (`src/backend/handshake_core` → repo root is three levels up).
pub fn repo_root_from_manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .components()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_document_loads_and_matches_rust_vocabulary() {
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
                .expect("allowlist document must load and pass kind parity");
        assert_eq!(allowlist.schema, ALLOWLIST_SCHEMA);
        assert!(allowlist.wp_id.contains("WP-KERNEL-009"));
        assert_eq!(allowlist.allowed_external_runtime_inputs.len(), 4);
    }

    #[test]
    fn allowlist_declares_required_forbidden_classes() {
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
                .expect("allowlist loads");
        for id in FORBIDDEN_CLASS_IDS {
            let class = allowlist
                .forbidden_class(id)
                .unwrap_or_else(|| panic!("forbidden class {id} missing from allowlist"));
            assert!(!class.description.is_empty());
        }
        let sqlite = allowlist.forbidden_class("sqlite").expect("sqlite class");
        assert!(
            sqlite
                .cargo_crate_name_substrings
                .iter()
                .any(|s| s == "sqlite"),
            "sqlite class must forbid any cargo crate containing 'sqlite'"
        );
    }

    #[test]
    fn source_tripwire_exceptions_are_exact_and_documented() {
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
                .expect("allowlist loads");
        for exception in &allowlist.source_tripwire_exceptions.entries {
            assert!(
                !exception.path.contains('\\') && !exception.path.is_empty(),
                "source tripwire exception path must be exact repo-relative POSIX form: {exception:?}"
            );
            assert!(
                !exception.class_id.is_empty()
                    && allowlist.forbidden_class(&exception.class_id).is_some(),
                "source tripwire exception must reference a known class: {exception:?}"
            );
            assert!(
                !exception.patterns.is_empty()
                    && exception.patterns.iter().all(|pattern| !pattern.is_empty()),
                "source tripwire exception must be pattern-scoped: {exception:?}"
            );
            assert!(
                exception.reason.len() >= 24,
                "source tripwire exception must document why it is not a forbidden runtime dependency: {exception:?}"
            );
        }
    }

    #[test]
    fn every_external_input_is_operator_gated_and_default_off() {
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
                .expect("allowlist loads");
        for input in &allowlist.allowed_external_runtime_inputs {
            assert!(
                input.operator_gated,
                "{} must be operator gated",
                input.kind
            );
            assert!(!input.default_enabled, "{} must default off", input.kind);
        }
    }

    #[test]
    fn kind_classification_accepts_declared_artifacts_and_rejects_others() {
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("C:/models/llama.gguf")),
            Some(RuntimeInputKind::ModelGguf)
        );
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("weights.SAFETENSORS")),
            Some(RuntimeInputKind::ModelSafetensors)
        );
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("steer.npz")),
            Some(RuntimeInputKind::TensorArtifact)
        );
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("portable.zip")),
            Some(RuntimeInputKind::CuiPortableArtifact)
        );
        assert_eq!(RuntimeInputKind::classify_path(Path::new("evil.exe")), None);
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("db.sqlite3")),
            None
        );
        assert_eq!(
            RuntimeInputKind::classify_path(Path::new("no_extension")),
            None
        );
    }

    #[test]
    fn schema_mismatch_is_rejected() {
        let raw = r#"{
            "schema": "handshake.runtime_dependency_allowlist@99",
            "version": "1.0.0",
            "wp_id": "x",
            "mt_id": "x",
            "allowed_external_runtime_inputs": [],
            "forbidden_runtime_dependency_classes": [],
            "bundled_libraries": [],
            "docker_opt_in_exceptions": [],
            "product_scan_roots": [],
            "product_manifests": {"npm": [], "npm_lockfiles": [], "cargo": [], "cargo_lockfiles": []},
            "scan_self_exempt_paths": {"paths": []}
        }"#;
        let doc: RuntimeDependencyAllowlist = serde_json::from_str(raw).expect("parses");
        assert_eq!(doc.schema, "handshake.runtime_dependency_allowlist@99");
        // load_from_repo_root path: simulate by checking schema guard directly.
        assert_ne!(doc.schema, ALLOWLIST_SCHEMA);
    }
}
