//! WP-KERNEL-005 atelier foundation: real PostgreSQL round-trip proof.
//! Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_foundation_tests -- --nocapture
//!
//! No mocks: this exercises the actual atelier store against a real Postgres,
//! proving stable identity (MT-006), append-only sheet versions (MT-012),
//! content-hash media dedup (MT-015), and event recording (MT-005).

mod atelier_pg_support;

use handshake_core::atelier::{
    AtelierStore, NewCharacter, NewMediaAsset, NewSheetVersion, event_family,
};
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[tokio::test]
async fn atelier_foundation_postgres_round_trip() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_foundation_postgres_round_trip: DATABASE_URL not set");
        return;
    };

    let store = AtelierStore::connect(&url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");

    // --- stable identity (MT-006): public_id is distinct from internal_id ---
    let public_id = format!("test-char-{}", Uuid::new_v4());
    let character = store
        .create_character(&NewCharacter {
            public_id: public_id.clone(),
            display_name: "Adversarial Test Subject".to_string(),
        })
        .await
        .expect("create character");
    assert_ne!(
        character.public_id,
        character.internal_id.to_string(),
        "public_id must not equal internal_id"
    );
    let fetched = store
        .get_character_by_public_id(&public_id)
        .await
        .expect("fetch by public_id");
    assert_eq!(fetched.internal_id, character.internal_id);

    // --- append-only sheet versions (MT-012): no overwrite, parent linkage ---
    let v1 = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Subject\nrole: protagonist".to_string(),
            author: "operator".to_string(),
            tool: None,
        })
        .await
        .expect("append v1");
    let v2 = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Subject\nrole: antagonist".to_string(),
            author: "operator".to_string(),
            tool: Some("model:gpt".to_string()),
        })
        .await
        .expect("append v2");
    assert_eq!(v1.seq, 1, "first version is seq 1");
    assert_eq!(v2.seq, 2, "second version is seq 2");
    assert_eq!(
        v2.parent_version_id,
        Some(v1.version_id),
        "v2 parent links to v1"
    );
    let history = store
        .sheet_version_history(character.internal_id)
        .await
        .expect("history");
    assert_eq!(history.len(), 2, "two versions preserved (append-only)");
    assert_eq!(history[0].raw_text, "name: Subject\nrole: protagonist");
    let latest = store
        .latest_sheet_version(character.internal_id)
        .await
        .expect("latest")
        .expect("has latest");
    assert_eq!(latest.seq, 2);

    // --- media dedup on content hash (MT-015): idempotent materialize ---
    let artifact = atelier_pg_support::write_native_media_artifact(b"foundation-media");
    let content_hash = artifact.content_hash.clone();
    let asset_first = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("clipboard".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize first");
    let asset_again = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("clipboard".to_string()),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize again");
    assert_eq!(
        asset_first.asset_id, asset_again.asset_id,
        "identical content hash dedups to the same asset"
    );

    // --- event families recorded (MT-005) ---
    assert!(
        store
            .count_events(event_family::CHARACTER_CREATED)
            .await
            .expect("count character events")
            >= 1
    );
    assert!(
        store
            .count_events(event_family::SHEET_VERSION_APPENDED)
            .await
            .expect("count sheet events")
            >= 2
    );
    assert!(
        store
            .count_events(event_family::MEDIA_ASSET_MATERIALIZED)
            .await
            .expect("count media events")
            >= 1
    );
}
