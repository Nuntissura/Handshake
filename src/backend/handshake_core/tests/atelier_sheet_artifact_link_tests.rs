//! MT-016 CKC sheet-version artifact links on the embedded SurrealDB store.
//!
//! Port of the reference (PostgreSQL) test file onto `AtelierSurrealHarness`. Sheet versions
//! must hold typed reusable references to Posekit OpenPose exports and ComfyUI render artifacts
//! without copying files by hand. The reference's direct `sqlx` inserts that bypassed the Rust
//! validator become schema-governed `SurrealTestMutator::create_row` attempts, so the store
//! ASSERTs (cross-character ownership, `artifact://` floor, lowercase `reuse_role`, object
//! `metadata`) are proven at the database boundary, not only in Rust.

mod atelier_surreal_support;

use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::atelier::refs::sheet_version_ref;
use handshake_core::atelier::sheet_artifacts::{
    sheet_artifact_event_family, NewSheetArtifactLink, SheetArtifactKind,
};
use handshake_core::atelier::{AtelierError, AtelierStore, NewCharacter, NewSheetVersion};
use handshake_core::storage::surreal::{TableSelector, TestFieldMutation, TestMutationValue};
use uuid::Uuid;

fn artifact_payload_ref(artifact_id: Uuid) -> String {
    format!("artifact://.handshake/artifacts/L1/{artifact_id}/payload")
}

fn artifact_manifest_ref(artifact_id: Uuid) -> String {
    format!("artifact://.handshake/artifacts/L1/{artifact_id}/artifact.json")
}

fn base_link(character_internal_id: Uuid, sheet_version_id: Uuid) -> NewSheetArtifactLink {
    NewSheetArtifactLink {
        character_internal_id,
        sheet_version_id,
        artifact_kind: SheetArtifactKind::OpenPosePng,
        artifact_ref: artifact_payload_ref(Uuid::now_v7()),
        manifest_ref: None,
        source_ref: None,
        label: Some("guard fixture".to_string()),
        reuse_role: Some("cui_openpose_conditioning".to_string()),
        linked_by: "mt016-test".to_string(),
        metadata: serde_json::json!({}),
    }
}

async fn character_with_sheet(store: &AtelierStore, label: &str) -> (Uuid, Uuid) {
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt016-{label}-{}", Uuid::now_v7()),
            display_name: format!("MT-016 {label}"),
        })
        .await
        .expect("create character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: format!(
                "CHAR-ID-001 - Character_ID: {}\nCHAR-ID-002 - Name: MT-016",
                character.public_id
            ),
            author: "mt016-test".to_string(),
            tool: Some("argus".to_string()),
        })
        .await
        .expect("append sheet version");
    (character.internal_id, sheet.version_id)
}

/// A schema-governed direct row for `atelier_sheet_artifact_link`, with the link id doubling as
/// the record key (the table asserts `link_id = record::id($this.id)`).
struct DirectLinkRow<'a> {
    link_table: &'a TableSelector,
    character_table: &'a TableSelector,
    version_table: &'a TableSelector,
}

impl DirectLinkRow<'_> {
    fn field(&self, name: &str, value: TestMutationValue) -> TestFieldMutation {
        TestFieldMutation::new(
            self.link_table
                .field(name)
                .unwrap_or_else(|err| panic!("atelier_sheet_artifact_link.{name}: {err}")),
            value,
        )
    }

    fn fields(
        &self,
        link_id: Uuid,
        character_internal_id: Uuid,
        sheet_version_id: Uuid,
        artifact_ref: &str,
        reuse_role: Option<&str>,
        metadata: serde_json::Value,
    ) -> Vec<TestFieldMutation> {
        let mut fields = vec![
            self.field("link_id", TestMutationValue::uuid(link_id)),
            self.field(
                "character_internal_id",
                TestMutationValue::record(self.character_table, character_internal_id),
            ),
            self.field(
                "sheet_version_id",
                TestMutationValue::record(self.version_table, sheet_version_id),
            ),
            self.field("artifact_kind", TestMutationValue::string("openpose_png")),
            self.field("artifact_ref", TestMutationValue::string(artifact_ref)),
            self.field("linked_by", TestMutationValue::string("mt016-test")),
            self.field("metadata", TestMutationValue::json(metadata)),
        ];
        if let Some(reuse_role) = reuse_role {
            fields.push(self.field("reuse_role", TestMutationValue::string(reuse_role)));
        }
        fields
    }
}

