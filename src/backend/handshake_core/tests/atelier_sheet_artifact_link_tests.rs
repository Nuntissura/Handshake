//! MT-016 CKC sheet-version artifact links.
//!
//! These tests pin the missing product contract first: sheet versions must be
//! able to hold typed reusable references to Posekit OpenPose exports and
//! ComfyUI render artifacts without copying files by hand.

use handshake_core::atelier::{
    event_family, sheet_version_ref, AtelierError, AtelierStore, NewCharacter,
    NewSheetArtifactLink, NewSheetVersion, SheetArtifactKind,
};
use uuid::Uuid;

mod atelier_pg_support;

async fn connected_store() -> Option<AtelierStore> {
    let url = atelier_pg_support::database_url().await?;
    let store = AtelierStore::connect(&url)
        .await
        .expect("connect to postgres");
    store.ensure_schema().await.expect("ensure atelier schema");
    Some(store)
}

#[tokio::test]
async fn ckc_sheet_artifact_links_reject_cross_character_and_local_runtime_refs() {
    let Some(store) = connected_store().await else {
        return;
    };

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt016-artifact-guards-{}", Uuid::new_v4()),
            display_name: "MT-016 Artifact Guards".to_string(),
        })
        .await
        .expect("create character");
    let other_character = store
        .create_character(&NewCharacter {
            public_id: format!("mt016-artifact-other-{}", Uuid::new_v4()),
            display_name: "MT-016 Artifact Other".to_string(),
        })
        .await
        .expect("create second character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "CHAR-ID-001 - Character_ID: mt016-guards".to_string(),
            author: "mt016-test".to_string(),
            tool: Some("argus".to_string()),
        })
        .await
        .expect("append sheet version");

    let wrong_character = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: other_character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: format!("artifact://atelier/posekit/openpose/{}.png", Uuid::new_v4()),
            manifest_ref: None,
            source_ref: None,
            label: Some("wrong character".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect_err("cross-character artifact link must be rejected");
    assert!(
        matches!(wrong_character, AtelierError::Validation(_)),
        "wrong-character link should be a validation error: {wrong_character:?}"
    );

    let local_path_ref = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: "D:\\training\\openpose\\bad.png".to_string(),
            manifest_ref: None,
            source_ref: None,
            label: Some("local path".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect_err("machine-local artifact refs must be rejected");
    assert!(
        matches!(local_path_ref, AtelierError::ForbiddenStorage(_)),
        "machine-local artifact ref should be ForbiddenStorage: {local_path_ref:?}"
    );

    let non_object_metadata = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::ComfyRender,
            artifact_ref: format!("artifact://atelier/comfy/render/{}.png", Uuid::new_v4()),
            manifest_ref: None,
            source_ref: None,
            label: Some("bad metadata".to_string()),
            reuse_role: Some("cui_identity_reference".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!(["not", "object"]),
        })
        .await
        .expect_err("non-object metadata must be rejected");
    assert!(
        matches!(non_object_metadata, AtelierError::Validation(_)),
        "non-object metadata should be a validation error: {non_object_metadata:?}"
    );

    let spaced_ref = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: format!(
                "artifact://atelier/posekit/openpose/with space {}.png",
                Uuid::new_v4()
            ),
            manifest_ref: None,
            source_ref: None,
            label: Some("spaced ref".to_string()),
            reuse_role: Some("cui_openpose_conditioning".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect_err("artifact refs with whitespace must be rejected");
    assert!(
        matches!(spaced_ref, AtelierError::Validation(_)),
        "spaced artifact ref should be a validation error: {spaced_ref:?}"
    );

    let uppercase_reuse_role = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: format!("artifact://atelier/posekit/openpose/{}.png", Uuid::new_v4()),
            manifest_ref: None,
            source_ref: None,
            label: Some("bad reuse role".to_string()),
            reuse_role: Some("CUI_OpenPose".to_string()),
            linked_by: "mt016-test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect_err("reuse_role must be lowercase portable token");
    assert!(
        matches!(uppercase_reuse_role, AtelierError::Validation(_)),
        "uppercase reuse_role should be a validation error: {uppercase_reuse_role:?}"
    );

    let direct_cross_character = sqlx::query(
        r#"INSERT INTO atelier_sheet_artifact_link
             (character_internal_id, sheet_version_id, artifact_kind, artifact_ref, linked_by, metadata)
           VALUES ($1, $2, 'openpose_png', $3, 'mt016-test', '{}'::jsonb)"#,
    )
    .bind(other_character.internal_id)
    .bind(sheet.version_id)
    .bind(format!(
        "artifact://atelier/posekit/openpose/direct-{}.png",
        Uuid::new_v4()
    ))
    .execute(store.pool())
    .await
    .expect_err("DB composite FK must reject cross-character sheet artifact links");
    let direct_cross_character_message = direct_cross_character.to_string();
    assert!(
        direct_cross_character_message.contains("fk_atelier_sheet_artifact_link_sheet_owner")
            || direct_cross_character_message.contains("foreign key"),
        "direct DB cross-character insert should fail the ownership constraint: {direct_cross_character_message}"
    );

    let direct_local_path = sqlx::query(
        r#"INSERT INTO atelier_sheet_artifact_link
             (character_internal_id, sheet_version_id, artifact_kind, artifact_ref, linked_by, metadata)
           VALUES ($1, $2, 'openpose_png', '/tmp/openpose.png', 'mt016-test', '{}'::jsonb)"#,
    )
    .bind(character.internal_id)
    .bind(sheet.version_id)
    .execute(store.pool())
    .await
    .expect_err("DB check must reject machine-local artifact refs");
    let direct_local_path_message = direct_local_path.to_string();
    assert!(
        direct_local_path_message.contains("chk_atelier_sheet_artifact_link_artifact_ref")
            || direct_local_path_message.contains("check constraint"),
        "direct DB local-path insert should fail the portable-ref check: {direct_local_path_message}"
    );

    let direct_sqlite_query_ref = sqlx::query(
        r#"INSERT INTO atelier_sheet_artifact_link
             (character_internal_id, sheet_version_id, artifact_kind, artifact_ref, linked_by, metadata)
           VALUES ($1, $2, 'openpose_png', 'artifact://atelier/cache.db?x=1', 'mt016-test', '{}'::jsonb)"#,
    )
    .bind(character.internal_id)
    .bind(sheet.version_id)
    .execute(store.pool())
    .await
    .expect_err("DB check must reject SQLite-like refs before query/fragment suffixes");
    let direct_sqlite_query_ref_message = direct_sqlite_query_ref.to_string();
    assert!(
        direct_sqlite_query_ref_message.contains("chk_atelier_sheet_artifact_link_artifact_ref")
            || direct_sqlite_query_ref_message.contains("check constraint"),
        "direct DB .db? insert should fail the portable-ref check: {direct_sqlite_query_ref_message}"
    );

    let direct_bad_reuse_role = sqlx::query(
        r#"INSERT INTO atelier_sheet_artifact_link
             (character_internal_id, sheet_version_id, artifact_kind, artifact_ref, reuse_role, linked_by, metadata)
           VALUES ($1, $2, 'openpose_png', $3, 'BadRole', 'mt016-test', '{}'::jsonb)"#,
    )
    .bind(character.internal_id)
    .bind(sheet.version_id)
    .bind(format!(
        "artifact://atelier/posekit/openpose/direct-reuse-{}.png",
        Uuid::new_v4()
    ))
    .execute(store.pool())
    .await
    .expect_err("DB check must reject non-lowercase reuse_role");
    let direct_bad_reuse_role_message = direct_bad_reuse_role.to_string();
    assert!(
        direct_bad_reuse_role_message.contains("chk_atelier_sheet_artifact_link_reuse_role")
            || direct_bad_reuse_role_message.contains("check constraint"),
        "direct DB bad reuse_role insert should fail the reuse_role check: {direct_bad_reuse_role_message}"
    );

    let direct_non_object_metadata = sqlx::query(
        r#"INSERT INTO atelier_sheet_artifact_link
             (character_internal_id, sheet_version_id, artifact_kind, artifact_ref, linked_by, metadata)
           VALUES ($1, $2, 'openpose_png', $3, 'mt016-test', '[]'::jsonb)"#,
    )
    .bind(character.internal_id)
    .bind(sheet.version_id)
    .bind(format!(
        "artifact://atelier/posekit/openpose/direct-metadata-{}.png",
        Uuid::new_v4()
    ))
    .execute(store.pool())
    .await
    .expect_err("DB check must reject non-object metadata");
    let direct_non_object_metadata_message = direct_non_object_metadata.to_string();
    assert!(
        direct_non_object_metadata_message.contains("chk_atelier_sheet_artifact_link_metadata")
            || direct_non_object_metadata_message.contains("check constraint"),
        "direct DB bad metadata insert should fail the metadata check: {direct_non_object_metadata_message}"
    );
}

#[tokio::test]
async fn ckc_sheet_versions_round_trip_typed_posekit_and_comfy_artifact_links() {
    let Some(store) = connected_store().await else {
        return;
    };

    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt016-artifact-links-{}", Uuid::new_v4()),
            display_name: "MT-016 Artifact Links".to_string(),
        })
        .await
        .expect("create character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "CHAR-ID-001 - Character_ID: mt016\nCHAR-ID-002 - Name: MT-016".to_string(),
            author: "mt016-test".to_string(),
            tool: Some("argus".to_string()),
        })
        .await
        .expect("append sheet version");

    let expected_sheet_ref = sheet_version_ref(character.internal_id, sheet.version_id);
    let openpose_ref = format!("artifact://atelier/posekit/openpose/{}.png", Uuid::new_v4());
    let comfy_render_ref = format!("artifact://atelier/comfy/render/{}.png", Uuid::new_v4());
    let comfy_receipt_ref = format!("receipt://atelier/comfy/{}", Uuid::new_v4());

    let openpose = store
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::OpenPosePng,
            artifact_ref: openpose_ref.clone(),
            manifest_ref: Some(format!(
                "manifest://atelier/posekit/openpose/{}",
                Uuid::new_v4()
            )),
            source_ref: Some(format!("posekit://rig/{}", Uuid::new_v4())),
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
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            artifact_kind: SheetArtifactKind::ComfyRender,
            artifact_ref: comfy_render_ref.clone(),
            manifest_ref: Some(comfy_receipt_ref.clone()),
            source_ref: Some(format!("comfy://workflow-run/{}", Uuid::new_v4())),
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
    assert_eq!(
        openpose.reuse_role.as_deref(),
        Some("cui_openpose_conditioning")
    );
    assert_eq!(comfy.reuse_role.as_deref(), Some("cui_identity_reference"));

    let resolved = store
        .get_sheet_artifact(openpose.link_id)
        .await
        .expect("resolve sheet artifact typed ref");
    assert_eq!(resolved.typed_ref, openpose.typed_ref);
    assert_eq!(resolved.artifact_ref, openpose_ref);

    let listed = store
        .list_sheet_artifacts(sheet.version_id)
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
        .link_sheet_artifact(&NewSheetArtifactLink {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
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
    assert_eq!(
        duplicate.link_id, openpose.link_id,
        "same sheet/kind/artifact_ref must not create duplicate reusable refs"
    );
    let link_event_count = store
        .count_events_for_aggregate(
            event_family::SHEET_ARTIFACT_LINKED,
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
            event_family::SHEET_ARTIFACT_DETACHED,
            "atelier_sheet_artifact_link",
            &openpose.link_id.to_string(),
        )
        .await
        .expect("count sheet artifact detached events");
    assert_eq!(
        detach_event_count, 1,
        "second detach must not emit a duplicate detached event"
    );

    let after_detach = store
        .list_sheet_artifacts(sheet.version_id)
        .await
        .expect("list after detach");
    assert_eq!(
        after_detach
            .iter()
            .map(|link| link.artifact_ref.as_str())
            .collect::<Vec<_>>(),
        vec![comfy_render_ref.as_str()],
        "detaching a sheet artifact removes it from the reusable sheet-version set"
    );
}
