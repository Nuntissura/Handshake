use std::collections::BTreeSet;
use std::io::Write;
use std::process::Command;

use handshake_core::bundles::redactor::SecretRedactor;
use handshake_core::bundles::schemas::RedactionMode;
use handshake_core::bundles::validator::ValBundleValidator;
use handshake_core::dependency_policy::source_tripwires::scan_source_text;
use handshake_core::dependency_policy::{
    assert_source_tripwire_policy_for_files, repo_root_from_manifest_dir,
    RuntimeDependencyAllowlist,
};
use handshake_core::knowledge_ingestion::paths::normalize_source_relative_path;
use handshake_core::knowledge_ingestion::secrets::{
    redact_text as redact_ingested_text, scan_text as scan_ingested_text,
};
use handshake_core::storage::{ControlPlaneStorageConfig, StorageError};
use serde_json::json;

fn allowlist() -> RuntimeDependencyAllowlist {
    RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
        .expect("runtime dependency allowlist loads")
}

#[test]
fn mt_225_redacts_secrets_from_bundle_wiki_log_and_usermanual_payloads() {
    let github_suffix_22 = "A1b2C3d4E5f6G7h8I9j0K1";
    let github_suffix_59 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456";
    let github_pat = format!("github_pat_{github_suffix_22}_{github_suffix_59}");
    let slack_app_token = "xapp-1-A012BCDEFGH-1234567890-abcdef0123456789";
    let credential_url = "postgres://svc_user:superpostgrespass@db.internal:5432/app";
    let payload = json!({
        "debug_bundle": {
            "trace_jsonl": format!("cloud helper saw token {github_pat}")
        },
        "generated_wiki_page": format!("compiled page references {slack_app_token}"),
        "log_like_output": format!("worker emitted {credential_url}"),
        "user_manual_projection": "setup note: password = q7Xz2pLm9KvR4tNw8YbD3cFgH6sJaUeP"
    });

    let (redacted, logs) =
        SecretRedactor::new().redact_value(&payload, RedactionMode::SafeDefault, "$");
    let rendered = serde_json::to_string(&redacted).expect("redacted payload serializes");

    for raw in [
        github_pat.as_str(),
        github_suffix_59,
        slack_app_token,
        credential_url,
        "superpostgrespass",
        "q7Xz2pLm9KvR4tNw8YbD3cFgH6sJaUeP",
    ] {
        assert!(
            !rendered.contains(raw),
            "raw secret leaked into redacted output: {raw}"
        );
    }
    assert!(
        logs.iter()
            .any(|entry| entry.location.contains("debug_bundle")),
        "bundle redaction should be logged with payload location: {logs:?}"
    );
    assert!(
        logs.iter()
            .any(|entry| entry.location.contains("user_manual_projection")),
        "UserManual projection redaction should be logged: {logs:?}"
    );

    let source_text = format!(
        "pub const TOKEN: &str = \"{github_pat}\";\n\
         pub const DATABASE_URL: &str = \"{credential_url}\";\n"
    );
    let ingestion_report = scan_ingested_text(&source_text);
    assert!(
        !ingestion_report.is_clean(),
        "source-ingestion preflight must detect shaped secret fixtures"
    );
    let ingested_redacted = redact_ingested_text(&source_text, &ingestion_report);
    for raw in [github_pat.as_str(), credential_url, "superpostgrespass"] {
        assert!(
            !ingested_redacted.contains(raw),
            "source-ingestion redaction leaked raw secret: {raw}"
        );
    }
}

