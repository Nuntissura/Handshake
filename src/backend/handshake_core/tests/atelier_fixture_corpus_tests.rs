//! WP-KERNEL-005 atelier fixture corpus: portable JSON fixtures round-tripped
//! through the real `AtelierStore` against live PostgreSQL (no SQLite, ever).
//!
//! Each fixture (MT-061..066, MT-076..079) is a portable, non-SQLite JSON document under
//! `tests/fixtures/atelier_core_data/`. The loader reads each fixture via
//! `include_str!`, deserializes it with serde, makes every public id / slug /
//! `{{NONCE}}` marker run-unique, persists it through the real store APIs, then
//! reloads and asserts round-trip fidelity (ids, key fields, ordering, version
//! chains). Every test is fully run-scoped: it owns its own character / media /
//! collection / document ids and asserts only on its own rows. No global table
//! counts are ever used.
//!
//! Run one MT, e.g.:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test atelier_fixture_corpus_tests \
//!     mt061_character_sheet_fixture_corpus_round_trips \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use handshake_core::atelier::collections::NewCollection;
use handshake_core::atelier::documents::{
    CharacterDocumentType, NewCharacterDocument, NewStoryBeat, NewStoryCard,
};
use handshake_core::atelier::exports::{
    build_llm_evidence_pack_manifest, export_event_family, validate_llm_evidence_pack_manifest,
    BackupManifestFile, BackupRestorePreflightRequest, BackupRestorePreflightStatus, ExportFormat,
    LlmEvidencePackFile, LlmEvidencePackFileKind, LlmEvidenceSourceAnchor, ManifestItemKind,
    NewBackupManifest, NewExportRequest, NewWebPortfolioExportRequest, SharePackBuildRequest,
    SharePackSubsetSelector, SharePackUsageReadmeArtifact, WebPortfolioManifestItem,
    BACKUP_MANIFEST_SCHEMA_ID, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID,
    WEB_PORTFOLIO_MANIFEST_SCHEMA_ID,
};
use handshake_core::atelier::intake::{
    intake_event_family, AtelierResetMode, AtelierResetRequest, IntakeBatchMode, IntakeLane,
    IntakeProfileMode, NewIntakeBatch, NewIntakeItem, OrphanAdoptionRequest, OrphanAdoptionStatus,
};
use handshake_core::atelier::moodboards::NewMoodboardSnapshot;
use handshake_core::atelier::relationships::NewCharacterRelationship;
use handshake_core::atelier::search::{
    MatchType, NewAiTagSuggestion, NewSavedSearch, NewTagRule, SavedSearchFilters, TagType,
};
use handshake_core::atelier::settings::{PreferenceScope, PreferenceType, SetPreference};
use handshake_core::atelier::{
    event_family, AtelierStore, MediaReviewMetadataUpdate, NewCharacter, NewMediaAsset,
    NewSheetVersion,
};
use serde::Deserialize;
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every fixture test runs.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// A short, run-unique suffix used to scope every public id / slug / nonce so
/// the fixtures never collide across runs (tables persist between runs).
fn run_suffix() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Append the run suffix to a slug, keeping it a stable, portable token.
fn run_unique(slug: &str, suffix: &str) -> String {
    format!("{slug}-{suffix}")
}

/// Deterministic, valid 16-hex (64-bit) dHash derived from a seed string.
/// `upsert_similarity_projection` requires exactly 16 lowercase hex chars.
fn dhash_from_seed(seed: &str) -> String {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(seed.as_bytes());
    hex::encode(&digest[..8])
}

// =====================================================================
// MT-061: character identity + sheet versioning
// =====================================================================

#[derive(Debug, Deserialize)]
struct CharacterSheetFixture {
    characters: Vec<FixtureCharacterWithSheets>,
}

#[derive(Debug, Deserialize)]
struct FixtureCharacterWithSheets {
    public_id_slug: String,
    display_name: String,
    sheet_versions: Vec<FixtureSheetVersion>,
}

#[derive(Debug, Deserialize)]
struct FixtureSheetVersion {
    raw_text: String,
    author: String,
    tool: Option<String>,
}

fn load_character_sheet_fixture() -> CharacterSheetFixture {
    let raw = include_str!("fixtures/atelier_core_data/character_sheet.json");
    serde_json::from_str(raw).expect("parse character_sheet.json fixture")
}

#[tokio::test]
async fn mt061_character_sheet_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt061_character_sheet_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_character_sheet_fixture();
    assert!(
        !fixture.characters.is_empty(),
        "fixture must define characters"
    );

    for fixture_char in &fixture.characters {
        let public_id = run_unique(&fixture_char.public_id_slug, &suffix);
        let character = store
            .create_character(&NewCharacter {
                public_id: public_id.clone(),
                display_name: fixture_char.display_name.clone(),
            })
            .await
            .expect("create character from fixture");

        // Stable identity: public_id is distinct from the storage key.
        assert_ne!(
            character.public_id,
            character.internal_id.to_string(),
            "public_id must not equal internal_id"
        );
        let fetched = store
            .get_character_by_public_id(&public_id)
            .await
            .expect("fetch character by public_id");
        assert_eq!(fetched.internal_id, character.internal_id);
        assert_eq!(fetched.display_name, fixture_char.display_name);

        // Append-only sheet versions: seq increments, parent chain links, raw
        // bytes preserved exactly (including trailing whitespace).
        let mut appended = Vec::new();
        for sv in &fixture_char.sheet_versions {
            let version = store
                .append_sheet_version(&NewSheetVersion {
                    character_internal_id: character.internal_id,
                    raw_text: sv.raw_text.clone(),
                    author: sv.author.clone(),
                    tool: sv.tool.clone(),
                })
                .await
                .expect("append sheet version from fixture");
            appended.push(version);
        }

        let history = store
            .sheet_version_history(character.internal_id)
            .await
            .expect("sheet version history");
        assert_eq!(
            history.len(),
            fixture_char.sheet_versions.len(),
            "all fixture sheet versions preserved (append-only)"
        );
        for (idx, (persisted, fixture_sv)) in
            history.iter().zip(fixture_char.sheet_versions.iter()).enumerate()
        {
            assert_eq!(persisted.seq, (idx as i64) + 1, "seq is 1-based and ordered");
            assert_eq!(
                persisted.raw_text, fixture_sv.raw_text,
                "raw sheet bytes round-trip exactly"
            );
            assert_eq!(persisted.author, fixture_sv.author);
            assert_eq!(persisted.tool, fixture_sv.tool);
            if idx == 0 {
                assert_eq!(persisted.parent_version_id, None, "first version has no parent");
            } else {
                assert_eq!(
                    persisted.parent_version_id,
                    Some(history[idx - 1].version_id),
                    "version parent links to the prior version"
                );
            }
        }

        if !fixture_char.sheet_versions.is_empty() {
            let latest = store
                .latest_sheet_version(character.internal_id)
                .await
                .expect("latest sheet version")
                .expect("has a latest version");
            assert_eq!(latest.seq, fixture_char.sheet_versions.len() as i64);

            // MT-061 parsing half: the latest sheet version parses into a real
            // template AST (sha256 template hash), proving the "parsing"
            // deliverable, not only append/versioning.
            let parsed = store
                .parse_sheet_template_version(
                    latest.version_id,
                    "wp-kernel-005-mt-061",
                    Some("test://wp-kernel-005/mt-061"),
                )
                .await
                .expect("parse latest sheet template version");
            assert_eq!(parsed.version_id, latest.version_id);
            assert_eq!(parsed.ast.template_hash.len(), 64, "sha256 template hash");
        }
    }
}

// =====================================================================
// MT-062: media identity + intake review behavior
// =====================================================================

#[derive(Debug, Deserialize)]
struct MediaIntakeFixture {
    media: Vec<FixtureMedia>,
    intake_batch: FixtureIntakeBatch,
}

#[derive(Debug, Deserialize)]
struct FixtureMedia {
    payload_seed: String,
    mime: String,
    source_provenance: String,
    review: FixtureMediaReview,
}

#[derive(Debug, Deserialize)]
struct FixtureMediaReview {
    favorite: bool,
    rating: i16,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    review_status: String,
}

#[derive(Debug, Deserialize)]
struct FixtureIntakeBatch {
    idempotency_slug: String,
    source_label: String,
    source_ref_slug: String,
    items: Vec<FixtureIntakeItem>,
}

#[derive(Debug, Deserialize)]
struct FixtureIntakeItem {
    source_path_slug: String,
    file_name: String,
    byte_len: i64,
    use_media_index: Option<usize>,
    lane: String,
}

fn load_media_intake_fixture() -> MediaIntakeFixture {
    let raw = include_str!("fixtures/atelier_core_data/media_intake.json");
    serde_json::from_str(raw).expect("parse media_intake.json fixture")
}

