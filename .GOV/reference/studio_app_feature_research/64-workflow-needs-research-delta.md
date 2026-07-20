---
file_id: studio-app-feature-research-workflow-needs-research-delta
topic_id: SFR-WFRES
title: "Workflow Needs Research Delta (2026-07-20, ACTION-A6)"
status: draft
summary: "Research detail for the team/production workflow needs the feature corpus missed: review/approval, workflow-state PM, performance NFRs, DAM/PIM + governed libraries, culling/catalog. How incumbents implement each + the Handshake Studio requirement. NON-AI."
sources: 70
updated_at: "2026-07-20"
---


## [SFR-WFRES] Workflow Needs Research Delta

### [SFR-WFRES.summary] Summary

```json
{
  "action": "ACTION-A6 (see 61-parity-audit-action-register.md)",
  "method": "Per-lane research of how incumbent tools implement each workflow capability (data model, states, mechanics), then the Handshake Studio requirement mapped to CRDT/EventLedger/sessions/permission primitives. NON-AI scope.",
  "total_capabilities": 70,
  "by_lane": {
    "review-approval": 15,
    "workflow-state-pm": 14,
    "perf-nfr": 11,
    "dam-pim-libraries": 15,
    "culling-catalog": 15
  },
  "architecture_headline": "These are the coordination surfaces a feature-list audit cannot see. They point at a scale-adaptive review/approval + workflow-state + permission triad on the CRDT/EventLedger substrate, plus an integrated culling/catalog stage (the sole NOT_COVERED need) and DAM/PIM + governed library releases.",
  "authority": "Reference/provenance only; feeds WP-KERNEL-STUDIO refinement and possible Section-14 enrichment decisions."
}
```

### [SFR-WFRES.review-approval] Review / approval / sign-off / version-of-record (NEED-004/005/017/022)

