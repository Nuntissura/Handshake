use std::fs::File;
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};
use zip::write::FileOptions;

use crate::bundles::exporter::BundleExportError;
use crate::bundles::schemas::BundleManifest;

#[derive(Debug, Clone)]
pub struct BundleFileEntry {
    pub path: String,
    pub bytes: Vec<u8>,
    pub redacted: bool,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    hex::encode(hash)
}

/// Create a deterministic ZIP archive.
pub fn write_deterministic_zip(
    output_path: &Path,
    files: &[BundleFileEntry],
) -> Result<(), BundleExportError> {
    let file = File::create(output_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let timestamp = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
        .unwrap_or_else(|_| zip::DateTime::default());
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6))
        .last_modified_time(timestamp);

    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in sorted {
        writer.start_file(entry.path, options)?;
        writer.write_all(&entry.bytes)?;
    }

    writer.finish()?;
    Ok(())
}

pub fn compute_bundle_hash(manifest: &BundleManifest, files: &[(String, String)]) -> String {
    // manifest_without_hash: set bundle_hash to empty string for hashing
    let mut manifest_clone = manifest.clone();
    manifest_clone.bundle_hash = String::new();
    let manifest_json = serde_json::to_string(&manifest_clone).unwrap_or_else(|_| "{}".to_string());

    let mut hashes = files.to_vec();
    hashes.sort_by(|a, b| a.0.cmp(&b.0));
    let hashes_joined = hashes
        .iter()
        .map(|(_, h)| h.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    sha256_hex(format!("{}\n{}", manifest_json, hashes_joined).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn bundle_determinism_hash_stable() {
        let manifest = BundleManifest {
            schema_version: "1.0".to_string(),
            bundle_id: "test".to_string(),
            bundle_kind: "debug_bundle".to_string(),
            created_at: Utc::now(),
            scope: crate::bundles::schemas::ManifestScope {
                kind: crate::bundles::schemas::ScopeKind::Job,
                problem_id: None,
                job_id: Some("job-123".to_string()),
                time_range: None,
                wsid: None,
            },
            redaction_mode: crate::bundles::schemas::RedactionMode::SafeDefault,
            workflow_run_id: "wf-1".to_string(),
            job_id: "job-123".to_string(),
            exporter_version: "0.1.0".to_string(),
            platform: crate::bundles::schemas::PlatformInfo {
                os: "test".to_string(),
                arch: "test".to_string(),
                app_version: "0.1.0".to_string(),
                build_hash: "hash".to_string(),
            },
            files: Vec::new(),
            included: crate::bundles::schemas::IncludedCounts {
                job_count: 1,
                diagnostic_count: 0,
                event_count: 0,
            },
            missing_evidence: Vec::new(),
            bundle_hash: String::new(),
        };

        let files = vec![("a.txt".to_string(), sha256_hex(b"hello"))];
        let hash1 = compute_bundle_hash(&manifest, &files);
        let hash2 = compute_bundle_hash(&manifest, &files);
        assert_eq!(hash1, hash2, "bundle hash should be stable for same inputs");
    }

    #[test]
    fn write_zip_is_deterministic() -> Result<(), BundleExportError> {
        let dir = tempdir()?;
        let path = dir.path().join("bundle.zip");
        let files = vec![
            BundleFileEntry {
                path: "a.txt".to_string(),
                bytes: b"alpha".to_vec(),
                redacted: false,
            },
            BundleFileEntry {
                path: "b.txt".to_string(),
                bytes: b"beta".to_vec(),
                redacted: false,
            },
        ];
        write_deterministic_zip(&path, &files)?;
        let first = std::fs::read(&path)?;
        write_deterministic_zip(&path, &files)?;
        let second = std::fs::read(&path)?;
        assert_eq!(first, second, "zip output must be deterministic");
        Ok(())
    }
}