#[tokio::test]
async fn mt062_media_intake_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt062_media_intake_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_media_intake_fixture();
    assert!(!fixture.media.is_empty(), "fixture must define media");

    // Materialize each media seed into a real native ArtifactStore payload so
    // the content_hash + artifact_ref are produced exactly like production.
    let mut asset_ids = Vec::new();
    let mut review_updates = Vec::new();
    for fixture_media in &fixture.media {
        let seed = run_unique(&fixture_media.payload_seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: fixture_media.mime.clone(),
                byte_len: artifact.byte_len,
                source_provenance: Some(fixture_media.source_provenance.clone()),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize media asset from fixture");
        assert_eq!(asset.mime, fixture_media.mime);
        assert_eq!(asset.content_hash, artifact.content_hash);

        // Idempotent dedup: re-materializing the same content hash is the same asset.
        let again = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: fixture_media.mime.clone(),
                byte_len: artifact.byte_len,
                source_provenance: Some(fixture_media.source_provenance.clone()),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("re-materialize media asset (dedup)");
        assert_eq!(asset.asset_id, again.asset_id, "content-hash dedup");

        asset_ids.push(asset.asset_id);
        review_updates.push(MediaReviewMetadataUpdate {
            asset_id: asset.asset_id,
            favorite: fixture_media.review.favorite,
            rating: fixture_media.review.rating,
            frontpage: fixture_media.review.frontpage,
            carousel: fixture_media.review.carousel,
            notes: fixture_media.review.notes.clone(),
            review_status: fixture_media.review.review_status.clone(),
        });
    }

    store
        .bulk_update_media_review_metadata(&review_updates, "mt-062-reviewer")
        .await
        .expect("apply fixture review metadata");

    for (asset_id, fixture_media) in asset_ids.iter().zip(fixture.media.iter()) {
        let meta = store
            .get_media_review_metadata(*asset_id)
            .await
            .expect("read review metadata")
            .expect("review metadata present");
        assert_eq!(meta.favorite, fixture_media.review.favorite);
        assert_eq!(meta.rating, fixture_media.review.rating);
        assert_eq!(meta.frontpage, fixture_media.review.frontpage);
        assert_eq!(meta.carousel, fixture_media.review.carousel);
        assert_eq!(meta.notes, fixture_media.review.notes);
        assert_eq!(
            meta.review_status,
            fixture_media.review.review_status.to_ascii_lowercase()
        );
    }

    // Intake batch + items, scoped to this run by idempotency key.
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: run_unique(&fixture.intake_batch.idempotency_slug, &suffix),
            source_label: fixture.intake_batch.source_label.clone(),
            source_ref: Some(run_unique(&fixture.intake_batch.source_ref_slug, &suffix)),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open intake batch from fixture");

    let mut added_item_ids = Vec::new();
    for fixture_item in &fixture.intake_batch.items {
        let content_hash = fixture_item
            .use_media_index
            .map(|_| format!("sha256:{}", Uuid::new_v4().simple()));
        let item = store
            .add_intake_item(
                batch.batch_id,
                &NewIntakeItem {
                    source_path: run_unique(&fixture_item.source_path_slug, &suffix),
                    file_name: fixture_item.file_name.clone(),
                    byte_len: fixture_item.byte_len,
                    content_hash,
                },
            )
            .await
            .expect("add intake item from fixture");
        // Newly added items start in the Pending lane regardless of fixture lane.
        assert_eq!(item.lane, IntakeLane::Pending);
        added_item_ids.push(item.item_id);
    }

    let listed = store
        .list_intake_items(batch.batch_id, None)
        .await
        .expect("list intake items");
    let _ = &added_item_ids;
    assert_eq!(
        listed.len(),
        fixture.intake_batch.items.len(),
        "all fixture intake items persisted under this run's batch"
    );
    for (persisted, fixture_item) in listed.iter().zip(fixture.intake_batch.items.iter()) {
        assert_eq!(persisted.batch_id, batch.batch_id);
        assert_eq!(persisted.file_name, fixture_item.file_name);
        assert_eq!(persisted.byte_len, fixture_item.byte_len);
        assert_eq!(
            persisted.source_path,
            run_unique(&fixture_item.source_path_slug, &suffix),
            "original source path preserved exactly"
        );
        // Fixture lane is a documented target; parse proves it is a valid lane token.
        IntakeLane::parse(&fixture_item.lane).expect("fixture lane token is valid");
    }
}

// =====================================================================
// MT-063: collections + contact sheet
// =====================================================================

#[derive(Debug, Deserialize)]
struct CollectionContactSheetFixture {
    media_seeds: Vec<String>,
    collection: FixtureCollection,
    contact_sheet: FixtureContactSheet,
}

#[derive(Debug, Deserialize)]
struct FixtureCollection {
    name_slug: String,
    notes: String,
    tags: Vec<String>,
    member_media_indices: Vec<usize>,
}

#[derive(Debug, Deserialize)]
struct FixtureContactSheet {
    name_slug: String,
    source_type: String,
    member_media_indices: Vec<usize>,
    tags: Vec<String>,
}

fn load_collection_contact_sheet_fixture() -> CollectionContactSheetFixture {
    let raw = include_str!("fixtures/atelier_core_data/collection_contact_sheet.json");
    serde_json::from_str(raw).expect("parse collection_contact_sheet.json fixture")
}

