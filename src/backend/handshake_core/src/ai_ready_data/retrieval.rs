use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai_ready_data::indexing::{KeywordIndexArtifact, VectorIndexArtifact};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridWeights {
    pub vector: f64,
    pub keyword: f64,
    pub graph: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridRetrievalParams {
    pub k: usize,
    pub vector_candidates: usize,
    pub keyword_candidates: usize,
    pub graph_hops: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridQuery {
    pub query: String,
    pub query_intent: String,
    pub weights: HybridWeights,
    pub retrieval: HybridRetrievalParams,
    pub rerank: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub silver_id: String,
    pub final_score: f64,
    pub vector_score: Option<f64>,
    pub keyword_score: Option<f64>,
    pub graph_score: Option<f64>,
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    let mut dot: f64 = 0.0;
    let mut left_norm: f64 = 0.0;
    let mut right_norm: f64 = 0.0;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        let left_f64 = *left_value as f64;
        let right_f64 = *right_value as f64;
        dot += left_f64 * right_f64;
        left_norm += left_f64 * left_f64;
        right_norm += right_f64 * right_f64;
    }

    let denom = left_norm.sqrt() * right_norm.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

pub fn vector_search(
    index: &VectorIndexArtifact,
    query_vector: &[f32],
    max_candidates: usize,
) -> Vec<(String, f64)> {
    let mut scored: Vec<(String, f64)> = index
        .entries
        .iter()
        .map(|entry| {
            let score = cosine_similarity(&entry.vector, query_vector);
            (entry.silver_id.clone(), score)
        })
        .collect();

    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    if scored.len() > max_candidates {
        scored.truncate(max_candidates);
    }

    scored
}

fn bm25_idf(doc_count: u32, doc_freq: u32) -> f64 {
    let numerator = (doc_count as f64) - (doc_freq as f64) + 0.5;
    let denominator = (doc_freq as f64) + 0.5;
    ((numerator / denominator) + 1.0).ln()
}

pub fn keyword_search(
    index: &KeywordIndexArtifact,
    query_tokens: &[String],
    max_candidates: usize,
) -> Vec<(String, f64)> {
    let doc_count = index.doc_count.max(1);
    let avg_len = index.avg_doc_length.max(1.0);
    let mut scores: Vec<(String, f64)> = Vec::with_capacity(index.documents.len());

    for doc in &index.documents {
        let doc_len = doc.length.max(1) as f64;
        let mut score: f64 = 0.0;

        for token in query_tokens {
            let Some(tf) = doc.term_frequencies.get(token) else {
                continue;
            };
            let df = index.doc_freq.get(token).copied().unwrap_or(0);
            let idf = bm25_idf(doc_count, df);
            let tf_f64 = *tf as f64;
            let denom = tf_f64
                + index.config.k1 * (1.0 - index.config.b + index.config.b * (doc_len / avg_len));
            if denom > 0.0 {
                score += idf * (tf_f64 * (index.config.k1 + 1.0)) / denom;
            }
        }

        scores.push((doc.silver_id.clone(), score));
    }

    scores.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    if scores.len() > max_candidates {
        scores.truncate(max_candidates);
    }

    scores
}

pub fn hybrid_fuse_rrf(
    vector_results: &[(String, f64)],
    keyword_results: &[(String, f64)],
    graph_results: &[(String, f64)],
    weights: &HybridWeights,
    top_k: usize,
) -> Vec<(String, f64)> {
    const RRF_K: f64 = 60.0;
    let mut combined: HashMap<String, f64> = HashMap::new();

    for (rank, (silver_id, _score)) in vector_results.iter().enumerate() {
        let rrf = weights.vector * (1.0 / (RRF_K + (rank as f64) + 1.0));
        *combined.entry(silver_id.clone()).or_insert(0.0) += rrf;
    }
    for (rank, (silver_id, _score)) in keyword_results.iter().enumerate() {
        let rrf = weights.keyword * (1.0 / (RRF_K + (rank as f64) + 1.0));
        *combined.entry(silver_id.clone()).or_insert(0.0) += rrf;
    }
    for (rank, (silver_id, _score)) in graph_results.iter().enumerate() {
        let rrf = weights.graph * (1.0 / (RRF_K + (rank as f64) + 1.0));
        *combined.entry(silver_id.clone()).or_insert(0.0) += rrf;
    }

    let mut out: Vec<(String, f64)> = combined.into_iter().collect();
    out.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    if out.len() > top_k {
        out.truncate(top_k);
    }
    out
}

pub fn build_hybrid_results(
    fused: &[(String, f64)],
    vector_results: &[(String, f64)],
    keyword_results: &[(String, f64)],
) -> Vec<HybridSearchResult> {
    let vector_map: HashMap<&str, f64> = vector_results
        .iter()
        .map(|(silver_id, score)| (silver_id.as_str(), *score))
        .collect();
    let keyword_map: HashMap<&str, f64> = keyword_results
        .iter()
        .map(|(silver_id, score)| (silver_id.as_str(), *score))
        .collect();

    fused
        .iter()
        .map(|(silver_id, final_score)| HybridSearchResult {
            silver_id: silver_id.clone(),
            final_score: *final_score,
            vector_score: vector_map.get(silver_id.as_str()).copied(),
            keyword_score: keyword_map.get(silver_id.as_str()).copied(),
            graph_score: None,
        })
        .collect()
}
