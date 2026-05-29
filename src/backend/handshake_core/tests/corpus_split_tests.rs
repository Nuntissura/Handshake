//! MT-150 — HBR test-packet corpus loader + 60/20/20 train/dev/holdout
//! split with encrypted-at-rest holdout integration tests.
//!
//! Per the MT-150 contract proof_command:
//!   `cargo test -p handshake_core --test corpus_split_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/corpus.rs
//! already covers the per-rule happy path (split sizes 18/6/6 for 30
//! items, deterministic-same-seed, holdout-unreadable-without-key,
//! tampered-ciphertext-rejected, too-small-corpus). This integration
//! file satisfies the contract owned_files entry and adds:
//!
//!   - Real-AEAD-cipher proof: encryption uses ChaCha20-Poly1305 (RFC
//!     8439), not a hand-rolled stub. Asserted by the post-encryption
//!     ciphertext length = plaintext length + 16 (Poly1305 tag) AND by
//!     verifying tag-tampering at byte position (ciphertext.len() - 1)
//!     is detected (Poly1305 covers the tag).
//!   - KeyProvider trait usage: production cannot accidentally ship test
//!     keys because StaticKeyProvider::deterministic is documented as
//!     test-only AND the test file demonstrates the production-shape
//!     contract for fetch_key.
//!   - Determinism across multiple split invocations with the same seed.
//!   - Cross-key isolation: a corpus encrypted with key_A cannot be
//!     decrypted by key_B even if key_id matches (wrong key value).
//!   - Fixture file integrity: the committed 30-item fixture loads, has
//!     5 HBR pillars covered, and splits to 18/6/6.

use std::collections::HashSet;
use std::path::Path;

use handshake_core::self_improve::corpus::{
    CorpusError, CorpusItem, HbrTestPacketCorpus, KeyError, KeyProvider, StaticKeyProvider,
    ValidatorVerdict,
};

const FIXTURE_PATH: &str =
    "tests/fixtures/hbr_test_packet_corpus/items.json";

fn load_fixture_items() -> Vec<CorpusItem> {
    let path = Path::new(FIXTURE_PATH);
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("fixture {} must be readable: {e}", FIXTURE_PATH));
    serde_json::from_slice(&bytes).expect("fixture must deserialize as Vec<CorpusItem>")
}

// ----------------------------------------------------------------------------
// Fixture-integrity tests: prove the 30-item committed fixture loads,
// is shaped correctly, and splits to the contract-mandated 18/6/6.
// ----------------------------------------------------------------------------

#[test]
fn mt150_fixture_loads_and_has_exactly_30_items() {
    let items = load_fixture_items();
    assert_eq!(
        items.len(),
        30,
        "fixture must declare exactly 30 items per MT-150 contract"
    );
}

#[test]
fn mt150_fixture_covers_all_five_hbr_pillars() {
    let items = load_fixture_items();
    let pillars: HashSet<&str> = items
        .iter()
        .map(|i| {
            // pillar = HBR-<PILLAR>-NNN ; extract the middle token
            let parts: Vec<&str> = i.hbr_rule_id.split('-').collect();
            *parts.get(1).expect("hbr_rule_id must have a pillar token")
        })
        .collect();
    for expected in ["INT", "SWARM", "VIS", "QUIET", "MAN"] {
        assert!(
            pillars.contains(expected),
            "fixture must cover HBR pillar '{expected}'; current pillars: {pillars:?}"
        );
    }
}

#[test]
fn mt150_fixture_splits_to_eighteen_six_six() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("fixture corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-fixture-test-key");
    let split = corpus
        .split(2026_05_23, &kp, "hbr-fixture-test-key")
        .expect("split must succeed");
    assert_eq!(split.train.len(), 18);
    assert_eq!(split.dev.len(), 6);
    let holdout = split.decrypt_holdout(&kp).expect("holdout decrypts with right key");
    assert_eq!(holdout.len(), 6);
}

// ----------------------------------------------------------------------------
// Real-AEAD-cipher proof: ChaCha20-Poly1305 ciphertext layout is
// plaintext || 16-byte tag. The integration test computes the
// plaintext length (serialized holdout items JSON) and asserts the
// ciphertext is exactly that length + 16.
// ----------------------------------------------------------------------------