#[tokio::test]
async fn mt063_collection_contact_sheet_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt063_collection_contact_sheet_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_collection_contact_sheet_fixture();

    // Materialize the media seeds into real assets.
    let mut asset_ids = Vec::new();
    for seed in &fixture.media_seeds {
        let seeded = run_unique(seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seeded.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(format!("fixture://mt-063/{seeded}")),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize collection media seed");
        asset_ids.push(asset.asset_id);
    }

    let collection = store
        .create_collection(&NewCollection {
            name: run_unique(&fixture.collection.name_slug, &suffix),
            notes: fixture.collection.notes.clone(),
            tags: fixture.collection.tags.clone(),
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create collection from fixture");

    let member_assets: Vec<Uuid> = fixture
        .collection
        .member_media_indices
        .iter()
        .map(|idx| asset_ids[*idx])
        .collect();
    let inserted = store
        .add_images_to_collection(collection.collection_id, &member_assets)
        .await
        .expect("add images to collection");
    assert_eq!(inserted, member_assets.len() as i64);

    // Reload collection + membership; assert ids, ordering, and tag de-dup/trim.
    let reloaded = store
        .get_collection(collection.collection_id)
        .await
        .expect("reload collection");
    assert_eq!(reloaded.collection_id, collection.collection_id);
    assert_eq!(reloaded.notes, fixture.collection.notes);
    // Tags are trimmed + de-duped by the store; "fixture" and "  fixture  " collapse.
    assert!(reloaded.tags.contains(&"fixture".to_string()));
    assert!(reloaded.tags.contains(&"portfolio".to_string()));
    assert_eq!(
        reloaded.tags.len(),
        reloaded
            .tags
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "collection tags are de-duplicated"
    );

    let members = store
        .list_collection_images(collection.collection_id)
        .await
        .expect("list collection images");
    assert_eq!(members.len(), member_assets.len());
    for (member, expected_asset) in members.iter().zip(member_assets.iter()) {
        assert_eq!(member.collection_id, collection.collection_id);
        assert_eq!(member.asset_id, *expected_asset, "membership order preserved");
    }
    for (idx, member) in members.iter().enumerate() {
        assert_eq!(member.sort_order, idx as i64, "membership sort order is dense");
    }

    // Contact sheet snapshots membership by content hash (immutable manifest).
    let sheet_assets: Vec<Uuid> = fixture
        .contact_sheet
        .member_media_indices
        .iter()
        .map(|idx| asset_ids[*idx])
        .collect();
    let sheet = store
        .create_contact_sheet(
            &run_unique(&fixture.contact_sheet.name_slug, &suffix),
            &fixture.contact_sheet.source_type,
            None,
            &sheet_assets,
            &fixture.contact_sheet.tags,
            None,
            None,
        )
        .await
        .expect("create contact sheet from fixture");
    assert_eq!(sheet.image_count, sheet_assets.len() as i64);
    assert_eq!(sheet.source_type, fixture.contact_sheet.source_type);

    let reloaded_sheet = store
        .get_contact_sheet(sheet.sheet_id)
        .await
        .expect("reload contact sheet");
    assert_eq!(reloaded_sheet.sheet_id, sheet.sheet_id);
    assert_eq!(reloaded_sheet.image_count, sheet_assets.len() as i64);
    let manifest_items = reloaded_sheet.manifest["items"]
        .as_array()
        .expect("contact sheet manifest has items");
    assert_eq!(manifest_items.len(), sheet_assets.len());
    for (item, expected_asset) in manifest_items.iter().zip(sheet_assets.iter()) {
        assert_eq!(
            item["asset_id"].as_str().expect("manifest asset_id string"),
            expected_asset.to_string(),
            "contact sheet manifest snapshots membership by asset id"
        );
        assert!(
            item["content_hash"].as_str().is_some(),
            "contact sheet manifest snapshots content hash"
        );
    }
}

// =====================================================================
// MT-064: documents / story / moodboards / relationships
// =====================================================================

#[derive(Debug, Deserialize)]
struct DocsMoodboardRelationsFixture {
    characters: Vec<FixtureDocCharacter>,
    relationships: Vec<FixtureRelationship>,
}

#[derive(Debug, Deserialize)]
struct FixtureDocCharacter {
    public_id_slug: String,
    display_name: String,
    documents: Vec<FixtureDocument>,
}

#[derive(Debug, Deserialize)]
struct FixtureDocument {
    doc_type: String,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    author: String,
    #[serde(default)]
    extra_versions: Vec<FixtureDocVersion>,
    #[serde(default)]
    story_cards: Vec<FixtureStoryCard>,
    #[serde(default)]
    moodboard_raw_json_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureDocVersion {
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    author: String,
}

#[derive(Debug, Deserialize)]
struct FixtureStoryCard {
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    beats: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureRelationship {
    source_character_index: usize,
    target_character_index: usize,
    relationship_kind: String,
    label: String,
    notes: String,
}

fn load_docs_moodboard_relations_fixture() -> DocsMoodboardRelationsFixture {
    let raw = include_str!("fixtures/atelier_core_data/docs_moodboard_relations.json");
    serde_json::from_str(raw).expect("parse docs_moodboard_relations.json fixture")
}

#[tokio::test]
async fn mt064_docs_moodboard_relations_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt064_docs_moodboard_relations_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_docs_moodboard_relations_fixture();

    let mut character_ids = Vec::new();
    for fixture_char in &fixture.characters {
        let character = store
            .create_character(&NewCharacter {
                public_id: run_unique(&fixture_char.public_id_slug, &suffix),
                display_name: fixture_char.display_name.clone(),
            })
            .await
            .expect("create character for docs fixture");
        character_ids.push(character.internal_id);

        for fixture_doc in &fixture_char.documents {
            let doc_type = CharacterDocumentType::from_token(&fixture_doc.doc_type)
                .expect("fixture doc_type is a valid token");
            let created = store
                .create_character_document(&NewCharacterDocument {
                    character_internal_id: character.internal_id,
                    doc_type,
                    title: fixture_doc.title.clone(),
                    body_raw_text: fixture_doc.body_raw_text.clone(),
                    tags: fixture_doc.tags.clone(),
                    author: fixture_doc.author.clone(),
                })
                .await
                .expect("create character document from fixture");
            assert_eq!(created.version_seq, 1, "first document version is seq 1");
            assert_eq!(
                created.body_raw_text, fixture_doc.body_raw_text,
                "document body raw bytes round-trip exactly"
            );
            let document_id = created.document_id;

            // Extra versions append onto the chain.
            for extra in &fixture_doc.extra_versions {
                store
                    .append_character_document_version(
                        document_id,
                        &handshake_core::atelier::documents::AppendCharacterDocumentVersion {
                            title: extra.title.clone(),
                            body_raw_text: extra.body_raw_text.clone(),
                            tags: extra.tags.clone(),
                            author: extra.author.clone(),
                        },
                    )
                    .await
                    .expect("append document version from fixture");
            }

            let history = store
                .character_document_history(document_id)
                .await
                .expect("document history");
            assert_eq!(
                history.len(),
                1 + fixture_doc.extra_versions.len(),
                "all document versions preserved (append-only)"
            );
            for (idx, version) in history.iter().enumerate() {
                assert_eq!(version.version_seq, (idx as i64) + 1, "version seq ordered");
                if idx == 0 {
                    assert_eq!(version.parent_version_id, None);
                    assert_eq!(version.body_raw_text, fixture_doc.body_raw_text);
                } else {
                    assert_eq!(
                        version.parent_version_id,
                        Some(history[idx - 1].version_id),
                        "document version parent chain links"
                    );
                    let extra = &fixture_doc.extra_versions[idx - 1];
                    assert_eq!(version.body_raw_text, extra.body_raw_text);
                    assert_eq!(version.title, extra.title);
                }
            }

            // Story cards + beats (only present on the story document).
            if !fixture_doc.story_cards.is_empty() {
                for fixture_card in &fixture_doc.story_cards {
                    let card = store
                        .add_story_card(&NewStoryCard {
                            story_document_id: document_id,
                            title: fixture_card.title.clone(),
                            body_raw_text: fixture_card.body_raw_text.clone(),
                            tags: fixture_card.tags.clone(),
                        })
                        .await
                        .expect("add story card from fixture");
                    assert_eq!(
                        card.body_raw_text, fixture_card.body_raw_text,
                        "story card raw bytes round-trip exactly"
                    );
                    for beat_text in &fixture_card.beats {
                        store
                            .add_story_beat(&NewStoryBeat {
                                story_document_id: document_id,
                                card_id: Some(card.card_id),
                                beat_text: beat_text.clone(),
                            })
                            .await
                            .expect("add story beat from fixture");
                    }
                }

                let cards = store
                    .list_story_cards(document_id)
                    .await
                    .expect("list story cards");
                assert_eq!(cards.len(), fixture_doc.story_cards.len());
                for (idx, (card, fixture_card)) in
                    cards.iter().zip(fixture_doc.story_cards.iter()).enumerate()
                {
                    assert_eq!(card.seq, (idx as i64) + 1, "story card seq ordered");
                    assert_eq!(card.body_raw_text, fixture_card.body_raw_text);
                }

                let beats = store
                    .list_story_beats(document_id)
                    .await
                    .expect("list story beats");
                let expected_beats: usize =
                    fixture_doc.story_cards.iter().map(|c| c.beats.len()).sum();
                assert_eq!(beats.len(), expected_beats, "all story beats persisted");
            }

            // Moodboard snapshot (only present on the moodboard document).
            if let Some(raw_json) = &fixture_doc.moodboard_raw_json_text {
                let resolved = raw_json
                    .replace("{{NONCE}}", &suffix)
                    .replace("{{MOODBOARD_UUID}}", &Uuid::new_v4().to_string())
                    .replace("{{HISTORY_UUID}}", &Uuid::new_v4().to_string());
                let snapshot = store
                    .record_moodboard_snapshot(&NewMoodboardSnapshot {
                        document_id,
                        raw_json_text: resolved.clone(),
                        author: fixture_doc.author.clone(),
                    })
                    .await
                    .expect("record moodboard snapshot from fixture");
                assert_eq!(
                    snapshot.raw_json_text, resolved,
                    "moodboard raw JSON bytes round-trip exactly"
                );
                let latest = store
                    .latest_moodboard_snapshot(document_id)
                    .await
                    .expect("latest moodboard snapshot")
                    .expect("has a moodboard snapshot");
                assert_eq!(latest.snapshot_id, snapshot.snapshot_id);
                assert_eq!(latest.raw_json_text, resolved);
            }
        }
    }

    // Relationships between the run-scoped characters.
    for fixture_rel in &fixture.relationships {
        let created = store
            .create_character_relationship(&NewCharacterRelationship {
                source_character_id: character_ids[fixture_rel.source_character_index],
                target_character_id: character_ids[fixture_rel.target_character_index],
                relationship_kind: fixture_rel.relationship_kind.clone(),
                label: Some(fixture_rel.label.clone()),
                notes: Some(fixture_rel.notes.clone()),
            })
            .await
            .expect("create relationship from fixture");
        // Store trims kind/label/notes; round-trip is the trimmed value.
        assert_eq!(created.relationship_kind, fixture_rel.relationship_kind.trim());
        assert_eq!(created.label.as_deref(), Some(fixture_rel.label.trim()));
        assert_eq!(created.notes, fixture_rel.notes.trim());

        let fetched = store
            .get_character_relationship(created.relationship_id)
            .await
            .expect("reload relationship");
        assert_eq!(fetched.relationship_id, created.relationship_id);
        assert_eq!(
            fetched.source_character_id,
            character_ids[fixture_rel.source_character_index]
        );
        assert_eq!(
            fetched.target_character_id,
            character_ids[fixture_rel.target_character_index]
        );
        assert_eq!(fetched.relationship_kind, fixture_rel.relationship_kind.trim());
    }
}

// =====================================================================
// MT-065: search tags / rules / similarity
// =====================================================================

#[derive(Debug, Deserialize)]
struct SearchTagsSimilarityFixture {
    character: FixtureSearchCharacter,
    manual_tags: Vec<String>,
    tag_rules: Vec<FixtureTagRule>,
    ai_suggestions: Vec<FixtureAiSuggestion>,
    saved_search: FixtureSavedSearch,
    similarity: FixtureSimilarity,
}

#[derive(Debug, Deserialize)]
struct FixtureSearchCharacter {
    public_id_slug: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct FixtureTagRule {
    source_field_id: String,
    match_type: String,
    pattern: String,
    emit_tag: String,
    enabled: bool,
    #[serde(default)]
    #[allow(dead_code)]
    field_value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureAiSuggestion {
    tag_text: String,
    confidence: f64,
    suggested_by: String,
}

#[derive(Debug, Deserialize)]
struct FixtureSavedSearch {
    name_slug: String,
    include_tags: Vec<String>,
    exclude_tags: Vec<String>,
    created_by: String,
}

#[derive(Debug, Deserialize)]
struct FixtureSimilarity {
    near_payload_seed: String,
    far_payload_seed: String,
    near_dominant_color: String,
    far_dominant_color: String,
}

fn load_search_tags_similarity_fixture() -> SearchTagsSimilarityFixture {
    let raw = include_str!("fixtures/atelier_core_data/search_tags_similarity.json");
    serde_json::from_str(raw).expect("parse search_tags_similarity.json fixture")
}

fn nonce(value: &str, suffix: &str) -> String {
    value.replace("{{NONCE}}", suffix)
}

#[tokio::test]
async fn mt065_search_tags_similarity_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt065_search_tags_similarity_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_search_tags_similarity_fixture();

    let character = store
        .create_character(&NewCharacter {
            public_id: run_unique(&fixture.character.public_id_slug, &suffix),
            display_name: fixture.character.display_name.clone(),
        })
        .await
        .expect("create search-subject character");

    // Manual tags.
    let mut expected_manual = Vec::new();
    for raw_tag in &fixture.manual_tags {
        let tag_text = nonce(raw_tag, &suffix);
        store
            .tag_character(character.internal_id, &tag_text, TagType::Manual)
            .await
            .expect("apply manual tag from fixture");
        expected_manual.push(tag_text.to_ascii_lowercase());
    }
    let character_tags = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list character tags");
    for expected in &expected_manual {
        assert!(
            character_tags.iter().any(|t| &t.text == expected),
            "manual tag {expected} round-trips on the character"
        );
    }

    // Derived tag rules (kept separate from applied tags).
    for fixture_rule in &fixture.tag_rules {
        let match_type = match fixture_rule.match_type.as_str() {
            "equals" => MatchType::Equals,
            "contains" => MatchType::Contains,
            "regex" => MatchType::Regex,
            other => panic!("fixture tag rule match_type {other} is not a valid MatchType"),
        };
        let emit_tag = nonce(&fixture_rule.emit_tag, &suffix);
        let created = store
            .create_tag_rule(&NewTagRule {
                source_field_id: nonce(&fixture_rule.source_field_id, &suffix),
                match_type,
                pattern: fixture_rule.pattern.clone(),
                emit_tag: emit_tag.clone(),
                enabled: fixture_rule.enabled,
            })
            .await
            .expect("create tag rule from fixture");
        assert_eq!(created.match_type, match_type);
        assert_eq!(created.enabled, fixture_rule.enabled);

        let rules = store.list_tag_rules().await.expect("list tag rules");
        assert!(
            rules.iter().any(|r| r.rule_id == created.rule_id
                && r.emit_tag == emit_tag.to_ascii_lowercase()),
            "created tag rule round-trips in the rule list"
        );
    }

    // AI suggestions stay separate from applied tags (status starts proposed).
    for fixture_sug in &fixture.ai_suggestions {
        let suggestion = store
            .record_ai_tag_suggestion(&NewAiTagSuggestion {
                character_internal_id: character.internal_id,
                asset_id: None,
                tag_text: nonce(&fixture_sug.tag_text, &suffix),
                confidence: Some(fixture_sug.confidence),
                model_receipt_ref: format!("receipt://atelier/model/{}", Uuid::new_v4()),
                tool_receipt_ref: format!("receipt://atelier/tool/{}", Uuid::new_v4()),
                suggested_by: fixture_sug.suggested_by.clone(),
            })
            .await
            .expect("record ai tag suggestion from fixture");
        assert_eq!(
            suggestion.status,
            handshake_core::atelier::search::AiTagSuggestionStatus::Proposed
        );
    }
    let suggestions = store
        .list_ai_tag_suggestions_for_character(character.internal_id)
        .await
        .expect("list ai suggestions");
    assert_eq!(
        suggestions.len(),
        fixture.ai_suggestions.len(),
        "all fixture AI suggestions persisted for this character"
    );

    // MT-065: AI suggestions are SEPARATE from applied tags — a proposed
    // suggestion's tag_text must NOT appear in the character's applied tag list
    // (separation is explicit, not incidental).
    let applied_after_suggest = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list applied tags after suggestions");
    for sug in &suggestions {
        assert!(
            !applied_after_suggest.iter().any(|t| t.text == sug.tag_text),
            "proposed AI suggestion '{}' must not be an applied tag",
            sug.tag_text
        );
    }

    // Saved search.
    let saved = store
        .save_saved_search(&NewSavedSearch {
            name: run_unique(&fixture.saved_search.name_slug, &suffix),
            filters: SavedSearchFilters {
                include_tags: fixture
                    .saved_search
                    .include_tags
                    .iter()
                    .map(|t| nonce(t, &suffix))
                    .collect(),
                exclude_tags: fixture
                    .saved_search
                    .exclude_tags
                    .iter()
                    .map(|t| nonce(t, &suffix))
                    .collect(),
                ..SavedSearchFilters::default()
            },
            created_by: fixture.saved_search.created_by.clone(),
        })
        .await
        .expect("save saved search from fixture");
    let reloaded_search = store
        .get_saved_search(saved.saved_search_id)
        .await
        .expect("reload saved search")
        .expect("saved search present");
    assert_eq!(reloaded_search.saved_search_id, saved.saved_search_id);
    assert_eq!(reloaded_search.created_by, fixture.saved_search.created_by);

    // Similarity projections from dHash seeds + dominant color.
    let near_seed = run_unique(&fixture.similarity.near_payload_seed, &suffix);
    let far_seed = run_unique(&fixture.similarity.far_payload_seed, &suffix);
    let near_artifact = atelier_pg_support::write_native_media_artifact(near_seed.as_bytes());
    let far_artifact = atelier_pg_support::write_native_media_artifact(far_seed.as_bytes());
    let near_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: near_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: near_artifact.byte_len,
            source_provenance: Some(format!("fixture://mt-065/{near_seed}")),
            artifact_ref: near_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize near asset");
    let far_asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: far_artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: far_artifact.byte_len,
            source_provenance: Some(format!("fixture://mt-065/{far_seed}")),
            artifact_ref: far_artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize far asset");

    let near_dhash = dhash_from_seed(&near_seed);
    let far_dhash = dhash_from_seed(&far_seed);
    store
        .upsert_similarity_projection(
            near_asset.asset_id,
            Some(&near_dhash),
            serde_json::json!({
                "algorithm": "mt065-fixture",
                "dominant": [{"hex": fixture.similarity.near_dominant_color, "ratio": 1.0}]
            }),
        )
        .await
        .expect("upsert near similarity projection");
    store
        .upsert_similarity_projection(
            far_asset.asset_id,
            Some(&far_dhash),
            serde_json::json!({
                "algorithm": "mt065-fixture",
                "dominant": [{"hex": fixture.similarity.far_dominant_color, "ratio": 1.0}]
            }),
        )
        .await
        .expect("upsert far similarity projection");

    let near_projection = store
        .get_similarity_projection(near_asset.asset_id)
        .await
        .expect("reload near similarity projection")
        .expect("near projection present");
    assert_eq!(near_projection.asset_internal_id, near_asset.asset_id);
    assert_eq!(near_projection.dhash_hex.as_deref(), Some(near_dhash.as_str()));
    assert_eq!(
        near_projection.palette_json["dominant"][0]["hex"]
            .as_str()
            .expect("near palette hex"),
        fixture.similarity.near_dominant_color
    );
    let far_projection = store
        .get_similarity_projection(far_asset.asset_id)
        .await
        .expect("reload far similarity projection")
        .expect("far projection present");
    assert_eq!(far_projection.dhash_hex.as_deref(), Some(far_dhash.as_str()));
}

// =====================================================================
// Shared portability rules (MT-066 / MT-076 / MT-079 fixtures)
// =====================================================================

#[derive(Debug, Deserialize)]
struct FixturePortabilityRules {
    required_artifact_prefix: String,
    forbidden_substrings: Vec<String>,
}

/// Assert a persisted handle is portable per the fixture rules: relocatable
/// `artifact://` root, no-space naming, no drive-letter paths, and none of the
/// fixture's forbidden namespace/storage tokens (.GOV, sqlite, ckc, ...).
fn assert_portable_fixture_ref(field: &str, value: &str, rules: &FixturePortabilityRules) {
    assert!(
        value.starts_with(&rules.required_artifact_prefix),
        "{field} must start with {}, got {value}",
        rules.required_artifact_prefix
    );
    assert!(
        !value.chars().any(char::is_whitespace),
        "{field} must use no-space portable naming, got {value}"
    );
    let bytes = value.as_bytes();
    assert!(
        !(bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()),
        "{field} must not be a drive-letter path, got {value}"
    );
    let lower = value.to_ascii_lowercase();
    for token in &rules.forbidden_substrings {
        assert!(
            !lower.contains(&token.to_ascii_lowercase()),
            "{field} must not contain forbidden token {token}, got {value}"
        );
    }
}

/// Assert a relative pack path / logical path is portable: no spaces, no
/// backslashes, no drive letters, none of the fixture's forbidden tokens.
fn assert_portable_pack_path(field: &str, value: &str, forbidden_tokens: &[String]) {
    assert!(
        !value.chars().any(char::is_whitespace),
        "{field} must use no-space portable naming, got {value}"
    );
    assert!(
        !value.contains('\\'),
        "{field} must use portable forward slashes, got {value}"
    );
    let bytes = value.as_bytes();
    assert!(
        !(bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()),
        "{field} must not be a drive-letter path, got {value}"
    );
    let lower = value.to_ascii_lowercase();
    for token in forbidden_tokens {
        assert!(
            !lower.contains(&token.to_ascii_lowercase()),
            "{field} must not contain forbidden token {token}, got {value}"
        );
    }
}

// =====================================================================
// MT-066: reset + orphan adoption
// =====================================================================

#[derive(Debug, Deserialize)]
struct ResetOrphanFixture {
    original_media: Vec<FixtureResetMedia>,
    preference: FixtureResetPreference,
    preferences_only_reset: FixtureResetOperation,
    full_reset: FixtureResetOperation,
    adoption: FixtureOrphanAdoption,
    portability: FixturePortabilityRules,
}

#[derive(Debug, Deserialize)]
struct FixtureResetMedia {
    payload_seed: String,
    source_provenance: String,
}

#[derive(Debug, Deserialize)]
struct FixtureResetPreference {
    key_slug: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct FixtureResetOperation {
    mode: String,
    requested_by: String,
    reason_slug: String,
}

#[derive(Debug, Deserialize)]
struct FixtureOrphanAdoption {
    requested_by: String,
    adopt_media_index: usize,
}

fn load_reset_orphan_fixture() -> ResetOrphanFixture {
    let raw = include_str!("fixtures/atelier_core_data/reset_orphan.json");
    serde_json::from_str(raw).expect("parse reset_orphan.json fixture")
}

fn reset_mode_from_token(token: &str) -> AtelierResetMode {
    match token {
        "preferences_only" => AtelierResetMode::PreferencesOnly,
        "full_preserve_original_media" => AtelierResetMode::FullPreserveOriginalMedia,
        other => panic!("fixture reset mode {other} is not a valid AtelierResetMode token"),
    }
}

#[tokio::test]
async fn mt066_reset_orphan_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt066_reset_orphan_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_reset_orphan_fixture();
    assert!(
        !fixture.original_media.is_empty(),
        "fixture must define original media"
    );

    // Materialize the fixture's original media into real ArtifactStore-backed
    // PostgreSQL rows so the reset has originals to preserve.
    let mut assets = Vec::new();
    for fixture_media in &fixture.original_media {
        let seed = run_unique(&fixture_media.payload_seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(run_unique(&fixture_media.source_provenance, &suffix)),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize original media from fixture");
        assets.push(asset);
    }

    // Seed a resettable preference, then run the preferences-only reset.
    let preference_key = run_unique(&fixture.preference.key_slug, &suffix);
    store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: preference_key.clone(),
            value_type: PreferenceType::String,
            value: fixture.preference.value.clone(),
            redacted: false,
        })
        .await
        .expect("seed resettable preference from fixture");

    let preferences_reset = store
        .record_atelier_reset(&AtelierResetRequest {
            mode: reset_mode_from_token(&fixture.preferences_only_reset.mode),
            requested_by: fixture.preferences_only_reset.requested_by.clone(),
            reason: run_unique(&fixture.preferences_only_reset.reason_slug, &suffix),
        })
        .await
        .expect("record preferences-only reset from fixture");
    assert!(
        preferences_reset.preferences_deleted_count >= 1,
        "preferences-only reset deletes the seeded preference row"
    );
    assert_eq!(
        store
            .get_preference(PreferenceScope::Global, &preference_key)
            .await
            .expect("read preference after reset"),
        None,
        "preferences-only reset removes the persisted preference"
    );
    for asset in &assets {
        assert!(
            store
                .get_media_asset_by_hash(&asset.content_hash)
                .await
                .expect("read media after preferences reset")
                .is_some(),
            "preferences-only reset never deletes original media"
        );
    }

    // Full reset preserving original media: must emit an orphan manifest that
    // snapshots every fixture original with checksums + relocatable roots.
    let full_reset = store
        .record_atelier_reset(&AtelierResetRequest {
            mode: reset_mode_from_token(&fixture.full_reset.mode),
            requested_by: fixture.full_reset.requested_by.clone(),
            reason: run_unique(&fixture.full_reset.reason_slug, &suffix),
        })
        .await
        .expect("record full reset from fixture");
    assert!(
        full_reset.original_media_preserved_count >= assets.len() as i64,
        "full reset preserves at least the fixture originals"
    );
    let manifest_id = full_reset
        .orphan_manifest_id
        .expect("full reset records an orphan manifest");

    // RE-READ the orphan manifest items from PostgreSQL and assert checksums,
    // relocatable artifact roots, and no drive-letter paths per the fixture.
    let manifest_items = store
        .list_orphan_manifest_items(manifest_id)
        .await
        .expect("re-read orphan manifest items");
    let mut fixture_items = Vec::new();
    for asset in &assets {
        let item = manifest_items
            .iter()
            .find(|item| item.asset_id == asset.asset_id)
            .expect("orphan manifest preserves the fixture original");
        assert_eq!(item.content_hash, asset.content_hash, "checksum preserved");
        assert_eq!(item.artifact_ref, asset.artifact_ref);
        assert_eq!(item.byte_len, asset.byte_len);
        assert_eq!(item.adoption_status, OrphanAdoptionStatus::Orphaned);
        assert_portable_fixture_ref(
            "orphan manifest artifact_ref",
            &item.artifact_ref,
            &fixture.portability,
        );
        fixture_items.push(item.clone());
    }

    // RE-READ both reset operations from PostgreSQL via the diagnostics
    // projection (canonical atelier_reset_operation rows).
    let diagnostics = store
        .list_reset_orphan_diagnostics()
        .await
        .expect("re-read reset operations");
    let persisted_pref_reset = diagnostics
        .resets
        .iter()
        .find(|row| row.reset_id == preferences_reset.reset_id)
        .expect("preferences-only reset row persisted");
    assert_eq!(persisted_pref_reset.mode, fixture.preferences_only_reset.mode);
    assert_eq!(
        persisted_pref_reset.requested_by,
        fixture.preferences_only_reset.requested_by
    );
    let persisted_full_reset = diagnostics
        .resets
        .iter()
        .find(|row| row.reset_id == full_reset.reset_id)
        .expect("full reset row persisted");
    assert_eq!(persisted_full_reset.mode, fixture.full_reset.mode);
    assert_eq!(persisted_full_reset.orphan_manifest_id, Some(manifest_id));

    // Adopt the fixture-selected orphan back into intake.
    let adopt_target = &fixture_items[fixture.adoption.adopt_media_index];
    let adoption = store
        .adopt_orphan_manifest_item(&OrphanAdoptionRequest {
            manifest_item_id: adopt_target.manifest_item_id,
            requested_by: fixture.adoption.requested_by.clone(),
        })
        .await
        .expect("adopt orphan manifest item from fixture");
    assert_eq!(adoption.manifest_item.adoption_status, OrphanAdoptionStatus::Adopted);
    assert_eq!(
        adoption.item.content_hash.as_deref(),
        Some(adopt_target.content_hash.as_str()),
        "adoption preserves the checksum"
    );
    assert_eq!(adoption.item.byte_len, adopt_target.byte_len);
    assert_eq!(
        adoption.item.source_path, adopt_target.artifact_ref,
        "adoption rehydrates the preserved relocatable handle as the intake source"
    );
    assert_portable_fixture_ref(
        "adopted intake source_path",
        &adoption.item.source_path,
        &fixture.portability,
    );

    // Re-adoption is idempotent: the same manifest item resolves to the same
    // batch/item rather than duplicating intake rows.
    let re_adoption = store
        .adopt_orphan_manifest_item(&OrphanAdoptionRequest {
            manifest_item_id: adopt_target.manifest_item_id,
            requested_by: fixture.adoption.requested_by.clone(),
        })
        .await
        .expect("re-adopt orphan manifest item");
    assert_eq!(re_adoption.batch.batch_id, adoption.batch.batch_id);
    assert_eq!(re_adoption.item.item_id, adoption.item.item_id);

    // EventLedger proof: reset, orphan manifest, and adoption events recorded
    // exactly once for this run's aggregates (idempotent re-adopt adds none).
    for reset_id in [preferences_reset.reset_id, full_reset.reset_id] {
        assert_eq!(
            store
                .count_events_for_aggregate(
                    intake_event_family::RESET_RECORDED,
                    "atelier_reset_operation",
                    &reset_id.to_string(),
                )
                .await
                .expect("count reset events"),
            1
        );
    }
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::ORPHAN_MANIFEST_RECORDED,
                "atelier_orphan_manifest",
                &manifest_id.to_string(),
            )
            .await
            .expect("count orphan manifest events"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::ORPHAN_MANIFEST_ITEM_ADOPTED,
                "atelier_orphan_manifest_item",
                &adopt_target.manifest_item_id.to_string(),
            )
            .await
            .expect("count orphan adoption events"),
        1
    );
}

// =====================================================================
// MT-076: web portfolio export
// =====================================================================

#[derive(Debug, Deserialize)]
struct WebPortfolioExportFixture {
    media: Vec<FixtureWebPortfolioMedia>,
    collection: FixtureWebPortfolioCollection,
    export: FixtureWebPortfolioExport,
    manifest_payload_seed: String,
    invalid_blank_space_slug: String,
    invalid_blank_space_pack_path: String,
    invalid_machine_local_artifact_ref: String,
    portability: FixturePortabilityRules,
}

#[derive(Debug, Deserialize)]
struct FixtureWebPortfolioMedia {
    payload_seed: String,
    source_provenance: String,
    pack_path_suffix: String,
}

#[derive(Debug, Deserialize)]
struct FixtureWebPortfolioCollection {
    name_slug: String,
    notes: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureWebPortfolioExport {
    slug_prefix: String,
    title: String,
    requested_by: String,
}

fn load_web_portfolio_export_fixture() -> WebPortfolioExportFixture {
    let raw = include_str!("fixtures/atelier_core_data/web_portfolio_export.json");
    serde_json::from_str(raw).expect("parse web_portfolio_export.json fixture")
}

#[tokio::test]
async fn mt076_web_portfolio_export_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt076_web_portfolio_export_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_web_portfolio_export_fixture();
    assert!(!fixture.media.is_empty(), "fixture must define media");

    // Materialize the fixture media and collect them into a source collection.
    let mut content_hashes = Vec::new();
    for fixture_media in &fixture.media {
        let seed = run_unique(&fixture_media.payload_seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(run_unique(&fixture_media.source_provenance, &suffix)),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize web portfolio media from fixture");
        content_hashes.push(asset.content_hash);
    }

    // RE-READ the persisted assets so the manifest items are built from
    // PostgreSQL rows, not in-memory values.
    let mut persisted_assets = Vec::new();
    for hash in &content_hashes {
        let asset = store
            .get_media_asset_by_hash(hash)
            .await
            .expect("re-read web portfolio media asset")
            .expect("web portfolio media asset persisted");
        persisted_assets.push(asset);
    }

    let collection = store
        .create_collection(&NewCollection {
            name: run_unique(&fixture.collection.name_slug, &suffix),
            notes: fixture.collection.notes.clone(),
            tags: fixture.collection.tags.clone(),
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create web portfolio source collection");
    let member_ids: Vec<Uuid> = persisted_assets.iter().map(|a| a.asset_id).collect();
    store
        .add_images_to_collection(collection.collection_id, &member_ids)
        .await
        .expect("add web portfolio members");

    let slug = run_unique(&fixture.export.slug_prefix, &suffix);
    let request = store
        .request_web_portfolio_export(&NewWebPortfolioExportRequest {
            source_collection_id: collection.collection_id,
            slug: slug.clone(),
            title: fixture.export.title.clone(),
            requested_by: fixture.export.requested_by.clone(),
        })
        .await
        .expect("request web portfolio export from fixture");
    assert_eq!(request.slug, slug);
    assert_eq!(request.source_collection_id, collection.collection_id);

    let items: Vec<WebPortfolioManifestItem> = persisted_assets
        .iter()
        .zip(fixture.media.iter())
        .map(|(asset, fixture_media)| WebPortfolioManifestItem {
            asset_id: asset.asset_id,
            artifact_ref: asset.artifact_ref.clone(),
            pack_path: format!("images/{}-{}", asset.asset_id, fixture_media.pack_path_suffix),
            content_hash: asset.content_hash.clone(),
            byte_len: asset.byte_len,
        })
        .collect();
    let manifest_seed = run_unique(&fixture.manifest_payload_seed, &suffix);
    let manifest_artifact =
        atelier_pg_support::write_native_media_artifact(manifest_seed.as_bytes());
    let result = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            &manifest_artifact.artifact_ref,
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &items,
        )
        .await
        .expect("record web portfolio export result from fixture");

    // RE-READ the rendered result from PostgreSQL and assert the full manifest
    // contract round-trips: schema, slug, output checksums, portable items.
    let reloaded = store
        .get_web_portfolio_export_result(request.portfolio_export_id)
        .await
        .expect("re-read web portfolio export result")
        .expect("web portfolio export result persisted");
    assert_eq!(reloaded.result_id, result.result_id);
    assert_eq!(reloaded.manifest_json, result.manifest_json);
    assert_eq!(
        reloaded.manifest_json["schema_id"],
        serde_json::json!(WEB_PORTFOLIO_MANIFEST_SCHEMA_ID)
    );
    assert_eq!(reloaded.manifest_json["slug"], serde_json::json!(slug));
    assert_eq!(
        reloaded.manifest_json["source_collection_id"],
        serde_json::json!(collection.collection_id)
    );
    assert_eq!(
        reloaded.manifest_json["output"]["content_hash"],
        serde_json::json!(manifest_artifact.content_hash)
    );
    assert_portable_fixture_ref(
        "web portfolio output artifact_ref",
        &reloaded.artifact_ref,
        &fixture.portability,
    );

    let manifest_items = reloaded.manifest_json["items"]
        .as_array()
        .expect("web portfolio manifest items array");
    assert_eq!(manifest_items.len(), fixture.media.len());
    for (manifest_item, expected) in manifest_items.iter().zip(items.iter()) {
        assert_eq!(
            manifest_item["asset_id"],
            serde_json::json!(expected.asset_id)
        );
        assert_eq!(
            manifest_item["content_hash"],
            serde_json::json!(expected.content_hash)
        );
        let pack_path = manifest_item["pack_path"]
            .as_str()
            .expect("manifest item pack_path");
        assert_eq!(pack_path, expected.pack_path);
        assert_portable_pack_path(
            "web portfolio pack_path",
            pack_path,
            &fixture.portability.forbidden_substrings,
        );
        assert_portable_fixture_ref(
            "web portfolio item artifact_ref",
            manifest_item["artifact_ref"]
                .as_str()
                .expect("manifest item artifact_ref"),
            &fixture.portability,
        );
    }

    // Fixture-driven portability rejections: blank-space slug, blank-space
    // pack path, machine-local .GOV artifact ref.
    let bad_slug = store
        .request_web_portfolio_export(&NewWebPortfolioExportRequest {
            source_collection_id: collection.collection_id,
            slug: fixture.invalid_blank_space_slug.clone(),
            title: fixture.export.title.clone(),
            requested_by: fixture.export.requested_by.clone(),
        })
        .await
        .expect_err("blank-space slugs are rejected");
    assert!(bad_slug.to_string().contains("slug"));

    let bad_path = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            &manifest_artifact.artifact_ref,
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &[WebPortfolioManifestItem {
                pack_path: fixture.invalid_blank_space_pack_path.clone(),
                ..items[0].clone()
            }],
        )
        .await
        .expect_err("blank-space pack paths are rejected");
    assert!(bad_path.to_string().contains("pack_path"));

    let bad_ref = store
        .record_web_portfolio_export_result(
            request.portfolio_export_id,
            &fixture.invalid_machine_local_artifact_ref,
            &manifest_artifact.content_hash,
            manifest_artifact.byte_len,
            &items,
        )
        .await
        .expect_err("machine-local .GOV output refs are rejected");
    assert!(bad_ref.to_string().contains("artifact_ref"));

    // EventLedger proof: exactly one requested + one rendered event for this
    // run's portfolio export aggregate.
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::WEB_PORTFOLIO_EXPORT_REQUESTED,
                "atelier_web_portfolio_export_request",
                &request.portfolio_export_id.to_string(),
            )
            .await
            .expect("count web portfolio requested events"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::WEB_PORTFOLIO_EXPORT_RENDERED,
                "atelier_web_portfolio_export_request",
                &request.portfolio_export_id.to_string(),
            )
            .await
            .expect("count web portfolio rendered events"),
        1
    );
}