#[test]
fn mt_226_machine_local_paths_are_rejected_or_redacted_for_portable_outputs() {
    for bad_path in [
        "C:/Users/Ilja/Desktop/handshake/secrets.rs",
        "D:\\Projects\\Handshake\\local-cache\\index.json",
        "C:\\",
        "C:/",
        "/home/ilja/handshake/.env",
        "/tmp",
        "/tmp/",
        "/tmp/x",
        "/tmp/handshake/x.env",
        "/var/cache/handshake/token.txt",
        "/var/lib/handshake/x.env",
        "/var/log/handshake/secrets.log",
        "/var/tmp/handshake/cache/token.txt",
    ] {
        assert!(
            normalize_source_relative_path(bad_path).is_err(),
            "ingestion path normalizer must reject machine-local path authority: {bad_path}"
        );
    }

    let payload = json!({
        "wiki": "source path C:/Users/Ilja/Desktop/handshake/secrets.rs",
        "manual": "workspace root /home/ilja/handshake/.env",
        "bundle": "artifact D:\\Projects\\Handshake\\local-cache\\index.json",
        "drive_root_backslash": "drive root C:\\",
        "drive_root_forward": "drive root C:/",
        "drive_root_assignment": "assigned path=C:\\",
        "spaced_windows": "artifact D:\\Projects\\LLM projects\\Handshake\\secret.txt",
        "comma_windows": "artifact D:\\Projects\\Foo, Inc\\secret.txt",
        "semicolon_windows": "artifact D:\\Projects\\Foo; Inc\\secret.txt",
        "paren_windows": "(D:\\Projects\\Foo, Inc\\secret.txt)",
        "bracket_windows": "[D:\\Projects\\Foo; Inc\\secret.txt]",
        "angle_windows": "<D:\\Projects\\Foo\\secret.txt>",
        "backtick_windows": "`D:\\Projects\\Foo\\secret.txt`",
        "comma_prefixed_windows": "csv-field,D:\\Projects\\Foo\\secret.txt",
        "semicolon_prefixed_windows": "semicolon-field;D:\\Projects\\Foo\\secret.txt",
        "paren_unc": "(\\\\server\\share\\Foo, Inc\\secret.txt)",
        "paren_tmp": "(/tmp/handshake/secret.env)",
        "bracket_var": "[/var/log/handshake/secrets.log]",
        "tmp_root": "temp root /tmp",
        "tmp_root_slash": "temp root /tmp/",
        "tmp_short": "temp path /tmp/x",
        "tmp": "temp credential path /tmp/handshake/x.env",
        "tmp_nested": "temp credential path /tmp/handshake/cache/token.txt",
        "varcache": "cache path /var/cache/handshake/token.txt",
        "varlib": "machine state path /var/lib/handshake/x.env",
        "varlog": "log path /var/log/handshake/secrets.log",
        "vartmp": "var tmp path /var/tmp/handshake/cache/token.txt",
        "tmp_assignment": "assigned path=/tmp/handshake/assigned.env",
        "var_assignment": "assigned log_path=/var/log/handshake/assigned.log",
        "prefix_control": "not machine-local paths: /tmpfile /tmp-cache /various/cache /homework"
    });
    let (redacted, logs) =
        SecretRedactor::new().redact_value(&payload, RedactionMode::SafeDefault, "$");
    let rendered = serde_json::to_string(&redacted).expect("redacted payload serializes");

    for raw in [
        "C:/Users/Ilja/Desktop",
        "C:\\",
        "C:/",
        "path=C:\\",
        "/home/ilja/handshake",
        "D:\\Projects\\Handshake",
        "D:\\Projects\\LLM projects\\Handshake",
        "projects\\Handshake\\secret.txt",
        "D:\\Projects\\Foo, Inc\\secret.txt",
        ", Inc\\secret.txt",
        "D:\\Projects\\Foo; Inc\\secret.txt",
        "; Inc\\secret.txt",
        "D:\\Projects\\Foo\\secret.txt",
        "\\\\server\\share\\Foo, Inc\\secret.txt",
        "server\\share\\Foo, Inc\\secret.txt",
        "/tmp/handshake/secret.env",
        "/var/log/handshake/secrets.log",
        "/tmp\"",
        "/tmp/\"",
        "/tmp/x",
        "/tmp/handshake",
        "/var/cache/handshake",
        "/var/lib/handshake",
        "/var/log/handshake",
        "/var/tmp/handshake",
        "path=/tmp/handshake",
        "log_path=/var/log/handshake",
    ] {
        assert!(
            !rendered.contains(raw),
            "machine-local path leaked into portable output: {raw}"
        );
    }
    assert!(
        logs.iter().any(|entry| entry.category == "path"),
        "expected path redaction log entry: {logs:?}"
    );
    for safe_prefix in ["/tmpfile", "/tmp-cache", "/various/cache", "/homework"] {
        assert!(
            rendered.contains(safe_prefix),
            "path redactor should not consume non-path prefix controls: {rendered}"
        );
    }
}

