use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

pub(crate) use handshake_storage_support::canonical_json::{canonical_json_bytes, sha256_hex};