// =====================================================================
// MT-077: share-pack export
// =====================================================================

#[derive(Debug, Deserialize)]
struct SharePackExportFixture {
    character: FixtureSharePackCharacter,
    sheet: FixtureSharePackSheet,
    export: FixtureSharePackExport,
    sheet_payload_seed: String,
    selected_media_payload_seeds: Vec<String>,
    unselected_media_payload_seeds: Vec<String>,
    usage_readme_payload_seed: String,
    forbidden_namespace_tokens: Vec<String>,
    invalid_ckc_artifact_ref: String,
    invalid_gov_pack_path: String,
}

#[derive(Debug, Deserialize)]
struct FixtureSharePackCharacter {
    public_id_slug: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct FixtureSharePackSheet {
    raw_text: String,
    author: String,
    tool: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureSharePackExport {
    format: String,
    label: String,
    requested_by: String,
}

fn load_share_pack_export_fixture() -> SharePackExportFixture {
    let raw = include_str!("fixtures/atelier_core_data/share_pack_export.json");
    serde_json::from_str(raw).expect("parse share_pack_export.json fixture")
}

#[tokio::test]
async fn mt077_share_pack_export_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt077_share_pack_export_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_share_pack_export_fixture();

    let character = store
        .create_character(&NewCharacter {
            public_id: run_unique(&fixture.character.public_id_slug, &suffix),
            display_name: fixture.character.display_name.clone(),
        })
        .await
        .expect("create share-pack character from fixture");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: fixture.sheet.raw_text.clone(),
            author: fixture.sheet.author.clone(),
            tool: fixture.sheet.tool.clone(),
        })
        .await
        .expect("append share-pack sheet from fixture");
    let format = ExportFormat::from_token(&fixture.export.format)
        .expect("fixture export format token is valid");
    let export = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: sheet.version_id,
            format,
            label: Some(fixture.export.label.clone()),
            requested_by: fixture.export.requested_by.clone(),
        })
        .await
        .expect("request share-pack export from fixture");

    let sheet_seed = run_unique(&fixture.sheet_payload_seed, &suffix);
    let sheet_artifact = atelier_pg_support::write_native_media_artifact(sheet_seed.as_bytes());
    store
        .record_export_result(
            export.export_id,
            &sheet_artifact.artifact_ref,
            &sheet_artifact.content_hash,
            sheet_artifact.byte_len,
        )
        .await
        .expect("record share-pack sheet result");

    let mut selected_assets = Vec::new();
    for seed_slug in &fixture.selected_media_payload_seeds {
        let seed = run_unique(seed_slug, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(format!("fixture://mt-077/selected/{seed}")),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize selected share-pack media");
        selected_assets.push(asset);
    }
    let mut unselected_assets = Vec::new();
    for seed_slug in &fixture.unselected_media_payload_seeds {
        let seed = run_unique(seed_slug, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let asset = store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(format!("fixture://mt-077/unselected/{seed}")),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("materialize unselected share-pack media");
        unselected_assets.push(asset);
    }

    let readme_seed = run_unique(&fixture.usage_readme_payload_seed, &suffix);
    let readme_artifact = atelier_pg_support::write_native_media_artifact(readme_seed.as_bytes());
    let build = store
        .build_share_pack_manifest(&SharePackBuildRequest {
            export_id: export.export_id,
            selector: SharePackSubsetSelector {
                include_sheet: true,
                media_asset_ids: selected_assets.iter().map(|a| a.asset_id).collect(),
            },
            usage_readme: SharePackUsageReadmeArtifact {
                artifact_ref: readme_artifact.artifact_ref.clone(),
                content_hash: readme_artifact.content_hash.clone(),
                byte_len: readme_artifact.byte_len,
            },
            requested_by: fixture.export.requested_by.clone(),
        })
        .await
        .expect("build share-pack manifest from fixture");
    let expected_entries = 1 + selected_assets.len() + 1;
    assert_eq!(build.entries.len(), expected_entries);
    assert_eq!(build.selected_media_count, selected_assets.len() as i64);

    // RE-READ the manifest from PostgreSQL: manifest completeness (sheet +
    // every selected media + usage README) and subset safety (no unselected
    // media), all on portable non-CKC pack paths.
    let manifest = store
        .export_manifest(export.export_id)
        .await
        .expect("re-read share-pack manifest");
    assert_eq!(manifest.len(), expected_entries, "manifest completeness");
    assert!(
        manifest
            .iter()
            .any(|entry| entry.kind == ManifestItemKind::Sheet
                && entry.artifact_ref == sheet_artifact.artifact_ref),
        "manifest bundles the rendered sheet"
    );
    assert!(
        manifest
            .iter()
            .any(|entry| entry.kind == ManifestItemKind::UsageReadme
                && entry.pack_path == "README.md"
                && entry.artifact_ref == readme_artifact.artifact_ref),
        "manifest bundles the usage README at README.md"
    );
    for asset in &selected_assets {
        assert!(
            manifest.iter().any(|entry| {
                entry.kind == ManifestItemKind::Media
                    && entry.artifact_ref == asset.artifact_ref
                    && entry.pack_path.contains(&asset.asset_id.to_string())
            }),
            "manifest bundles every selected media asset"
        );
    }
    for asset in &unselected_assets {
        assert!(
            !manifest.iter().any(|entry| {
                entry.artifact_ref == asset.artifact_ref
                    || entry.pack_path.contains(&asset.asset_id.to_string())
            }),
            "subset safety: unselected media never leaks into the share pack"
        );
    }
    for entry in &manifest {
        assert_portable_pack_path(
            "share-pack pack_path",
            &entry.pack_path,
            &fixture.forbidden_namespace_tokens,
        );
        let lower_ref = entry.artifact_ref.to_ascii_lowercase();
        for token in &fixture.forbidden_namespace_tokens {
            assert!(
                !lower_ref.contains(&token.to_ascii_lowercase()),
                "share-pack artifact_ref must not use the {token} namespace, got {}",
                entry.artifact_ref
            );
        }
    }

    // Fixture-driven namespace rejections: CKC artifact refs and .GOV pack
    // paths can never enter a share pack.
    let ckc_ref = store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            &fixture.invalid_ckc_artifact_ref,
            "images/legacy.png",
        )
        .await
        .expect_err("CKC-namespace artifact refs are rejected");
    assert!(ckc_ref.to_string().contains("artifact_ref"));

    let gov_path = store
        .add_manifest_entry(
            export.export_id,
            ManifestItemKind::Media,
            &selected_assets[0].artifact_ref,
            &fixture.invalid_gov_pack_path,
        )
        .await
        .expect_err(".GOV pack paths are rejected");
    assert!(gov_path.to_string().contains("pack_path"));

    // EventLedger proof: one request, one render, and one manifest-item event
    // per bundled entry on this run's export aggregate (rejections add none).
    let export_aggregate = export.export_id.to_string();
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::EXPORT_REQUESTED,
                "atelier_export_request",
                &export_aggregate,
            )
            .await
            .expect("count export requested events"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::EXPORT_RENDERED,
                "atelier_export_request",
                &export_aggregate,
            )
            .await
            .expect("count export rendered events"),
        1
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                export_event_family::EXPORT_MANIFEST_ITEM_ADDED,
                "atelier_export_request",
                &export_aggregate,
            )
            .await
            .expect("count manifest item events"),
        expected_entries as i64
    );
}

