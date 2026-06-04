use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvQuantSupport {
    #[default]
    None,
    Q4,
    Q8,
    Q4Q8Mix,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_lora: bool,
    pub supports_kv_prefix_cache: bool,
    pub supports_kv_quantization: KvQuantSupport,
    pub supports_activation_steering: bool,
    pub supports_subquadratic: bool,
    pub supports_speculative_draft: bool,
    pub supports_eagle3: bool,
}
