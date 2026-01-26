#[derive(Debug, Clone)]
pub struct QualitySLOs {
    pub min_mrr: f64,
    pub min_recall_at_10: f64,
    pub min_ndcg_at_5: f64,
    pub min_validation_pass_rate: f64,
    pub min_metadata_completeness: f64,
    pub max_stale_records_ratio: f64,
    pub max_p95_retrieval_ms: u64,
    pub max_p99_retrieval_ms: u64,
    pub max_indexing_delay_seconds: u64,
    pub max_orphan_record_ratio: f64,
}

impl Default for QualitySLOs {
    fn default() -> Self {
        Self {
            min_mrr: 0.6,
            min_recall_at_10: 0.8,
            min_ndcg_at_5: 0.7,
            min_validation_pass_rate: 0.95,
            min_metadata_completeness: 0.99,
            max_stale_records_ratio: 0.05,
            max_p95_retrieval_ms: 500,
            max_p99_retrieval_ms: 1000,
            max_indexing_delay_seconds: 5,
            max_orphan_record_ratio: 0.01,
        }
    }
}

pub fn metadata_completeness_ratio(required_fields: &[&str], present_field_names: &[&str]) -> f64 {
    if required_fields.is_empty() {
        return 1.0;
    }

    let present = required_fields
        .iter()
        .filter(|required| present_field_names.iter().any(|p| p == *required))
        .count();

    present as f64 / required_fields.len() as f64
}