// =====================================================================
// MT-078: LLM evidence pack
// =====================================================================

#[derive(Debug, Deserialize)]
struct LlmEvidencePackFixture {
    requested_by: String,
    files: Vec<FixtureEvidenceFile>,
    invalid_sqlite_source_id: String,
    invalid_machine_local_pack_path: String,
}

#[derive(Debug, Deserialize)]
struct FixtureEvidenceFile {
    kind: String,
    pack_path: String,
    payload_seed: String,
    source_provenance: String,
    redaction_required: bool,
    redacted: bool,
    source_anchors: Vec<FixtureEvidenceAnchor>,
}

#[derive(Debug, Deserialize)]
struct FixtureEvidenceAnchor {
    source_id_slug: String,
    source_path: String,
    source_range: String,
}

fn load_llm_evidence_pack_fixture() -> LlmEvidencePackFixture {
    let raw = include_str!("fixtures/atelier_core_data/llm_evidence_pack.json");
    serde_json::from_str(raw).expect("parse llm_evidence_pack.json fixture")
}

fn evidence_file_kind_from_token(token: &str) -> LlmEvidencePackFileKind {
    match token {
        "readme" => LlmEvidencePackFileKind::Readme,
        "evidence" => LlmEvidencePackFileKind::Evidence,
        "redaction_report" => LlmEvidencePackFileKind::RedactionReport,
        "source_index" => LlmEvidencePackFileKind::SourceIndex,
        other => panic!("fixture evidence file kind {other} is not a valid token"),
    }
}

