use std::{cell::RefCell, collections::BTreeMap};

use handshake_core::{
    ace::{FemsSourceRef, FemsSourceRefKind},
    kernel::fems_mt_handoff_memory_context::{
        FemsMtHandoffItemKind, FemsMtHandoffMemoryContextV1, FemsMtHandoffMemoryItemV1,
        FemsMtHandoffReason, FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID,
    },
    memory::{
        BuildContext, CapsuleBuilder, CapsulePolicyTable, DegradationTier, FemsError,
        FemsMtHandoffRetriever, FemsRetriever, RetrievalPolicy, RetrievedItem, TaskType,
        RETRIEVAL_SCORING_FORMULA_V0,
    },
};

#[derive(Default)]
struct TestFemsRetriever {
    items: Vec<RetrievedItem>,
    error: Option<FemsError>,
    calls: RefCell<Vec<(String, u32)>>,
}

impl TestFemsRetriever {
    fn with_items(items: Vec<RetrievedItem>) -> Self {
        Self {
            items,
            error: None,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn with_error(message: &str) -> Self {
        Self {
            items: Vec::new(),
            error: Some(FemsError::new(message)),
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl FemsRetriever for TestFemsRetriever {
    fn retrieve(&self, query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        self.calls.borrow_mut().push((query.to_string(), top_k));
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(self.items.clone())
    }
}

#[test]
fn capsule_builder_empty_retrieval_returns_empty_capsule_with_policy() {
    let fems = TestFemsRetriever::with_items(Vec::new());
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);

    let capsule = builder.build(context()).unwrap();

    assert_eq!(capsule.task_type, TaskType::KernelBuilderMtImplementation);
    assert_eq!(
        capsule.policy.task_type,
        TaskType::KernelBuilderMtImplementation
    );
    assert!(capsule.pack.items.is_empty());
    assert!(capsule.audit.entries.is_empty());
    assert_eq!(
        fems.calls.borrow().as_slice(),
        &[("build query".to_string(), 12)]
    );
}

#[test]
fn capsule_builder_honors_override_policy_and_records_budget_drops() {
    let fems = TestFemsRetriever::with_items(vec![
        retrieved("pinned", 0.2, 70, true),
        retrieved("fit", 0.9, 30, false),
        retrieved("drop", 0.8, 40, false),
    ]);
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);

    let mut ctx = context();
    ctx.override_policy = Some(RetrievalPolicy {
        top_k: 3,
        capsule_budget_bytes: 100,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    });

    let capsule = builder.build(ctx).unwrap();

    assert_eq!(
        fems.calls.borrow().as_slice(),
        &[("build query".to_string(), 3)]
    );
    assert_eq!(
        capsule
            .pack
            .items
            .iter()
            .map(|item| item.memory_id.as_str())
            .collect::<Vec<_>>(),
        vec!["pinned", "fit"]
    );
    assert!(capsule.audit.entry("pinned").unwrap().included);
    assert!(capsule.audit.entry("fit").unwrap().included);
    let dropped = capsule.audit.entry("drop").unwrap();
    assert!(!dropped.included);
    assert_eq!(dropped.suppression_reason.as_deref(), Some("budget"));
    assert_eq!(dropped.score_breakdown["capsule_bytes"], 40.0);
}

#[test]
fn capsule_builder_propagates_fems_errors_as_typed_builder_error() {
    let fems = TestFemsRetriever::with_error("handoff context unavailable");
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);

    let error = builder.build(context()).unwrap_err();

    assert!(error.to_string().contains("handoff context unavailable"));
}

#[test]
fn capsule_builder_rejects_override_policy_for_different_task_type() {
    let fems = TestFemsRetriever::with_items(Vec::new());
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);
    let mut ctx = context();
    ctx.override_policy = Some(CapsulePolicyTable::default_policy_for(
        TaskType::ValidatorHbrTestPacket,
    ));

    let error = builder.build(ctx).unwrap_err();

    assert!(error.to_string().contains("does not match build context"));
}

#[test]
fn capsule_builder_rejects_invalid_override_policy_before_retrieval() {
    let fems = TestFemsRetriever::with_items(vec![retrieved("unused", 0.5, 10, false)]);
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);
    let mut ctx = context();
    ctx.override_policy = Some(RetrievalPolicy {
        top_k: 0,
        capsule_budget_bytes: 0,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    });

    let error = builder.build(ctx).unwrap_err();

    assert!(error.to_string().contains("top_k"));
    assert!(fems.calls.borrow().is_empty());
}

#[test]
fn capsule_builder_rejects_non_finite_retrieved_scores() {
    let fems = TestFemsRetriever::with_items(vec![retrieved("bad-score", f64::NAN, 10, false)]);
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);

    let error = builder.build(context()).unwrap_err();

    assert!(error.to_string().contains("score"));
}