#[tokio::test]
async fn ckc_sheet_artifact_links_reject_cross_character_and_local_runtime_refs() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let (character_internal_id, sheet_version_id) = character_with_sheet(store, "guards").await;
    let (other_character_internal_id, _) = character_with_sheet(store, "other").await;

    let wrong_character = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: other_character_internal_id,
            label: Some("wrong character".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("cross-character artifact link must be rejected");
    assert!(
        matches!(wrong_character, AtelierError::Validation(_)),
        "wrong-character link should be a validation error: {wrong_character:?}"
    );

    let local_path_ref = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            artifact_ref: "D:\\training\\openpose\\bad.png".to_string(),
            label: Some("local path".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("machine-local artifact refs must be rejected");
    assert!(
        matches!(local_path_ref, AtelierError::ForbiddenStorage(_)),
        "machine-local artifact ref should be ForbiddenStorage: {local_path_ref:?}"
    );

    let sqlite_query_ref = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            artifact_ref: "artifact://atelier/cache.db?x=1".to_string(),
            label: Some("sqlite behind a query string".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("SQLite-like refs must be rejected before query/fragment suffixes");
    assert!(
        matches!(sqlite_query_ref, AtelierError::ForbiddenStorage(_)),
        ".db?x=1 ref should be ForbiddenStorage: {sqlite_query_ref:?}"
    );

    let non_artifact_scheme = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            artifact_ref: format!("posekit://rig/{}", Uuid::now_v7()),
            label: Some("portable but not an ArtifactStore ref".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("artifact_ref must name an ArtifactStore payload (artifact://)");
    assert!(
        matches!(non_artifact_scheme, AtelierError::Validation(_)),
        "non-artifact:// ref should be a validation error: {non_artifact_scheme:?}"
    );

    let non_object_metadata = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            artifact_kind: SheetArtifactKind::ComfyRender,
            label: Some("bad metadata".to_string()),
            reuse_role: Some("cui_identity_reference".to_string()),
            metadata: serde_json::json!(["not", "object"]),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("non-object metadata must be rejected");
    assert!(
        matches!(non_object_metadata, AtelierError::Validation(_)),
        "non-object metadata should be a validation error: {non_object_metadata:?}"
    );

    let spaced_ref = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            artifact_ref: format!(
                "artifact://atelier/posekit/openpose/with space {}.png",
                Uuid::now_v7()
            ),
            label: Some("spaced ref".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("artifact refs with whitespace must be rejected");
    assert!(
        matches!(spaced_ref, AtelierError::Validation(_)),
        "spaced artifact ref should be a validation error: {spaced_ref:?}"
    );

    let uppercase_reuse_role = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            label: Some("bad reuse role".to_string()),
            reuse_role: Some("CUI_OpenPose".to_string()),
            ..base_link(character_internal_id, sheet_version_id)
        })
        .await
        .expect_err("reuse_role must be lowercase portable token");
    assert!(
        matches!(uppercase_reuse_role, AtelierError::Validation(_)),
        "uppercase reuse_role should be a validation error: {uppercase_reuse_role:?}"
    );

    // Database-boundary proofs: rows written past the Rust validator through the schema-governed
    // test mutator. A well-formed row must land (so the refusals below are the asserted
    // invariants, not a malformed fixture), then each invariant is violated one at a time.
    let inspector = harness.storage.test_inspector();
    let mutator = harness.storage.test_mutator();
    let link_table = inspector
        .table_selector("atelier_sheet_artifact_link")
        .await
        .expect("atelier_sheet_artifact_link is in the schema");
    let character_table = inspector
        .table_selector("atelier_character")
        .await
        .expect("atelier_character is in the schema");
    let version_table = inspector
        .table_selector("atelier_sheet_version")
        .await
        .expect("atelier_sheet_version is in the schema");
    let direct = DirectLinkRow {
        link_table: &link_table,
        character_table: &character_table,
        version_table: &version_table,
    };

    let direct_valid_id = Uuid::now_v7();
    let direct_valid_ref = artifact_payload_ref(Uuid::now_v7());
    mutator
        .create_row(
            &link_table,
            direct_valid_id,
            &direct.fields(
                direct_valid_id,
                character_internal_id,
                sheet_version_id,
                &direct_valid_ref,
                Some("cui_openpose_conditioning"),
                serde_json::json!({}),
            ),
        )
        .await
        .expect("a well-formed direct link row must satisfy the schema");
    let listed = store
        .list_sheet_artifacts(sheet_version_id)
        .await
        .expect("list after direct insert");
    assert!(
        listed.iter().any(|link| link.link_id == direct_valid_id
            && link.artifact_ref == direct_valid_ref),
        "the well-formed direct row must be visible as an active link"
    );

    let direct_cross_character_id = Uuid::now_v7();
    let direct_cross_character = mutator
        .create_row(
            &link_table,
            direct_cross_character_id,
            &direct.fields(
                direct_cross_character_id,
                other_character_internal_id,
                sheet_version_id,
                &artifact_payload_ref(Uuid::now_v7()),
                None,
                serde_json::json!({}),
            ),
        )
        .await
        .expect_err("schema must reject a sheet_version_id owned by another character");
    let message = direct_cross_character.to_string();
    assert!(
        message.contains("sheet_version_id") || message.contains("character_internal_id"),
        "direct cross-character insert should fail the ownership ASSERT: {message}"
    );

    let direct_local_path_id = Uuid::now_v7();
    let direct_local_path = mutator
        .create_row(
            &link_table,
            direct_local_path_id,
            &direct.fields(
                direct_local_path_id,
                character_internal_id,
                sheet_version_id,
                "/tmp/openpose.png",
                None,
                serde_json::json!({}),
            ),
        )
        .await
        .expect_err("schema must reject machine-local artifact refs");
    let message = direct_local_path.to_string();
    assert!(
        message.contains("artifact_ref"),
        "direct local-path insert should fail the artifact_ref ASSERT: {message}"
    );

    let direct_bad_reuse_role_id = Uuid::now_v7();
    let direct_bad_reuse_role = mutator
        .create_row(
            &link_table,
            direct_bad_reuse_role_id,
            &direct.fields(
                direct_bad_reuse_role_id,
                character_internal_id,
                sheet_version_id,
                &artifact_payload_ref(Uuid::now_v7()),
                Some("BadRole"),
                serde_json::json!({}),
            ),
        )
        .await
        .expect_err("schema must reject non-lowercase reuse_role");
    let message = direct_bad_reuse_role.to_string();
    assert!(
        message.contains("reuse_role"),
        "direct bad reuse_role insert should fail the reuse_role ASSERT: {message}"
    );

    let direct_non_object_metadata_id = Uuid::now_v7();
    let direct_non_object_metadata = mutator
        .create_row(
            &link_table,
            direct_non_object_metadata_id,
            &direct.fields(
                direct_non_object_metadata_id,
                character_internal_id,
                sheet_version_id,
                &artifact_payload_ref(Uuid::now_v7()),
                None,
                serde_json::json!([]),
            ),
        )
        .await
        .expect_err("schema must reject non-object metadata");
    let message = direct_non_object_metadata.to_string();
    assert!(
        message.contains("metadata"),
        "direct bad metadata insert should fail the metadata type: {message}"
    );

    let after_rejections = store
        .list_sheet_artifacts(sheet_version_id)
        .await
        .expect("list after rejected direct inserts");
    assert_eq!(
        after_rejections.len(),
        listed.len(),
        "rejected direct inserts must leave no partial rows behind"
    );
    harness.shutdown().await;
}

#[tokio::test]
async fn ckc_sheet_versions_round_trip_typed_posekit_and_comfy_artifact_links() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let (character_internal_id, sheet_version_id) = character_with_sheet(store, "links").await;

    let expected_sheet_ref = sheet_version_ref(character_internal_id, sheet_version_id);
    let openpose_ref = artifact_payload_ref(Uuid::now_v7());
    let comfy_render_artifact_id = Uuid::now_v7();
    let comfy_render_ref = artifact_payload_ref(comfy_render_artifact_id);
    let comfy_receipt_ref = format!("receipt://atelier/comfy/{}", Uuid::now_v7());

    let openpose = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id,
            sheet_version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: openpose_ref.clone(),
            manifest_ref: Some(artifact_manifest_ref(Uuid::now_v7())),
            source_ref: Some(format!("posekit://rig/{}", Uuid::now_v7())),
            label: Some("yaw +45 openpose conditioning".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({
                "yaw_degrees": 45,
                "export_schema": "hsk.atelier.posekit.openpose_export@1"
            }),
        })
        .await
        .expect("link OpenPose artifact");

    let comfy = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id,
            sheet_version_id,
            artifact_kind: SheetArtifactKind::ComfyRender,
            artifact_ref: comfy_render_ref.clone(),
            manifest_ref: Some(artifact_manifest_ref(comfy_render_artifact_id)),
            source_ref: Some(format!("comfy://workflow-run/{}", Uuid::now_v7())),
            label: Some("approved identity render".to_string()),
            reuse_role: Some("cui_identity_reference".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({
                "receipt_schema": "hsk.atelier.comfy.workflow_receipt@1",
                "receipt_ref": comfy_receipt_ref
            }),
        })
        .await
        .expect("link Comfy render artifact");

    assert_eq!(openpose.sheet_version_ref, expected_sheet_ref);
    assert_eq!(openpose.artifact_ref, openpose_ref);
    assert_eq!(openpose.artifact_kind, SheetArtifactKind::OpenPosePng);
    assert_eq!(
        openpose.typed_ref,
        format!("atelier://sheet-artifact/{}", openpose.link_id)
    );
    assert_eq!(openpose.link_id.get_version_num(), 7, "link_id must be UUID v7");
    assert_eq!(
        openpose.reuse_role.as_deref(),
        Some("cui_openpose_conditioning")
    );
    assert_eq!(comfy.reuse_role.as_deref(), Some("cui_identity_reference"));
    assert_eq!(comfy.metadata["receipt_ref"], comfy_receipt_ref);

    let resolved = store
        .get_sheet_artifact(openpose.link_id)
        .await
        .expect("resolve sheet artifact typed ref");
    assert_eq!(resolved.typed_ref, openpose.typed_ref);
    assert_eq!(resolved.artifact_ref, openpose_ref);

    let listed = store
        .list_sheet_artifacts(sheet_version_id)
        .await
        .expect("list linked artifacts");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].sheet_version_ref, expected_sheet_ref);
    assert_eq!(
        listed
            .iter()
            .map(|link| (&link.artifact_kind, link.artifact_ref.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (&SheetArtifactKind::OpenPosePng, openpose_ref.as_str()),
            (&SheetArtifactKind::ComfyRender, comfy_render_ref.as_str()),
        ]
    );

    let duplicate = store
        .link_sheet_artifact_with_status(&NewSheetArtifactLink {
            character_internal_id,
            sheet_version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: openpose_ref.clone(),
            manifest_ref: None,
            source_ref: None,
            label: Some("duplicate should return existing link".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("idempotent duplicate link");
    assert!(!duplicate.created, "duplicate attach must report created=false");
    assert_eq!(
        duplicate.link.link_id, openpose.link_id,
        "same sheet/kind/artifact_ref must not create duplicate reusable refs"
    );
    let link_event_count = store
        .count_events_for_aggregate(
            sheet_artifact_event_family::SHEET_ARTIFACT_LINKED,
            "atelier_sheet_artifact_link",
            &openpose.link_id.to_string(),
        )
        .await
        .expect("count sheet artifact linked events");
    assert_eq!(
        link_event_count, 1,
        "idempotent duplicate attach must not emit a second linked event"
    );

    let detached = store
        .detach_sheet_artifact(openpose.link_id, "mt016-test")
        .await
        .expect("detach linked artifact");
    assert_eq!(detached.link_id, openpose.link_id);
    assert!(detached.detached_at_utc.is_some());
    assert_eq!(detached.detached_by.as_deref(), Some("mt016-test"));

    let detached_resolve = store
        .get_sheet_artifact(openpose.link_id)
        .await
        .expect_err("detached sheet artifact typed refs are no longer active");
    assert!(
        matches!(detached_resolve, AtelierError::NotFound(_)),
        "detached typed ref should resolve as not found: {detached_resolve:?}"
    );

    let duplicate_detach = store
        .detach_sheet_artifact(openpose.link_id, "mt016-test")
        .await
        .expect_err("second detach must not emit another detach event");
    assert!(
        matches!(duplicate_detach, AtelierError::NotFound(_)),
        "second detach should be not found because detach is active-only: {duplicate_detach:?}"
    );
    let detach_event_count = store
        .count_events_for_aggregate(
            sheet_artifact_event_family::SHEET_ARTIFACT_DETACHED,
            "atelier_sheet_artifact_link",
            &openpose.link_id.to_string(),
        )
        .await
        .expect("count sheet artifact detached events");
    assert_eq!(
        detach_event_count, 1,
        "second detach must not emit a duplicate detached event"
    );

    // The row survives detach (soft delete): the same ref can be attached again as a NEW link
    // because the stored `active_link_key` discriminator moved to `detached:<link_id>`.
    let reattached = store
        .link_sheet_artifact_with_status(&NewSheetArtifactLink {
            character_internal_id,
            sheet_version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: openpose_ref.clone(),
            manifest_ref: None,
            source_ref: None,
            label: Some("re-attached after detach".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("re-attach after detach");
    assert!(reattached.created, "a detached ref must be attachable again as a new link");
    assert_ne!(reattached.link.link_id, openpose.link_id);
    let detached_rows = harness
        .row_count_by_field(
            "atelier_sheet_artifact_link",
            "active_link_key",
            &format!("detached:{}", openpose.link_id),
        )
        .await;
    assert_eq!(detached_rows, 1, "detach keeps the row and re-keys its active discriminator");

    let after_detach = store
        .list_sheet_artifacts(sheet_version_id)
        .await
        .expect("list after detach");
    assert_eq!(
        after_detach
            .iter()
            .map(|link| link.artifact_ref.as_str())
            .collect::<Vec<_>>(),
        vec![comfy_render_ref.as_str(), openpose_ref.as_str()],
        "detaching removes the old link from the active set; the re-attach is a new, newer link"
    );
    harness.shutdown().await;
}