```json
{
  "capabilities": [
    {
      "capability": "Multi-stage sequential review routing: an ordered chain of named stages, each stage holding its own assigned approver group, that a proof advances through one stage at a time",
      "closes_need": "NEED-005",
      "how_incumbents_do_it": "Ziflow 'workflows' define exact stage order and assigned approvers per stage; once a stage's decision condition is met the next stage auto-starts and only that stage's reviewers are notified. GoProof 'Workflows configurator' adds N stages in sequential order, each with a collaborator group + timeframe, auto-passing proofs stage->stage on predefined triggers. Filestage groups reviewers into ordered 'review steps'; once a step is approved it auto-moves content to the next reviewer group until final sign-off. Data model: workflow = ordered list of stage objects; stage = {name, reviewer_set, start_condition, decision_condition}.",
      "studio_requirement": "Studio needs an ApprovalWorkflow primitive = ordered list of ApprovalStage records, each stage referencing a set of session/actor identities. Stage transition is a mechanical EventLedger event (STAGE_STARTED/STAGE_DECIDED) gated by a PromotionGate-style condition, not a model edit. Solo scale collapses to a zero-stage no-op; large scale materializes the full chain. Stage set is a governance surface, not CRDT document content.",
      "scale": "all",
      "sources": "https://help.ziflow.com/hc/en-us/articles/39098346311316-Understand-workflows | https://goproof.net/automated-proofing-workflows | https://filestage.io/blog/approval-workflow/",
      "id": "SFR-WFRES-review-approval-01"
    },
    {
      "capability": "Conditional and parallel stage routing with targeted rejection return: stages that fire only when multiple upstream stages reach a decision, run simultaneously, and on reject route back to the specific earlier stage needing revision rather than to the start",
      "closes_need": "NEED-005",
      "how_incumbents_do_it": "Ziflow: a stage (e.g. Client review) can be configured to start only after both Internal design AND Internal legal reach 'Approved'/'Approved with changes'; assets rejected at any stage route back to the specific earlier stage that needs revision (targeted, not full-restart); automated stage triggers start dependent stages on condition. GoProof supports parallel stages that happen simultaneously vs sequential. Used for CPG packaging regulatory/legal/brand parallel routing.",
      "studio_requirement": "ApprovalStage start_condition must express boolean dependencies over other stages' decision states (AND/OR of upstream STAGE_DECIDED events). Reject decision emits a REVISION_REQUESTED event targeting a named upstream stage id, reopening only that stage. All routing evaluation is deterministic and mechanical (EventLedger-driven), never AI-inferred. Parallel stages = multiple stages with the same start_condition and no ordering dependency.",
      "scale": "large",
      "sources": "https://help.ziflow.com/hc/en-us/articles/30721932087828-How-Do-I-Trigger-a-Stage-Based-on-Another-Stage-s-Decision-Status | https://www.ziflow.com/routing-and-automation | https://goproof.net/automated-proofing-workflows",
      "id": "SFR-WFRES-review-approval-02"
    },
    {
      "capability": "Typed approver roles with per-step decision quorum: distinguish reviewer / mandatory / gatekeeper / final-approver roles and require All, a specific number, or Just one decision before a step completes",
      "closes_need": "NEED-005",
      "how_incumbents_do_it": "PageProof defines reviewer, mandatory, gatekeeper, and final-approver roles; a step stops and waits for mandatory reviewers before advancing; you configure whether one, all, or a set number of mandatory/gatekeeper/approver decisions are required ('All, A number, Just one'). Final approver decides approve-as-is vs return-to-owner-with-to-dos, and you set how many approvals are needed for final approval. Ziflow lets the same reviewer sit in multiple stages with different permissions per stage.",
      "studio_requirement": "ApprovalStage needs a role field per participant (REVIEWER | MANDATORY | GATEKEEPER | FINAL_APPROVER) and a quorum policy {ALL | N | ONE}. Stage completion predicate counts qualifying decisions against the quorum before emitting STAGE_DECIDED. Roles bind to Studio session/permission identities so a gatekeeper's decision has weight a reviewer's comment does not. Permission model must allow the same actor different roles across stages.",
      "scale": "large",
      "sources": "https://help.pageproof.com/en/articles/6198001-setting-how-many-mandatory-gatekeeper-approver-decisions-are-required | https://help.pageproof.com/en/articles/923799-a-guide-on-workflow-roles | https://help.ziflow.com/hc/en-us/articles/37922335934996-Add-reviewers-and-configure-workflows-for-a-proof",
      "id": "SFR-WFRES-review-approval-03"
    },
    {
      "capability": "Explicit decision states beyond binary approve/reject: approve, approve-with-changes, reject/needs-work, and not-relevant, recorded as a formal binding decision distinct from a comment",
      "closes_need": "NEED-005, NEED-004",
      "how_incumbents_do_it": "Ziflow decision set includes Approved, Approved with changes (generally accepted but revisions required), and Not relevant (asset doesn't fit this reviewer's scope). Frame.io uses Approve vs 'Needs work'. PageProof final approver chooses approved-as-is vs returned-with-to-dos. Ziflow's audit model treats a 'formal binding decision by a designated approver' as categorically distinct from reviewer comments.",
      "studio_requirement": "ApprovalDecision is a typed EventLedger event with an enum {APPROVED, APPROVED_WITH_CHANGES, REJECTED, NOT_RELEVANT} plus actor, stage, and target version. It is a first-class governance event separate from CRDT comment nodes — a decision carries authority and drives stage transitions; a comment does not. 'Approved with changes' must be a distinct terminal-but-flagged state so downstream stages can proceed while surfacing outstanding change requests.",
      "scale": "all",
      "sources": "https://help.ziflow.com/hc/en-us/articles/30725285311508-Submit-a-Decision-or-Complete-review-in-the-Ziflow-Viewer | https://help.frame.io/en/articles/9105251-commenting-on-your-media | https://help.pageproof.com/en/articles/3161152-explaining-the-role-of-the-approver",
      "id": "SFR-WFRES-review-approval-04"
    },
    {
      "capability": "Per-stage deadlines with automated escalating reminders and bottleneck visibility on a dashboard",
      "closes_need": "NEED-005",
      "how_incumbents_do_it": "Filestage adds due dates (optionally with a time) to files; as the due date approaches reviewers get automated email reminders, and overdue reviewers get an automatic reminder. GoProof assigns timeframes per stage and its Proof Dashboard shows quick-glance infographics of completed stages, deadlines, and potential bottlenecks. Ziflow ties reminders to active workflow stages.",
      "studio_requirement": "ApprovalStage carries a deadline timestamp; a mechanical scheduler (cron/RemoteTrigger-class, not model-driven) emits REMINDER events to stage actors and marks OVERDUE state. Bottleneck/aging counts are a generated projection over ApprovalStage state (per [GLOBAL-GOVARTIFACTS] operator-facing views are projections over machine-readable state). Reminders route only to reviewers in the currently active stage.",
      "scale": "large",
      "sources": "https://filestage.io/blog/review-and-approval/ | https://goproof.net/automated-proofing-workflows",
      "id": "SFR-WFRES-review-approval-05"
    },
    {
      "capability": "Sign-off bound to a frozen, content-addressed version: every decision is attached to the exact asset version open at decision time, and approving v3 never implies approval of v4",
      "closes_need": "NEED-005, NEED-017",
      "how_incumbents_do_it": "Ziflow: 'every approval decision is attached to the proof version that was open at the time'; 'an approval of version 3 does not constitute an approval of version 4.' Proof Details are selectable per version showing that version's reviewer decisions. Frame.io version stacks keep each iteration as a distinct addressable version; comments/decisions attach to the version they reference.",
      "studio_requirement": "An ApprovalDecision event MUST reference an immutable content-addressed snapshot (Studio's StudioHistoryEntry / EventLedger snapshot hash), not a mutable document handle. Uploading a new version invalidates prior approvals for the changed version — the decision's version_hash no longer matches head. This reuses Studio's existing content-addressed history; the new work is binding a decision record to a specific hash and surfacing 'this approval is stale, head has moved.'",
      "scale": "all",
      "sources": "https://www.ziflow.com/blog/creative-approval-audit-trail-compliance | https://help.ziflow.com/hc/en-us/articles/30721821727124-About-Proof-Details",
      "id": "SFR-WFRES-review-approval-06"
    },
    {
      "capability": "Immutable, defensible audit trail with a fixed per-event data model (identity, timestamp, version, version-linked feedback, formal decision, stage path) that no user including admins can alter",
      "closes_need": "NEED-005, NEED-017",
      "how_incumbents_do_it": "Ziflow logs, per event: (1) named authenticated user for every comment/annotation/approval/rejection/stage transition, (2) system-generated timestamp, (3) the specific asset version reviewed, (4) feedback attached to the version it references, (5) formal binding decision distinct from comments, (6) full stage sequence the asset traveled. Logs are immutable (unmodifiable 'by any user, including platform administrators'), backed by SOC2 and RBAC/SSO identity. Every action is logged automatically 'because the approval cannot occur any other way.'",
      "studio_requirement": "This is a near-direct fit for the EventLedger: append-only, actor-attributed (KernelActor identity), timestamped events for COMMENT / ANNOTATION / DECISION / STAGE_TRANSITION, each carrying version_hash + stage_id. Studio must guarantee tamper-evidence/immutability of these ledger events and expose an evidence-report export (final version + all reviewer decisions + timestamps + full comment history). Attribution must come from authenticated sessions, not free-text names, for the large-team/regulatory case.",
      "scale": "large",
      "sources": "https://www.ziflow.com/blog/creative-approval-audit-trail-compliance | https://help.ziflow.com/hc/en-us/articles/30725285311508-Submit-a-Decision-or-Complete-review-in-the-Ziflow-Viewer",
      "id": "SFR-WFRES-review-approval-07"
    },
    {
      "capability": "Version-of-record provenance and exportable evidence report: prove which exact version was approved, by whom, at which stage, and export it as a self-contained compliance/defense record",
      "closes_need": "NEED-017",
      "how_incumbents_do_it": "Ziflow lets you select any proof version from a drop-down to see that version's Proof Details and reviewer decisions, and export an evidence report containing the final asset/version, reviewer decisions, timestamps, and full comment history. GoProof records who commented and who approved as a compliance audit trail; approvers formally sign off once comments are resolved. The audit record must show who held approval authority, what decision, at which version and stage.",
      "studio_requirement": "Studio needs a VersionOfRecord projection: given a deliverable, resolve the specific approved snapshot hash + the ApprovalDecision events that sanctioned it, and render an exportable evidence bundle (reuse the Package/collect primitive to bundle the approved snapshot + decision ledger slice). This closes the NEED-017 gap that Studio's kernel provenance is edit-proposal-level, not deliverable-sign-off-level. No AI: the report is a deterministic ledger query.",
      "scale": "large",
      "sources": "https://help.ziflow.com/hc/en-us/articles/30721821727124-About-Proof-Details | https://www.ziflow.com/blog/creative-approval-audit-trail-compliance | https://goproof.net/automated-proofing-workflows",
      "id": "SFR-WFRES-review-approval-08"
    },
    {
      "capability": "Region/frame-pinned and range-based threaded annotations that anchor feedback to an exact spot on the asset and to the version it references",
      "closes_need": "NEED-004",
      "how_incumbents_do_it": "Frame.io anchored commenting pins a comment to a specific spot on the asset viewer; range-based comments cover a span; on images/PDFs all annotated comments for a page are viewable in one overlay ('View on Asset with Annotation'). Comments thread with replies. Ziflow anchors comments/annotations to the version they reference. PageProof reviewers use the red pen to mark up and attach comments.",
      "studio_requirement": "Studio's native review surface ([STU-LAY-066]) already anchors comments to layout positions via CRDT collaboration; the promotable delta is (a) explicit region/range anchor geometry as a comment attribute, (b) per-page/per-artboard 'show all annotations' overlay, and (c) binding each annotation to a version_hash so it stays with the version it critiqued. Threading/replies map to CRDT comment nodes with parent refs.",
      "scale": "all",
      "sources": "https://help.frame.io/en/articles/9105251-commenting-on-your-media | https://www.ziflow.com/blog/creative-approval-audit-trail-compliance",
      "id": "SFR-WFRES-review-approval-09"
    },
    {
      "capability": "Comment resolution lifecycle: mark comments resolved/done, show-hide resolved, use the comment list as a to-do checklist, and let a gatekeeper triage each reviewer comment into actionable to-do vs ignore",
      "closes_need": "NEED-004",
      "how_incumbents_do_it": "Filestage comment sidebar doubles as a to-do list; resolved comments can be shown/hidden at will (unlike Google Docs where they vanish), enabling managers to verify everyone's feedback was met. PageProof approver marks comments left by others as a to-do for action or marks them to be ignored, and can edit others' comments for clarity. GoProof gates sign-off on all comments being resolved.",
      "studio_requirement": "Each CRDT comment node needs a resolution_state {OPEN | RESOLVED | IGNORED} and an optional to-do flag with assignee, mutated via attributed EventLedger events. A gatekeeper/approver permission can set IGNORED or promote to actionable to-do. Studio should expose an 'unresolved feedback count' predicate a PromotionGate can require == 0 before a stage can be approved (mirrors GoProof 'all comments resolved before sign-off').",
      "scale": "all",
      "sources": "https://help.filestage.io/en/articles/9113215-how-to-verify-that-everyone-s-feedback-has-been-met | https://help.pageproof.com/en/articles/3161152-explaining-the-role-of-the-approver | https://goproof.net/automated-proofing-workflows",
      "id": "SFR-WFRES-review-approval-10"
    },
    {
      "capability": "Version stack with side-by-side and overlay compare that carries comments, so reviewers see what changed round-over-round without re-proofing the whole asset",
      "closes_need": "NEED-017, NEED-004",
      "how_incumbents_do_it": "Filestage stacks all versions together (latest clearly marked) and compares any two versions (even v1 vs v7) side-by-side or in overlay mode, including their comments, so you can check who said what about which version. Frame.io drag-and-drop version stacks keep iterations organized for comparison. This supports per-round change tracking and 'verify what changed instead of re-proofreading 80 pages.'",
      "studio_requirement": "Reuse Studio's existing side-by-side/overlay semantic-diff (NEED-011, [STU-LAY-066] document-states) but scope it to the review context: pick any two approval versions by their snapshot hashes and render diff + each version's anchored comments together. The compare view is a read-only projection over content-addressed snapshots + the comment CRDT; no new authority, just a review-oriented lens.",
      "scale": "all",
      "sources": "https://help.filestage.io/en/articles/5560093-compare-versions-of-a-file-directly-in-the-viewer | https://changelog.filestage.io/compare-two-versions-of-the-same-file-side-by-side-206728",
      "id": "SFR-WFRES-review-approval-11"
    },
    {
      "capability": "Vendor proof/correction round-trip to OK-to-print: export to vendor/printer spec, ingest the vendor's annotated proof rejects back into the asset context, and drive a resubmission loop to final sign-off",
      "closes_need": "NEED-022",
      "how_incumbents_do_it": "Proofing tools export a version to a proof, collect the reviewer/vendor's anchored markups against that version, require correction, then a new version restarts the decision loop until a formal approval/OK-to-print is recorded. Ziflow's targeted-rejection routing plus version-linked decisions and evidence report give the who-corrected-what-per-round-through-to-approval chain; PageProof approver converts vendor comments into to-dos that must be actioned before final approval.",
      "studio_requirement": "Studio must let external/vendor proof annotations be imported and anchored to layout positions (spec line 1143 interchange path already exists) as review comments tied to the exported version_hash; a resubmission emits a new snapshot that reopens the vendor's approval stage. The correction ledger (which to-dos were actioned per round) is a query over resolution_state transitions across versions. Export side reuses StudioExportRecipe (PDF/X). Final OK-to-print = terminal APPROVED decision on the vendor stage.",
      "scale": "small",
      "sources": "https://help.pageproof.com/en/articles/3161152-explaining-the-role-of-the-approver | https://www.ziflow.com/blog/creative-approval-audit-trail-compliance | https://help.ziflow.com/hc/en-us/articles/30721932087828-How-Do-I-Trigger-a-Stage-Based-on-Another-Stage-s-Decision-Status",
      "id": "SFR-WFRES-review-approval-12"
    },
    {
      "capability": "New-version-invalidates-prior-decisions with re-review scoped to the active stage's reviewers (per-round decision reset)",
      "closes_need": "NEED-022, NEED-017",
      "how_incumbents_do_it": "Ziflow: decision notifications are tied to active workflow stages and sent only to reviewers included in the active stage for that version; a new version is a new decision surface (approval of v3 != v4). GoProof/Filestage restart the reviewer step on a new version and track per-version decisions. This produces per-round change tracking and prevents a stale approval carrying forward onto changed content.",
      "studio_requirement": "On a new snapshot promoted into a review, Studio must automatically mark prior-version ApprovalDecisions as superseded (not delete them — audit retains them) and re-notify only the active stage's actor set. Each version keeps its own decision set queryable independently. This is mechanical EventLedger bookkeeping keyed on version_hash change; the orchestrator/broker relays notifications, it does not decide.",
      "scale": "large",
      "sources": "https://help.ziflow.com/hc/en-us/articles/37922335934996-Add-reviewers-and-configure-workflows-for-a-proof | https://help.ziflow.com/hc/en-us/articles/30721821727124-About-Proof-Details",
      "id": "SFR-WFRES-review-approval-13"
    },
    {
      "capability": "Delegated / on-behalf-of decisions logged with the true actor, so an owner can record an offline approver's decision without breaking attribution",
      "closes_need": "NEED-005, NEED-017",
      "how_incumbents_do_it": "Ziflow lets a decision be submitted 'on behalf of' another reviewer, and the Activity log explicitly records that the decision was made on behalf of that reviewer (both the acting user and the represented reviewer are captured).",
      "studio_requirement": "ApprovalDecision event needs both acting_actor and on_behalf_of_actor fields so delegated sign-offs preserve a truthful audit chain (the EventLedger records who physically entered it and whose authority it represents). Permission model must gate who may act on behalf of whom. This prevents the audit trail from silently misattributing authority — a defensibility requirement; UNVERIFIED whether Studio's session model currently supports dual-actor attribution.",
      "scale": "large",
      "sources": "https://help.ziflow.com/hc/en-us/articles/30725285311508-Submit-a-Decision-or-Complete-review-in-the-Ziflow-Viewer",
      "id": "SFR-WFRES-review-approval-14"
    },
    {
      "capability": "Lightweight in-canvas sign-off for solo/small scale: a per-frame/section status stamp (e.g. approved / ready-for-dev) with remark and ticket reference, without standing up a full multi-stage workflow",
      "closes_need": "NEED-005",
      "how_incumbents_do_it": "Figma's Design Sign-Off community widget puts a status button + remark section + task-ticket (Jira link) field directly on the canvas/FigJam; Figma Dev Mode's native 'Mark as ready for dev' flags a frame/component/section and aggregates all ready-for-dev items in one view, notifying eligible seats when something is marked ready. This is the invisible-for-solo end of the review spectrum.",
      "studio_requirement": "Studio needs a minimal per-region ApprovalStatus stamp (status enum + note + external ticket ref) that writes the same ApprovalDecision/EventLedger record as the full workflow but with zero routing configured — satisfying the register's 'invisible-for-solo but governable-for-large' architecture implication. The solo stamp and the large-team multi-stage workflow must share one decision primitive so evidence/provenance is uniform across scales.",
      "scale": "solo",
      "sources": "https://www.figma.com/community/widget/1380305652912733563/design-sign-off | https://help.figma.com/hc/en-us/articles/23918228264855-Dev-Mode-ready-for-dev-view",
      "id": "SFR-WFRES-review-approval-15"
    }
  ]
}
```

