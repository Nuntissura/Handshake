use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const HBR_SWARM_INVARIANT_FAIL: &str = "HBR_SWARM_INVARIANT_FAIL";
pub const FR_EVT_LOOP_CAP: &str = "FR-EVT-LOOP-CAP";
pub const HBR_SWARM_002_LOOP_CAP: usize = 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HbrSwarmInvariantFail {
    pub receipt_kind: String,
    pub receipt_uuid: Uuid,
    pub wp_id: String,
    pub invariant_id: String,
    pub details: Value,
    pub emitted_at_utc: DateTime<Utc>,
}

impl HbrSwarmInvariantFail {
    pub fn new(invariant_id: impl Into<String>, wp_id: impl Into<String>, details: Value) -> Self {
        Self {
            receipt_kind: HBR_SWARM_INVARIANT_FAIL.to_string(),
            receipt_uuid: Uuid::now_v7(),
            wp_id: wp_id.into(),
            invariant_id: invariant_id.into(),
            details,
            emitted_at_utc: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbrSwarmLoopCapReceipt {
    pub event_type: String,
    pub receipt_uuid: Uuid,
    pub loop_id: String,
    pub reason: String,
    pub iterations: usize,
    pub cap: usize,
    pub emitted_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HbrSwarmLoopCounter {
    loop_id: String,
    cap: usize,
    iterations: usize,
    terminated: bool,
}

impl HbrSwarmLoopCounter {
    pub fn new(loop_id: impl Into<String>, cap: usize) -> Self {
        Self {
            loop_id: loop_id.into(),
            cap,
            iterations: 0,
            terminated: false,
        }
    }

    pub fn tick(&mut self, reason: impl Into<String>) -> Option<HbrSwarmLoopCapReceipt> {
        if self.terminated {
            return None;
        }
        self.iterations = self.iterations.saturating_add(1);
        if self.iterations < self.cap {
            return None;
        }
        self.terminated = true;
        Some(HbrSwarmLoopCapReceipt {
            event_type: FR_EVT_LOOP_CAP.to_string(),
            receipt_uuid: Uuid::now_v7(),
            loop_id: self.loop_id.clone(),
            reason: reason.into(),
            iterations: self.iterations,
            cap: self.cap,
            emitted_at_utc: Utc::now(),
        })
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn cap(&self) -> usize {
        self.cap
    }
}
