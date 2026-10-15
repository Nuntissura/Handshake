use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducerJobIdentity {
    job_id: Uuid,
    initial_trace_id: Uuid,
    reserved_workflow_run_id: Option<Uuid>,
}

impl ProducerJobIdentity {
    pub fn allocate(reserve_workflow_run: bool) -> Self {
        Self {
            job_id: Uuid::now_v7(),
            initial_trace_id: Uuid::now_v7(),
            reserved_workflow_run_id: reserve_workflow_run.then(Uuid::now_v7),
        }
    }

    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    pub fn initial_trace_id(&self) -> Uuid {
        self.initial_trace_id
    }

    pub fn reserved_workflow_run_id(&self) -> Option<Uuid> {
        self.reserved_workflow_run_id
    }

    pub(crate) fn from_parts(
        job_id: Uuid,
        initial_trace_id: Uuid,
        reserved_workflow_run_id: Option<Uuid>,
    ) -> Option<Self> {
        if !is_uuid_v7(job_id)
            || !is_uuid_v7(initial_trace_id)
            || reserved_workflow_run_id.is_some_and(|id| !is_uuid_v7(id))
        {
            return None;
        }
        Some(Self {
            job_id,
            initial_trace_id,
            reserved_workflow_run_id,
        })
    }
}

fn is_uuid_v7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_reserves_original_version_seven_identity_once() {
        let identity = ProducerJobIdentity::allocate(true);
        assert_eq!(identity.job_id().get_version_num(), 7);
        assert_eq!(identity.initial_trace_id().get_version_num(), 7);
        assert_eq!(identity.reserved_workflow_run_id().unwrap().get_version_num(), 7);
        assert_ne!(identity.job_id(), identity.initial_trace_id());
        assert_ne!(identity.job_id(), identity.reserved_workflow_run_id().unwrap());
        let copied_data = identity.clone();
        assert_eq!(copied_data, identity);
        assert_eq!(copied_data.job_id(), identity.job_id());
        assert!(ProducerJobIdentity::allocate(false).reserved_workflow_run_id().is_none());
    }

    #[test]
    fn stored_identity_rejects_each_non_version_seven_component() {
        let identity = ProducerJobIdentity::allocate(true);
        let job = identity.job_id();
        let trace = identity.initial_trace_id();
        let workflow = identity.reserved_workflow_run_id();
        assert_eq!(ProducerJobIdentity::from_parts(job, trace, workflow), Some(identity));
        assert!(ProducerJobIdentity::from_parts(Uuid::nil(), trace, workflow).is_none());
        assert!(ProducerJobIdentity::from_parts(job, Uuid::nil(), workflow).is_none());
        assert!(ProducerJobIdentity::from_parts(job, trace, Some(Uuid::nil())).is_none());
        let wrong_variant = Uuid::parse_str("00000000-0000-7000-0000-000000000000").unwrap();
        assert_eq!(wrong_variant.get_version_num(), 7);
        assert!(ProducerJobIdentity::from_parts(wrong_variant, trace, workflow).is_none());
        assert!(ProducerJobIdentity::from_parts(job, trace, None).is_some());
    }
}
