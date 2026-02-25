use handshake_core::ace::{
    validators::{AceRuntimeValidator, ViewModeHardDropGuard},
    CandidateRef, CandidateScores, ContentTier, ProjectionKind, QueryKind, QueryPlan,
    RetrievalCandidate, RetrievalTrace, SelectedEvidence, SourceRef, SpanExtraction, StoreKind,
    ViewMode,
};
use uuid::Uuid;

#[test]
fn view_mode_default_is_nsfw() {
    let plan = QueryPlan::new(
        "test query".to_string(),
        QueryKind::FactLookup,
        "test_policy".to_string(),
    );
    assert_eq!(plan.filters.view_mode, ViewMode::Nsfw);
}

#[test]
fn sfw_hard_drop_strictly_filters_non_sfw_and_unknown() {
    let mut plan = QueryPlan::new(
        "test query".to_string(),
        QueryKind::FactLookup,
        "test_policy".to_string(),
    );
    plan.filters.view_mode = ViewMode::Sfw;

    let mut trace = RetrievalTrace::new(&plan);

    let sfw_source = SourceRef::new(Uuid::from_u128(1), "hash_sfw".to_string());
    let adult_source = SourceRef::new(Uuid::from_u128(2), "hash_adult".to_string());
    let unknown_source = SourceRef::new(Uuid::from_u128(3), "hash_unknown".to_string());

    let mut candidate_sfw = RetrievalCandidate::from_source(
        sfw_source.clone(),
        StoreKind::ContextPacks,
        CandidateScores::default(),
    );
    candidate_sfw.candidate_id = "candidate_sfw".to_string();
    candidate_sfw.content_tier = Some(ContentTier::Sfw);

    let mut candidate_adult = RetrievalCandidate::from_source(
        adult_source.clone(),
        StoreKind::ContextPacks,
        CandidateScores::default(),
    );
    candidate_adult.candidate_id = "candidate_adult".to_string();
    candidate_adult.content_tier = Some(ContentTier::AdultSoft);

    let mut candidate_unknown = RetrievalCandidate::from_source(
        unknown_source.clone(),
        StoreKind::ContextPacks,
        CandidateScores::default(),
    );
    candidate_unknown.candidate_id = "candidate_unknown".to_string();
    // content_tier remains None (unknown/unclassified)

    trace.candidates.push(candidate_sfw);
    trace.candidates.push(candidate_adult);
    trace.candidates.push(candidate_unknown);

    trace.selected.push(SelectedEvidence {
        candidate_ref: CandidateRef::Source(sfw_source.clone()),
        final_rank: 0,
        final_score: 1.0,
        why: "sfw".to_string(),
    });
    trace.selected.push(SelectedEvidence {
        candidate_ref: CandidateRef::Source(adult_source.clone()),
        final_rank: 1,
        final_score: 0.5,
        why: "adult".to_string(),
    });
    trace.selected.push(SelectedEvidence {
        candidate_ref: CandidateRef::Source(unknown_source.clone()),
        final_rank: 2,
        final_score: 0.1,
        why: "unknown".to_string(),
    });

    trace.spans.push(SpanExtraction {
        source_ref: sfw_source.clone(),
        selector: "sfw".to_string(),
        start: 0,
        end: 10,
        token_estimate: 3,
    });
    trace.spans.push(SpanExtraction {
        source_ref: adult_source.clone(),
        selector: "adult".to_string(),
        start: 0,
        end: 10,
        token_estimate: 3,
    });
    trace.spans.push(SpanExtraction {
        source_ref: unknown_source.clone(),
        selector: "unknown".to_string(),
        start: 0,
        end: 10,
        token_estimate: 3,
    });

    trace.apply_view_mode_hard_drop();

    // Strict-drop: only SFW remains.
    assert_eq!(trace.candidates.len(), 1);
    assert_eq!(trace.selected.len(), 1);
    assert_eq!(trace.spans.len(), 1);

    assert_eq!(trace.candidates[0].content_tier, Some(ContentTier::Sfw));
    assert_eq!(
        trace.selected[0].candidate_ref,
        CandidateRef::Source(sfw_source.clone())
    );
    assert_eq!(trace.spans[0].source_ref, sfw_source);

    // Required labeling (spec 11.3).
    assert!(trace.projection_applied);
    assert_eq!(trace.projection_kind, Some(ProjectionKind::Sfw));
    assert_eq!(
        trace.projection_ruleset_id.as_deref(),
        Some("viewmode_sfw_hard_drop@v1")
    );

    // Audit warning is count-only and does not leak dropped content.
    assert!(trace
        .warnings
        .iter()
        .any(|w| w.contains("view_mode_sfw_hard_drop")));

    // Trace must record view_mode as an applied metadata filter.
    let trace_json = serde_json::to_value(&trace).expect("serialize trace");
    assert_eq!(trace_json["filters_applied"]["view_mode"], "SFW");
    assert_eq!(trace_json["projection_applied"], true);
    assert_eq!(trace_json["projection_kind"], "SFW");
    assert_eq!(
        trace_json["projection_ruleset_id"],
        "viewmode_sfw_hard_drop@v1"
    );
}

#[tokio::test]
async fn sfw_guard_blocks_unfiltered_trace() {
    let mut plan = QueryPlan::new(
        "test query".to_string(),
        QueryKind::FactLookup,
        "test_policy".to_string(),
    );
    plan.filters.view_mode = ViewMode::Sfw;

    let mut trace = RetrievalTrace::new(&plan);
    let unknown_source = SourceRef::new(Uuid::from_u128(1), "hash_unknown".to_string());

    // Unknown tier is default-deny in SFW and must be dropped before validation.
    trace.candidates.push(RetrievalCandidate::from_source(
        unknown_source.clone(),
        StoreKind::ContextPacks,
        CandidateScores::default(),
    ));
    trace.selected.push(SelectedEvidence {
        candidate_ref: CandidateRef::Source(unknown_source.clone()),
        final_rank: 0,
        final_score: 1.0,
        why: "unknown".to_string(),
    });
    trace.spans.push(SpanExtraction {
        source_ref: unknown_source,
        selector: "unknown".to_string(),
        start: 0,
        end: 10,
        token_estimate: 3,
    });

    let guard = ViewModeHardDropGuard;
    let err = guard
        .validate_trace(&trace)
        .await
        .expect_err("guard must reject SFW traces containing unknown/non-sfw tiers");

    match err {
        handshake_core::ace::AceError::ValidationFailed { message } => {
            assert!(message.contains("ViewMode=\"SFW\""));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
