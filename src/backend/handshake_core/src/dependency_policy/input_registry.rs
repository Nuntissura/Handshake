//! WP-KERNEL-009 / MT-023 — ModelTensorInputBoundary.
//!
//! Model/GGUF/tensor inputs are *data artifacts*, never runtime authority,
//! storage authority, or proof shortcuts. This module enforces the boundary
//! with a declare-before-use registry:
//!
//! - every external runtime input must be DECLARED (kind + path + declaring
//!   surface) before any consumer may validate/use it;
//! - the kind must be one of the allowlist vocabulary ([`RuntimeInputKind`]),
//!   and the path's extension must classify to that same kind — a `.exe`
//!   cannot be declared as a model, a `.gguf` cannot masquerade as a tensor;
//! - CUI-portable declarations additionally pass through the MT-022
//!   [`CuiPortableGate`] (closed by default);
//! - undeclared paths and undeclarable kinds are rejected with typed errors
//!   (negative tests below).
//!
//! The registry holds declarations only — it never loads, copies, or executes
//! the artifact, keeping "data artifact" a structural property.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::cui_gate::{CuiGateError, CuiPortableGate};
use super::RuntimeInputKind;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeInputError {
    #[error(
        "undeclarable runtime input {path}: extension does not map to any allowed external \
         runtime input kind (allowed kinds: model_gguf, model_safetensors, tensor_artifact, cui_portable_artifact)"
    )]
    UndeclarableKind { path: String },
    #[error(
        "runtime input kind mismatch for {path}: declared {declared} but path classifies as {classified}"
    )]
    KindMismatch {
        path: String,
        declared: String,
        classified: String,
    },
    #[error("runtime input {path} is not declared; declare it before use (MT-023 boundary)")]
    UndeclaredInput { path: String },
    #[error("duplicate declaration for runtime input {path}")]
    DuplicateDeclaration { path: String },
    #[error("declaring surface must be non-empty (e.g. \"model_runtime\")")]
    MissingDeclaringSurface,
    #[error(transparent)]
    CuiGate(#[from] CuiGateError),
}

/// A single declared external runtime input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInputDeclaration {
    pub kind: RuntimeInputKind,
    pub path: PathBuf,
    /// Product surface that owns the input (e.g. "model_runtime").
    pub declared_by: String,
}

/// Declare-before-use registry for operator-provided runtime inputs.
#[derive(Debug, Default)]
pub struct RuntimeInputRegistry {
    gate: CuiPortableGate,
    declarations: BTreeMap<PathBuf, RuntimeInputDeclaration>,
}

impl RuntimeInputRegistry {
    /// Registry with the default (closed) CUI gate — the WP-009 configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry with an explicitly provided CUI gate (an owning WP that holds
    /// an operator grant constructs the open gate itself, MT-022).
    pub fn with_cui_gate(gate: CuiPortableGate) -> Self {
        RuntimeInputRegistry {
            gate,
            declarations: BTreeMap::new(),
        }
    }

    /// Declares an input before use. The path extension must classify to
    /// `kind`; CUI-portable kinds require the gate to be open.
    pub fn declare(
        &mut self,
        kind: RuntimeInputKind,
        path: &Path,
        declared_by: &str,
    ) -> Result<&RuntimeInputDeclaration, RuntimeInputError> {
        if declared_by.trim().is_empty() {
            return Err(RuntimeInputError::MissingDeclaringSurface);
        }
        let classified = RuntimeInputKind::classify_path(path).ok_or_else(|| {
            RuntimeInputError::UndeclarableKind {
                path: path.display().to_string(),
            }
        })?;
        if classified != kind {
            return Err(RuntimeInputError::KindMismatch {
                path: path.display().to_string(),
                declared: kind.as_str().to_string(),
                classified: classified.as_str().to_string(),
            });
        }
        self.gate.permit(kind)?;
        let key = path.to_path_buf();
        if self.declarations.contains_key(&key) {
            return Err(RuntimeInputError::DuplicateDeclaration {
                path: path.display().to_string(),
            });
        }
        let declaration = RuntimeInputDeclaration {
            kind,
            path: key.clone(),
            declared_by: declared_by.trim().to_string(),
        };
        self.declarations.insert(key.clone(), declaration);
        Ok(self.declarations.get(&key).expect("just inserted"))
    }