#[tokio::test]
async fn mt078_llm_evidence_pack_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt078_llm_evidence_pack_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_llm_evidence_pack_fixture();
    assert_eq!(
        fixture.files.len(),
        4,
        "fixture must define the four required evidence-pack files"
    );

    // Persist every evidence-pack file payload through the real store, then
    // RE-READ the rows from PostgreSQL and build the strict manifest from the
    // re-read values (never from in-memory literals).
    let mut files = Vec::new();
    let mut file_hashes = Vec::new();
    for fixture_file in &fixture.files {
        let seed = run_unique(&fixture_file.payload_seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        store
            .materialize_media_asset(&NewMediaAsset {
                content_hash: artifact.content_hash.clone(),
                mime: "image/png".to_string(),
                byte_len: artifact.byte_len,
                source_provenance: Some(run_unique(&fixture_file.source_provenance, &suffix)),
                artifact_ref: artifact.artifact_ref.clone(),
            })
            .await
            .expect("persist evidence-pack file payload");
        let persisted = store
            .get_media_asset_by_hash(&artifact.content_hash)
            .await
            .expect("re-read evidence-pack file payload")
            .expect("evidence-pack file payload persisted");
        file_hashes.push(persisted.content_hash.clone());

        files.push(LlmEvidencePackFile {
            kind: evidence_file_kind_from_token(&fixture_file.kind),
            pack_path: fixture_file.pack_path.clone(),
            artifact_ref: persisted.artifact_ref.clone(),
            content_hash: persisted.content_hash.clone(),
            byte_len: persisted.byte_len,
            source_anchors: fixture_file
                .source_anchors
                .iter()
                .map(|anchor| LlmEvidenceSourceAnchor {
                    source_id: run_unique(&anchor.source_id_slug, &suffix),
                    source_path: anchor.source_path.clone(),
                    source_range: anchor.source_range.clone(),
                    content_hash: persisted.content_hash.clone(),
                })
                .collect(),
            redaction_required: fixture_file.redaction_required,
            redacted: fixture_file.redacted,
        });
    }

    let manifest = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        fixture.requested_by.clone(),
        files.clone(),
    )
    .expect("build strict evidence-pack manifest from fixture");
    validate_llm_evidence_pack_manifest(&manifest)
        .expect("fixture-built evidence-pack manifest passes strict validation");
    assert_eq!(manifest.schema_id, LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID);
    assert_eq!(
        manifest
            .files
            .iter()
            .map(|file| file.pack_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "README.md",
            "evidence.json",
            "redactions.json",
            "source-index.json"
        ],
        "manifest files are in deterministic model-consumable order"
    );
    for file in &manifest.files {
        assert!(
            !file.source_anchors.is_empty(),
            "every evidence-pack file carries source anchors"
        );
        assert!(
            file.artifact_ref.starts_with("artifact://"),
            "evidence-pack files are ArtifactStore-backed, got {}",
            file.artifact_ref
        );
        for anchor in &file.source_anchors {
            assert!(
                !anchor.source_path.contains('\\')
                    && !anchor.source_path.to_ascii_lowercase().contains("sqlite"),
                "anchors never point at SQLite/machine-local paths, got {}",
                anchor.source_path
            );
        }
    }
    assert!(
        manifest.files.iter().any(|file| {
            file.kind == LlmEvidencePackFileKind::Evidence
                && file.redaction_required
                && file.redacted
        }),
        "sensitive evidence carries explicit redaction flags"
    );

    // Strict-format rejections, all driven by fixture probes:
    // SQLite source anchors are forbidden.
    let mut sqlite_files = files.clone();
    sqlite_files[0].source_anchors[0].source_id = fixture.invalid_sqlite_source_id.clone();
    let sqlite_err = build_llm_evidence_pack_manifest(
        Uuid::new_v4(),
        fixture.requested_by.clone(),
        sqlite_files,
    )
    .expect_err("SQLite source anchors are rejected");
    assert!(sqlite_err.to_string().contains("source_anchor.source_id"));

    // Machine-local .GOV anchor paths are forbidden.
    let mut gov_files = files.clone();
    gov_files[0].source_anchors[0].source_path =
        fixture.invalid_machine_local_pack_path.clone();
    let gov_err =
        build_llm_evidence_pack_manifest(Uuid::new_v4(), fixture.requested_by.clone(), gov_files)
            .expect_err(".GOV machine-local anchor paths are rejected");
    assert!(gov_err.to_string().contains("pack_path"));

    // Every one of the four files is required.
    let incomplete: Vec<LlmEvidencePackFile> = files
        .iter()
        .filter(|file| file.kind != LlmEvidencePackFileKind::RedactionReport)
        .cloned()
        .collect();
    let missing_err =
        build_llm_evidence_pack_manifest(Uuid::new_v4(), fixture.requested_by.clone(), incomplete)
            .expect_err("missing redactions.json is rejected");
    assert!(missing_err.to_string().contains("redactions.json"));

    // Redaction-required evidence must be marked redacted.
    let mut unredacted = files.clone();
    for file in &mut unredacted {
        if file.kind == LlmEvidencePackFileKind::Evidence {
            file.redacted = false;
        }
    }
    let unredacted_err =
        build_llm_evidence_pack_manifest(Uuid::new_v4(), fixture.requested_by.clone(), unredacted)
            .expect_err("redaction-required evidence without redacted flag is rejected");
    assert!(unredacted_err.to_string().contains("redacted"));

    // EventLedger proof: each persisted evidence-pack payload recorded exactly
    // one materialization event for this run's content-hash aggregate.
    for hash in &file_hashes {
        assert_eq!(
            store
                .count_events_for_aggregate(
                    event_family::MEDIA_ASSET_MATERIALIZED,
                    "atelier_media_asset",
                    hash,
                )
                .await
                .expect("count evidence payload materialization events"),
            1
        );
    }
}