#[test]
fn capsule_builder_source_hash_is_deterministic_for_identical_inputs() {
    let table = CapsulePolicyTable;
    let first_fems = TestFemsRetriever::with_items(vec![
        retrieved("a", 0.9, 12, false),
        retrieved("b", 0.8, 12, false),
    ]);
    let second_fems = TestFemsRetriever::with_items(vec![
        retrieved("b", 0.8, 12, false),
        retrieved("a", 0.9, 12, false),
    ]);

    let first = CapsuleBuilder::new(&first_fems, &table)
        .build(context())
        .unwrap();
    let second = CapsuleBuilder::new(&second_fems, &table)
        .build(context())
        .unwrap();

    assert_eq!(first.source_hash, second.source_hash);
}

#[test]
fn fems_mt_handoff_retriever_maps_projection_items_to_retrieved_items() {
    let retriever = FemsMtHandoffRetriever::new(sample_handoff_context());

    let items = retriever.retrieve("handoff", 2).unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item_id, "item-procedure-recommended");
    assert!(items[0].pinned);
    assert_eq!(items[0].item_type, "recommended_procedural_item");
    assert_eq!(
        items[0].source_refs[0].id,
        "provenance://item-procedure-recommended"
    );
}

fn context() -> BuildContext {
    BuildContext {
        task_type: TaskType::KernelBuilderMtImplementation,
        query: "build query".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        session_id: "session-1".to_string(),
        override_policy: None,
    }
}

fn sample_handoff_context() -> FemsMtHandoffMemoryContextV1 {
    FemsMtHandoffMemoryContextV1 {
        schema_id: "hsk.kernel.fems_mt_handoff_memory_context@1".to_string(),
        context_id: "handoff-context-memory-builder".to_string(),
        folded_stub_ids: vec![FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID.to_string()],
        wp_id: "WP-KERNEL-004".to_string(),
        mt_id: "MT-143".to_string(),
        source_session_id: "source-session".to_string(),
        target_session_id: "target-session".to_string(),
        handoff_reason: FemsMtHandoffReason::RoleSwitch,
        carried_items: vec![
            handoff_item(
                "item-procedure-recommended",
                FemsMtHandoffItemKind::RecommendedProceduralItem,
                20,
                70,
                true,
            ),
            handoff_item(
                "item-memory-fact",
                FemsMtHandoffItemKind::MemoryPackItem,
                10,
                60,
                false,
            ),
        ],
        failed_attempts: Vec::new(),
        recommended_item_ids: vec!["item-procedure-recommended".to_string()],
        max_handoff_tokens: 100,
        fr_event_ref: "FR-EVT-MEM-004-memory-builder".to_string(),
        locus_mt_iteration_ref: "locus://WP-KERNEL-004/MT-143/iteration-1".to_string(),
        automatic_long_term_merge_allowed: false,
        cross_wp_handoff_allowed: false,
        product_authority_refs: vec![
            "ace.memory_pack".to_string(),
            "flight_recorder.memory_handoff_context".to_string(),
            "locus.mt_iteration".to_string(),
            "kernel.fems_memory_poisoning_drift_guardrails".to_string(),
        ],
        folded_source_refs: vec![FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID.to_string()],
    }
}

fn handoff_item(
    item_id: &str,
    kind: FemsMtHandoffItemKind,
    token_count: u32,
    base_score_x100: u8,
    predecessor_recommended: bool,
) -> FemsMtHandoffMemoryItemV1 {
    FemsMtHandoffMemoryItemV1 {
        item_id: item_id.to_string(),
        kind,
        source_session_id: "source-session".to_string(),
        memory_ref: format!("memory://{item_id}"),
        scope_refs: vec!["WP-KERNEL-004".to_string(), "MT-143".to_string()],
        provenance_ref: format!("provenance://{item_id}"),
        token_count,
        base_score_x100,
        predecessor_recommended,
        source_attempt_failed: false,
    }
}

fn retrieved(id: &str, score: f64, capsule_bytes: u64, pinned: bool) -> RetrievedItem {
    RetrievedItem {
        item_id: id.to_string(),
        memory_class: "episodic".to_string(),
        item_type: "note".to_string(),
        summary: format!("summary {id}"),
        content: format!("content {id}"),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: 0.9,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: format!("artifact-{id}"),
            hash: None,
            selector: Some(format!("#{}", id)),
            created_at: None,
            classification: None,
        }],
        score,
        score_breakdown: BTreeMap::from([("similarity".to_string(), score)]),
        capsule_bytes,
        token_estimate: capsule_bytes as u32,
        pinned,
    }
}