#[test]
fn mt_226_safe_default_bundle_validator_rejects_windows_drive_roots() {
    let dir = tempfile::tempdir().expect("temp bundle dir");
    let coder_prompt = "C:\\\npath=C:\\";
    std::fs::write(dir.path().join("coder_prompt.md"), coder_prompt)
        .expect("write leaked prompt fixture");

    let manifest = json!({
        "schema_version": "1.0",
        "bundle_id": "018f8b4d-6e86-7000-8000-000000000226",
        "bundle_kind": "debug_bundle",
        "created_at": "2026-06-12T00:00:00Z",
        "scope": {
            "kind": "workspace",
            "wsid": "ws-test"
        },
        "redaction_mode": "SAFE_DEFAULT",
        "workflow_run_id": "run-test",
        "job_id": "job-test",
        "exporter_version": "test",
        "platform": {
            "os": "windows",
            "arch": "x86_64",
            "app_version": "test",
            "build_hash": "build"
        },
        "files": [{
            "path": "coder_prompt.md",
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_bytes": coder_prompt.len(),
            "redacted": false
        }],
        "included": {
            "job_count": 0,
            "diagnostic_count": 0,
            "event_count": 0
        },
        "missing_evidence": [],
        "bundle_hash": ""
    });
    std::fs::write(
        dir.path().join("bundle_manifest.json"),
        serde_json::to_vec(&manifest).expect("manifest serializes"),
    )
    .expect("write manifest fixture");

    let report = ValBundleValidator
        .validate_dir(dir.path())
        .expect("bundle validator runs");
    assert!(
        report.findings.iter().any(|finding| {
            finding.code == "VAL-REDACT-001"
                && finding
                    .message
                    .contains("detector `path_absolute` matched in `coder_prompt.md`")
        }),
        "SAFE_DEFAULT validator must reject bare Windows drive roots: {:?}",
        report.findings
    );
}

#[test]
fn mt_226_safe_default_bundle_validator_rejects_rooted_var_and_tmp_paths() {
    let dir = tempfile::tempdir().expect("temp bundle dir");
    let coder_prompt =
        "inspect /var/log/handshake/secrets.log and /var/tmp/handshake/cache/token.txt and /tmp/handshake/x.env and path=/tmp/handshake/assigned.env plus wrapped (D:\\Projects\\Foo, Inc\\secret.txt), [D:\\Projects\\Foo; Inc\\secret.txt], <D:\\Projects\\Foo\\secret.txt>, `D:\\Projects\\Foo\\secret.txt`, (/tmp/handshake/secret.env), [/var/log/handshake/secrets.log], csv-field,D:\\Projects\\Foo\\secret.txt, and semicolon-field;D:\\Projects\\Foo\\secret.txt";
    std::fs::write(dir.path().join("coder_prompt.md"), coder_prompt)
        .expect("write leaked prompt fixture");

    let manifest = json!({
        "schema_version": "1.0",
        "bundle_id": "018f8b4d-6e86-7000-8000-000000000226",
        "bundle_kind": "debug_bundle",
        "created_at": "2026-06-12T00:00:00Z",
        "scope": {
            "kind": "workspace",
            "wsid": "ws-test"
        },
        "redaction_mode": "SAFE_DEFAULT",
        "workflow_run_id": "run-test",
        "job_id": "job-test",
        "exporter_version": "test",
        "platform": {
            "os": "windows",
            "arch": "x86_64",
            "app_version": "test",
            "build_hash": "build"
        },
        "files": [{
            "path": "coder_prompt.md",
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_bytes": coder_prompt.len(),
            "redacted": false
        }],
        "included": {
            "job_count": 0,
            "diagnostic_count": 0,
            "event_count": 0
        },
        "missing_evidence": [],
        "bundle_hash": ""
    });
    std::fs::write(
        dir.path().join("bundle_manifest.json"),
        serde_json::to_vec(&manifest).expect("manifest serializes"),
    )
    .expect("write manifest fixture");

    let report = ValBundleValidator
        .validate_dir(dir.path())
        .expect("bundle validator runs");
    assert!(
        report.findings.iter().any(|finding| {
            finding.code == "VAL-REDACT-001"
                && finding
                    .message
                    .contains("detector `path_absolute` matched in `coder_prompt.md`")
        }),
        "SAFE_DEFAULT validator must reject rooted /var/... and /tmp/... path leaks: {:?}",
        report.findings
    );
}