// =====================================================================
// MT-079: backup + restore preflight
// =====================================================================

#[derive(Debug, Deserialize)]
struct BackupRestorePreflightFixture {
    backups: Vec<FixtureBackup>,
    preflights: Vec<FixtureBackupPreflight>,
    portability: FixturePortabilityRules,
}

#[derive(Debug, Deserialize)]
struct FixtureBackup {
    key: String,
    app_version: String,
    spec_version: String,
    schema_version: i32,
    payload_seed: String,
    created_by: String,
    extra_files: Vec<FixtureBackupFile>,
}

#[derive(Debug, Deserialize)]
struct FixtureBackupFile {
    logical_path: String,
    content_hash: String,
    byte_len: i64,
}

#[derive(Debug, Deserialize)]
struct FixtureBackupPreflight {
    backup_key: String,
    current_app_version: String,
    current_spec_version: String,
    current_schema_version: i32,
    requested_by: String,
    expected_status: String,
    refusal_contains: Option<String>,
}

fn load_backup_restore_preflight_fixture() -> BackupRestorePreflightFixture {
    let raw = include_str!("fixtures/atelier_core_data/backup_restore_preflight.json");
    serde_json::from_str(raw).expect("parse backup_restore_preflight.json fixture")
}

fn preflight_status_from_token(token: &str) -> BackupRestorePreflightStatus {
    match token {
        "accepted" => BackupRestorePreflightStatus::Accepted,
        "refused" => BackupRestorePreflightStatus::Refused,
        other => panic!("fixture preflight status {other} is not a valid token"),
    }
}

