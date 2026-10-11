//! WP-1 MT-014: backend-independent projection provenance guards.

use handshake_native::model_runtime_panel::PROJECTION_SCHEMA_ID;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const PROJECTION_PROVENANCE_SCHEMA_ID: &str = "hsk.mt014_model_runtime_projection_provenance@1";
const MAX_PRODUCER_AGE_MS: u64 = 30 * 60 * 1_000;

#[derive(Clone, Debug, Deserialize)]
struct ModelRuntimeProjectionProvenance {
    schema_id: String,
    proof_nonce: String,
    projection_schema_id: String,
    artifact_sha256: String,
    producer_test_id: String,
    producer_status: String,
    producer_completed_at_unix_ms: u64,
}

fn validate_configured_artifact_path(
    configured: &std::path::Path,
    expected: &std::path::Path,
    variable: &str,
) -> Result<(), String> {
    let configured = match (configured.parent(), configured.file_name()) {
        (Some(parent), Some(file_name)) => std::fs::canonicalize(parent)
            .map(|parent| parent.join(file_name))
            .unwrap_or_else(|_| configured.to_path_buf()),
        _ => configured.to_path_buf(),
    };
    if configured != expected {
        return Err(format!(
            "{variable} must name canonical artifact {}; got {}",
            expected.display(),
            configured.display()
        ));
    }
    Ok(())
}

fn validate_projection_provenance(
    provenance: &ModelRuntimeProjectionProvenance,
    artifact_bytes: &[u8],
    expected_nonce: &str,
    projection_schema_id: &str,
    now_unix_ms: u64,
) -> Result<(), String> {
    if provenance.schema_id != PROJECTION_PROVENANCE_SCHEMA_ID {
        return Err(format!(
            "unsupported provenance schema `{}`",
            provenance.schema_id
        ));
    }
    if provenance.proof_nonce != expected_nonce {
        return Err("projection provenance nonce does not match this producer run".to_owned());
    }
    if provenance.projection_schema_id != projection_schema_id {
        return Err("projection schema does not match producer provenance".to_owned());
    }
    if provenance.producer_test_id
        != "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256"
    {
        return Err("projection provenance names an unexpected producer".to_owned());
    }
    if provenance.producer_status != "passed_all_backend_assertions" {
        return Err("projection producer did not complete every backend assertion".to_owned());
    }
    if provenance.producer_completed_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err("projection provenance timestamp is in the future".to_owned());
    }
    if now_unix_ms.saturating_sub(provenance.producer_completed_at_unix_ms) > MAX_PRODUCER_AGE_MS {
        return Err("projection provenance is stale".to_owned());
    }
    let actual_sha256 = format!("{:x}", Sha256::digest(artifact_bytes));
    if provenance.artifact_sha256 != actual_sha256 {
        return Err("projection bytes do not match producer provenance".to_owned());
    }
    Ok(())
}

#[test]
fn mt014_projection_provenance_rejects_stale_nonce_root_status_and_hash() {
    let bytes = br#"{"schema_id":"hsk.model_runtime_registry_projection@3"}"#;
    let now = 2_000_000_000_u64;
    let mut provenance = ModelRuntimeProjectionProvenance {
        schema_id: PROJECTION_PROVENANCE_SCHEMA_ID.to_owned(),
        proof_nonce: "current-run".to_owned(),
        projection_schema_id: PROJECTION_SCHEMA_ID.to_owned(),
        artifact_sha256: format!("{:x}", Sha256::digest(bytes)),
        producer_test_id:
            "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256".to_owned(),
        producer_status: "passed_all_backend_assertions".to_owned(),
        producer_completed_at_unix_ms: now - 1_000,
    };
    validate_projection_provenance(&provenance, bytes, "current-run", PROJECTION_SCHEMA_ID, now)
        .expect("fresh matching producer provenance is accepted");

    provenance.proof_nonce = "old-run".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("nonce"));
    provenance.proof_nonce = "current-run".to_owned();

    provenance.producer_completed_at_unix_ms = now - MAX_PRODUCER_AGE_MS - 1;
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("stale"));
    provenance.producer_completed_at_unix_ms = now - 1_000;

    provenance.producer_status = "failed_before_publish".to_owned();
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("did not complete"));
    provenance.producer_status = "passed_all_backend_assertions".to_owned();

    provenance.artifact_sha256 = "0".repeat(64);
    assert!(validate_projection_provenance(
        &provenance,
        bytes,
        "current-run",
        PROJECTION_SCHEMA_ID,
        now
    )
    .unwrap_err()
    .contains("bytes"));

    assert!(validate_configured_artifact_path(
        std::path::Path::new("D:/wrong-root/projection.json"),
        std::path::Path::new("D:/canonical-root/projection.json"),
        "HANDSHAKE_MT014_PROJECTION_ARTIFACT"
    )
    .unwrap_err()
    .contains("canonical artifact"));
}

