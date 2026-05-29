use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    select, AdapterId, BindSpec, ImageRef, NetPolicy, ProcessHandle, ProcessSpec,
    RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    SandboxAdapterRegistry, SandboxSelectionFailure, TrustClass,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationJobSpec {
    pub job_id: String,
    pub image_or_root: ImageRef,
    pub cmd: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub binds: Vec<BindSpec>,
    pub net_policy: Option<NetPolicy>,
    pub resource_limits: ResourceLimits,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub work_profile_override: Option<AdapterId>,
    pub metadata: BTreeMap<String, String>,
}

impl ValidationJobSpec {
    pub fn new(
        job_id: impl Into<String>,
        image_or_root: ImageRef,
        cmd: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            image_or_root,
            cmd: cmd.into_iter().map(Into::into).collect(),
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            net_policy: None,
            resource_limits: ResourceLimits::default(),
            required_capabilities: BTreeSet::new(),
            work_profile_override: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_net_policy(mut self, net_policy: NetPolicy) -> Self {
        self.net_policy = Some(net_policy);
        self
    }
}

#[derive(Debug, Error)]
pub enum ValidationRunnerBindingError {
    #[error("validation job_id is required")]
    EmptyJobId,
    #[error("validation job cmd must not be empty")]
    EmptyCommand,
    #[error(transparent)]
    SandboxSelection(#[from] SandboxSelectionFailure),
    #[error(transparent)]
    SandboxAdapter(#[from] SandboxAdapterError),
}

#[derive(Clone, Debug, Default)]
pub struct ValidationProcessSpecBuilder;

impl ValidationProcessSpecBuilder {
    pub fn build(
        &self,
        job: ValidationJobSpec,
    ) -> Result<ProcessSpec, ValidationRunnerBindingError> {
        let job_id = job.job_id.trim();
        if job_id.is_empty() {
            return Err(ValidationRunnerBindingError::EmptyJobId);
        }
        if job.cmd.is_empty() {
            return Err(ValidationRunnerBindingError::EmptyCommand);
        }

        let mut metadata = job.metadata;
        metadata.insert("validation_job_id".to_string(), job_id.to_string());
        metadata.insert(
            "validation_lane".to_string(),
            "model_written_code".to_string(),
        );
        if let Some(adapter_id) = &job.work_profile_override {
            metadata.insert(
                "work_profile_override".to_string(),
                adapter_id.as_str().to_string(),
            );
        }

        Ok(ProcessSpec {
            id: AdapterId::new(format!("validation-job:{job_id}")),
            image_or_root: job.image_or_root,
            cmd: job.cmd,
            env: job.env,
            cwd: job.cwd,
            binds: job.binds,
            net_policy: job.net_policy.unwrap_or(NetPolicy::DenyAll),
            resource_limits: job.resource_limits,
            required_capabilities: job.required_capabilities,
            // Model-written-code validation jobs are untrusted-agent work by
            // construction; carry the safe default so the trust->tier minimum
            // (Master Spec v02.187 §3.5.5) applies once a Tier-2/3 adapter exists.
            trust_class: TrustClass::default(),
            metadata,
        })
    }
}

pub fn select_validation_adapter(
    registry: &SandboxAdapterRegistry,
    job: &ValidationJobSpec,
) -> Result<Arc<dyn SandboxAdapter>, ValidationRunnerBindingError> {
    let process_spec = ValidationProcessSpecBuilder.build(job.clone())?;
    Ok(select(
        registry,
        &process_spec,
        job.work_profile_override.as_ref(),
    )?)
}

#[derive(Clone)]
pub struct ValidationSandboxRunner {
    sandbox: Arc<dyn SandboxAdapter>,
    process_spec: ProcessSpec,
}

impl ValidationSandboxRunner {
    fn new(sandbox: Arc<dyn SandboxAdapter>, process_spec: ProcessSpec) -> Self {
        Self {
            sandbox,
            process_spec,
        }
    }

    pub fn from_registry(
        registry: &SandboxAdapterRegistry,
        job: &ValidationJobSpec,
    ) -> Result<Self, ValidationRunnerBindingError> {
        let process_spec = ValidationProcessSpecBuilder.build(job.clone())?;
        let sandbox = select(registry, &process_spec, job.work_profile_override.as_ref())?;
        Ok(Self::new(sandbox, process_spec))
    }

    pub async fn spawn(&self) -> Result<ProcessHandle, ValidationRunnerBindingError> {
        Ok(self.sandbox.spawn(self.process_spec.clone()).await?)
    }
}
