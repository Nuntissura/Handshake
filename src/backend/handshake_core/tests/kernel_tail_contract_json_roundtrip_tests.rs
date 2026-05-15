use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};

use handshake_core::kernel::{
    coder_handoff_validation_request::{
        build_kernel002_coder_handoff_validation_request, project_coder_handoff_validation_request,
        CoderHandoffValidationRequestContractV1, CoderHandoffValidationRequestProjectionV1,
    },
    local_model_microtask_loop::{
        build_kernel002_local_model_microtask_loop, project_local_model_microtask_loop,
        LocalModelFreshContextMicrotaskLoopV1, LocalModelMicrotaskLoopProjectionV1,
    },
    locus_mt_validation_work_graph::{
        build_kernel002_locus_mt_validation_work_graph, project_locus_mt_validation_work_graph,
        LocusMtValidationWorkGraphContractV1, LocusMtValidationWorkGraphProjectionV1,
    },
    mt_loop_scheduler_contract::{
        build_kernel002_mt_loop_scheduler, evaluate_mt_loop_scheduler, MtLoopSchedulerContractV1,
        MtLoopSchedulerProjectionV1,
    },
    remediation_work_generation_contract::{
        build_kernel002_remediation_work_generation, project_remediation_work_generation,
        RemediationWorkGenerationContractV1, RemediationWorkGenerationProjectionV1,
    },
    validator_finding_report_contract::{
        build_kernel002_validator_finding_reports, project_validator_finding_reports,
        ValidatorFindingReportsContractV1, ValidatorFindingReportsProjectionV1,
    },
    validator_verdict_mediation_contract::{
        build_kernel002_validator_verdict_mediation_contract, project_validator_verdict_mediation,
        ValidatorVerdictMediationContractV1, ValidatorVerdictMediationProjectionV1,
    },
    work_packet_full_detail_authority::{
        build_kernel002_work_packet_full_detail_authority, WorkPacketFullDetailAuthorityV1,
    },
};

#[test]
fn tail_machine_contracts_and_projections_json_round_trip() {
    let authority = build_kernel002_work_packet_full_detail_authority();
    assert_json_round_trip::<WorkPacketFullDetailAuthorityV1>(&authority);

    let coder = build_kernel002_coder_handoff_validation_request();
    let coder_projection =
        project_coder_handoff_validation_request(&coder).expect("coder projection derives");
    assert_json_round_trip::<CoderHandoffValidationRequestContractV1>(&coder);
    assert_json_round_trip::<CoderHandoffValidationRequestProjectionV1>(&coder_projection);

    let verdict = build_kernel002_validator_verdict_mediation_contract();
    let verdict_projection =
        project_validator_verdict_mediation(&verdict).expect("verdict projection derives");
    assert_json_round_trip::<ValidatorVerdictMediationContractV1>(&verdict);
    assert_json_round_trip::<ValidatorVerdictMediationProjectionV1>(&verdict_projection);

    let findings = build_kernel002_validator_finding_reports();
    let findings_projection =
        project_validator_finding_reports(&findings).expect("finding projection derives");
    assert_json_round_trip::<ValidatorFindingReportsContractV1>(&findings);
    assert_json_round_trip::<ValidatorFindingReportsProjectionV1>(&findings_projection);

    let remediation = build_kernel002_remediation_work_generation();
    let remediation_projection =
        project_remediation_work_generation(&remediation).expect("remediation projection derives");
    assert_json_round_trip::<RemediationWorkGenerationContractV1>(&remediation);
    assert_json_round_trip::<RemediationWorkGenerationProjectionV1>(&remediation_projection);

    let scheduler = build_kernel002_mt_loop_scheduler();
    let scheduler_projection =
        evaluate_mt_loop_scheduler(&scheduler).expect("scheduler projection derives");
    assert_json_round_trip::<MtLoopSchedulerContractV1>(&scheduler);
    assert_json_round_trip::<MtLoopSchedulerProjectionV1>(&scheduler_projection);

    let local_loop = build_kernel002_local_model_microtask_loop();
    let local_loop_projection =
        project_local_model_microtask_loop(&local_loop).expect("local loop projection derives");
    assert_json_round_trip::<LocalModelFreshContextMicrotaskLoopV1>(&local_loop);
    assert_json_round_trip::<LocalModelMicrotaskLoopProjectionV1>(&local_loop_projection);

    let work_graph = build_kernel002_locus_mt_validation_work_graph();
    let work_graph_projection =
        project_locus_mt_validation_work_graph(&work_graph).expect("work graph projection derives");
    assert_json_round_trip::<LocusMtValidationWorkGraphContractV1>(&work_graph);
    assert_json_round_trip::<LocusMtValidationWorkGraphProjectionV1>(&work_graph_projection);
}

fn assert_json_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).expect("serializes to JSON");
    let decoded: T = serde_json::from_str(&json).expect("deserializes from JSON");
    assert_eq!(&decoded, value);
}
