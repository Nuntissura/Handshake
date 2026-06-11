//! WP-KERNEL-005 MT-074/MT-075 manual source rows: runtime proof.
//!
//! The model manual ships static `CommandReference` rows for the LLM
//! evidence-pack export (MT-074) and the backup/restore-preflight surface
//! (MT-075). `model_manual_tests.rs` already proves the rows exist; these
//! tests close the v2 concern by exercising the documented surfaces against
//! live Handshake-managed PostgreSQL, RE-READING the persisted rows, and
//! asserting the manual's documented schema ids, field names, ordering, and
//! error semantics match what the real store does (including EventLedger
//! evidence). No assertion below is satisfiable by manual text alone.
//!
//! Run, e.g.:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test atelier_manual_source_rows_pg_tests \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use handshake_core::atelier::exports::{
    build_llm_evidence_pack_manifest, export_event_family, BackupManifestFile,
    BackupRestorePreflightRequest, BackupRestorePreflightStatus, LlmEvidencePackFile,
    LlmEvidencePackFileKind, LlmEvidenceSourceAnchor, NewBackupManifest,
    BACKUP_MANIFEST_SCHEMA_ID, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID,
};
use handshake_core::atelier::{event_family, AtelierStore, NewMediaAsset};
use handshake_core::model_manual::{model_manual, CommandReference};
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every fixture test runs.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Look up a manual command row by stable id; the manual is the runtime
/// surface Diagnostics serves to no-context models.
fn manual_command(id: &str) -> &'static CommandReference {
    model_manual()
        .command_reference
        .iter()
        .find(|command| command.id == id)
        .unwrap_or_else(|| panic!("model manual must document command {id}"))
}

/// Assert every documented schema field names a real field of the runtime
/// value (top level, or one level down inside the named nested document).
fn assert_schema_fields_are_real(
    row: &CommandReference,
    value: &serde_json::Value,
    nested: &[&serde_json::Value],
) {
    for field in row.schema_fields {
        let found = value.get(field).is_some()
            || nested.iter().any(|doc| doc.get(field).is_some());
        assert!(
            found,
            "manual row {} documents schema field {field} that the runtime value does not carry",
            row.id
        );
    }
}

