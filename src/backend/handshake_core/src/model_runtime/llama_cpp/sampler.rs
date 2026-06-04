use crate::model_runtime::SamplingParams;

pub const DEFAULT_LLAMA_CPP_SEED: u32 = 1234;
const MIN_KEEP: usize = 1;
const PENALTY_LAST_N_CONTEXT: i32 = -1;
const DISABLED_REPETITION_PENALTY: f32 = 1.0;
const DISABLED_FREQUENCY_PENALTY: f32 = 0.0;
const DISABLED_PRESENCE_PENALTY: f32 = 0.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SamplerPlan {
    seed: u32,
    steps: Vec<SamplerStep>,
}

impl SamplerPlan {
    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn steps(&self) -> &[SamplerStep] {
        &self.steps
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    pub(crate) fn build_llama_sampler(&self) -> llama_cpp_2::sampling::LlamaSampler {
        let mut samplers = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            match *step {
                SamplerStep::Penalties {
                    penalty_last_n,
                    repetition,
                    frequency,
                    presence,
                } => samplers.push(llama_cpp_2::sampling::LlamaSampler::penalties(
                    penalty_last_n,
                    repetition,
                    frequency,
                    presence,
                )),
                SamplerStep::TopK(k) => {
                    samplers.push(llama_cpp_2::sampling::LlamaSampler::top_k(k))
                }
                SamplerStep::TopP {
                    probability,
                    min_keep,
                } => samplers.push(llama_cpp_2::sampling::LlamaSampler::top_p(
                    probability,
                    min_keep,
                )),
                SamplerStep::MinP {
                    probability,
                    min_keep,
                } => samplers.push(llama_cpp_2::sampling::LlamaSampler::min_p(
                    probability,
                    min_keep,
                )),
                SamplerStep::Temperature(temperature) => {
                    samplers.push(llama_cpp_2::sampling::LlamaSampler::temp(temperature))
                }
                SamplerStep::Dist(seed) => {
                    samplers.push(llama_cpp_2::sampling::LlamaSampler::dist(seed))
                }
                SamplerStep::Greedy => samplers.push(llama_cpp_2::sampling::LlamaSampler::greedy()),
            }
        }

        llama_cpp_2::sampling::LlamaSampler::chain_simple(samplers)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SamplerStep {
    Penalties {
        penalty_last_n: i32,
        repetition: f32,
        frequency: f32,
        presence: f32,
    },
    TopK(i32),
    TopP {
        probability: f32,
        min_keep: usize,
    },
    MinP {
        probability: f32,
        min_keep: usize,
    },
    Temperature(f32),
    Dist(u32),
    Greedy,
}

pub fn sampler_plan(params: &SamplingParams) -> SamplerPlan {
    let seed = params.seed.unwrap_or(DEFAULT_LLAMA_CPP_SEED);
    let mut steps = Vec::new();

    let repetition = params
        .repetition_penalty
        .filter(|value| value.is_finite())
        .unwrap_or(DISABLED_REPETITION_PENALTY);
    let frequency = params
        .frequency_penalty
        .filter(|value| value.is_finite())
        .unwrap_or(DISABLED_FREQUENCY_PENALTY);
    let presence = params
        .presence_penalty
        .filter(|value| value.is_finite())
        .unwrap_or(DISABLED_PRESENCE_PENALTY);

    if repetition != DISABLED_REPETITION_PENALTY
        || frequency != DISABLED_FREQUENCY_PENALTY
        || presence != DISABLED_PRESENCE_PENALTY
    {
        steps.push(SamplerStep::Penalties {
            penalty_last_n: PENALTY_LAST_N_CONTEXT,
            repetition,
            frequency,
            presence,
        });
    }

    if let Some(top_k) = params.top_k.filter(|value| *value > 0) {
        steps.push(SamplerStep::TopK(i32::try_from(top_k).unwrap_or(i32::MAX)));
    }

    if let Some(top_p) = params
        .top_p
        .filter(|value| value.is_finite() && *value > 0.0 && *value < 1.0)
    {
        steps.push(SamplerStep::TopP {
            probability: top_p,
            min_keep: MIN_KEEP,
        });
    }

    if let Some(min_p) = params
        .min_p
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        steps.push(SamplerStep::MinP {
            probability: min_p,
            min_keep: MIN_KEEP,
        });
    }

    let explicit_greedy = params
        .temperature
        .is_some_and(|value| value.is_finite() && value <= 0.0);
    let temperature = params
        .temperature
        .filter(|value| value.is_finite() && *value > 0.0);
    if let Some(temperature) = temperature {
        steps.push(SamplerStep::Temperature(temperature));
    }

    let stochastic = !explicit_greedy
        && (temperature.is_some()
            || params.seed.is_some()
            || params.top_k.is_some()
            || params.top_p.is_some()
            || params.min_p.is_some());

    if stochastic {
        steps.push(SamplerStep::Dist(seed));
    } else {
        steps.push(SamplerStep::Greedy);
    }

    SamplerPlan { seed, steps }
}
