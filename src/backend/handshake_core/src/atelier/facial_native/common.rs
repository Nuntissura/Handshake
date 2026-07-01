use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FACIAL_NATIVE_REGISTRY_SCHEMA_ID: &str = "hsk.atelier.facial_native.registry@1";
pub const FACIAL_NATIVE_RUN_SCHEMA_ID: &str = "hsk.atelier.facial_native.run@1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialNativeFeature {
    pub feature_id: String,
    pub capability: String,
    pub source_family: String,
    pub native_field: String,
    pub artifact_contract: String,
    pub status: String,
    pub native_route: String,
    pub provenance_note: String,
    pub required_config_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

impl FacialNativeFeature {
    pub fn is_selected_by_profile(&self, profile_tokens: &[String]) -> bool {
        profile_tokens
            .iter()
            .any(|token| token.as_str() == self.capability)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialNativeRunItem {
    pub item_id: String,
    pub source_ref: String,
    pub lane: String,
    pub decode_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialNativeImageContext {
    pub item_id: String,
    pub source_ref: String,
    pub file_name: String,
    pub lane: String,
    pub byte_len: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub decode_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub megapixels: Option<f64>,
}

impl FacialNativeImageContext {
    pub fn has_content_hash(&self) -> bool {
        self.content_hash
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    }

    pub fn is_decoded(&self) -> bool {
        self.decode_status == "decoded"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacialNativeRunRequest {
    pub batch_id: String,
    pub profile: String,
    pub requested_by: String,
    pub profile_tokens: Vec<String>,
    pub items: Vec<FacialNativeRunItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialNativeRunFeatureRecord {
    pub feature_id: String,
    pub capability: String,
    pub source_family: String,
    pub status: String,
    pub native_route: String,
    pub artifact_contract: String,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialNativeRunReport {
    pub schema_id: String,
    pub registry_schema_id: String,
    pub run_id: String,
    pub batch_id: String,
    pub profile: String,
    pub requested_by: String,
    pub profile_tokens: Vec<String>,
    pub item_count: usize,
    pub decoded_count: usize,
    pub selected_feature_ids: Vec<String>,
    pub run_status: String,
    pub status_counts: BTreeMap<String, usize>,
    pub degraded_reasons: Vec<String>,
    pub feature_records: Vec<FacialNativeRunFeatureRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifest_refs: Vec<String>,
    pub run_hash: String,
}
