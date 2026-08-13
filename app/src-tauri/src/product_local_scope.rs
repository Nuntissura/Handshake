//! Stable WP-local resource attribution for the single-user Tauri runtime.
//!
//! This is deliberately not authentication or authorization semantics from
//! WP-KERNEL-006/007. It is an opaque, persisted five-field isolation key used
//! by the existing WP-1 application-layer privacy boundary until those WPs own
//! the identifiers. Renderer IPC never supplies or mutates it.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const HANDSHAKE_PRODUCT_LOCAL_RESOURCE_SCOPE_JSON_ENV: &str =
    "HANDSHAKE_PRODUCT_LOCAL_RESOURCE_SCOPE_JSON";
const PRODUCT_LOCAL_SCOPE_SCHEMA_VERSION: u32 = 1;
const PRODUCT_LOCAL_SCOPE_FILE: &str = "product_local_resource_scope.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductLocalScopeDocument {
    schema_version: u32,
    scope: StrictExactResourceScopeAttribution,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictExactResourceScopeAttribution {
    owner_account_id: OwnerAccountId,
    actor_principal_id: ActorPrincipalId,
    authenticated_session_id: AuthenticatedSessionRef,
    access_space_id: AccessSpaceRef,
    workspace_id: WorkspaceScopeRef,
}

impl From<&ExactResourceScopeAttribution> for StrictExactResourceScopeAttribution {
    fn from(scope: &ExactResourceScopeAttribution) -> Self {
        Self {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

impl From<StrictExactResourceScopeAttribution> for ExactResourceScopeAttribution {
    fn from(scope: StrictExactResourceScopeAttribution) -> Self {
        Self {
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }
}

pub(crate) fn load_or_init_product_local_scope(
    app_data_root: &Path,
) -> Result<ExactResourceScopeAttribution, String> {
    let path = app_data_root.join(PRODUCT_LOCAL_SCOPE_FILE);
    match fs::read(&path) {
        Ok(bytes) => return decode_scope_document(&path, &bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "failed to read product-local resource scope at {}: {error}",
                path.display()
            ))
        }
    }

    let scope = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("tauri-workspace-{}", Uuid::now_v7()))
            .map_err(|error| error.to_string())?,
    };
    let document = ProductLocalScopeDocument {
        schema_version: PRODUCT_LOCAL_SCOPE_SCHEMA_VERSION,
        scope: StrictExactResourceScopeAttribution::from(&scope),
    };
    if persist_scope_document(&path, &document)? {
        Ok(scope)
    } else {
        let bytes = fs::read(&path).map_err(|error| {
            format!(
                "failed to read concurrently installed product-local scope at {}: {error}",
                path.display()
            )
        })?;
        decode_scope_document(&path, &bytes)
    }
}

pub(crate) fn resource_scope_from_exact(scope: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
        .with_session(scope.authenticated_session_id)
        .with_access_space(scope.access_space_id)
        .with_workspace(scope.workspace_id.clone())
}

pub(crate) fn serialize_product_local_scope_handoff(
    scope: &ExactResourceScopeAttribution,
) -> Result<String, String> {
    let document = ProductLocalScopeDocument {
        schema_version: PRODUCT_LOCAL_SCOPE_SCHEMA_VERSION,
        scope: StrictExactResourceScopeAttribution::from(scope),
    };
    serde_json::to_string(&document)
        .map_err(|error| format!("failed to serialize product-local scope handoff: {error}"))
}

fn decode_scope_document(
    path: &Path,
    bytes: &[u8],
) -> Result<ExactResourceScopeAttribution, String> {
    let document: ProductLocalScopeDocument = serde_json::from_slice(bytes).map_err(|error| {
        scope_recovery_error(path, &format!("strict JSON decode failed: {error}"))
    })?;
    if document.schema_version != PRODUCT_LOCAL_SCOPE_SCHEMA_VERSION {
        return Err(scope_recovery_error(
            path,
            &format!(
                "unsupported schema_version {}; expected {}",
                document.schema_version, PRODUCT_LOCAL_SCOPE_SCHEMA_VERSION
            ),
        ));
    }
    let scope = ExactResourceScopeAttribution::from(document.scope);
    validate_exact_scope(path, &scope)?;
    Ok(scope)
}

fn validate_exact_scope(path: &Path, scope: &ExactResourceScopeAttribution) -> Result<(), String> {
    for (name, value) in [
        ("owner_account_id", scope.owner_account_id.as_uuid()),
        ("actor_principal_id", scope.actor_principal_id.as_uuid()),
        (
            "authenticated_session_id",
            scope.authenticated_session_id.as_uuid(),
        ),
        ("access_space_id", scope.access_space_id.as_uuid()),
    ] {
        if value.is_nil() {
            return Err(scope_recovery_error(
                path,
                &format!("{name} must be a non-nil UUID"),
            ));
        }
    }
    if scope.workspace_id.as_str().trim().is_empty() {
        return Err(scope_recovery_error(path, "workspace_id must not be empty"));
    }
    Ok(())
}

fn scope_recovery_error(path: &Path, reason: &str) -> String {
    format!(
        "product-local resource scope at {} is invalid ({reason}); recovery: restore this file from backup, or explicitly remove it only after acknowledging that existing scoped sessions, templates, and telemetry will become inaccessible",
        path.display()
    )
}

