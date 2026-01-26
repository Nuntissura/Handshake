use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexConfig {
    pub algorithm: String,
    #[serde(rename = "M")]
    pub m: u32,
    pub ef_construction: u32,
    pub ef_search: u32,
    pub metric: String,
    pub dimensions: u32,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            algorithm: "hnsw".to_string(),
            m: 16,
            ef_construction: 200,
            ef_search: 100,
            metric: "cosine".to_string(),
            dimensions: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordIndexConfig {
    pub algorithm: String,
    pub k1: f64,
    pub b: f64,
}

impl Default for KeywordIndexConfig {
    fn default() -> Self {
        Self {
            algorithm: "bm25".to_string(),
            k1: 1.2,
            b: 0.75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexEntry {
    pub silver_id: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexArtifact {
    pub schema_version: String,
    pub config: VectorIndexConfig,
    pub model_id: String,
    pub model_version: String,
    pub dimensions: u32,
    pub entries: Vec<VectorIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordDocumentEntry {
    pub silver_id: String,
    pub length: u32,
    pub term_frequencies: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordIndexArtifact {
    pub schema_version: String,
    pub config: KeywordIndexConfig,
    pub doc_count: u32,
    pub avg_doc_length: f64,
    pub doc_freq: BTreeMap<String, u32>,
    pub documents: Vec<KeywordDocumentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub relationship_type: String,
    pub source_id: String,
    pub target_id: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphArtifact {
    pub schema_version: String,
    pub edges: Vec<GraphEdge>,
}

pub fn tokenize_keyword(text: &str) -> Vec<String> {
    let normalized = crate::ace::normalize_query(text);
    normalized
        .split_ascii_whitespace()
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_string())
        .collect()
}

pub fn build_keyword_index(
    documents: Vec<(String, String)>,
    config: KeywordIndexConfig,
) -> KeywordIndexArtifact {
    let mut doc_freq: BTreeMap<String, u32> = BTreeMap::new();
    let mut entries: Vec<KeywordDocumentEntry> = Vec::with_capacity(documents.len());
    let mut total_length: u64 = 0;

    let mut documents_sorted = documents;
    documents_sorted.sort_by(|left, right| left.0.cmp(&right.0));

    for (silver_id, text) in documents_sorted {
        let tokens = tokenize_keyword(&text);
        let mut term_frequencies: BTreeMap<String, u32> = BTreeMap::new();
        for token in tokens {
            *term_frequencies.entry(token).or_insert(0) += 1;
        }

        for term in term_frequencies.keys() {
            *doc_freq.entry(term.clone()).or_insert(0) += 1;
        }

        let length: u32 = term_frequencies.values().copied().sum();
        total_length = total_length.saturating_add(length as u64);

        entries.push(KeywordDocumentEntry {
            silver_id,
            length,
            term_frequencies,
        });
    }

    let doc_count = entries.len() as u32;
    let avg_doc_length = if doc_count == 0 {
        0.0
    } else {
        (total_length as f64) / (doc_count as f64)
    };

    KeywordIndexArtifact {
        schema_version: "1.0".to_string(),
        config,
        doc_count,
        avg_doc_length,
        doc_freq,
        documents: entries,
    }
}

pub fn build_vector_index(
    mut entries: Vec<VectorIndexEntry>,
    config: VectorIndexConfig,
    model_id: String,
    model_version: String,
) -> VectorIndexArtifact {
    entries.sort_by(|left, right| left.silver_id.cmp(&right.silver_id));
    let dimensions = config.dimensions;
    VectorIndexArtifact {
        schema_version: "1.0".to_string(),
        config,
        model_id,
        model_version,
        dimensions,
        entries,
    }
}
