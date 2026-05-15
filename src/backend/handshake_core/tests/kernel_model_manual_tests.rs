use handshake_core::kernel::model_manual::{kernel002_no_context_model_manual, ManualTopic};

#[test]
fn manual_covers_every_required_no_context_topic() {
    let manual = kernel002_no_context_model_manual();

    for topic in [
        ManualTopic::Purpose,
        ManualTopic::Startup,
        ManualTopic::ActionCatalog,
        ManualTopic::WriteBoxes,
        ManualTopic::DccPaths,
        ManualTopic::CrdtWorkflow,
        ManualTopic::SafetyConstraints,
        ManualTopic::FailureModes,
        ManualTopic::DenialRecovery,
        ManualTopic::ValidationEvidence,
    ] {
        let section = manual.section(topic).expect("manual section missing");
        assert!(
            !section.instructions.is_empty(),
            "manual section must have instructions: {:?}",
            topic
        );
    }
}

#[test]
fn manual_is_usable_without_hidden_chat_context() {
    let manual = kernel002_no_context_model_manual();

    assert!(manual.no_prior_context_required);
    assert!(manual
        .startup_commands
        .iter()
        .any(|cmd| cmd.contains("kbstart")));
    assert!(manual
        .startup_commands
        .iter()
        .any(|cmd| cmd.contains("mt-board")));
    assert!(manual
        .section(ManualTopic::ActionCatalog)
        .expect("action catalog section")
        .instructions
        .iter()
        .any(|line| line.contains("kernel.action_catalog.view")));
    assert!(manual
        .section(ManualTopic::WriteBoxes)
        .expect("write boxes section")
        .instructions
        .iter()
        .any(|line| line.contains("PromotionBox")));
}

#[test]
fn manual_explains_denial_recovery_and_validation_evidence() {
    let manual = kernel002_no_context_model_manual();
    let denial = manual
        .section(ManualTopic::DenialRecovery)
        .expect("denial recovery section");
    assert!(denial
        .instructions
        .iter()
        .any(|line| line.contains("lawful_replacement_action_ids")));

    let validation = manual
        .section(ManualTopic::ValidationEvidence)
        .expect("validation evidence section");
    assert!(validation
        .instructions
        .iter()
        .any(|line| line.contains("receipt")));
    assert!(validation
        .instructions
        .iter()
        .any(|line| line.contains("test")));
}
