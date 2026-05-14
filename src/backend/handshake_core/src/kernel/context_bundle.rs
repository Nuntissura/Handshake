use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{KernelError, KernelResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextBundle {
    pub context_bundle_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub allowed_context: Value,
    pub context_hash: String,
    pub created_at: DateTime<Utc>,
}

impl ContextBundle {
    pub fn new(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        allowed_context: Value,
    ) -> KernelResult<Self> {
        let kernel_task_run_id = kernel_task_run_id.into();
        let session_run_id = session_run_id.into();
        if kernel_task_run_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("kernel_task_run_id is required"));
        }
        if session_run_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("session_run_id is required"));
        }
        let context_hash = sha256_hex(&canonical_json_bytes(&allowed_context));
        let context_bundle_id = format!("CTX-{}", &context_hash[..16]);
        Ok(Self {
            context_bundle_id,
            kernel_task_run_id,
            session_run_id,
            allowed_context,
            context_hash,
            created_at: Utc::now(),
        })
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(crate) fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &Value) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    ch => output.push(ch),
                }
            }
            output.push('"');
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &Value::String((*key).clone()));
                output.push(':');
                if let Some(value) = map.get(*key) {
                    write_canonical_json(output, value);
                }
            }
            output.push('}');
        }
    }
}