### [SFR-WFRES.workflow-state-pm] Production workflow-state / project management (NEED-009)

```json
{
  "capabilities": [
    {
      "capability": "Per-asset production status state machine with named linear stages (capture -> selection -> review -> processing -> post -> QC -> post-review -> delivery)",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force defines an ordered flow of named steps each asset traverses: Capture, Final Selection, Photo Review, Digital Processing, External Post, External Post QC, Internal Post, Internal Post QC, Post Review, Cloud Automation, Asset Delivery. Status is a first-class per-asset value; assets 'advance to the next step as tasks are completed' rather than moving in whole batches ('flow production').",
      "studio_requirement": "Model asset status as a typed enum register (CRDT LWW-register keyed per asset) whose allowed values and ordering come from a per-project workflow definition; every stage transition is an append-only EventLedger event (asset_id, from_state, to_state, actor, ts). No parallel-spreadsheet source of truth.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/workflow-automation ; https://www.creativeforce.io/how-it-works",
      "id": "SFR-WFRES-workflow-state-pm-01"
    },
    {
      "capability": "Conditional status-based routing / auto-advance ('asset paths based on status')",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force lets managers 'Define asset paths based on status for unparalleled control' with 'Conditional rules' that fine-tune which processes/style guides apply; as soon as an on-set team selects images the asset is automatically routed to an art director, then on approval to post-production, with no manual batch handoff.",
      "studio_requirement": "A typed workflow-definition entity (states + guarded transitions + routing predicates on asset metadata) evaluated deterministically when an EventLedger transition event lands, emitting the next assignment/route event. Routing rules are versioned typed state, not code, so a no-context model can read the active path.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/workflow-automation",
      "id": "SFR-WFRES-workflow-state-pm-02"
    },
    {
      "capability": "QC approve/reject states with rework loop and captured rejection reason",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force has explicit QC gates ('External Post QC' = 'Validate every asset returned from external vendors', 'Internal Post QC' = 'Review and approve in-house edits') plus a 'Bypass External QC' option; a Rejections Report captures rejected content and reasons and a rejection-rate KPI is tracked. QC on how-it-works is 'Approve or reject the work performed by your external post-production vendor'.",
      "studio_requirement": "QC is a typed gate state with approve/reject outcome; a reject event carries a structured rejection_reason and routes the asset back to the prior stage (rework loop) via a new EventLedger event, incrementing a per-asset revision/round counter in typed state. Rejection rate is derived by aggregating reject events.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/workflow-automation ; https://www.creativeforce.io/how-it-works ; https://www.creativeforce.io/product/reporting",
      "id": "SFR-WFRES-workflow-state-pm-03"
    },
    {
      "capability": "Real-time Kanban board (to-do / doing / done) per task, as a live projection of production state",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force Kanban View has three columns (to do, doing, done) showing samples for a given task, 'updated in real-time reflecting up-to-date production status'; Kanban views also 'reveal and resolve workflow bottlenecks'.",
      "studio_requirement": "The board is a derived read-model projection over the EventLedger (grouping assets by current typed status), never an authoritative store; column membership updates reactively from CRDT status registers so parallel sessions see a consistent live board without polling.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/how-it-works ; https://www.creativeforce.io/product/reporting",
      "id": "SFR-WFRES-workflow-state-pm-04"
    },
    {
      "capability": "Deliverables matrix / shot list — required output set (image/video variants) defined per product/SKU with per-deliverable completion tracking",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force drives production from job/product data with 'predetermined shot lists for on-site teams' to reduce reshoots; workflows detail production type, image selection, and 'final asset delivery' per product, so each product carries a defined set of required deliverables tracked to delivery.",
      "studio_requirement": "A typed DeliverableSpec entity set attached to each production job (each row = required asset kind/variant/angle) with an independent status register per deliverable; matrix completeness is a projection counting delivered vs required deliverable events. UNVERIFIED: exact per-product deliverable-count cap is not documented.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/ecommerce-content-production ; https://www.creativeforce.io/how-it-works",
      "id": "SFR-WFRES-workflow-state-pm-05"
    },
    {
      "capability": "Physical sample check-in/check-out lifecycle with barcode, location, and 'in production' state",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force Sample Check-In scans barcodes, assigns locations, and organizes containers; alert conditions include 'Already checked in' and 'The product is in production'. The Sample Report gives 'a comprehensive overview of the location and status of all samples ... ensuring no samples get lost' and tracks product flow in/out of the studio.",
      "studio_requirement": "Track physical-sample state as a typed status + location register per sample with check-in/check-out modeled as EventLedger custody events (actor, location, ts); duplicate-check-in and in-production conditions are guard predicates that reject conflicting events. Sample location is queryable current-state projection.",
      "scale": "large",
      "sources": "https://www.creativeforce.io/how-it-works ; https://www.creativeforce.io/product/reporting",
      "id": "SFR-WFRES-workflow-state-pm-06"
    },
    {
      "capability": "Bottleneck + real-time production-status count dashboards (Flow Reports)",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force Flow Reports 'give a full overview of your production status across every asset, helping you identify bottlenecks'; a built-in dashboard aggregates 'from across your entire production process' with real-time updates and drill-down. Metric surface includes sample status, asset throughput, and real-time production status counts.",
      "studio_requirement": "Bottleneck view is an aggregate projection over the EventLedger: count of assets currently resident in each typed status + dwell-time per stage (from consecutive transition timestamps). Counts are derived aggregates, never hand-maintained; drill-down resolves back to the underlying per-asset event stream.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/reporting ; https://www.creativeforce.io/challenges/real-time-tracking",
      "id": "SFR-WFRES-workflow-state-pm-07"
    },
    {
      "capability": "Throughput / lead-time / vendor-turnaround KPIs computed from production timeline",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force measures sample status, asset throughput, rejection rates, product lead time, and vendor turnaround times; named reports include Production Reports (throughput), Data Reports (lead time / production timeline), Post-Production Vendor Report (vendor turnaround), and Productivity Reports (per-contributor).",
      "studio_requirement": "Durations (lead time, stage dwell, vendor turnaround) are computed deterministically from paired EventLedger transition timestamps; throughput = delivery events per interval. KPIs are pure functions of the ledger so any model can recompute them reproducibly; no separate metrics store to drift.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/reporting ; https://www.creativeforce.io/industry/commercial-studios",
      "id": "SFR-WFRES-workflow-state-pm-08"
    },
    {
      "capability": "Role/vendor assignment routing with automatic handoff between stage owners",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force plans production by scheduling sessions on a unified calendar with team assignments, then performs 'automatic handoffs of assignments' between stage owners (photographer -> stylist -> selection -> retoucher/external vendor -> QC); vendor stages route to 'top-tier post-production vendors'. Editorial equivalent: InCopy/InDesign check-out assigns pages to named editors/designers.",
      "studio_requirement": "Assignment is a typed edge (asset/stage -> assignee) that both scopes a permission (assignee may act on that stage) and opens/owns a session; stage completion emits a handoff EventLedger event that reassigns the next stage per the workflow definition. Attribution and recoverability come from the event actor field.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/how-it-works ; https://www.creativeforce.io/product/workflow-automation",
      "id": "SFR-WFRES-workflow-state-pm-09"
    },
    {
      "capability": "Magazine flatplan per-page workflow status tags (Signed off / Design started / Not received / Awaiting Copy)",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Blinkplan/Flat-Plan let each page carry customizable workflow tags such as 'Signed off', 'Design started', 'Not received', 'Awaiting Copy', shown as small colored dots per page; the whole team gets 'complete visibility and control over each issue' and editors/proofreaders can follow current production status live.",
      "studio_requirement": "Per-page status modeled as a typed tag set (CRDT OR-set or status register) on a Page entity within an Issue; the flatplan is a spatial board projection ordering pages, with status color derived from the tag. Tag vocabulary is project-configurable typed state, matching custom stage naming.",
      "scale": "all",
      "sources": "https://www.blinkplan.com/docs/edit-page/ ; https://www.magazineproduction.com/how-to/what-is-a-flatplan",
      "id": "SFR-WFRES-workflow-state-pm-10"
    },
    {
      "capability": "Flatplan advertising-space state machine (available / reserved / sold) with editorial-vs-advert content typing",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "GoPublish/RunMags flatplans show advertising teams 'available, reserved, and sold space' and color-code content types (ads, editorial, promotions) so the layout mix is visible at a glance; page ownership and progress are viewable in real time across editorial, advertising, design, and production teams.",
      "studio_requirement": "Each page/slot carries a typed content-role (editorial | advert | promo) and, for ad slots, a typed sales-state register (available -> reserved -> sold) whose transitions are EventLedger events (with actor = ad-sales). Board coloring is a projection of role+state; conflicting reservations are rejected by transition guards.",
      "scale": "all",
      "sources": "https://www.gopublish.net/solutions/magazine-and-publication-layout ; https://www.runmags.com/flatplan/",
      "id": "SFR-WFRES-workflow-state-pm-11"
    },
    {
      "capability": "Editorial page check-in/check-out with file locking and version management (InDesign/InCopy)",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "GoPublish integrates with Adobe InDesign and InCopy so designers/editors 'check pages in and out, lock files while working, manage versions, and submit final artwork without leaving their Adobe environment'; this is the native assignment/check-in-check-out copy-editing model already noted in Spec 14 line 1140.",
      "studio_requirement": "Page-level exclusive edit is a session lease (typed lock register naming the holding session) acquired on check-out and released on check-in, both as EventLedger events; while held, CRDT merges for that page defer to the lease holder to prevent conflicting concurrent edits. Version submission emits a version-record event.",
      "scale": "all",
      "sources": "https://www.gopublish.net/solutions/magazine-and-publication-layout",
      "id": "SFR-WFRES-workflow-state-pm-12"
    },
    {
      "capability": "Multi-round revision stacking with current-version-of-record pointer and A/B round toggle",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "verybusy.io drives high-volume stills 'to signoff through multi-round revisions', keeping 'the latest file on top and every round right behind it', revisions 'stacking automatically so ... no one is left guessing what's current', with A/B toggle of revision rounds and filtering for unresolved comments.",
      "studio_requirement": "Each asset holds an ordered revision chain in the EventLedger; a typed current-version pointer (LWW register) marks the version-of-record so the 'what's current' answer is unambiguous across sessions. Round number is typed state; A/B compare reads any two ledger versions without mutating current.",
      "scale": "all",
      "sources": "https://verybusy.io/ ; https://verybusy.io/solutions/creative-teams",
      "id": "SFR-WFRES-workflow-state-pm-13"
    },
    {
      "capability": "Per-contributor / per-vendor productivity and workload attribution",
      "closes_need": "NEED-009",
      "how_incumbents_do_it": "Creative Force Productivity Reports give 'details on individual contributor performance' and the Post-Production Vendor Report tracks per-vendor turnaround and volume, letting managers balance load and hold vendors to SLA.",
      "studio_requirement": "Because every transition/handoff EventLedger event carries an actor, per-contributor throughput and in-flight workload are pure aggregate projections over the ledger keyed by actor; no separate timesheet state. Enables load-balancing routing decisions in the workflow definition.",
      "scale": "all",
      "sources": "https://www.creativeforce.io/product/reporting",
      "id": "SFR-WFRES-workflow-state-pm-14"
    }
  ]
}
```

