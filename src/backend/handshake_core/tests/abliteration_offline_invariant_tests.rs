//! MT-108: INF-6 abliteration offline-only invariant enforcement.

use std::{
    fs,
    path::{Path, PathBuf},
};

use handshake_core::{
    distillation::abliterate_review::{
        AbliteratedSkillBankModelRegistration, SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION,
    },
    model_runtime::{
        assert_abliteration_offline_invariant, assert_abliteration_offline_invariant_at,
        LocalModelAdapterInvariant,
    },
    process_ledger::ProcessEngineKind,
};

#[test]
fn abliteration_offline_invariant_tests_current_repo_keeps_abliterate_out_of_hot_paths() {
    assert_abliteration_offline_invariant().unwrap_or_else(|violations| {
        panic!(
            "current repo violates the offline-only abliteration invariant:\n{}",
            violations.join("\n")
        )
    });
}

#[test]
fn abliteration_offline_invariant_tests_static_scan_catches_generate_path_violation() {
    let root = clean_core_fixture();
    write_file(
        root.path().join("src/model_runtime/llama_cpp/generate.rs"),
        "use crate::distillation::abliterate;\n",
    );

    let violations = assert_abliteration_offline_invariant_at(root.path())
        .expect_err("generate.rs references to distillation::abliterate must fail closed");

    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("llama_cpp") && violation.contains("generate.rs")),
        "expected violation to name the generate path: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("distillation::abliterate")),
        "expected violation to name the forbidden module reference: {violations:?}"
    );
}

#[test]
fn abliteration_offline_invariant_tests_static_scan_catches_model_runtime_abliterate_export() {
    let root = clean_core_fixture();
    write_file(
        root.path().join("src/model_runtime/mod.rs"),
        "pub mod abliterate;\n",
    );

    let violations = assert_abliteration_offline_invariant_at(root.path())
        .expect_err("model_runtime must not export an abliterate module");

    assert!(
        violations.iter().any(
            |violation| violation.contains("model_runtime") && violation.contains("abliterate")
        ),
        "expected model_runtime export violation: {violations:?}"
    );
}

#[test]
fn abliteration_offline_invariant_tests_static_scan_refuses_vacuous_fixture() {
    let root = tempfile::tempdir().expect("tempdir");
    write_file(root.path().join("src/model_runtime/mod.rs"), "");
    write_file(
        root.path().join("src/distillation/mod.rs"),
        "pub mod abliterate;\n",
    );

    let violations = assert_abliteration_offline_invariant_at(root.path())
        .expect_err("missing hot-path files must not produce a vacuous pass");

    assert!(
        violations
            .iter()
            .any(|violation| violation.contains("no runtime generate or technique files")),
        "expected non-vacuous scan violation: {violations:?}"
    );
}

#[test]
fn abliteration_offline_invariant_tests_static_scan_requires_each_generate_path() {
    let root = clean_core_fixture();
    fs::remove_file(root.path().join("src/model_runtime/candle/generate.rs"))
        .expect("remove candle generate fixture");

    let violations = assert_abliteration_offline_invariant_at(root.path())
        .expect_err("missing required generate paths must fail closed");

    assert!(
        violations.iter().any(|violation| {
            violation.contains("missing required runtime hot-path file")
                && violation.contains("candle")
                && violation.contains("generate.rs")
        }),
        "expected missing candle generate violation: {violations:?}"
    );
}

#[test]
fn abliteration_offline_invariant_tests_model_runtime_rejects_abliteration_tool_engine_kind() {
    let error = LocalModelAdapterInvariant::assert_model_runtime_engine_kind(
        ProcessEngineKind::AbliterationTool,
    )
    .expect_err("AbliterationTool rows must never be accepted as ModelRuntime load rows");

    assert!(
        error.to_string().contains("offline-only"),
        "error should explain the offline-only invariant: {error}"
    );

    for allowed in [ProcessEngineKind::LlamaCpp, ProcessEngineKind::Candle] {
        LocalModelAdapterInvariant::assert_model_runtime_engine_kind(allowed)
            .unwrap_or_else(|error| panic!("{allowed:?} must be a regular model runtime: {error}"));
    }
}

#[test]
fn abliteration_offline_invariant_tests_reviewed_artifact_reregisters_as_regular_runtime_only() {
    let reviewed = reviewed_abliteration_registration();

    for allowed in [ProcessEngineKind::LlamaCpp, ProcessEngineKind::Candle] {
        LocalModelAdapterInvariant::assert_reviewed_abliteration_reregistration_engine_kind(
            &reviewed, allowed,
        )
        .unwrap_or_else(|error| {
            panic!("reviewed artifact must be accepted after regular {allowed:?} re-registration: {error}")
        });
    }

    for rejected in [
        ProcessEngineKind::AbliterationTool,
        ProcessEngineKind::ExternalCompat,
        ProcessEngineKind::SandboxContainer,
    ] {
        let error =
            LocalModelAdapterInvariant::assert_reviewed_abliteration_reregistration_engine_kind(
                &reviewed, rejected,
            )
            .expect_err(
                "reviewed abliterated artifacts must load only through regular local runtimes",
            );
        assert!(
            error.to_string().contains("re-register"),
            "error should explain re-registration requirement for {rejected:?}: {error}"
        );
    }
}

fn reviewed_abliteration_registration() -> AbliteratedSkillBankModelRegistration {
    AbliteratedSkillBankModelRegistration {
        action: SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION.to_string(),
        artifact_path: PathBuf::from("fixtures/models/reviewed-abliterated.gguf"),
        base_model_sha256: "a".repeat(64),
        refusal_direction_sha256: "b".repeat(64),
        license_tag: "Permissive-Test".to_string(),
    }
}

fn clean_core_fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    write_file(root.path().join("src/model_runtime/mod.rs"), "");
    write_file(
        root.path().join("src/distillation/mod.rs"),
        "pub mod abliterate;\n",
    );
    write_file(
        root.path().join("src/model_runtime/llama_cpp/generate.rs"),
        "pub fn generate() {}\n",
    );
    write_file(
        root.path().join("src/model_runtime/candle/generate.rs"),
        "pub fn generate() {}\n",
    );
    write_file(
        root.path()
            .join("src/model_runtime/techniques/refusal_vector.rs"),
        "pub fn refusal_vector() {}\n",
    );
    root
}

fn write_file(path: impl AsRef<Path>, contents: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!("create {}: {error}", parent.display());
        });
    }
    fs::write(path, contents).unwrap_or_else(|error| {
        panic!("write {}: {error}", path.display());
    });
}