#[test]
fn mt_226_safe_default_bundle_validator_rejects_bare_tmp_root_tokens() {
    let dir = tempfile::tempdir().expect("temp bundle dir");
    let coder_prompt = "inspect temp roots /tmp and /tmp/ before reading relative tmp/cache";
    std::fs::write(dir.path().join("coder_prompt.md"), coder_prompt)
        .expect("write leaked prompt fixture");

    let manifest = json!({
        "schema_version": "1.0",
        "bundle_id": "018f8b4d-6e86-7000-8000-000000000226",
        "bundle_kind": "debug_bundle",
        "created_at": "2026-06-12T00:00:00Z",
        "scope": {
            "kind": "workspace",
            "wsid": "ws-test"
        },
        "redaction_mode": "SAFE_DEFAULT",
        "workflow_run_id": "run-test",
        "job_id": "job-test",
        "exporter_version": "test",
        "platform": {
            "os": "windows",
            "arch": "x86_64",
            "app_version": "test",
            "build_hash": "build"
        },
        "files": [{
            "path": "coder_prompt.md",
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_bytes": coder_prompt.len(),
            "redacted": false
        }],
        "included": {
            "job_count": 0,
            "diagnostic_count": 0,
            "event_count": 0
        },
        "missing_evidence": [],
        "bundle_hash": ""
    });
    std::fs::write(
        dir.path().join("bundle_manifest.json"),
        serde_json::to_vec(&manifest).expect("manifest serializes"),
    )
    .expect("write manifest fixture");

    let report = ValBundleValidator
        .validate_dir(dir.path())
        .expect("bundle validator runs");
    assert!(
        report.findings.iter().any(|finding| {
            finding.code == "VAL-REDACT-001"
                && finding
                    .message
                    .contains("detector `path_absolute` matched in `coder_prompt.md`")
        }),
        "SAFE_DEFAULT validator must reject bare /tmp and /tmp/ path leaks: {:?}",
        report.findings
    );
}

#[test]
fn mt_227_missing_postgres_authority_fails_closed_without_sqlite_fallback() {
    let cases = [
        (None, None, None),
        (Some("postgres_primary"), Some("true"), None),
        (
            Some("postgres_primary"),
            Some("true"),
            Some("sqlite://tmp/cache.sqlite3"),
        ),
        (Some("sqlite"), None, Some("sqlite://tmp/cache.sqlite3")),
    ];

    for (mode, requires_postgres, database_url) in cases {
        let err = ControlPlaneStorageConfig::resolve(mode, requires_postgres, database_url)
            .expect_err("missing/non-PostgreSQL authority must fail closed");
        match err {
            StorageError::Validation(message) => {
                assert!(
                    message.contains("postgres") || message.contains("unsupported storage mode"),
                    "unexpected fail-closed message: {message}"
                );
            }
            other => panic!("expected validation failure, got {other:?}"),
        }
    }
}

#[test]
fn mt_228_no_sqlite_tripwire_rejects_storage_cache_fixture_and_temp_adapter_refs() {
    let allowlist = allowlist();
    let fixtures = [
        (
            "src/backend/handshake_core/src/storage/cache.rs",
            r#"let url = "sqlite://authority-cache.db";"#,
        ),
        (
            "src/backend/handshake_core/tests/fixtures/knowledge/cache_fixture.rs",
            r#"let fixture = "tmp/wp009-cache.sqlite3";"#,
        ),
        (
            "src/backend/handshake_core/examples/temp_adapter.rs",
            r#"struct SqliteDatabase; impl SqliteDatabase { fn open() {} }"#,
        ),
        (
            "src/backend/handshake_core/src/storage/sqlx_adapter.rs",
            r#"let pool: sqlx::SqlitePool = connect();"#,
        ),
        (
            "src/backend/handshake_core/src/storage/sqlx_type.rs",
            r#"use sqlx::Sqlite; fn bind(_: Sqlite) {}"#,
        ),
        (
            "src/backend/handshake_core/tests/fixtures/sqlite_pool_options.rs",
            r#"let _ = SqlitePoolOptions::new();"#,
        ),
        (
            "src/backend/handshake_core/tests/fixtures/sqlite_connect_options.rs",
            r#"let _ = SqliteConnectOptions::new();"#,
        ),
    ];

    for (path, content) in fixtures {
        let violations = scan_source_text(path, content, &allowlist);
        assert!(
            violations
                .iter()
                .any(|violation| violation.class_id == "sqlite"),
            "SQLite fixture must trip sqlite class for {path}: {violations:?}"
        );
    }
}