    /// Validates that `path` was declared before use. The boundary check every
    /// consumer calls before touching an operator-provided artifact.
    pub fn validate_use(&self, path: &Path) -> Result<&RuntimeInputDeclaration, RuntimeInputError> {
        self.declarations
            .get(path)
            .ok_or_else(|| RuntimeInputError::UndeclaredInput {
                path: path.display().to_string(),
            })
    }

    pub fn declarations(&self) -> impl Iterator<Item = &RuntimeInputDeclaration> {
        self.declarations.values()
    }

    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_model_inputs_validate_for_use() {
        let mut registry = RuntimeInputRegistry::new();
        let path = Path::new("C:/models/llama-3.gguf");
        registry
            .declare(RuntimeInputKind::ModelGguf, path, "model_runtime")
            .expect("declaring a gguf model is allowed");
        let declaration = registry
            .validate_use(path)
            .expect("declared input validates");
        assert_eq!(declaration.kind, RuntimeInputKind::ModelGguf);
        assert_eq!(declaration.declared_by, "model_runtime");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn undeclared_paths_are_rejected_at_use() {
        let registry = RuntimeInputRegistry::new();
        let err = registry
            .validate_use(Path::new("C:/models/never-declared.gguf"))
            .expect_err("undeclared input must be rejected");
        assert!(matches!(err, RuntimeInputError::UndeclaredInput { .. }));
        assert!(err.to_string().contains("declare it before use"));
    }

    #[test]
    fn undeclarable_kinds_are_rejected_at_declare() {
        let mut registry = RuntimeInputRegistry::new();
        let err = registry
            .declare(
                RuntimeInputKind::ModelGguf,
                Path::new("C:/tools/outside-app.exe"),
                "model_runtime",
            )
            .expect_err("exe is not a declarable runtime input");
        assert!(matches!(err, RuntimeInputError::UndeclarableKind { .. }));
        // SQLite databases are not declarable artifacts either.
        let err = registry
            .declare(
                RuntimeInputKind::TensorArtifact,
                Path::new("C:/data/cache.sqlite3"),
                "model_runtime",
            )
            .expect_err("sqlite file is not a declarable runtime input");
        assert!(matches!(err, RuntimeInputError::UndeclarableKind { .. }));
    }

    #[test]
    fn kind_mismatch_is_rejected() {
        let mut registry = RuntimeInputRegistry::new();
        let err = registry
            .declare(
                RuntimeInputKind::TensorArtifact,
                Path::new("weights.gguf"),
                "model_runtime",
            )
            .expect_err("gguf cannot be declared as tensor artifact");
        assert_eq!(
            err,
            RuntimeInputError::KindMismatch {
                path: "weights.gguf".to_string(),
                declared: "tensor_artifact".to_string(),
                classified: "model_gguf".to_string(),
            }
        );
    }

    #[test]
    fn cui_portable_declaration_requires_open_gate() {
        let mut closed = RuntimeInputRegistry::new();
        let err = closed
            .declare(
                RuntimeInputKind::CuiPortableArtifact,
                Path::new("portable-tools.zip"),
                "future_cui_wp",
            )
            .expect_err("closed gate must reject CUI portable declarations");
        assert!(matches!(
            err,
            RuntimeInputError::CuiGate(CuiGateError::GateClosed { .. })
        ));

        let gate = CuiPortableGate::open_for_owning_wp("WP-FUTURE-CUI", "grant-7")
            .expect("gate opens with owning WP + grant");
        let mut open = RuntimeInputRegistry::with_cui_gate(gate);
        open.declare(
            RuntimeInputKind::CuiPortableArtifact,
            Path::new("portable-tools.zip"),
            "future_cui_wp",
        )
        .expect("open gate permits CUI portable declarations");
    }

    #[test]
    fn duplicate_declarations_are_rejected() {
        let mut registry = RuntimeInputRegistry::new();
        let path = Path::new("steering.npz");
        registry
            .declare(RuntimeInputKind::TensorArtifact, path, "model_runtime")
            .expect("first declaration");
        let err = registry
            .declare(RuntimeInputKind::TensorArtifact, path, "model_runtime")
            .expect_err("duplicate declaration must be rejected");
        assert!(matches!(
            err,
            RuntimeInputError::DuplicateDeclaration { .. }
        ));
    }

    #[test]
    fn empty_declaring_surface_is_rejected() {
        let mut registry = RuntimeInputRegistry::new();
        let err = registry
            .declare(RuntimeInputKind::ModelGguf, Path::new("m.gguf"), "  ")
            .expect_err("declaring surface required");
        assert_eq!(err, RuntimeInputError::MissingDeclaringSurface);
    }
}