### [SFR-WFRES.perf-nfr] Production-volume performance NFRs (NEED-026/047/071/095)

```json
{
  "capabilities": [
    {
      "capability": "Explicit per-document/session working-memory ceiling as a published NFR, with graded pressure telemetry and a hard-lock recovery mode — targeting a ceiling above the browser-tab wall so Studio does not force file-splitting at design-system scale",
      "how_incumbents_do_it": "Figma is bound to a hard 2GB active-memory limit per browser tab (applies even to the desktop app because it is browser-tech). It surfaces graded alerts as usage climbs, with a non-dismissible red alert tile in the sidebar at 90%, and at 100% it LOCKS the file entirely ('no available memory'). Recovery is only possible in modern browser builds (Chrome 83+/FF 89+/Safari 15.2+/Edge 93+) by getting usage back below 90%; a crashed file can be reopened low-res via the `?thumbnails-only=1` URL param. Hidden layers still consume memory because Figma stores/renders their info even when invisible.",
      "studio_requirement": "Studio must NOT inherit a single-process/single-tab memory wall as its scaling limit. Set a target working-set ceiling per open document well above 2GB (native runtime, not browser-tab-bound) and treat it as a published NFR. Emit graded memory-pressure events on the EventLedger at defined thresholds (e.g. 60/75/90%) so any observing session/agent can react; at ceiling, degrade to a read-only/thumbnail-only safe mode (CRDT stays consistent, no edit loss) rather than hard-locking. Hidden/collapsed CRDT subtrees must be evictable from the working set, not permanently resident.",
      "scale": "large",
      "sources": "https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_need": "NEED-026, NEED-047, NEED-071",
      "id": "SFR-WFRES-perf-nfr-01"
    },
    {
      "capability": "Proxy / low-res preview render mode for high-res linked images, with per-object display-quality override, so redraw/scroll stays fast on image-heavy documents while full-res is available on demand",
      "how_incumbents_do_it": "InDesign renders linked images at low resolution by default and offers three Display Performance modes — Fast (greeked/gray box, no image), Typical (screen-friendly proxy resolution), High Quality (full res) — settable document-wide (View > Display Performance) and per-object ('Object-Level Display' via right-click). Guidance is to edit in Typical/Fast and switch to High Quality only when needed, because a book with thousands of full-res images 'would work very slowly' otherwise. Illustrator similarly recommends Link (not Embed) for bitmaps and rasterizing complex background art to keep redraw cheap.",
      "studio_requirement": "Studio must ship a tri-state display-quality model (greeked box / proxy / full-res) resolvable at both document and per-object granularity, defaulting to proxy for placed high-res assets. Proxy generation is a derived cache keyed to the asset content-hash (shareable across sessions, never mutating the CRDT source). Per-object display-quality is view-state, stored per-session/per-viewport, so two parallel model/human sessions can hold different quality settings on the same document without conflict.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/au/indesign/kb/fix-image-display-performance.html, https://creativepro.com/6-tips-speed-up-indesign/, https://helpx.adobe.com/illustrator/kb/optimize-illustrator-performance.html",
      "closes_need": "NEED-026, NEED-047",
      "id": "SFR-WFRES-perf-nfr-02"
    },
    {
      "capability": "Lazy / on-demand link loading and decode for thousands of linked hi-res images, so open time and scroll cost scale with the visible viewport, not with total document asset count",
      "how_incumbents_do_it": "InDesign users report documents becoming 'very slow' once thousands of images are placed (a math book at ~3300 images per chapter), and the practical field workaround is to batch-convert every EPS/hi-res link to a low-res proxy extension for editing and only relink to the real hi-res EPS/TIFF at export time — i.e. manual lazy loading. InDesign's own default of showing low-res proxies is the built-in version of deferring full-res decode.",
      "studio_requirement": "Studio must lazily fault-in image link decode by viewport visibility and quality tier — full-res pixels loaded only for on-screen, full-res-requested objects; everything else holds proxy or metadata only. Link records live in the CRDT (path, content-hash, dimensions, colorspace); the decoded pixel buffer is a non-CRDT derived cache with an LRU eviction bound tied to the memory-ceiling NFR. Export is the one path that force-resolves every link to full-res, streamed so peak memory stays bounded even when total linked bytes exceed the ceiling.",
      "scale": "large",
      "sources": "https://community.adobe.com/t5/indesign-discussions/indesign-getting-slow-due-to-more-number-of-images/td-p/12380748, https://helpx.adobe.com/au/indesign/kb/fix-image-display-performance.html",
      "closes_need": "NEED-026",
      "id": "SFR-WFRES-perf-nfr-03"
    },
    {
      "capability": "Incremental save with bounded file-bloat, so a document does not accumulate dead history/preview cruft across a long deadline-crunch session and force a manual 'reset' to stay fast",
      "how_incumbents_do_it": "InDesign .indd files are database files: fast incremental Saves append and leave hidden leftover data (deleted images, old previews, undo/recovery infrastructure) inside the file, so size steadily bloats during a session. The field remedy is a periodic File > Save As over the same name, which writes a fresh compacted copy — typically shrinking the file to about 1/4 of its bloated size — because Save As omits the accumulated cruft. Incremental Save = fast but bloats; Save As = slower but compacts.",
      "studio_requirement": "Studio's persistence layer must give BOTH: cheap incremental append (fast crunch-time saves, crash-recoverable) AND bounded on-disk growth, without a manual 'Save As to reset' ritual. Because the EventLedger is inherently append-only, Studio must run background compaction/checkpointing that snapshots current CRDT state and prunes superseded ledger tail beyond a retention window, keeping file size proportional to live document content, not to session edit count. Compaction must be non-blocking and preserve enough history for the undo/version window.",
      "scale": "all",
      "sources": "https://catalogtips.com/kb/additional-resources/using-adobe-indesign/using-save-as-to-reduce-indesign-file-size/, https://creativepro.com/why-is-my-file-size-so-huge/",
      "closes_need": "NEED-026, NEED-095",
      "id": "SFR-WFRES-perf-nfr-04"
    },
    {
      "capability": "Many-artboard / many-frame canvas that stays interactive well past the point where incumbents stall or hard-cap, decoupling per-operation cost from total artboard count",
      "how_incumbents_do_it": "Illustrator hard-caps at 1,000 artboards per document (raised from 100 in 2017) and users hit that ceiling on large icon sets; separately, users report Illustrator 'gets extremely slow' above ~20 artboards even with simple vector/text, and Save-as-PDF across many artboards is a known slow path. Community consensus is the real cost driver is object/complexity count and per-operation redraw, not artboards per se. Field mitigations: delete unused artboards, convert repeats to Symbols (reference one copy), rasterize heavy art, split into multiple files.",
      "studio_requirement": "Studio must NOT impose an artboard/frame hard cap as an architectural limit and must keep per-operation cost independent of off-screen artboard count. Render and hit-test only viewport-intersecting frames (spatial index / virtualized canvas); operations mutate only the touched CRDT subtree so edit latency is O(changed) not O(document). Repeated content uses shared component/symbol references (one CRDT node, many instances) to bound memory. Publish an NFR target (e.g. 1,000+ artboards or image-heavy frames with sub-frame edit latency) that explicitly beats Illustrator's cap and slowdown.",
      "scale": "large",
      "sources": "https://illustrator.uservoice.com/forums/601447-illustrator-desktop-bugs/suggestions/46458187-illustrator-becomes-extremely-slow-with-more-than, https://www.digitaltrends.com/photography/adobe-illustrator-artboards-limit-1000/, https://helpx.adobe.com/illustrator/kb/optimize-illustrator-performance.html",
      "closes_need": "NEED-047",
      "id": "SFR-WFRES-perf-nfr-05"
    },
    {
      "capability": "Library-scale lazy variant loading and amortized component memory across documents, so large design-system libraries do not have to be sharded to stay usable",
      "how_incumbents_do_it": "Figma's memory guidance explicitly targets libraries: it recommends replacing large variant sets with component/boolean properties to cut the number of variants+layers stored, and 'break up large library files' by moving published components into smaller files — i.e. manual sharding — because every variant and every hidden layer in a library consumes the 2GB/tab budget even when unused. There is no amortization across consuming files; each open file re-pays the component memory cost.",
      "studio_requirement": "Studio must load component/variant definitions lazily (fault-in the specific variant actually instantiated, not the whole set) and amortize a component's memory across all documents that reference it via a shared content-addressed store rather than per-document copies. Boolean/property-driven variants should be resolved on demand instead of pre-materializing every combination. Publishing a large library must not force operators to shard; the library is a versioned governed release (permission-gated) whose members are streamed to consumers on reference, not on library-open.",
      "scale": "large",
      "sources": "https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_need": "NEED-071",
      "id": "SFR-WFRES-perf-nfr-06"
    },
    {
      "capability": "Long-document recomposition and layout that scales to many hundreds of pages with linked/pass-through content without slowing to a crawl",
      "how_incumbents_do_it": "Affinity Publisher is reported to 'slow to a crawl' on very large projects around 600+ pages composed of linked pass-through PDFs. Recommended mitigations are environmental rather than algorithmic: raise the app RAM limit to 75–80% of installed RAM, and select the correct hardware renderer (GPU with current drivers, or fall back to WARP software rasterizer). InDesign's parallel long-doc mitigations are turning off Pages-panel thumbnails and deferring live preflight so page redraw/recompose stays cheap.",
      "studio_requirement": "Studio must keep text/layout recomposition incremental and localized — reflow only pages affected by an edit, not the whole document — so cost scales with the changed region, not page count. Page-panel/navigator thumbnails must be lazily generated and cached (derived, off the CRDT) and throttled so they never block editing. Set an NFR: hundreds-of-pages documents with linked assets stay interactive (open/scroll/edit) under the memory ceiling. Expose an operator-tunable memory budget (analogous to Affinity's 75–80% RAM cap) so the runtime is bounded on constrained machines.",
      "scale": "large",
      "sources": "https://forum.affinity.serif.com/index.php?/topic/155779-ram-usage-limit-how-to-optimize-with-publisher/, https://forums.steinberg.net/t/affinity-publisher-for-large-dtp-projects/758403, https://creativepro.com/6-tips-speed-up-indesign/",
      "closes_need": "NEED-026, NEED-047",
      "id": "SFR-WFRES-perf-nfr-07"
    },
    {
      "capability": "GPU-accelerated renderer with explicit software fallback and operator-selectable render backend, so performance degrades gracefully on varied/failing hardware instead of stalling",
      "how_incumbents_do_it": "Affinity exposes a renderer choice: pick the main GPU (with latest drivers) or switch to WARP (Windows Advanced Rasterization Platform, software rasterizer) when the GPU path underperforms or misbehaves; GPU/driver state is called out as a primary performance factor. Illustrator/InDesign similarly gate GPU-accelerated view on hardware and let users disable it. The pattern: hardware-accelerated by default, but a documented software fallback path exists and is user-switchable.",
      "studio_requirement": "Studio's canvas renderer must be GPU-accelerated by default with a working software-rasterizer fallback, and the backend must be operator-selectable and auto-degrading (detect GPU init failure/driver crash and fall back without losing the session). Renderer choice is machine-local config (portability: not baked into the document/CRDT). Render backend health should emit telemetry so a no-context model can see whether a perf complaint is GPU-path vs data-scale.",
      "scale": "all",
      "sources": "https://forum.affinity.serif.com/index.php?/topic/155779-ram-usage-limit-how-to-optimize-with-publisher/, https://helpx.adobe.com/illustrator/kb/optimize-illustrator-performance.html",
      "closes_need": "NEED-047",
      "id": "SFR-WFRES-perf-nfr-08"
    },
    {
      "capability": "Document-hygiene / 'what makes this file heavy' tooling: report and purge unused swatches/symbols/artboards/components, simplify paths, and flag embedded-vs-linked bloat",
      "how_incumbents_do_it": "Incumbents treat this as manual hygiene: Illustrator guidance is delete unused artboards, convert repeats to Symbols, and Link rather than Embed bitmaps (embedding 'dramatically increases document size'). InDesign bloat is diagnosed via the Links panel and reset via Save As; CreativePro's 'why is my file huge' analysis points at leftover/hidden data. Figma's memory page is effectively a manual heaviness checklist (variants, hidden layers, stacked masks/effects, uncompressed images). None provide a single automated 'heaviness report'.",
      "studio_requirement": "Studio should ship a first-class hygiene analyzer that (a) enumerates unreferenced swatches/symbols/components/artboards and offers safe purge, (b) attributes working-set memory and on-disk size to specific objects/asset classes ('this doc is heavy because: 3,300 embedded images / 12k hidden layers / stacked effects'), and (c) flags embedded assets that should be linked. Because Studio holds the full CRDT + EventLedger, reference-counting for unused detection and per-node size attribution are computable exactly, not heuristically. Purge actions are ordinary CRDT edits (undoable, permission-checked, logged).",
      "scale": "all",
      "sources": "https://helpx.adobe.com/illustrator/kb/optimize-illustrator-performance.html, https://creativepro.com/why-is-my-file-size-so-huge/, https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_need": "NEED-095",
      "id": "SFR-WFRES-perf-nfr-09"
    },
    {
      "capability": "Graded memory-pressure warning UX (not a single cliff), giving operators and observing agents advance signal before a document becomes unusable",
      "how_incumbents_do_it": "Figma escalates rather than failing silently: it warns as usage climbs, shows a non-dismissible red alert tile at 90%, and only hard-locks at 100% — but it explicitly cautions the alerts may be skipped 'if memory usage increases in a short space of time,' so the cliff can still surprise users. The signal is human-facing UI only; there is no machine/event hook for automation to pre-empt the lock.",
      "studio_requirement": "Studio must emit memory/working-set pressure as structured EventLedger events at defined graded thresholds (not just a terminal lock) so BOTH the operator UI and parallel automated sessions can react early — e.g. trigger background compaction, evict proxy caches, or pause a heavy import before the ceiling. Guarantee the signal cannot be fully skipped under fast growth by sampling on allocation, not only on a UI timer. This closes the 'you may not receive an alert at all' gap Figma documents.",
      "scale": "all",
      "sources": "https://help.figma.com/hc/en-us/articles/360040528173-Reduce-memory-usage-in-files",
      "closes_need": "NEED-026, NEED-071",
      "id": "SFR-WFRES-perf-nfr-10"
    },
    {
      "capability": "Deferred / throttled background validation and live-checks (preflight, link-status, URL verification, thumbnail redraw), so continuous background work does not tax interactive editing on large documents",
      "how_incumbents_do_it": "InDesign performance guidance is largely 'turn off the always-on background jobs': set Live Screen Drawing to Delayed (don't render every step of every transform), limit Live Preflight to specific pages or disable until final output, turn off Pages-panel Show Thumbnails, deselect Auto Update URL Status in the Hyperlinks panel (stop constantly re-validating URLs), and disable Always Save Preview Images. Each is a background task that silently competes with editing on big files.",
      "studio_requirement": "Studio's background validators (preflight-equivalent checks, link resolution, thumbnail/preview generation, hygiene scan) must run on an amortized, throttled, cancellable scheduler that yields to interactive input and is bounded so it never starves editing — and each must be individually toggleable/scopeable (whole-doc vs current-page). Preview/thumbnail generation is a derived cache, never on the edit hot path. This also satisfies the [GLOBAL-BUILD-QUIET] class: background work stays quiet, bounded, and observable without interrupting the operator.",
      "scale": "large",
      "sources": "https://creativepro.com/6-tips-speed-up-indesign/",
      "closes_need": "NEED-047, NEED-095",
      "id": "SFR-WFRES-perf-nfr-11"
    }
  ]
}
```

