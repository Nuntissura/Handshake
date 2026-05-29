//! MT-150: HBR test-packet corpus loader + 60/20/20 train/dev/holdout split.
//!
//! Holdout is encrypted-at-rest using a real AEAD cipher (XChaCha20-Poly1305
//! via SHA-256-derived keystream and an HMAC-SHA256 authentication tag).
//! The keystream is implemented via SHA-256 in counter mode so we do not
//! pull a new crypto dep — the operator credential store supplies a 32-byte
//! key, and the IV is a 96-bit nonce.
//!
//! Identity is content-addressed: `content_hash = sha256(canonical_json(items))`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Validator first-pass verdict shape stored in corpus items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorVerdict {
    Pass,
    Fail,
    Skip,
}

/// One corpus item: a HBR-rule-bound test packet shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusItem {
    pub id: Uuid,
    pub hbr_rule_id: String,
    pub packet_under_test: String,
    pub expected_first_pass_verdict: ValidatorVerdict,
    pub fixtures: serde_json::Value,
}

/// Full corpus with content hash. Identity is the hash; two corpora with
/// the same items produce the same `content_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbrTestPacketCorpus {
    pub items: Vec<CorpusItem>,
    pub content_hash: String,
}

impl HbrTestPacketCorpus {
    /// Build from items. Sorts deterministically by id then recomputes the
    /// content hash.
    pub fn from_items(mut items: Vec<CorpusItem>) -> Result<Self, CorpusError> {
        if items.is_empty() {
            return Err(CorpusError::EmptyCorpus);
        }
        // Sort deterministically so the hash is stable across iteration order.
        items.sort_by_key(|it| it.id);
        let content_hash = content_hash_of(&items)?;
        Ok(Self {
            items,
            content_hash,
        })
    }

    /// Build a corpus from a JSON byte slice.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CorpusError> {
        let items: Vec<CorpusItem> = serde_json::from_slice(bytes)
            .map_err(|e| CorpusError::Deserialization(e.to_string()))?;
        Self::from_items(items)
    }

    /// Apply the canonical 60/20/20 split with the given seed. The split is
    /// deterministic from (content_hash, split_seed).
    pub fn split(
        &self,
        split_seed: u64,
        key_provider: &dyn KeyProvider,
        key_id: &str,
    ) -> Result<CorpusSplit, CorpusError> {
        let total = self.items.len();
        if total < 5 {
            return Err(CorpusError::CorpusTooSmall { got: total, min: 5 });
        }
        let train_count = total * 60 / 100;
        let dev_count = total * 20 / 100;
        let holdout_count = total - train_count - dev_count;

        // Deterministic shuffle: seed-derived stable permutation.
        let permutation = deterministic_permutation(&self.content_hash, split_seed, total);

        let mut train = Vec::with_capacity(train_count);
        let mut dev = Vec::with_capacity(dev_count);
        let mut holdout_plain = Vec::with_capacity(holdout_count);

        for (rank, source_index) in permutation.iter().enumerate() {
            let item = self.items[*source_index].clone();
            if rank < train_count {
                train.push(item);
            } else if rank < train_count + dev_count {
                dev.push(item);
            } else {
                holdout_plain.push(item);
            }
        }

        let key = key_provider
            .fetch_key(key_id)
            .map_err(CorpusError::KeyProvider)?;
        if key.len() != 32 {
            return Err(CorpusError::InvalidKey {
                message: "AEAD key must be exactly 32 bytes".to_string(),
            });
        }

        let plaintext = serde_json::to_vec(&holdout_plain)
            .map_err(|e| CorpusError::Serialization(e.to_string()))?;

        // Deterministic 12-byte nonce derived from (content_hash, split_seed)
        // so re-encrypting the same split with the same key reproduces the
        // same ciphertext. Determinism is required for replay.
        let nonce = derive_nonce(&self.content_hash, split_seed);
        let holdout = encrypt(plaintext.as_slice(), &key, &nonce, key_id)?;

        Ok(CorpusSplit {
            train,
            dev,
            holdout,
            split_seed,
            content_hash: self.content_hash.clone(),
        })
    }
}

/// Split produced by [`HbrTestPacketCorpus::split`]. Holdout is encrypted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusSplit {
    pub train: Vec<CorpusItem>,
    pub dev: Vec<CorpusItem>,
    pub holdout: EncryptedHoldout,
    pub split_seed: u64,
    pub content_hash: String,
}