#[tokio::test]
async fn mt074_llm_evidence_pack_manual_row_documents_live_export_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP mt074_llm_evidence_pack_manual_row_documents_live_export_surface: no PostgreSQL"
        );
        return;
    };
    let store = connected_store(&url).await;
    let row = manual_command("atelier_build_llm_evidence_pack_manifest");
    assert_eq!(
        row.name, "exports::build_llm_evidence_pack_manifest",
        "manual row must name the real export surface"
    );

    // Persist the four pack-file payloads through the real store, then
    // RE-READ each from PostgreSQL and build the manifest from re-read values.
    let marker = format!("mt074-manual-{}", Uuid::new_v4().simple());
    let pack_kinds = [
        (LlmEvidencePackFileKind::Readme, "README.md", false),
        (LlmEvidencePackFileKind::Evidence, "evidence.json", true),
        (
            LlmEvidencePackFileKind::RedactionReport,
            "redactions.json",
            false,
        ),
        (
            LlmEvidencePackFileKind::SourceIndex,
            "source-index.json",
            false,
        ),
    ];
    let mut files = Vec::new();
    for (kind, pack_path, redaction_required) in pack_kinds {
        let artifact = atelier_pg_support::write_native_media_artifact(
            format!("{marker}-{pack_path}").as_bytes(),
        );
        store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(format!("{marker}-provenance")),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("persist evidence-pack file payload");
        let persisted = store
            .get_media_asset_by_hash(&artifact.content_hash)
            .await
            .expect("re-read evidence-pack file payload")
            .expect("evidence-pack file payload persisted");

        // The persistence path the manual documents emits EventLedger
        // evidence, not just a table row.
        assert_eq!(
            store
                .count_events_for_aggregate(
                    event_family::MEDIA_ASSET_MATERIALIZED,
                    "atelier_media_asset",
                    &persisted.content_hash,
                )
                .await
                .expect("count materialization event for pack-file payload"),
            1,
            "each evidence-pack payload write must land in the EventLedger"
        );

        files.push(LlmEvidencePackFile {
            kind,
            pack_path: pack_path.to_string(),
            artifact_ref: persisted.artifact_ref.clone(),
            content_hash: persisted.content_hash.clone(),
            byte_len: persisted.byte_len,
            source_anchors: vec![LlmEvidenceSourceAnchor {
                source_id: format!("{marker}-{}", kind.as_token()),
                source_path: format!("source-index/{marker}.json"),
                source_range: "lines:1-10".to_string(),
                content_hash: persisted.content_hash.clone(),
            }],
            redaction_required,
            redacted: redaction_required,
        });
    }

    let manifest = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        files.clone(),
    )
    .expect("build strict evidence-pack manifest from re-read PostgreSQL values");

    // The manual's expected_output must quote the schema id the runtime
    // manifest actually carries (not a hand-written approximation).
    assert_eq!(manifest.schema_id, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID);
    assert!(
        row.expected_output.contains(&manifest.schema_id),
        "manual expected_output must document the runtime schema id {}",
        manifest.schema_id
    );

    // The manual's recovery guidance must name every file the runtime
    // manifest requires, in the runtime's deterministic order.
    let runtime_paths: Vec<&str> = manifest
        .files
        .iter()
        .map(|file| file.pack_path.as_str())
        .collect();
    assert_eq!(
        runtime_paths,
        vec![
            "README.md",
            "evidence.json",
            "redactions.json",
            "source-index.json"
        ],
        "manifest files are in deterministic model-consumable order"
    );
    for pack_path in &runtime_paths {
        assert!(
            row.recovery_steps
                .iter()
                .any(|step| step.contains(pack_path)),
            "manual recovery steps must name required pack file {pack_path}"
        );
    }
    for file in &manifest.files {
        assert!(
            file.artifact_ref.starts_with("artifact://"),
            "evidence-pack files are ArtifactStore-backed, got {}",
            file.artifact_ref
        );
    }

    // Every documented schema field exists on the runtime manifest document.
    let manifest_value =
        serde_json::to_value(&manifest).expect("serialize runtime evidence-pack manifest");
    let first_file = manifest_value["files"][0].clone();
    assert_schema_fields_are_real(row, &manifest_value, &[&first_file]);

    // Every documented common error is a real strict-validation refusal.
    let missing_required = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        files[..3].to_vec(),
    )
    .expect_err("dropping source-index.json must be refused");
    assert!(
        missing_required
            .to_string()
            .contains("missing required file"),
        "missing-file refusal must be explicit: {missing_required}"
    );

    let mut unredacted = files.clone();
    unredacted[1].redacted = false;
    let unredacted_err = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        unredacted,
    )
    .expect_err("redaction_required file not marked redacted must be refused");
    assert!(
        unredacted_err.to_string().contains("redaction_required"),
        "unredacted refusal must be explicit: {unredacted_err}"
    );

    let mut duplicated = files.clone();
    duplicated.push(files[0].clone());
    let duplicate_err = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        format!("{marker}-requested-by"),
        duplicated,
    )
    .expect_err("duplicate file kind/pack_path must be refused");
    assert!(
        duplicate_err.to_string().contains("duplicate"),
        "duplicate refusal must be explicit: {duplicate_err}"
    );
}

