//! WP-KERNEL-005 atelier fixture corpus: portable JSON fixtures round-tripped
//! through the real `AtelierStore` against live PostgreSQL (no SQLite, ever).
//!
//! Each fixture (MT-061..065) is a portable, non-SQLite JSON document under
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
use handshake_core::atelier::intake::{
    IntakeBatchMode, IntakeLane, IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
};
use handshake_core::atelier::moodboards::NewMoodboardSnapshot;
use handshake_core::atelier::relationships::NewCharacterRelationship;
use handshake_core::atelier::search::{
    MatchType, NewAiTagSuggestion, NewSavedSearch, NewTagRule, SavedSearchFilters, TagType,
};
use handshake_core::atelier::{
    AtelierStore, MediaReviewMetadataUpdate, NewCharacter, NewMediaAsset, NewSheetVersion,
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