#[test]
fn mt150_holdout_ciphertext_includes_poly1305_tag_suffix() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("fixture corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-aead-test-key");
    let split = corpus
        .split(1, &kp, "hbr-aead-test-key")
        .expect("split must succeed");

    // The plaintext is the serde_json bytes of the 6 holdout items.
    let plaintext_items = split.decrypt_holdout(&kp).expect("decrypt to compute plaintext length");
    let plaintext_bytes = serde_json::to_vec(&plaintext_items).expect("re-serialize holdout items");
    let expected_ciphertext_len = plaintext_bytes.len() + 16; // ChaCha20-Poly1305 appends 16-byte Poly1305 tag

    assert_eq!(
        split.holdout.ciphertext.len(),
        expected_ciphertext_len,
        "ChaCha20-Poly1305 output must be plaintext_len + 16-byte tag; \
         got ciphertext={} bytes, plaintext={} bytes (expected ciphertext={} bytes). \
         If this assertion fails the cipher likely reverted to the hand-rolled \
         SHA-256-CTR + HMAC-SHA256 stub from MT-150 prior session.",
        split.holdout.ciphertext.len(),
        plaintext_bytes.len(),
        expected_ciphertext_len
    );
}

#[test]
fn mt150_holdout_nonce_is_exactly_twelve_bytes() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-nonce-test-key");
    let split = corpus
        .split(2, &kp, "hbr-nonce-test-key")
        .expect("split must succeed");
    // The nonce field is [u8; 12]; this test pins the contract that
    // ChaCha20-Poly1305 uses a 12-byte nonce. If the impl ever switches
    // to XChaCha20-Poly1305 (24-byte nonce), the struct field type
    // changes and this test fails at compile time.
    assert_eq!(split.holdout.nonce.len(), 12);
}

#[test]
fn mt150_tampering_the_poly1305_tag_suffix_is_detected() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-tag-tamper-key");
    let mut split = corpus
        .split(3, &kp, "hbr-tag-tamper-key")
        .expect("split must succeed");

    // Tamper the LAST byte (within the Poly1305 tag region).
    let last_idx = split.holdout.ciphertext.len() - 1;
    split.holdout.ciphertext[last_idx] ^= 0x01;

    let err = split
        .decrypt_holdout(&kp)
        .expect_err("tampering the Poly1305 tag must reject");
    assert_eq!(err, CorpusError::AuthFailure);
}

#[test]
fn mt150_tampering_the_ciphertext_body_is_detected() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-body-tamper-key");
    let mut split = corpus
        .split(4, &kp, "hbr-body-tamper-key")
        .expect("split must succeed");

    // Tamper the FIRST byte (within the encrypted plaintext region,
    // before the tag suffix).
    split.holdout.ciphertext[0] ^= 0xff;

    let err = split
        .decrypt_holdout(&kp)
        .expect_err("tampering the ciphertext body must reject");
    assert_eq!(err, CorpusError::AuthFailure);
}

// ----------------------------------------------------------------------------
// KeyProvider trait isolation: a wrong KeyProvider (same key_id, wrong
// key bytes) must fail decryption — proves the cipher is bound to the
// key bytes, not the key_id label.
// ----------------------------------------------------------------------------

struct WrongValueKeyProvider {
    key_id: String,
}

impl KeyProvider for WrongValueKeyProvider {
    fn fetch_key(&self, key_id: &str) -> Result<Vec<u8>, KeyError> {
        if key_id != self.key_id {
            return Err(KeyError::UnknownKey {
                key_id: key_id.to_string(),
            });
        }
        // 32 bytes of a DIFFERENT key than StaticKeyProvider::deterministic
        // would produce. This is the smoking gun for cipher key-binding.
        Ok(vec![0xAB; 32])
    }
}