fn persist_scope_document(
    path: &Path,
    document: &ProductLocalScopeDocument,
) -> Result<bool, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create product-local scope directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| format!("failed to serialize product-local scope: {error}"))?;
    let temp_path = scope_temp_path(path);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp_path).map_err(|error| {
        format!(
            "failed to create product-local scope temp file {}: {error}",
            temp_path.display()
        )
    })?;
    let write_result = (|| -> Result<bool, String> {
        file.write_all(&bytes)
            .map_err(|error| format!("failed to write product-local scope: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to fsync product-local scope: {error}"))?;
        install_scope_file(&temp_path, path)
    })();
    let _ = fs::remove_file(&temp_path);
    write_result
}

#[cfg(windows)]
fn install_scope_file(temp_path: &Path, path: &Path) -> Result<bool, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    match unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
    } {
        Ok(()) => Ok(true),
        // No REPLACE flag is supplied. If a concurrent bootstrap installed the
        // destination first, its exact scope is authoritative and the caller
        // must read it. Any corrupt winner is decoded fail-closed by the caller.
        Err(_) if path.exists() => Ok(false),
        Err(error) => Err(format!(
            "failed to atomically install product-local scope: {error}"
        )),
    }
}

#[cfg(not(windows))]
fn install_scope_file(temp_path: &Path, path: &Path) -> Result<bool, String> {
    match fs::hard_link(temp_path, path) {
        Ok(()) => {
            let parent = path.parent().ok_or_else(|| {
                format!(
                    "product-local scope path {} has no parent directory",
                    path.display()
                )
            })?;
            fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| {
                    format!(
                        "failed to fsync product-local scope directory {}: {error}",
                        parent.display()
                    )
                })?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(format!(
            "failed to atomically install product-local scope: {error}"
        )),
    }
}

fn scope_temp_path(path: &Path) -> PathBuf {
    path.with_extension(format!(
        "json.tmp.{}.{}",
        std::process::id(),
        Uuid::now_v7()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_local_scope_persists_exactly_across_restart() {
        let root = tempfile::tempdir().expect("tempdir");
        let first = load_or_init_product_local_scope(root.path()).expect("first bootstrap");
        let second = load_or_init_product_local_scope(root.path()).expect("restart bootstrap");
        assert_eq!(first, second);
        assert!(root.path().join(PRODUCT_LOCAL_SCOPE_FILE).is_file());
    }

    #[test]
    fn concurrent_first_bootstraps_share_one_durable_scope() {
        let root = tempfile::tempdir().expect("tempdir");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(9));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let root = root.path().to_path_buf();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                load_or_init_product_local_scope(&root).expect("concurrent bootstrap")
            }));
        }
        barrier.wait();
        let scopes = workers
            .into_iter()
            .map(|worker| worker.join().expect("bootstrap worker"))
            .collect::<Vec<_>>();
        assert!(scopes.windows(2).all(|pair| pair[0] == pair[1]));

        let after_restart =
            load_or_init_product_local_scope(root.path()).expect("restart bootstrap");
        assert_eq!(scopes[0], after_restart);
        assert_eq!(
            fs::read_dir(root.path())
                .expect("scope directory")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
                .count(),
            0,
            "all losing bootstrap temp files must be removed"
        );
    }

    #[test]
    fn corrupt_product_local_scope_fails_closed_with_recovery() {
        let root = tempfile::tempdir().expect("tempdir");
        fs::write(root.path().join(PRODUCT_LOCAL_SCOPE_FILE), b"{broken")
            .expect("write corrupt scope");
        let error = load_or_init_product_local_scope(root.path()).expect_err("must fail closed");
        assert!(error.contains("restore this file from backup"));
        assert!(error.contains("existing scoped sessions"));
    }

    #[test]
    fn incomplete_or_unknown_scope_document_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join(PRODUCT_LOCAL_SCOPE_FILE);
        fs::write(
            &path,
            br#"{"schema_version":1,"scope":{"owner_account_id":"00000000-0000-0000-0000-000000000000"},"unexpected":true}"#,
        )
        .expect("write invalid scope");
        let error = load_or_init_product_local_scope(root.path()).expect_err("must reject");
        assert!(error.contains("strict JSON decode failed"));
    }

    #[test]
    fn handoff_uses_the_same_strict_versioned_exact_scope_schema() {
        let root = tempfile::tempdir().expect("tempdir");
        let expected = load_or_init_product_local_scope(root.path()).expect("bootstrap");
        let encoded = serialize_product_local_scope_handoff(&expected).expect("serialize handoff");
        let decoded = decode_scope_document(Path::new("handoff-env"), encoded.as_bytes())
            .expect("decode handoff");
        assert_eq!(decoded, expected);

        let mut value: serde_json::Value = serde_json::from_str(&encoded).expect("handoff JSON");
        value["scope"]["unexpected"] = serde_json::Value::Bool(true);
        let error = decode_scope_document(
            Path::new("handoff-env"),
            serde_json::to_string(&value)
                .expect("mutated handoff")
                .as_bytes(),
        )
        .expect_err("nested unknown fields must be rejected");
        assert!(error.contains("strict JSON decode failed"));
    }
}
