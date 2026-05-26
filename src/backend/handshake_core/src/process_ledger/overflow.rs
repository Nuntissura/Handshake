use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::table::PROCESS_LEDGER_METADATA_CAP_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataCapOutcome {
    pub value: Value,
    pub was_capped: bool,
    pub original_bytes: Option<usize>,
}

pub fn cap_metadata_jsonb(metadata: &BTreeMap<String, String>) -> MetadataCapOutcome {
    let value = json!(metadata);
    cap_metadata_value(value)
}

pub fn cap_metadata_value(value: Value) -> MetadataCapOutcome {
    let bytes = serde_json::to_vec(&value).unwrap_or_default();
    if bytes.len() <= PROCESS_LEDGER_METADATA_CAP_BYTES {
        return MetadataCapOutcome {
            value,
            was_capped: false,
            original_bytes: None,
        };
    }

    MetadataCapOutcome {
        value: json!({
            "capped": true,
            "original_bytes": bytes.len(),
        }),
        was_capped: true,
        original_bytes: Some(bytes.len()),
    }
}