#[test]
fn mt_228_229_allowlist_declares_requested_source_tripwire_patterns() {
    let allowlist = allowlist();
    let required = [
        ("sqlite", "sqlx::Sqlite"),
        ("sqlite", "SqlitePool"),
        ("sqlite", "SqlitePoolOptions"),
        ("sqlite", "SqliteConnectOptions"),
        ("outside_app", "photoshop.exe"),
        ("outside_server_daemon", "ollama serve"),
        ("outside_server_daemon", "localhost:11434"),
        ("outside_server_daemon", "npm run dev"),
        ("outside_server_daemon", "localhost:5173"),
    ];

    for (class_id, pattern) in required {
        let class = allowlist
            .forbidden_class(class_id)
            .unwrap_or_else(|| panic!("forbidden class {class_id} missing"));
        assert!(
            class
                .source_scan_patterns
                .iter()
                .any(|candidate| candidate == pattern),
            "allowlist class {class_id} must declare source pattern {pattern:?}; got {:?}",
            class.source_scan_patterns
        );
    }
}

#[test]
fn mt_229_external_dependency_tripwire_rejects_unmanaged_proof_defaults() {
    let allowlist = allowlist();
    let fixtures = [
        (
            "src/backend/handshake_core/src/cache/defaults.rs",
            r#"const CACHE_URL: &str = "redis://127.0.0.1:6379/0";"#,
        ),
        (
            "src/backend/handshake_core/src/proof/default_runtime.rs",
            r#"let proof = "docker compose up external-validator";"#,
        ),
        (
            "app/src/components/editor/defaultWorker.ts",
            r#"const worker = "https://cdn.jsdelivr.net/npm/monaco-editor/min/vs/editor/editor.worker.js";"#,
        ),
        (
            "app/src/integrations/photoshopProof.ts",
            r#"const app = "photoshop.exe";"#,
        ),
        (
            "src/backend/handshake_core/src/model_runtime/defaults.rs",
            r#"let cmd = "ollama serve"; let url = "http://localhost:11434";"#,
        ),
        (
            "app/src/devServerProof.ts",
            r#"const manual = "npm run dev"; const vite = "http://localhost:5173";"#,
        ),
    ];

    let mut classes = BTreeSet::new();
    for (path, content) in fixtures {
        for violation in scan_source_text(path, content, &allowlist) {
            classes.insert(violation.class_id);
        }
    }

    for expected in [
        "outside_server_daemon",
        "outside_app",
        "docker_default",
        "cdn_runtime_asset",
    ] {
        assert!(
            classes.contains(expected),
            "external dependency class {expected} should be rejected, got {classes:?}"
        );
    }
}

#[test]
fn mt_228_229_source_tripwires_are_wired_into_dependency_policy_check_path() {
    let repo_root = repo_root_from_manifest_dir();
    assert_source_tripwire_policy_for_files(
        &repo_root,
        [
            repo_root.join("src/backend/handshake_core/src/bundles/redactor.rs"),
            repo_root.join("src/backend/handshake_core/src/bundles/validator.rs"),
        ],
    )
    .expect("dependency-policy source tripwire gate must scan touched product files cleanly");

    let artifact_root = repo_root
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test");
    std::fs::create_dir_all(&artifact_root).expect("artifact test root exists");
    let mut fixture = tempfile::Builder::new()
        .prefix("mt-228-sqlite-source-")
        .suffix(".rs")
        .tempfile_in(&artifact_root)
        .expect("temp fixture under artifact root");
    write!(
        fixture,
        r#"fn adapter(pool: sqlx::SqlitePool) {{ let _ = pool; }}"#
    )
    .expect("write sqlite fixture");
    fixture.flush().expect("flush sqlite fixture");

    let err = assert_source_tripwire_policy_for_files(&repo_root, [fixture.path()])
        .expect_err("dependency-policy source tripwire gate must fail closed on real file hits");
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("sqlite") && rendered.contains("SqlitePool"),
        "file-scoped policy gate should report sqlite source pattern: {rendered}"
    );
}