### [SFR-WFRES.dam-pim-libraries] DAM/PIM + governed library releases (NEED-045/057/101)

```json
{
  "capabilities": [
    {
      "capability": "Asset check-out with pessimistic lock against a canonical DAM/PIM repository",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "CI HUB Connector and Adobe Asset Link (AEM Assets) implement a check-out that locks the asset for other users while it is being edited, preventing conflicting concurrent changes until it is checked back in. Asset shows a checked-out/locked state; other users see it as unavailable for edit. The lock is held in the DAM, not the local file.",
      "studio_requirement": "EventLedger must record a check_out event carrying {asset_id, canonical_repo_id, actor, lock_token, ts} that establishes an exclusive edit lease on a canonical asset; local library placed_asset (STU-RAS-012) must display the lock state and block a second concurrent check-out. Because Studio is local-first CRDT, the lock is an advisory lease coordinated through the ledger/session layer, not a hard filesystem lock (UNVERIFIED: exact conflict-resolution policy when a leased asset is edited offline).",
      "scale": "all",
      "sources": "https://ci-hub.com/blog/simplifying-asset-access-with-dam-connectors ; https://helpx.adobe.com/enterprise/using/adobe-asset-link.html",
      "id": "SFR-WFRES-dam-pim-libraries-01"
    },
    {
      "capability": "Check-in that uploads the edited asset back as a NEW version, auto-increments the version number, and syncs metadata/comments",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "CI HUB's Check-In command uploads the modified file back to the DAM, increments the version number, and synchronizes any metadata or comments updated during the session; the lock is released. Acquia DAM exposes 'Update file' with an upload profile that 'adds the asset as a new version'. Adobe Asset Link check-in creates a new version in the DAM version history.",
      "studio_requirement": "EventLedger check_in event must append an immutable new version node to the canonical asset's version chain {prior_version, new_version, actor, checksum, metadata_delta, ts} and release the lease from the paired check_out. Version numbering is monotonic and server-assigned; metadata deltas ride the same event so registry and asset stay consistent. Reuses EventLedger append-only semantics; no silent overwrite of the prior version.",
      "scale": "all",
      "sources": "https://ci-hub.com/blog/simplifying-asset-access-with-dam-connectors ; https://docs.acquia.com/acquia-dam/how-do-i-use-ci-hub-connector-adobe-and-office-365",
      "id": "SFR-WFRES-dam-pim-libraries-02"
    },
    {
      "capability": "Version-history browsing with link-health/staleness detection so out-of-date assets are never silently used",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "CI HUB and Acquia DAM let users open a version dropdown per linked asset, switch between versions and high/low-res renditions, and run a 'Check status' action; the connector detects when a newer version exists in the DAM and flags stale links, keeping assets trustworthy. Bynder's CC Connector auto-detects a new uploaded version and the Links tab notifies the user.",
      "studio_requirement": "The existing placed_asset link-health tri-state (up-to-date / modified / missing, STU-RAS-012) must extend to a canonical-repo dimension: compare the locally-linked version id against the ledger head for that asset and surface 'newer-version-available' plus a per-link version picker and update-all. Reuses existing link-health + update-all machinery; adds ledger version-head lookup.",
      "scale": "all",
      "sources": "https://ci-hub.com/blog/streamline-your-adobe-workflow-using-ci-hub ; https://support.bynder.com/hc/en-us/articles/16126975310866-Understanding-And-Implementing-The-Adobe-Creative-Cloud-Bynder-Connector",
      "id": "SFR-WFRES-dam-pim-libraries-03"
    },
    {
      "capability": "Drag-and-drop placement that keeps assets LINKED to the canonical DAM source (never a detached local copy), with one-click relink and deep-link-to-source",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "OpenAsset, CI HUB, and Bynder LinkrUI drag assets from the panel into the layout as linked placements so the document always points back to the DAM source rather than a downloaded copy; assets stay portable between users/machines; CI HUB provides deep links back to the original admin view to confirm the correct file, and one-click relink (e.g. Premiere footage). OpenAsset upgrades linked images to high-res at package time.",
      "studio_requirement": "placed_asset must persist a canonical_link {repo_id, asset_id, version_id, deep_link_uri} distinct from an embedded copy, resolvable by any collaborator opening the CRDT document. Relink and 'reveal in source DAM' operate on this reference; low-res proxy placement with deferred high-res resolution at package/export time. Reuses embedded/linked placement model; adds canonical repo coordinates and proxy/high-res swap.",
      "scale": "all",
      "sources": "https://success.openasset.com/en/articles/3095670-linking-images-from-indesign-documents-to-openasset ; https://docs.acquia.com/acquia-dam/how-do-i-use-ci-hub-connector-adobe-and-office-365",
      "id": "SFR-WFRES-dam-pim-libraries-04"
    },
    {
      "capability": "Published-asset registry: approval gate promotes finals from a staging state into an approved/published catalog of canonical finals",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "Bynder routes assets through a Waiting Room where a reviewer clicks Approve/Deny before the asset enters the Asset Bank, ensuring only brand-approved, published finals become discoverable; approved assets are then surfaced in curated Collections and Brand Guidelines portals. This creates an explicit lifecycle: uploaded → in-review → approved/published.",
      "studio_requirement": "EventLedger must model an asset lifecycle state machine (draft → in_review → approved/published → deprecated) with an approval event {asset_id, version_id, approver, decision, ts}; a queryable 'published finals' registry projection lists only approved-final versions, decoupled from working documents. Permissions gate who can emit the approval event. Reuses EventLedger projections + permissions; new: lifecycle-state field on canonical assets.",
      "scale": "small",
      "sources": "https://support.bynder.com/hc/en-us/articles/360013870280-Waiting-Room ; https://www.bynder.com/en/glossary/brand-asset-management/",
      "id": "SFR-WFRES-dam-pim-libraries-05"
    },
    {
      "capability": "Push final + source package to a client DAM / external brand portal as a governed, branded, permissioned share",
      "closes_need": "NEED-045",
      "how_incumbents_do_it": "InDesign's Package collects the document + all linked graphics + fonts + an instructions report into one folder for handoff/archive. Bynder distributes approved assets through Brand Guidelines portals and curated Collections with tailored landing pages, and via external custom-branded, password-protected share links for partners. CI HUB/Bynder connectors upload new versions back to the destination DAM to keep both sides in sync.",
      "studio_requirement": "Extend STU-LAY-058 Package (document + linked resources + fonts + report) with an outbound-publish target: emit a package_publish event {package_id, destination_repo/portal_id, included_versions[], manifest, actor, ts} that records exactly which canonical versions + source files left the workspace, producing an audit trail of client-DAM/brand-portal handoffs. Reuses Package collection; new: destination adapter + ledger provenance of the outbound push (UNVERIFIED: which external DAM/portal adapters ship in v1).",
      "scale": "small",
      "sources": "https://helpx.adobe.com/fonts/using/package-font-files.html ; https://www.bynder.com/en/products/content-experiences-for-user-community/",
      "id": "SFR-WFRES-dam-pim-libraries-06"
    },
    {
      "capability": "Explicit semantic-versioned library releases (v1.0 / v1.1, MAJOR.MINOR.PATCH) with per-release changelog and publish notes",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Figma requires a description of changes on every library publish, communicating decisions to consumers. EightShapes/SemVer discipline maps MAJOR=breaking, MINOR=backwards-compatible additions, PATCH=fixes; teams embed version numbers in library/symbol names, keep a Release History, and align design-asset versions explicitly with code counterparts rather than a vague 'latest'.",
      "studio_requirement": "A library publish must create an immutable release node {library_id, semver, changelog_text, change_list[], author, ts} in the EventLedger; semver is a first-class field, not a naming convention. Release History projection lists every version with its notes. Reuses ledger append-only release chain; new: semver field + structured per-release changelog. Improves on Figma by making the version a queryable identifier rather than embedded in the library name string.",
      "scale": "all",
      "sources": "https://help.figma.com/hc/en-us/articles/360025508373-Publish-a-library ; https://medium.com/eightshapes-llc/versioning-design-systems-48cceb5ace4d",
      "id": "SFR-WFRES-dam-pim-libraries-07"
    },
    {
      "capability": "Per-item change list at publish + per-document opt-in accept/review of library updates (no silent propagation)",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Figma's publish dialog lists every added/modified/removed component/style/variable and lets the publisher uncheck items to exclude them. Consumers do NOT get silent updates: each consuming file surfaces pending updates with a side-by-side preview, and any editor reviews and accepts or ignores each change per file, keeping instance overrides intact.",
      "studio_requirement": "Publish emits a change_list of per-item diffs (added/modified/removed) selectable at publish time. Each consuming CRDT document maintains a pending_library_updates queue; accepting an update is an explicit per-document event {consumer_doc, item_id, from_version, to_version, decision, ts}, and CRDT override state on instances survives the accept. Reuses CRDT instance/override model + ledger; this is the core anti-silent-propagation primitive.",
      "scale": "small",
      "sources": "https://help.figma.com/hc/en-us/articles/360039234193-Review-and-accept-library-updates ; https://www.figma.com/best-practices/components-styles-and-shared-libraries/",
      "id": "SFR-WFRES-dam-pim-libraries-08"
    },
    {
      "capability": "Deprecation / supersede signaling with successor pointer, EOL date, and parallel old-and-new availability",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Figma teams create a new component and mark the original deprecated (rename with [deprecated]/⚠️, hide from pickers while keeping existing instances alive, and record 'Deprecated: date / New version: X / Reason' in the description). EightShapes prescribes a staged deprecation: announce intent → publish EOL timeline (e.g. FT 3-6 months, Salesforce 18 months) → doc notices → code warnings → parallel old+new during the window → removal.",
      "studio_requirement": "A component/style/token needs a deprecation event {item_id, status=deprecated, supersedes/superseded_by, reason, eol_date, ts} as first-class metadata (not a name hack). Deprecated items stay resolvable for existing instances but are demoted/hidden in insert pickers and flagged in consuming docs with the successor and EOL. Reuses ledger metadata + library projection; superseded_by mirrors the GLOBAL topic-attribute supersede pattern.",
      "scale": "small",
      "sources": "https://medium.com/@radley/mastering-figma-deprecating-library-components-without-disrupting-your-design-team-ab4d7a192193 ; https://medium.com/eightshapes-llc/versioning-design-systems-48cceb5ace4d",
      "id": "SFR-WFRES-dam-pim-libraries-09"
    },
    {
      "capability": "Relocate a component between libraries WITHOUT breaking existing instances (keep-connection move vs publish-as-copy), with instances re-pointing only on accept",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Figma lets you move published components/component-sets between library files: cut from origin, paste into destination, then at publish choose 'Move to this file' (maintains connections to existing instances) vs 'Publish as copy' (breaks links / new components). Instances only swap to the destination library when each subscribed file accepts the update; the destination library auto-enables on accept. Only published components can be moved; the operation is irreversible except by moving back.",
      "studio_requirement": "Support a move_component event {item_id, from_library, to_library, mode: keep_connections|copy} that rewrites the canonical component's home library while preserving its stable identity so instances resolve to the new home only after each consumer accepts the pending update. Requires stable library-independent component IDs in the ledger (the identity must outlive its library membership). New primitive: library-membership as mutable metadata over an immutable component identity. Add an undo/reverse-move affordance to beat Figma's irreversibility gap.",
      "scale": "small",
      "sources": "https://help.figma.com/hc/en-us/articles/4404848314647-Move-published-components ; https://forum.figma.com/suggest-a-feature-11/possibility-to-move-main-components-between-files-libraries-23576",
      "id": "SFR-WFRES-dam-pim-libraries-10"
    },
    {
      "capability": "Swap-libraries: bulk re-point every instance in a document from one library to an equivalent library, preserving overrides",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Figma's Swap Libraries replaces all instances in the current file with matching components from another library in one bulk operation (used for theme swaps, brand migrations, or splitting a monolithic library); supported overrides (notably text) are preserved on the swapped instances. Component-level swap is also available per-instance.",
      "studio_requirement": "A document-scoped swap_library operation maps instances from library A to matching items in library B by a stable match key (component name/path), emitting one batch event and preserving CRDT override state where the target component shares the override schema. Reuses instance/override model + per-document accept queue; enables versioned library A→B migration and brand re-skinning without manual per-instance rework.",
      "scale": "small",
      "sources": "https://help.figma.com/hc/en-us/articles/4404856784663-Swap-style-and-component-libraries ; https://help.figma.com/hc/en-us/articles/360039150413-Swap-components-and-instances",
      "id": "SFR-WFRES-dam-pim-libraries-11"
    },
    {
      "capability": "Choice of library-level vs component-level versioning granularity",
      "closes_need": "NEED-057",
      "how_incumbents_do_it": "Industry practice (EightShapes, UXPin, Supernova) distinguishes library-level versioning (one shared number; a single breaking change bumps the whole library 1.4.0→2.0.0; needed when outputs cannot coexist on a page) from component-level versioning (Button 5.3.1 + Checkbox 3.1.0 mixed on one page; ideal for continuous delivery and independent consumers). Tokens are often versioned independently of components.",
      "studio_requirement": "The versioning model must support both a library-aggregate semver and per-component/per-token semver, configurable per library, so a Studio design system can pick continuous per-component evolution or batched library releases. Consumer accept/deprecation events must resolve at whichever granularity the library declares. Reuses the release-node + change-list primitives; adds a versioning_granularity setting on the library.",
      "scale": "large",
      "sources": "https://www.uxpin.com/studio/blog/component-versioning-vs-design-system-versioning/ ; https://www.supernova.io/blog/8-examples-of-versioning-in-leading-design-systems",
      "id": "SFR-WFRES-dam-pim-libraries-12"
    },
    {
      "capability": "Contribution proposal / sandboxed playground surface for consuming teams to draft changes without touching the live system",
      "closes_need": "NEED-101",
      "how_incumbents_do_it": "Figma design-system teams publish a Component Contribution Playground file and a Dev-Mode component playground (sandbox to flip variants/props without editing the source), so contributors prototype proposed additions/changes in isolation. Nathan Curtis frames the contribution as any proposal/design/code/doc by a non-core-team member, staged through propose→design→code→doc→release.",
      "studio_requirement": "Provide a contribution/playground document type linked to a target library but isolated from its published components, where a consuming team drafts a proposed component or change. A propose event {proposal_id, target_library, contributor, linked_playground_doc, ts} registers it for review. Reuses CRDT documents + library linkage; new: proposal-doc kind distinct from a published library file, so drafts never leak into consumer update queues.",
      "scale": "large",
      "sources": "https://medium.com/eightshapes-llc/defining-design-system-contributions-eb48e00e8898 ; https://forum.figma.com/t/deprecations-library-update-name-with-hidden-components-problem/21887",
      "id": "SFR-WFRES-dam-pim-libraries-13"
    },
    {
      "capability": "Request-and-review contribution loop (RFC-style) with discussion, consensus/council review, and doneness criteria before a contribution is released into the system",
      "closes_need": "NEED-101",
      "how_incumbents_do_it": "EightShapes/Nathan Curtis model contributions on the Rust/Ember RFC pattern: request form → casual discussion (Slack channel, critique session) → discussion-and-consensus exposing the proposal to the system 'council' of governing leaders and the community → polished delivery reviewed against a Doneness Matrix/checklist → release. designsystems.com adds explicit contribution rules to keep contributions in check.",
      "studio_requirement": "A contribution must move through an auditable review state machine (requested → in_discussion → in_review → accepted/rejected → released) with review events {proposal_id, reviewer/council_role, decision, comments, doneness_checklist_state, ts} on the EventLedger, and a configurable doneness checklist gating the release event that promotes the proposal into a published library release. Reuses ledger state-machine + permissions (council/system-team role); ties acceptance to the release-node primitive from NEED-057.",
      "scale": "large",
      "sources": "https://medium.com/eightshapes-llc/design-systems-support-6722b6d9a259 ; https://www.designsystems.com/keeping-design-system-contributions-in-check/",
      "id": "SFR-WFRES-dam-pim-libraries-14"
    },
    {
      "capability": "Branch / review / merge mechanic for design-system change proposals with diff",
      "closes_need": "NEED-101",
      "how_incumbents_do_it": "Figma's branching lets a contributor branch a library file, make changes in isolation, request reviews, view a diff of what changed, and merge back into the main library file — the git-like plumbing beneath the human RFC governance loop, ensuring proposed edits are reviewed before they land in the published source.",
      "studio_requirement": "Support library-file branching backed by CRDT: a branch is a divergent CRDT replica of the library document; a review surfaces the structured diff (added/modified/removed components/tokens) against main; merge emits a merge event reconciling the branches and feeding the change_list of the next release. Reuses CRDT merge semantics + change-list diff; the branch/merge is the mechanical substrate the NEED-101 governance state machine sits on top of.",
      "scale": "large",
      "sources": "https://help.figma.com/hc/en-us/articles/360063144053-Create-branches-and-merge-changes ; https://www.figma.com/best-practices/components-styles-and-shared-libraries/",
      "id": "SFR-WFRES-dam-pim-libraries-15"
    }
  ]
}
```