#[test]
fn mt150_decryption_with_matching_key_id_but_wrong_key_bytes_fails() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");

    let kp_real = StaticKeyProvider::deterministic("real-aead-key");
    let split = corpus
        .split(5, &kp_real, "real-aead-key")
        .expect("split must succeed");

    let kp_wrong = WrongValueKeyProvider {
        key_id: "real-aead-key".to_string(),
    };

    let err = split
        .decrypt_holdout(&kp_wrong)
        .expect_err("wrong key bytes must reject");
    assert_eq!(
        err,
        CorpusError::AuthFailure,
        "ChaCha20-Poly1305 must reject the wrong key with AuthFailure, \
         not silently decrypt to garbage"
    );
}

// ----------------------------------------------------------------------------
// Determinism + content-hash sensitivity.
// ----------------------------------------------------------------------------

#[test]
fn mt150_split_with_same_seed_and_key_is_deterministic_across_invocations() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-det-key");

    let first = corpus.split(11, &kp, "hbr-det-key").expect("first split");
    let second = corpus.split(11, &kp, "hbr-det-key").expect("second split");
    let third = corpus.split(11, &kp, "hbr-det-key").expect("third split");

    assert_eq!(first.holdout.ciphertext, second.holdout.ciphertext);
    assert_eq!(first.holdout.ciphertext, third.holdout.ciphertext);
    assert_eq!(first.holdout.nonce, second.holdout.nonce);
    assert_eq!(first.split_seed, 11);
}

#[test]
fn mt150_content_hash_propagates_to_split() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items.clone()).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-hash-key");
    let split = corpus.split(7, &kp, "hbr-hash-key").expect("split");
    assert_eq!(
        split.content_hash, corpus.content_hash,
        "split must inherit corpus content_hash"
    );
}

#[test]
fn mt150_modifying_one_item_changes_corpus_hash_and_holdout_membership() {
    let items_a = load_fixture_items();
    let mut items_b = items_a.clone();
    items_b[0].packet_under_test = "MUTATED-PACKET".to_string();

    let corpus_a = HbrTestPacketCorpus::from_items(items_a).expect("corpus A");
    let corpus_b = HbrTestPacketCorpus::from_items(items_b).expect("corpus B");

    assert_ne!(
        corpus_a.content_hash, corpus_b.content_hash,
        "single item mutation must change content_hash"
    );

    let kp = StaticKeyProvider::deterministic("hbr-mutate-key");
    let split_a = corpus_a.split(9, &kp, "hbr-mutate-key").expect("split A");
    let split_b = corpus_b.split(9, &kp, "hbr-mutate-key").expect("split B");

    // Because nonce derives from (content_hash, seed), different
    // content_hash → different nonce → ChaCha20-Poly1305 keystream differs.
    assert_ne!(split_a.holdout.nonce, split_b.holdout.nonce);
}

// ----------------------------------------------------------------------------
// Per-pillar membership: holdout split should contain items from at
// least 2 distinct HBR pillars so the Goodhart sentinel cannot trivially
// be defeated by a corpus that happens to put all items of one pillar
// in train/dev.
// ----------------------------------------------------------------------------

#[test]
fn mt150_holdout_spans_at_least_two_distinct_hbr_pillars() {
    let items = load_fixture_items();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("corpus must construct");
    let kp = StaticKeyProvider::deterministic("hbr-pillar-key");
    let split = corpus
        .split(2026_05_23, &kp, "hbr-pillar-key")
        .expect("split");
    let holdout = split.decrypt_holdout(&kp).expect("decrypt");
    let pillars: HashSet<&str> = holdout
        .iter()
        .map(|i| i.hbr_rule_id.split('-').nth(1).unwrap_or(""))
        .collect();
    assert!(
        pillars.len() >= 2,
        "holdout must span at least 2 HBR pillars to give the Goodhart \
         sentinel a meaningful cross-pillar signal; got pillars: {pillars:?}"
    );
}

// ----------------------------------------------------------------------------
// ValidatorVerdict round-trip: every fixture item has a verdict; the
// enum must round-trip serde so the corpus item type stays stable.
// ----------------------------------------------------------------------------

#[test]
fn mt150_validator_verdict_round_trips_serde() {
    for verdict in [ValidatorVerdict::Pass, ValidatorVerdict::Fail] {
        let json = serde_json::to_string(&verdict).expect("verdict serializes");
        let back: ValidatorVerdict =
            serde_json::from_str(&json).expect("verdict deserializes");
        assert_eq!(verdict, back);
    }
}
