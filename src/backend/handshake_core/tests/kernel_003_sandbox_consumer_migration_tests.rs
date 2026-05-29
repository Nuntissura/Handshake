use std::{fs, path::PathBuf};

use serde_json::Value;

const MIGRATION_LOG: &str =
    ".GOV/roles_shared/records/sandbox/WP-KERNEL-004-kernel-003-callsite-migration.json";

#[test]
fn mt054_migration_log_records_kernel_003_sandbox_consumer_audit() {
    let repo = repo_root();
    let log_path = repo.join(MIGRATION_LOG);
    let log_text = fs::read_to_string(&log_path).expect("MT-054 migration log must exist");
    let log: Value = serde_json::from_str(&log_text).expect("migration log must be valid json");

    assert_eq!(log["schema"], "hsk.sandbox_callsite_migration@1");
    assert_eq!(
        log["wp_id"],
        "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1"
    );
    assert_eq!(log["mt_id"], "MT-054");
    assert_eq!(
        log["source_scan"]["forbidden_patterns"],
        serde_json::json!([
            "DocketAdapter",
            "docket_adapter",
            "DockerRunner",
            "docker_runner"
        ])
    );
    assert!(
        log["changes"]
            .as_array()
            .is_some_and(|changes| !changes.is_empty()),
        "migration log must record at least one audited KERNEL-003 consumer or guard change"
    );
}

#[test]
fn mt054_kernel_and_storage_sources_have_no_direct_concrete_docker_consumers() {
    let repo = repo_root();
    let mut violations = Vec::new();

    for relative_root in [
        "src/backend/handshake_core/src/kernel",
        "src/backend/handshake_core/src/storage",
    ] {
        collect_forbidden_hits(&repo.join(relative_root), relative_root, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "KERNEL-003 consumers must not import or instantiate concrete Docker/Docket surfaces outside src/sandbox/docker: {violations:#?}"
    );
}

fn collect_forbidden_hits(root: &PathBuf, relative_root: &str, violations: &mut Vec<String>) {
    let entries = fs::read_dir(root).unwrap_or_else(|error| {
        panic!("failed to read source root {relative_root}: {error}");
    });
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            let nested_relative = format!(
                "{}/{}",
                relative_root,
                path.file_name().expect("dir name").to_string_lossy()
            );
            collect_forbidden_hits(&path, &nested_relative, violations);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("source file");
        for (line_index, line) in text.lines().enumerate() {
            if is_forbidden_direct_consumer_line(line) {
                violations.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(repo_root())
                        .unwrap_or(path.as_path())
                        .to_string_lossy()
                        .replace('\\', "/"),
                    line_index + 1,
                    line.trim()
                ));
            }
        }
    }
}

fn is_forbidden_direct_consumer_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("//!") {
        return false;
    }
    [
        "use crate::sandbox::docker::DocketAdapter",
        "use crate::sandbox::docker::DockerRunner",
        "DocketAdapter::",
        "DockerRunner::",
        "Arc<DocketAdapter>",
        "Arc<DockerRunner>",
        "Box<DocketAdapter>",
        "Box<DockerRunner>",
        "docket_adapter",
        "docker_runner",
    ]
    .iter()
    .any(|pattern| line.contains(pattern))
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}