#[tokio::test]
async fn mt075_backup_manual_rows_document_live_backup_and_preflight_surface() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP mt075_backup_manual_rows_document_live_backup_and_preflight_surface: no PostgreSQL"
        );
        return;
    };
    let store = connected_store(&url).await;
    let backup_row = manual_command("atelier_record_backup_manifest");
    let preflight_row = manual_command("atelier_backup_restore_preflight");
    assert_eq!(
        backup_row.name, "AtelierStore::record_backup_manifest",
        "backup manual row must name the real store method"
    );
    assert_eq!(
        preflight_row.name, "AtelierStore::preflight_backup_restore",
        "preflight manual row must name the real store method"
    );

    let marker = format!("mt075-manual-{}", Uuid::new_v4().simple());
    let artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-backup").as_bytes());
    let backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.10".to_string(),
            schema_version: 3,
            artifact_ref: artifact.artifact_ref.clone(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: artifact.content_hash.clone(),
                byte_len: artifact.byte_len,
            }],
            created_by: format!("{marker}-backup-author"),
        })
        .await
        .expect("record backup manifest through the real store");

    // RE-READ the manifest from PostgreSQL; all manual assertions run
    // against the re-read row, never the in-memory return value.
    let reread = store
        .get_backup_manifest(backup.backup_id)
        .await
        .expect("re-read backup manifest from PostgreSQL");
    assert_eq!(reread.backup_id, backup.backup_id);
    assert_eq!(reread.manifest_hash, backup.manifest_hash);
    assert_eq!(
        reread.manifest_json["schema_id"],
        serde_json::json!(BACKUP_MANIFEST_SCHEMA_ID)
    );
    assert!(
        backup_row.expected_output.contains(BACKUP_MANIFEST_SCHEMA_ID),
        "backup manual expected_output must document runtime schema id {BACKUP_MANIFEST_SCHEMA_ID}"
    );

    // Every documented backup schema field exists on the re-read record
    // (or inside its persisted manifest document, e.g. `files`).
    let backup_value =
        serde_json::to_value(&reread).expect("serialize re-read backup manifest record");
    let manifest_json = reread.manifest_json.clone();
    assert_schema_fields_are_real(backup_row, &backup_value, &[&manifest_json]);

    // Same-version preflight is accepted, exactly as the manual promises.
    let accepted = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: reread.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.10".to_string(),
            current_schema_version: 3,
            requested_by: format!("{marker}-restore"),
        })
        .await
        .expect("same-version restore preflight is accepted");
    assert_eq!(accepted.status, BackupRestorePreflightStatus::Accepted);
    assert!(accepted.refusal_reason.is_none());

    // A newer-schema backup is refused with the typed reason the manual's
    // common_errors row documents.
    let newer_artifact = atelier_pg_support::write_native_media_artifact(
        format!("{marker}-newer-schema-backup").as_bytes(),
    );
    let newer_backup = store
        .record_backup_manifest(&NewBackupManifest {
            app_version: "1.2.3".to_string(),
            spec_version: "2026.06.10".to_string(),
            schema_version: 4,
            artifact_ref: newer_artifact.artifact_ref.clone(),
            content_hash: newer_artifact.content_hash.clone(),
            byte_len: newer_artifact.byte_len,
            files: vec![BackupManifestFile {
                logical_path: "manifest/atelier.json".to_string(),
                content_hash: newer_artifact.content_hash.clone(),
                byte_len: newer_artifact.byte_len,
            }],
            created_by: format!("{marker}-backup-author"),
        })
        .await
        .expect("record newer-schema backup manifest");
    let refused = store
        .preflight_backup_restore(&BackupRestorePreflightRequest {
            backup_id: newer_backup.backup_id,
            current_app_version: "1.2.3".to_string(),
            current_spec_version: "2026.06.10".to_string(),
            current_schema_version: 3,
            requested_by: format!("{marker}-restore"),
        })
        .await
        .expect("newer-schema restore preflight returns a refusal record");
    assert_eq!(refused.status, BackupRestorePreflightStatus::Refused);
    assert!(
        refused
            .refusal_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("newer schema")),
        "newer-schema backups are refused with a typed reason"
    );
    assert!(
        preflight_row
            .common_errors
            .iter()
            .any(|error| error.contains("newer")),
        "preflight manual row must document the newer-version refusal the runtime enforces"
    );

    // Every documented preflight schema field exists on the runtime record.
    let preflight_value =
        serde_json::to_value(&refused).expect("serialize runtime preflight record");
    assert_schema_fields_are_real(preflight_row, &preflight_value, &[]);

    // Both documented surfaces emit canonical EventLedger evidence.
    assert!(event_family::ALL.contains(&export_event_family::BACKUP_MANIFEST_RECORDED));
    assert!(event_family::ALL.contains(&export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED));
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::BACKUP_MANIFEST_RECORDED,
                "atelier_backup_manifest",
                &reread.backup_id.to_string(),
            )
            .await
            .expect("count backup manifest event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED,
                "atelier_backup_manifest",
                &reread.backup_id.to_string(),
            )
            .await
            .expect("count accepted preflight event"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED,
                "atelier_backup_manifest",
                &newer_backup.backup_id.to_string(),
            )
            .await
            .expect("count refused preflight event"),
        1
    );
}
