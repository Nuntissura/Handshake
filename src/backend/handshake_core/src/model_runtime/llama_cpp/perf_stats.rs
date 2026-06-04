use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const LLAMA_CPP_PERF_STATS_EMA_ALPHA: f32 = 0.25;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlamaCppPerfStats {
    pub total_prompts: u64,
    pub total_tokens_generated: u64,
    pub prompt_eval_ms_total: u64,
    pub gen_eval_ms_total: u64,
    pub tokens_per_sec_ema: f32,
    pub vram_resident_bytes: u64,
    pub last_call_at_utc: Option<DateTime<Utc>>,
    pub time_since_last_call_ms: Option<u64>,
}

impl Default for LlamaCppPerfStats {
    fn default() -> Self {
        Self {
            total_prompts: 0,
            total_tokens_generated: 0,
            prompt_eval_ms_total: 0,
            gen_eval_ms_total: 0,
            tokens_per_sec_ema: 0.0,
            vram_resident_bytes: 0,
            last_call_at_utc: None,
            time_since_last_call_ms: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlamaCppPerfStatsUpdate {
    pub prompt_eval_ms: u64,
    pub gen_eval_ms: u64,
    pub tokens_generated: u64,
    pub vram_resident_bytes: u64,
    pub completed_at_utc: DateTime<Utc>,
}

impl LlamaCppPerfStats {
    pub fn record_call(&mut self, update: LlamaCppPerfStatsUpdate) {
        self.total_prompts = self.total_prompts.saturating_add(1);
        self.total_tokens_generated = self
            .total_tokens_generated
            .saturating_add(update.tokens_generated);
        self.prompt_eval_ms_total = self
            .prompt_eval_ms_total
            .saturating_add(update.prompt_eval_ms);
        self.gen_eval_ms_total = self.gen_eval_ms_total.saturating_add(update.gen_eval_ms);
        self.vram_resident_bytes = update.vram_resident_bytes;

        self.time_since_last_call_ms = self.last_call_at_utc.and_then(|previous| {
            update
                .completed_at_utc
                .signed_duration_since(previous)
                .to_std()
                .ok()
                .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        });
        self.last_call_at_utc = Some(update.completed_at_utc);

        if let Some(sample) = tokens_per_second(update.tokens_generated, update.gen_eval_ms) {
            self.tokens_per_sec_ema = if self.tokens_per_sec_ema == 0.0 {
                sample
            } else {
                (self.tokens_per_sec_ema * (1.0 - LLAMA_CPP_PERF_STATS_EMA_ALPHA))
                    + (sample * LLAMA_CPP_PERF_STATS_EMA_ALPHA)
            };
        }
    }
}

fn tokens_per_second(tokens_generated: u64, gen_eval_ms: u64) -> Option<f32> {
    if tokens_generated == 0 || gen_eval_ms == 0 {
        return None;
    }
    let sample = (tokens_generated as f32) / (gen_eval_ms as f32 / 1_000.0);
    sample.is_finite().then_some(sample)
}
