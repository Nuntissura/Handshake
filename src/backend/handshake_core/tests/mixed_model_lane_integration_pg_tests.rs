use handshake_core::swarm_orchestration::production_factory::execute_production_routing_wave;

#[test]
fn ac9_rejects_policy_labelled_arbitrary_dags_and_exposes_only_production_wrapper() {
    use handshake_core::swarm_orchestration::routing::{
        ModelLaneRoutingGraph, ModelLaneRoutingPolicy,
    };

    let _production_entrypoint = execute_production_routing_wave;
    for policy in ModelLaneRoutingPolicy::all().iter().copied() {
        let canonical = ModelLaneRoutingGraph::for_policy(policy);
        canonical
            .validate()
            .expect("canonical policy graph validates");
        let mut forged = canonical.clone();
        forged.stages.reverse();
        assert!(
            forged.validate().is_err(),
            "policy-labelled arbitrary DAG must be rejected for {}",
            policy.as_str()
        );
    }
}