#[tokio::test]
async fn mt079_backup_fixture_corpus_round_trips() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt079_backup_fixture_corpus_round_trips: no PostgreSQL");
        return;
    };
    let store = connected_store(&url).await;
    let suffix = run_suffix();
    let fixture = load_backup_restore_preflight_fixture();
    assert!(!fixture.backups.is_empty(), "fixture must define backups");

    // Record every fixture backup manifest, then RE-READ it from PostgreSQL
    // and assert version traceability, checksums, and portable refs.
    let mut backups_by_key = std::collections::HashMap::new();
    for fixture_backup in &fixture.backups {
        let seed = run_unique(&fixture_backup.payload_seed, &suffix);
        let artifact = atelier_pg_support::write_native_media_artifact(seed.as_bytes());
        let mut backup_files = vec![BackupManifestFile {
            logical_path: "manifest/atelier.json".to_string(),
            content_hash: artifact.content_hash.clone(),
            byte_len: artifact.byte_len,
        }];
        for extra in &fixture_backup.extra_files {
            backup_files.push(BackupManifestFile {
                logical_path: extra.logical_path.clone(),
                content_hash: extra.content_hash.clone(),
                byte_len: extra.byte_len,
            });
        }
        let recorded = store
            .record_backup_manifest(&NewBackupManifest {
                app_version: fixture_backup.app_version.clone(),
                spec_version: fixture_backup.spec_version.clone(),
                schema_version: fixture_backup.schema_version,
                artifact_ref: artifact.artifact_ref.clone(),
                content_hash: artifact.content_hash.clone(),
                byte_len: artifact.byte_len,
                files: backup_files.clone(),
                created_by: fixture_backup.created_by.clone(),
            })
            .await
            .expect("record backup manifest from fixture");

        let reloaded = store
            .get_backup_manifest(recorded.backup_id)
            .await
            .expect("re-read backup manifest");
        assert_eq!(reloaded.backup_id, recorded.backup_id);
        assert_eq!(reloaded.app_version, fixture_backup.app_version);
        assert_eq!(reloaded.spec_version, fixture_backup.spec_version);
        assert_eq!(reloaded.schema_version, fixture_backup.schema_version);
        assert_eq!(reloaded.content_hash, artifact.content_hash);
        assert_eq!(reloaded.manifest_json, recorded.manifest_json);
        assert_eq!(
            reloaded.manifest_json["schema_id"],
            serde_json::json!(BACKUP_MANIFEST_SCHEMA_ID)
        );
        assert_eq!(
            reloaded.manifest_json["files"]
                .as_array()
                .expect("backup manifest files array")
                .len(),
            backup_files.len()
        );
        assert!(
            reloaded.manifest_hash.len() >= 32,
            "backup manifest carries its own checksum"
        );
        assert_portable_fixture_ref(
            "backup artifact_ref",
            &reloaded.artifact_ref,
            &fixture.portability,
        );
        for file in &backup_files {
            assert_portable_pack_path(
                "backup logical_path",
                &file.logical_path,
                &fixture.portability.forbidden_substrings,
            );
        }

        backups_by_key.insert(fixture_backup.key.clone(), reloaded);
    }

    // Fixture-driven version/refusal rules: same-version restores are
    // accepted; newer app/spec/schema backups are refused before any restore.
    let mut preflight_counts: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for fixture_preflight in &fixture.preflights {
        let backup = backups_by_key
            .get(&fixture_preflight.backup_key)
            .expect("fixture preflight references a recorded backup");
        let preflight = store
            .preflight_backup_restore(&BackupRestorePreflightRequest {
                backup_id: backup.backup_id,
                current_app_version: fixture_preflight.current_app_version.clone(),
                current_spec_version: fixture_preflight.current_spec_version.clone(),
                current_schema_version: fixture_preflight.current_schema_version,
                requested_by: fixture_preflight.requested_by.clone(),
            })
            .await
            .expect("record restore preflight from fixture");
        let expected_status = preflight_status_from_token(&fixture_preflight.expected_status);
        assert_eq!(
            preflight.status, expected_status,
            "preflight for backup {} matches the fixture version rule",
            fixture_preflight.backup_key
        );
        match &fixture_preflight.refusal_contains {
            Some(needle) => {
                let reason = preflight
                    .refusal_reason
                    .as_deref()
                    .expect("refused preflight carries a refusal reason");
                assert!(
                    reason.contains(needle),
                    "refusal reason for {} must mention {needle}, got {reason}",
                    fixture_preflight.backup_key
                );
            }
            None => assert!(
                preflight.refusal_reason.is_none(),
                "accepted preflight carries no refusal reason"
            ),
        }
        *preflight_counts
            .entry(fixture_preflight.backup_key.clone())
            .or_insert(0) += 1;
    }

    // EventLedger proof: one manifest-recorded event per backup, and one
    // preflight event per fixture scenario on the backup aggregate.
    for (key, backup) in &backups_by_key {
        assert_eq!(
            store
                .count_events_for_aggregate(
                    export_event_family::BACKUP_MANIFEST_RECORDED,
                    "atelier_backup_manifest",
                    &backup.backup_id.to_string(),
                )
                .await
                .expect("count backup manifest events"),
            1,
            "backup {key} records exactly one manifest event"
        );
        let expected_preflights = preflight_counts.get(key).copied().unwrap_or(0);
        assert_eq!(
            store
                .count_events_for_aggregate(
                    export_event_family::BACKUP_RESTORE_PREFLIGHT_RECORDED,
                    "atelier_backup_manifest",
                    &backup.backup_id.to_string(),
                )
                .await
                .expect("count backup preflight events"),
            expected_preflights,
            "backup {key} records one preflight event per fixture scenario"
        );
    }
}