impl CorpusSplit {
    /// Decrypt the holdout into a plaintext item list. The plaintext list
    /// is owned by the caller and dropped after use.
    pub fn decrypt_holdout(
        &self,
        key_provider: &dyn KeyProvider,
    ) -> Result<Vec<CorpusItem>, CorpusError> {
        self.holdout.decrypt(key_provider)
    }
}

/// Encrypted holdout split. AEAD layout: ChaCha20-Poly1305 (RFC 8439). The
/// `ciphertext` field is the AEAD output (encrypted plaintext + 16-byte
/// Poly1305 tag concatenated) per the chacha20poly1305 crate convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedHoldout {
    /// ChaCha20-Poly1305 output: encrypted plaintext bytes followed by the
    /// 16-byte Poly1305 authentication tag.
    pub ciphertext: Vec<u8>,
    /// 12-byte ChaCha20-Poly1305 nonce, derived deterministically from
    /// `(content_hash, split_seed)` so re-encrypting the same split with
    /// the same key reproduces the same ciphertext (deterministic replay).
    /// SECURITY: deterministic nonces are safe ONLY because the (key, nonce,
    /// plaintext) tuple is unique per (content_hash, split_seed); reusing
    /// the same key with the same nonce on DIFFERENT plaintext would break
    /// ChaCha20-Poly1305 confidentiality. Callers must ensure each
    /// distinct corpus split has either a distinct content_hash, split_seed,
    /// or key_id to avoid nonce-reuse.
    pub nonce: [u8; 12],
    pub key_id: String,
}

impl EncryptedHoldout {
    pub fn decrypt(&self, key_provider: &dyn KeyProvider) -> Result<Vec<CorpusItem>, CorpusError> {
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305,
        };

        let key = key_provider
            .fetch_key(&self.key_id)
            .map_err(CorpusError::KeyProvider)?;
        if key.len() != 32 {
            return Err(CorpusError::InvalidKey {
                message: "AEAD key must be exactly 32 bytes".to_string(),
            });
        }
        let cipher =
            ChaCha20Poly1305::new_from_slice(&key).map_err(|_| CorpusError::InvalidKey {
                message: "failed to construct ChaCha20Poly1305 cipher from key".to_string(),
            })?;
        let nonce = chacha20poly1305::Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, self.ciphertext.as_slice())
            .map_err(|_| CorpusError::AuthFailure)?;
        let items: Vec<CorpusItem> = serde_json::from_slice(&plaintext)
            .map_err(|e| CorpusError::Deserialization(e.to_string()))?;
        Ok(items)
    }
}

/// Source of AEAD keys. Production wires to the OS keychain; tests inject
/// a static key.
pub trait KeyProvider {
    fn fetch_key(&self, key_id: &str) -> Result<Vec<u8>, KeyError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KeyError {
    #[error("unknown key id: {key_id}")]
    UnknownKey { key_id: String },
    #[error("keychain unavailable: {message}")]
    Unavailable { message: String },
}

/// Test KeyProvider that returns a fixed 32-byte key for the given key id.
/// Production code must NOT use this in production paths.
pub struct StaticKeyProvider {
    key_id: String,
    key: Vec<u8>,
}

impl StaticKeyProvider {
    /// Construct from a 32-byte key. Returns Err for shorter keys.
    pub fn with_key(key_id: impl Into<String>, key: Vec<u8>) -> Result<Self, KeyError> {
        if key.len() != 32 {
            return Err(KeyError::Unavailable {
                message: "test key must be exactly 32 bytes".to_string(),
            });
        }
        Ok(Self {
            key_id: key_id.into(),
            key,
        })
    }

    /// Convenience constructor for deterministic tests.
    pub fn deterministic(key_id: impl Into<String>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"handshake.self_improve.test_key.v1");
        let key = hasher.finalize().to_vec();
        Self {
            key_id: key_id.into(),
            key,
        }
    }
}

impl KeyProvider for StaticKeyProvider {
    fn fetch_key(&self, key_id: &str) -> Result<Vec<u8>, KeyError> {
        if key_id != self.key_id {
            return Err(KeyError::UnknownKey {
                key_id: key_id.to_string(),
            });
        }
        Ok(self.key.clone())
    }
}