#[test]
fn mt_228_229_node_dependency_policy_validator_missing_file_probe_fails_closed() {
    let repo_root = repo_root_from_manifest_dir();
    let missing = repo_root
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test")
        .join("mt-228-229-missing-source-probe.ts");

    let output = Command::new("node")
        .arg(repo_root.join("app/scripts/check-dependency-policy.mjs"))
        .arg("--skip-build")
        .arg("--source-tripwire-files")
        .arg(&missing)
        .current_dir(repo_root.join("app"))
        .output()
        .expect("node dependency-policy missing-file probe runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "missing explicit source fixture must fail closed; stdout={stdout}; stderr={stderr}"
    );

    let rendered = stdout.to_ascii_lowercase();
    assert!(
        rendered.contains("file_read_errors"),
        "missing explicit source fixture must report file_read_errors; stdout={stdout}; stderr={stderr}"
    );
    assert!(
        rendered.contains("mt-228-229-missing-source-probe.ts"),
        "missing explicit source fixture path should be reported; stdout={stdout}; stderr={stderr}"
    );
}

#[test]
fn mt_228_229_node_dependency_policy_validator_file_probe_fails_closed() {
    let repo_root = repo_root_from_manifest_dir();
    let artifact_root = repo_root
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test");
    std::fs::create_dir_all(&artifact_root).expect("artifact test root exists");
    let mut fixture = tempfile::Builder::new()
        .prefix("mt-228-229-node-source-")
        .suffix(".ts")
        .tempfile_in(&artifact_root)
        .expect("temp fixture under artifact root");
    write!(
        fixture,
        "const sqlite = 'sqlx::Sqlite SqlitePool SqlitePoolOptions SqliteConnectOptions';\n\
         const outsideApp = 'photoshop.exe';\n\
         const daemon = 'ollama serve http://localhost:11434';\n\
         const devServer = 'npm run dev http://localhost:5173';\n"
    )
    .expect("write dependency-policy fixture");
    fixture.flush().expect("flush dependency-policy fixture");

    let output = Command::new("node")
        .arg(repo_root.join("app/scripts/check-dependency-policy.mjs"))
        .arg("--skip-build")
        .arg("--source-tripwire-files")
        .arg(fixture.path())
        .current_dir(repo_root.join("app"))
        .output()
        .expect("node dependency-policy probe runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "forbidden source fixture must fail closed; stdout={stdout}; stderr={stderr}"
    );

    let rendered = stdout.to_ascii_lowercase();
    for expected in [
        "forbidden-source-tripwire",
        "sqlitepool",
        "sqlitepooloptions",
        "sqliteconnectoptions",
        "photoshop.exe",
        "ollama serve",
        "localhost:11434",
        "npm run dev",
        "localhost:5173",
    ] {
        assert!(
            rendered.contains(&expected.to_ascii_lowercase()),
            "dependency-policy validator probe must report {expected}; stdout={stdout}; stderr={stderr}"
        );
    }
}

#[test]
fn mt_228_229_node_dependency_policy_validator_file_probe_scans_path_names() {
    let repo_root = repo_root_from_manifest_dir();
    let artifact_root = repo_root
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test");
    std::fs::create_dir_all(&artifact_root).expect("artifact root exists");
    let fixture = artifact_root.join("mt-228-229-SqlitePool-path-only-probe.ts");
    std::fs::write(
        &fixture,
        "export const harmless = 'content does not name the forbidden adapter';\n",
    )
    .expect("write path-only source tripwire fixture");

    let output = Command::new("node")
        .arg(repo_root.join("app/scripts/check-dependency-policy.mjs"))
        .arg("--skip-build")
        .arg("--source-tripwire-files")
        .arg(&fixture)
        .current_dir(repo_root.join("app"))
        .output()
        .expect("node dependency-policy path-only probe runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "path-only forbidden source fixture must fail closed; stdout={stdout}; stderr={stderr}"
    );
    assert!(
        stdout.contains("SqlitePool") || stdout.to_ascii_lowercase().contains("sqlitepool"),
        "path-only source fixture should report the forbidden pattern; stdout={stdout}; stderr={stderr}"
    );
    assert!(
        stdout.contains("mt-228-229-SqlitePool-path-only-probe.ts"),
        "path-only source fixture path should be reported; stdout={stdout}; stderr={stderr}"
    );
}