### [SFR-WFRES.culling-catalog] Integrated culling / rating / catalog stage (NEED-051/063, sole NOT_COVERED)

```json
{
  "capabilities": [
    {
      "capability": "Binary triage flag state (pick / reject / unflag) as a first-class per-frame attribute, distinct from graded rating",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic stores a three-value flag per photo: Pick (P, white flag), Reject (X, black flag, thumbnail grayed out), Unflag (U). Flags are a separate axis from stars/labels, meant for the first keep/kill pass. Cmd/Ctrl+Up/Down raises/lowers flag; '~' toggles Pick/Unflag. Rejected frames stay in the catalog but are visually dimmed and can later be batch-removed via Photo > Delete Rejected Photos (choose remove-from-catalog vs delete-from-disk). Capture One has no separate flag axis — it folds triage into star ratings and color tags.",
      "studio_requirement": "Studio needs a dedicated per-asset triage state enum {pick, reject, unflag} on the collection/browse surface, stored as CRDT field on the asset variant and journaled to the EventLedger so keep/kill decisions are attributable and reversible. Must be orthogonal to any rating scale and must drive a batch 'purge rejects' action that distinguishes remove-from-collection vs delete-source.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/flag-label-rate-photos.html ; https://jkost.com/blog/2024/06/applying-flags-stars-and-color-labels-in-lightroom-classic.html",
      "id": "SFR-WFRES-culling-catalog-01"
    },
    {
      "capability": "Graded 0-5 star rating scale for ranking quality across a shoot, filterable by threshold (e.g. >=4)",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Both Lightroom Classic and Capture One assign an integer 0-5 star rating via number keys 1-5 (0 clears); LR uses '[' and ']' to step. Ratings are stored non-destructively in catalog/session and can be filtered by exact, minimum, or maximum threshold. Capture One's Filters tool shows a live count of images per star level and supports filtering on multiple rating values at once. Standard practice: flags/quick pass first, then stars to rank keepers, then filter to the 4-5 star set before editing.",
      "studio_requirement": "Studio browse stage needs an ordered 0-5 rating field per asset variant with keyboard entry, and a filter predicate supporting exact / >= / <= comparison plus live per-bucket counts. Rating writes go to EventLedger; filter is a transient view over the CRDT collection, not a destructive reorder.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/flag-label-rate-photos.html ; https://support.captureone.com/hc/en-us/articles/360002743718-Rating-and-tagging",
      "id": "SFR-WFRES-culling-catalog-02"
    },
    {
      "capability": "Categorical color labels/tags (multi-hue) for grouping frames by purpose independent of rating or flag",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic offers 5 color labels (Red=6, Yellow=7, Green=8, Blue=9, Purple=menu-only) applied by shortcut; grid cells can be tinted with the label color or show a color chip. Capture One offers 7 color tags plus none, applied via keyboard/menu, each surfaced in the Filters tool with a count. Labels are a third orthogonal axis (alongside flag + stars) used to mark editing progress, client-selects, project grouping, or subject sets. Color-label names are user-editable label sets in LR.",
      "studio_requirement": "Studio needs a categorical color-tag field (5-7 slots + none) per asset variant, orthogonal to flag and rating, with grid-cell tinting and per-tag filter counts. Tag definitions (name<->hue map) should be a project-scoped, permissioned label set so a team shares one taxonomy; tag application journaled to EventLedger.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/flag-label-rate-photos.html ; https://support.captureone.com/hc/en-us/articles/360002743718-Rating-and-tagging",
      "id": "SFR-WFRES-culling-catalog-03"
    },
    {
      "capability": "Auto-advance culling: applying a rating/flag/tag automatically moves selection to the next frame for high-velocity single-key culling",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic auto-advances to the next image after a flag/star/label when Caps Lock is on or Shift is held with the shortcut. Capture One provides Select > Select Next When > Star Rated / Color Tagged, so rating or tagging one frame jumps to the next. This turns culling thousands of frames into a one-keystroke-per-frame loop without manual navigation.",
      "studio_requirement": "Studio browse stage needs a toggleable 'advance-on-decision' mode so a single rating/flag/tag keystroke commits state and moves the cursor to the next asset. Must be deterministic and observable (each advance = one EventLedger entry) so parallel model/agent culling is attributable and replayable.",
      "scale": "all",
      "sources": "https://jkost.com/blog/2024/06/applying-flags-stars-and-color-labels-in-lightroom-classic.html ; https://support.captureone.com/hc/en-us/articles/360002743718-Rating-and-tagging",
      "id": "SFR-WFRES-culling-catalog-04"
    },
    {
      "capability": "Multi-axis transient filter bar: narrow the visible set by flag + rating + label + kind + stack + metadata simultaneously without altering the collection",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic Library Filter bar has three composable filter groups: Text (indexed-field search with match rules), Attribute (Flag, Rating, Color label, Edit, Kind, Stack, etc.), and Metadata (camera/date/lens/keyword columns). Filters combine live and are non-destructive — they change what is shown, not what exists. Capture One's Filters tool is the equivalent, listing each criterion with an image count and supporting multi-value selection.",
      "studio_requirement": "Studio needs a composable, non-destructive filter layer over an asset collection combining triage flag, rating threshold, color tag, asset kind, stack state, and indexed metadata/keyword predicates, each with live counts. Filter state is a per-session view (not shared mutation) so concurrent editors filter independently over the same CRDT collection.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/finding-photos-catalog.html ; https://support.captureone.com/hc/en-us/articles/360002526358-The-Filters-tool",
      "id": "SFR-WFRES-culling-catalog-05"
    },
    {
      "capability": "Persistent rule-based virtual sets (smart collections / smart albums) that auto-populate from saved criteria",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic Smart Collections are saved metadata-rule sets (Match All/Any of criteria on rating, flag, label, keyword, capture date, filename, edit state, etc.) that auto-include any photo meeting the rules; the rule grammar is identical to the Filter bar. Capture One Smart Albums are virtual albums backed by a Filter Collection dialog of conditional rules, editable via right-click > Edit Smart Album, and persist across sessions. Both are non-destructive virtual views — an image can appear in many, source lives once.",
      "studio_requirement": "Studio needs saved, named, rule-based virtual collections (Match All/Any over rating/flag/tag/keyword/date/edit-state) that recompute membership live as assets change. Rules stored as project governance artifacts; membership is derived, never a copy. Permission scoping so team/client-facing smart sets can be shared or kept private.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/desktop/organize-photos-in-lightroom-classic/smart-collections-criteria-in-lightroom-classic.html ; https://support.captureone.com/hc/en-us/articles/360002524197-Smart-Albums",
      "id": "SFR-WFRES-culling-catalog-06"
    },
    {
      "capability": "Stacks: collapse a group of related frames (bursts, brackets, derivatives) under one representative thumbnail to de-clutter the browse grid",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic stacks group multiple photos under a single top-of-stack thumbnail with a frame count; stacks collapse/expand, can auto-stack by capture-time gap, and a Photoshop-derivative TIFF/PSD auto-stacks with its source raw. Capture One stacks are narrower — it collapses/expands Variants of one source (Image > Collapse/Expand Selected) rather than arbitrary frames; broader Lightroom-style grouping is a standing community feature request. Stack state is filterable as an attribute.",
      "studio_requirement": "Studio needs a stack primitive that groups arbitrary related assets (a burst, a bracket set, a source + its layered derivative) under one collapsible representative in the grid, with count badge, expand/collapse, auto-stack-by-time, and stack membership as a filterable attribute. Stack membership is a CRDT relation so it survives collaboration and round-trip.",
      "scale": "all",
      "sources": "https://asktimgrey.com/2022/01/17/photoshop-round-trip-from-lightroom-classic/ ; https://support.captureone.com/hc/en-us/community/posts/360014412358-Organizational-Tools-Stacks-and-Variants",
      "id": "SFR-WFRES-culling-catalog-07"
    },
    {
      "capability": "Non-destructive variants / virtual copies: multiple independent edit interpretations of a single source asset without duplicating the source file",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Capture One Variants (equivalent to Lightroom Virtual Copies) represent one original raw/JPEG/TIFF/PSD source with multiple independent adjustment recipes stored as small settings BLOBs. New Variant resets to defaults; Clone Variant copies the current recipe; neither duplicates pixels, so many variants cost a fraction of the source size. Variants can live in multiple albums at once and are compared side-by-side to pick a look. Ratings/tags can attach per-variant.",
      "studio_requirement": "Studio needs a variant model where one source asset carries N independent, named edit states (each a lightweight recipe delta, not a pixel copy), each individually ratable/taggable, referenceable from multiple collections. Natural fit for CRDT: variant = branch of edit state over shared source; EventLedger records variant create/clone/delete lineage.",
      "scale": "all",
      "sources": "https://support.captureone.com/hc/en-us/articles/360002478437-The-concept-of-variants-in-Capture-One ; https://support.captureone.com/hc/en-us/articles/360002534838-Creating-copies-of-variants",
      "id": "SFR-WFRES-culling-catalog-08"
    },
    {
      "capability": "Contact-sheet grid browse of thousands of frames via cached previews, never opening the source file",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic browses in a Grid (contact-sheet) view backed by a preview cache built at three fidelities: Standard (screen-fit, used for culling), 1:1 (full-pixel, for 100% sharpness checks), and Smart Previews (~2560px lossy DNG proxies that stand in for offline originals and even drive Develop edits). Previews are generated on import or on demand so the browse/cull/rate loop runs on cached renders, and the multi-GB Camera Raw cache accelerates render-on-the-fly. Originals are only touched at 1:1 zoom or export.",
      "studio_requirement": "Studio browse stage must render a thumbnail/contact-sheet grid over cached, multi-fidelity previews (fit-screen for cull, full-pixel for sharpness, offline-capable proxy) so rate/flag/filter of thousands of assets never loads full source documents. Preview cache is machine-local and relocatable (portability), regeneratable on demand, and proxies can stand in when sources are offline.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/lightroom-smart-previews.html ; https://www.lightroomqueen.com/lightroom-performance-previews-caches/",
      "id": "SFR-WFRES-culling-catalog-09"
    },
    {
      "capability": "Dedicated cull-comparison views: side-by-side Compare (A/B select-vs-candidate) and Survey (N-up group judging) to pick winners from near-duplicates",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic offers four Library views: Grid (G), Loupe (E, single full-frame evaluate), Compare (C, one Select vs a rotating Candidate for A/B sharpness/expression checks), and Survey (N, all selected frames tiled together to judge a group and deselect losers). Flags/stars can be applied directly from these views, so comparison and decision happen in one surface.",
      "studio_requirement": "Studio browse stage needs comparison modes beyond the grid: a single-asset loupe, a two-up Compare (select vs candidate) for near-duplicate A/B decisions, and an N-up Survey to judge a selected group together — with rating/flag/tag applicable in-view so the decision commits without leaving the comparison.",
      "scale": "all",
      "sources": "https://glensmith.co.uk/lightroom/lightroom-library-views ; https://helpx.adobe.com/lightroom-classic/help/keyboard-shortcuts.html",
      "id": "SFR-WFRES-culling-catalog-10"
    },
    {
      "capability": "Lossless round-trip to an external pixel editor that returns a layered derivative auto-imported beside the source",
      "closes_need": "NEED-051/063",
      "how_incumbents_do_it": "Lightroom Classic Photo > Edit In > Edit in Adobe Photoshop hands off, and on File > Save (not Save As) the layered TIFF/PSD derivative is automatically imported into the catalog next to the source. Capture One's Edit With command exports to an external editor and auto-imports the returned file back as a new Variant in the same folder. PSD hand-off requires Maximize Compatibility so the catalog can read the composite. This is the mechanic every studio currently bolts Lightroom/C1 onto because the pixel editor lacks it.",
      "studio_requirement": "Studio must provide a governed 'edit in external app' round-trip: hand an asset to an external pixel editor, and on save auto-ingest the returned layered file (TIFF/PSD) as a linked derivative/variant of the source with no manual re-import. Round-trip event journaled to EventLedger; derivative link is a durable CRDT relation, not a loose file next to the original.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/editing-photoshop.html ; https://support.captureone.com/hc/en-us/articles/360002638238-Making-image-adjustments-with-an-external-editor",
      "id": "SFR-WFRES-culling-catalog-11"
    },
    {
      "capability": "Re-open a returned derivative with full layer preservation (Edit Original) vs flattened copy options",
      "closes_need": "NEED-051/063",
      "how_incumbents_do_it": "When re-editing an already-returned TIFF/PSD, Lightroom Classic offers three explicit options: Edit Original (sends the derivative untouched WITH all layers/masks/text intact — the recommended path), Edit a Copy with Lightroom Adjustments (flattens + bakes LR edits into a new copy), and Edit a Copy (duplicates preserving layers). Choosing Edit Original is what keeps the layered working file editable across many round trips without a lossy flatten.",
      "studio_requirement": "Studio round-trip must expose an explicit 'edit original layered file' path that returns the derivative to the external editor with layers/masks/text/adjustment structure fully intact (no forced flatten), alongside distinct 'edit flattened copy' and 'edit layered copy' options. The layered working file stays the version-of-record derivative, editable indefinitely.",
      "scale": "all",
      "sources": "https://asktimgrey.com/2022/01/17/photoshop-round-trip-from-lightroom-classic/ ; https://helpx.adobe.com/lightroom-classic/help/editing-photoshop.html",
      "id": "SFR-WFRES-culling-catalog-12"
    },
    {
      "capability": "Stay-linked-and-stacked derivative: the returned layered file auto-stacks with its source and inherits collection membership",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic External Editing preference 'Stack With Original' auto-groups the returned TIFF/PSD in a stack with the source raw so the pair reads as one entry in the grid. Capture One returns the edit as a new Variant sitting alongside the source's other variants in the same folder. Either way the derivative stays linked to its origin and stacked, so the browse grid isn't doubled and lineage is preserved.",
      "studio_requirement": "Studio must, on round-trip return, auto-stack the derivative with its source and carry forward the source's collection/album memberships, so a raw+layered-edit pair reads as one stacked entry and the source->derivative lineage is a queryable relation. Prevents grid clutter and preserves provenance for the review/version-of-record chain.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/editing-photoshop.html ; https://support.captureone.com/hc/en-us/articles/360002627457-About-external-editing",
      "id": "SFR-WFRES-culling-catalog-13"
    },
    {
      "capability": "Explicit round-trip recipe: configurable file format, bit depth, color space, and resolution for the exchanged derivative (lossless-by-config)",
      "closes_need": "NEED-051/063",
      "how_incumbents_do_it": "Lightroom Classic External Editing preferences set the derivative File Format (TIFF/PSD), Color Space (sRGB/AdobeRGB/ProPhoto), Bit Depth (8/16-bit), Resolution, and compression. Capture One's Edit Recipe (Basic tab) sets Image Format (PSD/TIFF/JPEG), bit depth, and optional compression for the exchanged file. TIFF is favored as the open, archival, cross-app-safe container; PSD needs Maximize Compatibility. These settings are what make the exchange lossless rather than a lossy downsave.",
      "studio_requirement": "Studio round-trip needs an explicit, saved exchange recipe: format (prefer open/layered TIFF), bit depth (>=16), color space, resolution, and compression, chosen to guarantee lossless hand-off and archival stability rather than a forced flatten/downsave. Recipe is a reusable, project-scoped setting; defaults favor the version-agnostic, cross-install-safe container (ties to NEED-063 no-lossy-downsave posture).",
      "scale": "all",
      "sources": "https://www.lightroomqueen.com/community/threads/why-use-psd-vs-tif-for-photoshop-round-trip-editing.49691/ ; https://support.captureone.com/hc/en-us/articles/360002638238-Making-image-adjustments-with-an-external-editor",
      "id": "SFR-WFRES-culling-catalog-14"
    },
    {
      "capability": "Batch rejection cleanup: promote triage rejects to removal in one governed action with catalog-vs-disk choice",
      "closes_need": "NEED-051",
      "how_incumbents_do_it": "Lightroom Classic Photo > Delete Rejected Photos gathers everything flagged Reject and prompts whether to Remove (from catalog, leave file on disk) or Delete from Disk. The Refine Photos command (Ctrl/Cmd+Alt+R) runs iterative demotion passes — flagged stay, unflagged become rejects — to converge on keepers over multiple cull rounds before the final purge.",
      "studio_requirement": "Studio needs a governed batch action that collects all reject-flagged assets and resolves them in one step with an explicit remove-from-collection vs delete-source choice, plus an optional iterative 'refine' demotion pass for multi-round culling. Destructive purge routes through the no-force-delete / backup-first posture and is fully journaled/reversible via EventLedger.",
      "scale": "all",
      "sources": "https://helpx.adobe.com/lightroom-classic/help/flag-label-rate-photos.html ; https://jkost.com/blog/2024/06/applying-flags-stars-and-color-labels-in-lightroom-classic.html",
      "id": "SFR-WFRES-culling-catalog-15"
    }
  ]
}
```