fn content_hash_of(items: &[CorpusItem]) -> Result<String, CorpusError> {
    let bytes = serde_json::to_vec(items).map_err(|e| CorpusError::Serialization(e.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex_encode(hasher.finalize().as_slice()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Build a deterministic permutation of `[0..total)` from `(content_hash, seed)`.
/// Fisher-Yates with a SHA-256-counter RNG seeded by the inputs.
fn deterministic_permutation(content_hash: &str, seed: u64, total: usize) -> Vec<usize> {
    let mut perm: Vec<usize> = (0..total).collect();
    let mut rng_state = Sha256::new();
    rng_state.update(content_hash.as_bytes());
    rng_state.update(seed.to_be_bytes());
    let mut buffer = rng_state.finalize().to_vec();
    let mut buffer_pos = 0;

    for i in (1..total).rev() {
        // Refill buffer when exhausted.
        if buffer_pos + 8 > buffer.len() {
            let mut next = Sha256::new();
            next.update(&buffer);
            buffer = next.finalize().to_vec();
            buffer_pos = 0;
        }
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&buffer[buffer_pos..buffer_pos + 8]);
        buffer_pos += 8;
        let j = (u64::from_be_bytes(raw) % (i as u64 + 1)) as usize;
        perm.swap(i, j);
    }

    perm
}

fn derive_nonce(content_hash: &str, seed: u64) -> [u8; 12] {
    let mut hasher = Sha256::new();
    hasher.update(b"handshake.self_improve.holdout_nonce.v1");
    hasher.update(content_hash.as_bytes());
    hasher.update(seed.to_be_bytes());
    let digest = hasher.finalize();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&digest[..12]);
    nonce
}

/// Encrypt plaintext under ChaCha20-Poly1305 (RFC 8439). The returned
/// `EncryptedHoldout.ciphertext` contains the encrypted plaintext bytes
/// followed by the 16-byte Poly1305 authentication tag, per the
/// chacha20poly1305 crate convention. Replaces the prior hand-rolled
/// SHA-256-CTR + HMAC-SHA256 construction.
fn encrypt(
    plaintext: &[u8],
    key: &[u8],
    nonce: &[u8; 12],
    key_id: &str,
) -> Result<EncryptedHoldout, CorpusError> {
    use chacha20poly1305::{
        aead::{Aead, KeyInit},
        ChaCha20Poly1305,
    };

    let cipher = ChaCha20Poly1305::new_from_slice(key).map_err(|_| CorpusError::InvalidKey {
        message: "failed to construct ChaCha20Poly1305 cipher from key".to_string(),
    })?;
    let nonce_ref = chacha20poly1305::Nonce::from_slice(nonce);
    let ciphertext = cipher
        .encrypt(nonce_ref, plaintext)
        .map_err(|_| CorpusError::InvalidKey {
            message: "ChaCha20Poly1305 encryption failed".to_string(),
        })?;
    Ok(EncryptedHoldout {
        ciphertext,
        nonce: *nonce,
        key_id: key_id.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CorpusError {
    #[error("corpus is empty")]
    EmptyCorpus,
    #[error("corpus too small: got {got} items, minimum is {min}")]
    CorpusTooSmall { got: usize, min: usize },
    #[error("corpus serialization failed: {0}")]
    Serialization(String),
    #[error("corpus deserialization failed: {0}")]
    Deserialization(String),
    #[error(transparent)]
    KeyProvider(#[from] KeyError),
    #[error("AEAD key invalid: {message}")]
    InvalidKey { message: String },
    #[error("AEAD authentication failure (ciphertext tampered or wrong key)")]
    AuthFailure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_item(idx: u128, hbr: &str, verdict: ValidatorVerdict) -> CorpusItem {
        CorpusItem {
            id: Uuid::from_u128(idx),
            hbr_rule_id: hbr.to_string(),
            packet_under_test: format!("packet-{idx}"),
            expected_first_pass_verdict: verdict,
            fixtures: json!({ "idx": idx }),
        }
    }

    fn sample_corpus(n: u128) -> HbrTestPacketCorpus {
        let items: Vec<CorpusItem> = (1..=n)
            .map(|i| {
                let pillars = [
                    "HBR-INT-001",
                    "HBR-SWARM-001",
                    "HBR-VIS-001",
                    "HBR-QUIET-001",
                    "HBR-MAN-001",
                ];
                sample_item(
                    i,
                    pillars[(i as usize - 1) % pillars.len()],
                    ValidatorVerdict::Pass,
                )
            })
            .collect();
        HbrTestPacketCorpus::from_items(items).unwrap()
    }

    #[test]
    fn corpus_hash_changes_with_items() {
        let a = sample_corpus(30);
        let mut items = a.items.clone();
        items[0].packet_under_test = "different-packet".to_string();
        let b = HbrTestPacketCorpus::from_items(items).unwrap();
        assert_ne!(a.content_hash, b.content_hash);
    }

    #[test]
    fn split_sizes_are_correct_for_30_items() {
        let corpus = sample_corpus(30);
        let kp = StaticKeyProvider::deterministic("hbr-test-corpus-key");
        let split = corpus.split(42, &kp, "hbr-test-corpus-key").unwrap();
        assert_eq!(split.train.len(), 18);
        assert_eq!(split.dev.len(), 6);

        let holdout_plain = split.decrypt_holdout(&kp).unwrap();
        assert_eq!(holdout_plain.len(), 6);
    }

    #[test]
    fn split_is_deterministic_for_same_seed() {
        let corpus = sample_corpus(30);
        let kp = StaticKeyProvider::deterministic("k");
        let a = corpus.split(42, &kp, "k").unwrap();
        let b = corpus.split(42, &kp, "k").unwrap();

        // The encrypted holdout should be byte-identical because nonce derives from
        // (content_hash, seed).
        assert_eq!(a.holdout.ciphertext, b.holdout.ciphertext);
        assert_eq!(a.holdout.nonce, b.holdout.nonce);
        assert_eq!(
            a.train.iter().map(|i| i.id).collect::<Vec<_>>(),
            b.train.iter().map(|i| i.id).collect::<Vec<_>>()
        );
        assert_eq!(
            a.dev.iter().map(|i| i.id).collect::<Vec<_>>(),
            b.dev.iter().map(|i| i.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn split_changes_with_different_seed() {
        let corpus = sample_corpus(30);
        let kp = StaticKeyProvider::deterministic("k");
        let a = corpus.split(42, &kp, "k").unwrap();
        let b = corpus.split(43, &kp, "k").unwrap();
        // With high probability, the holdout members differ.
        let a_ids: std::collections::BTreeSet<Uuid> = a
            .decrypt_holdout(&kp)
            .unwrap()
            .iter()
            .map(|i| i.id)
            .collect();
        let b_ids: std::collections::BTreeSet<Uuid> = b
            .decrypt_holdout(&kp)
            .unwrap()
            .iter()
            .map(|i| i.id)
            .collect();
        assert_ne!(a_ids, b_ids);
    }

    #[test]
    fn holdout_cannot_be_read_without_key() {
        let corpus = sample_corpus(30);
        let kp = StaticKeyProvider::deterministic("real-key");
        let split = corpus.split(7, &kp, "real-key").unwrap();
        // Trying to decrypt with a provider that doesn't know "real-key" fails.
        let wrong_kp = StaticKeyProvider::deterministic("other-key");
        let err = split.decrypt_holdout(&wrong_kp).unwrap_err();
        assert!(matches!(
            err,
            CorpusError::KeyProvider(KeyError::UnknownKey { .. })
        ));
    }

    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let corpus = sample_corpus(30);
        let kp = StaticKeyProvider::deterministic("k");
        let mut split = corpus.split(9, &kp, "k").unwrap();
        split.holdout.ciphertext[0] ^= 0xff;
        let err = split.decrypt_holdout(&kp).unwrap_err();
        assert_eq!(err, CorpusError::AuthFailure);
    }

    #[test]
    fn split_rejects_too_small_corpus() {
        let items = vec![sample_item(1, "HBR-INT-001", ValidatorVerdict::Pass)];
        let corpus = HbrTestPacketCorpus::from_items(items).unwrap();
        let kp = StaticKeyProvider::deterministic("k");
        let err = corpus.split(1, &kp, "k").unwrap_err();
        assert!(matches!(err, CorpusError::CorpusTooSmall { .. }));
    }
}
