---
file_id: studio-app-feature-research-workflow-needs-register
topic_id: SFR-WNEED
title: "Workflow Needs Register (2026-07-20 scenario audit)"
status: draft
summary: "Unified NON-AI workflow-level needs from 82 pro scenarios (large-team/small-team/solo + cross-app), each with corpus/spec coverage verdict. Surfaces team-scale coordination needs a feature-list audit cannot see."
sources: 113
updated_at: "2026-07-20"
---


## [SFR-WNEED] Workflow Needs Register

### [SFR-WNEED.summary] Scenario Audit Summary

```json
{
  "audit_date": "2026-07-20",
  "scope": "NON_AI workflow-level needs surfaced by 82 professional scenarios across large-team / small-team / solo scale + cross-app pipelines, for all 5 apps",
  "method": "Scenario research per app x team-scale -> unified deduped needs registry -> each need verified against the research corpus (00-57) and Master Spec module 14 incl. kernel primitives (CRDT/EventLedger/sessions).",
  "verdict_meaning": {
    "COVERED": "an explicit feature row/card/spec section addresses the need",
    "PARTIALLY_COVERED": "adjacent or raw-snapshot-only coverage",
    "NOT_COVERED": "corpus + spec searches came up empty",
    "UNVERIFIED": "coverage verifier did not return a result for this need id"
  },
  "total_needs": 113,
  "by_verdict": {
    "COVERED": 72,
    "PARTIALLY_COVERED": 40,
    "NOT_COVERED": 1
  },
  "by_criticality": {
    "BLOCKER": 74,
    "IMPORTANT": 34,
    "NICE_TO_HAVE": 5
  },
  "scoreboard_domain_x_verdict": {
    "architecture": {
      "COVERED": 1
    },
    "asset-management": {
      "COVERED": 3,
      "PARTIALLY_COVERED": 4,
      "NOT_COVERED": 1
    },
    "automation": {
      "COVERED": 4,
      "PARTIALLY_COVERED": 1
    },
    "business-model": {
      "PARTIALLY_COVERED": 1
    },
    "client-deliverables": {
      "PARTIALLY_COVERED": 1
    },
    "collaboration": {
      "COVERED": 7,
      "PARTIALLY_COVERED": 6
    },
    "color": {
      "COVERED": 3
    },
    "editing-core": {
      "COVERED": 3
    },
    "export": {
      "COVERED": 4,
      "PARTIALLY_COVERED": 1
    },
    "file-organization": {
      "COVERED": 1,
      "PARTIALLY_COVERED": 1
    },
    "handoff": {
      "COVERED": 12,
      "PARTIALLY_COVERED": 3
    },
    "layout": {
      "COVERED": 1
    },
    "localization": {
      "PARTIALLY_COVERED": 1
    },
    "migration": {
      "PARTIALLY_COVERED": 1
    },
    "multi-doc-consistency": {
      "COVERED": 3
    },
    "performance": {
      "PARTIALLY_COVERED": 4,
      "COVERED": 1
    },
    "photo-editing": {
      "COVERED": 1
    },
    "precision-drawing": {
      "COVERED": 1
    },
    "prepress": {
      "COVERED": 8,
      "PARTIALLY_COVERED": 4
    },
    "reliability": {
      "COVERED": 1
    },
    "review-approval": {
      "COVERED": 4,
      "PARTIALLY_COVERED": 5
    },
    "templates": {
      "COVERED": 2,
      "PARTIALLY_COVERED": 3
    },
    "typography": {
      "COVERED": 8
    },
    "versioning": {
      "COVERED": 4,
      "PARTIALLY_COVERED": 3
    },
    "workflow-state": {
      "PARTIALLY_COVERED": 1
    }
  },
  "uncovered_or_partial_need_ids": [
    "NEED-005",
    "NEED-009",
    "NEED-017",
    "NEED-018",
    "NEED-020",
    "NEED-022",
    "NEED-026",
    "NEED-035",
    "NEED-045",
    "NEED-047",
    "NEED-051",
    "NEED-057",
    "NEED-063",
    "NEED-066",
    "NEED-071",
    "NEED-075",
    "NEED-078",
    "NEED-081",
    "NEED-083",
    "NEED-084",
    "NEED-086",
    "NEED-088",
    "NEED-089",
    "NEED-090",
    "NEED-091",
    "NEED-094",
    "NEED-095",
    "NEED-098",
    "NEED-100",
    "NEED-101",
    "NEED-102",
    "NEED-103",
    "NEED-104",
    "NEED-105",
    "NEED-106",
    "NEED-107",
    "NEED-108",
    "NEED-109",
    "NEED-110",
    "NEED-111",
    "NEED-112"
  ],
  "systemic_holes": [
    "review / approval / sign-off / version-of-record provenance chain (NEED-005/017/022)",
    "production workflow-state / project-management surface (NEED-009)",
    "integrated culling / rating / catalog stage (NEED-051, sole NOT_COVERED)",
    "external asset ecosystem: DAM/PIM round-trip + governed versioned library releases (NEED-045/057)",
    "production-volume performance as explicit NFRs (NEED-026/047/071)"
  ],
  "architecture_implication": "Every fully-COVERED need is single-artifact craft; every hole is a team-scale coordination surface. Studio needs a scale-adaptive review/approval + workflow-state + permission triad layered on the CRDT/EventLedger substrate, invisible-for-solo but governable-for-large."
}
```

### [SFR-WNEED.authority] Authority

Reference/provenance only. Complements the feature-parity register (58): 58 covers 'can one person do X to one document', this covers 'can a team of a given scale run a professional production workflow'. Inputs for the WP-KERNEL-STUDIO refinement and Section-14 coverage; not product authority.

### [SFR-WNEED.needs] Needs

```json
{
  "needs": [
    {
      "id": "NEED-001",
      "need": "Shared team/brand asset libraries (colors, type styles, logos, approved assets, components) referenced live by documents across the whole suite, cloud-synced, with view/edit permissions and automatic update propagation to every consuming document",
      "domain": "asset-management",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: Retail campaign rollout (shared team libraries update across hundreds of adaptations); affinity/small: Brand identity design (top-voted gap vs CC Libraries)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14-studio-creative-suite.md 14.10 §5 [STU-DS-019/019a] library publish/subscribe over StudioComponent/StudioStyleRegistry/StudioVariableCollection with per-item change lists + publish notes (accept-updates propagation); line 383 'Studio asset library' explicitly replaces CC-Libraries-class linked assets; [STU-DS-005] asset-browser lists local+enabled-library components; [STU-COL-009] swatch/palette interchange + import from another document. Research: 03-indesign-feature-map.md indesign.cc_libraries; 38-figma domain ledger team/brand libraries; 56-integration-architecture Loom as the single library home (brushes/styles/palettes/components/export recipes/placed assets)."
    },
    {
      "id": "NEED-002",
      "need": "Batch multi-format/multi-scale export driven by named artboards/slices with per-item export presets, deterministic filename/folder templating, and one-operation export of whole size families",
      "domain": "export",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/solo: Brand identity full logo-package delivery; figma/solo: Recurring social campaign production (60+ assets/month)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 [STU-RAS-006] multiple named artboards each with own export path without forking; StudioExportRecipe primitive (05-studio-primitive-map.md, 10-studio-command-contracts.md 'Render Export Recipe', 12-cross-app-parity-matrix.md 'Export recipes with deterministic artifacts'). Research: 37-affinity domain ledger 'Export Persona behavior is a strong model for local-first batch asset export recipes'/export slices; 01-photoshop-feature-map.md Export As/Quick Export sizes+presets; 38-figma 'batch export recipes' for bulk brand output; 36-illustrator artboards as first-class regions with export recipes. Note: per-item preset + deterministic filename/folder templating implied via export-recipe determinism/slice naming, not enumerated field-by-field."
    },
    {
      "id": "NEED-003",
      "need": "Native version history with named checkpoints/milestones per round, cheap revert/restore, and per-story/sub-document rollback — replacing _FINAL2 filename versioning",
      "domain": "versioning",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/large: Multi-channel campaign production (named milestones and rollback); figma/small: Brand identity rounds (rejected direction recoverable months later)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.19 [STU-HIS-001..004] per-file history stack (StudioHistoryEntry), undo, revert-of-undo, content-addressed snapshot revert; [STU-LAY-013] document-states surface listing session edit states with jump-to-any-recorded-state beyond linear undo. Research: 55-figma-deep-feature-delta.md on-demand named versions with descriptions + dual-checkpoint reversible restore + retention; 52-illustrator File>Version History; 56-integration-architecture history-undo subtopic. Named milestones = Figma named-versions research + document-states; per-story sub-doc rollback is per-file granularity."
    },
    {
      "id": "NEED-004",
      "need": "In-context anchored review: region/frame-pinned threaded comments with @mentions, resolve state, round filtering, comment log export, and annotations surfacing inside the editing context on every document type — no third-party proofing silo",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: E-commerce retouching pipeline (art director reviews without opening working file); cross-app/mixed: Cross-scale review/approval backbone",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.6 §10 [STU-LAY-066]: native review surface — 'comments, annotations, and markup anchored to layout positions' plus document-states/history — is a first-class native Studio capability backed by kernel CRDT collaboration (hosted share-for-review with pin/highlight/strike/insert/reply comments is an optional adapter). 19-studio-local-first-rust-posture.md studio_collaboration engine owns [comments, attribution]; 03-indesign share_for_review (reply/resolve); 23-figma-leaf-index 'Guide to comments'. Note: @mentions, resolve-state, round filtering, and comment-log export are not individually enumerated in the spec, but the anchored threaded-review section is explicit."
    },
    {
      "id": "NEED-005",
      "need": "Formal approval workflow: multi-stage routing with named approvers, roles, deadlines, approve/reject states, sign-off records tied to frozen versions, and a defensible audit trail of who approved what and when",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Agency client website (recorded sign-off per round on frozen version); illustrator/large: CPG packaging (regulatory/legal/brand parallel routing)",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent only: Spec 14 §14.18 Propose-Work + [STU-COL/PromotionGate] operator-approval path, 56-integration-architecture operator-approval-surface routes to DCC Approval Inbox with OperatorApprovalEvidence + EventLedger audit (who/what/when) and no self-approval — but this is the model-edit-proposal gate, not a creative sign-off workflow. Corpus-wide grep for approval|sign-off|approver|deadline|frozen version returned no multi-stage human approval routing with named approvers, roles, deadlines, approve/reject states, or sign-off bound to a frozen version. No dedicated review-approval routing feature."
    },
    {
      "id": "NEED-006",
      "need": "Full ICC color management: working spaces, profile assign/convert with rendering intents, soft proofing against arbitrary vendor/press/substrate profiles, gamut warnings, and calibrated wide-gamut display support",
      "domain": "color",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Magazine prepress handoff; figma/solo: Print collateral job (RGB-only pipelines get files rejected)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.8 §1/§4 [STU-COL-001] StudioColorProfile ICC/OCIO working-space binding + document working-space profiles; [STU-COL-004] assign/convert to profile; [STU-COL-022] soft-proof against a device profile with rendering intent without converting values; [STU-COL-023] perceptual/relative/saturation/absolute intents + black-point compensation; [STU-COL-024] gamut warning with in-gamut substitute; [STU-COL-026] native ColorEngine CMM. Research: 09-affinity-desktop-delta.md Soft Proof adjustment + Color management; 01-photoshop OCIO/ACES; 05-studio-primitive-map StudioColorPipeline diagnostics [profile_trace, gamut_warning]."
    },
    {
      "id": "NEED-007",
      "need": "Spot/Pantone color system: named spot swatches including technical inks (die cut, crease, white, varnish, foil), overprint control, separations/ink-plate preview, ink-count visibility, all preserved as separate plates through export",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/small: Consumer packaging (vendor RIPs key off exact spot names); cross-app/mixed: Packaging studio spot-color prepress",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.8 §7 [STU-COL-020] native first-class spot color; [STU-COL-007/034] StudioSwatch(spot) with optional Lab def that separates as its own plate regardless of display alternate + tint; [STU-COL-035] mixed-ink/mixed-ink-group; [STU-COL-025] separations preview per-plate on/off + ink-limit + per-ink coverage; [STU-COL-026] overprint (fill/stroke/gap) + preview; [STU-COL-027] ink manager. Research: 03-indesign separations_inks_overprint; 07-indesign-leaf-index about-color-separations/overprinting/ink-trapping; 36-illustrator spot/process. Technical inks (die/varnish/foil/white) modeled as named spot plates; not individually named."
    },
    {
      "id": "NEED-008",
      "need": "Press-ready PDF/X export (X-1a/X-3/X-4) with bleed/trim boxes, printer's marks, ICC output intents, importable vendor joboptions, layer/plate control — trusted by commercial RIPs without external verification",
      "domain": "export",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: Packaging print production; affinity/solo: Book layout for POD and offset vendors",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.6 §9 export table lines 1097-1111: PDF General preset/standard (PDF/X) + Output PDF/X output intent 'at least PDF/X-1a, PDF/X-3, PDF/X-4 with embedded output intents through built-in presets' + ink-manager access; line 2707 Prepress PDF/X row 'color conversion, marks, bleed preserved to the chosen PDF/X standard'; 'Set printer's marks' + live preflight [STU-LAY-055]. Research: 52-illustrator-deep-feature-delta preset families incl PDF/X-1a/X-3/X-4 + trim/registration marks/color bars/per-edge bleed; 54-affinity PDF/X-1a:2003/X-3:2003/X-4; 51-photoshop PDF/X. Importable joboptions = named reusable PDF presets (line 2759)."
    },
    {
      "id": "NEED-009",
      "need": "Production workflow status per story/asset/deliverable (shot/draft/in-review/approved/delivered) with deliverables-matrix visibility, assignment routing, QC state machines, and bottleneck counts — replacing parallel spreadsheets",
      "domain": "workflow-state",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: Magazine editorial workflow (per-story status visible to whole team); affinity/small: 40-asset campaign approval tracking currently in a PM spreadsheet",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent only: Spec 14 [STU-LAY-042] Books 'show per-document status'; line 1140 native file-based assignment/check-in-check-out copy-editing workflow; document metadata status field (56-integration-architecture model-visibility 'document metadata: status'). 03-indesign incopy_workflows assignment files; 11-provider-posture-map create-and-manage-assignments. No explicit production-status state machine (shot/draft/in-review/approved/delivered), deliverables matrix, QC state machine, assignment routing dashboard, or bottleneck counts feature. Grep for deliverable|kanban|production status|QC returned only editorial assignment + book status."
    },
    {
      "id": "NEED-010",
      "need": "Package/collect-for-output: one operation bundling document + all linked assets + fonts + report into a self-contained portable folder, with cold-open verification that no external references remain",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/solo: Agency overflow contract (opens identically on another machine); illustrator/large: CPG packaging vendor exchange",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.6 §9 prepress/preflight+package flow (StudioPreflightProfile) with package/export receipts; 03-indesign-feature-map.md indesign.package_output 'Package for output — Collect InDesign file, linked graphics, fonts, and report into a handoff folder after preflight'; 05-studio-primitive-map.md StudioPreflightProfile diagnostics [missing_fonts, missing_links, package_manifest] + verification package_manifest_tests. Cold-open verification = 04-affinity-leaf-index Creating/Opening/Resaving packages leaves + package_manifest; live preflight surfaces missing dependencies ([STU-LAY-055/1066])."
    },
    {
      "id": "NEED-011",
      "need": "Version compare for reviewers and producers: side-by-side and overlay/difference views between any two versions with change attribution, round-over-round change visibility, and changed-asset delta detection since last delivery",
      "domain": "versioning",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "cross-app/mixed: Review/approval backbone; indesign/small: Annual report (verify what changed instead of re-proofreading 80 pages)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 55-figma-deep-feature-delta.md line 2346: 'diff a frame's current state against earlier versions side-by-side or overlaid to see what changed since last implementation' (explicit side-by-side + overlay diff). 56-integration-architecture semantic-diff view (before/after tree hashes, changed-node lists, structured render receipts) + history-query 'inspect an entry's semantic diff'. Spec 14 §14.16 model-visible history/semantic-diff query. Change attribution via EventLedger KernelActor per-actor identity. Round-over-round/changed-asset delta = version diff + link/preflight delta."
    },
    {
      "id": "NEED-012",
      "need": "Linked reusable components (smart-object/symbol class): one source placed many times within and across documents, edits propagate to all instances, stale-instance indicators, non-destructive embedding of vector art in raster docs, instances survive cross-file reuse",
      "domain": "asset-management",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Campaign key visual versioning (one packshot edit propagates to 40+ adaptations); affinity/solo: Multi-channel campaign symbols",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 §14.10 §1 [STU-DS-001] StudioComponent subsumes Figma component AND Illustrator symbol into one reusable-definition primitive; [STU-DS-003] StudioComponentInstance is a live reference 'not a copy'; [STU-DS-016] instance inherits every change to its main component except local overrides (edits propagate to all instances); [STU-DS-003a] instances survive component deletion + restore command. Non-destructive vector-in-raster embedding = placed_asset container (14.4 §3). Research: 01-photoshop smart_objects; 04-affinity Symbols panel; 20-illustrator relink all instances; 13-layer-graph vertical slice unifies Smart Objects/Affinity linked files/InDesign links. Stale-instance indicator via override/update model."
    },
    {
      "id": "NEED-013",
      "need": "Governed reusable templates: vendor-spec document presets (bleed class, print area, max inks, hardware safe zones), locked template layers, recurring-issue/season duplication with structure intact, controlled template updates and broken-template detection",
      "domain": "templates",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: Retail POS rollout (governed template library per output format); indesign/solo: Recurring monthly issue templates",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research: 04-affinity-leaf-index.md Document templates (photo/designer/publisher) + master pages (about/creating/editing/applying); 02-affinity Smart Master Pages and Shared Text Styles; 01-photoshop data-driven template variables. Spec 14 [STU-LAY-036/038/039] object/paragraph/table styles + master/parent pages [STU-LAY-008]; locked-layer denial (14-feature-use-card-schema.md 'Locked target layer denies', 13-layer-graph verify no mutation on locked layer); vendor-spec document presets (bleed/print-area/inks) = StudioPreflightProfile + document setup. Governed-update/broken-template detection not explicitly a feature but template+master-page+locked-layer primitives are explicit."
    },
    {
      "id": "NEED-014",
      "need": "Linked (not embedded) external asset model: live modified/missing status, bulk relink by folder, low-res/high-res swap, path-independent portable links that survive moves and machine changes, edit-original round-trip to the source editor",
      "domain": "asset-management",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Campaign collateral (bulk re-link is routine cleanup); cross-app/mixed: Retail catalog PS->AI->ID pipeline",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 line 2863 'linked and embedded placed assets, relink/embed' mapped to StudioLayer(placed_asset); 14.4 §3 placed-asset non-destructive container with edit/replace; [STU-LAY-055] live preflight reports missing links; export-includes-dependency contract. Research: 03-indesign-feature-map.md indesign.links_panel 'Track placed graphics/files, link status, instances, nested dependencies, relinking, production handoff'; 20-illustrator 'relink all instances, placed files'; 10-studio-command-contracts verification [link_manifest_test, missing_link_preflight, export_includes_dependency]; 14-feature-use-card-schema path-independent link recovery. Bulk-relink-by-folder / low-res proxy swap implied by relink but not enumerated."
    },
    {
      "id": "NEED-015",
      "need": "SKU-family/campaign-wide propagation: a master change (logo tweak, legal copy, background correction) fans out across dozens of sibling documents/deliverables via shared references instead of manual per-file re-editing; cross-document find-and-replace as minimum",
      "domain": "multi-doc-consistency",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: 40-SKU flavor family; affinity/small: Multi-channel campaign shared linked master",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 [STU-COL-033] global-swatch propagation: editing a global/spot swatch live-updates every object/text/gradient/pattern referencing it in ONE deterministic operation (master change fans out); [STU-VEC-051/067] pattern-definition edit updates every referencing object; [STU-DS-016] component change propagates to all instances; [STU-COL-015] find/replace color across objects+text. Research: 03-indesign find_change (text/format/GREP) + data_merge (CSV records into layout variants); 01-photoshop data-driven-graphics multiple document variants; 07-indesign-leaf-index automation-and-scripting 'link-and-update-content-across-documents' (explicit cross-document propagation). Cross-doc find/replace minimum + shared-reference fan-out both present."
    },
    {
      "id": "NEED-016",
      "need": "Configurable preflight profiles run continuously and at export (missing fonts/links, resolution floors, RGB-in-CMYK, overset text, bleed violations, barcode scale, minimum text size), shareable as per-vendor profile files, batch-runnable across whole variant sets, with vendor-preflight parity",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Magazine issue (preflight during work, not only at export); illustrator/solo: Packaging preflight automation before vendor submission",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "53-indesign-deep-feature-delta.md [SFR-INDESIGN-DEEP-DELTA.output-and-prepress]: 'Live preflight engine' (continuous validation against active profile, status-bar error count, per-error fix info, page-range limitable), 'Preflight profile rule categories' (General, Links missing/modified, Color blend-space/plates/color-spaces/overprint incl RGB-in-CMYK, Images/Objects resolution/transparency/stroke, Text missing-fonts/overset, Document page-size/bleed/slug), and 'Preflight profile management and embedding' (create/export/import as IDPP files, embed in document so recipients preflight with same rules = shareable per-vendor profiles). 03-indesign-feature-map.md indesign.preflight + indesign.package_output + 'Book-wide output' (preflights whole book = batch across variant sets); implementation note line 101 requires machine-readable missing-font/link/overset/print-readiness receipts. Spec 14-studio-creative-suite.md [STU-LAY-057] overset as preflight rule, [STU-LAY-067] preflight runs headless/quiet. Barcode-scale and minimum-text-size specific checks are not named (beyond even Adobe native; Pitstop-class), but the configurable-profile framework + continuous+at-export + shareable IDPP + book-wide batch is explicit."
    },
    {
      "id": "NEED-017",
      "need": "Approved-version-of-record provenance: explicit lineage per deliverable (v1..vN mapped to feedback rounds), branch-and-merge of comps, and proof of which exact version was approved/printed/on shelf — for reprints, recalls, and contract defense",
      "domain": "versioning",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Entertainment key art (combine elements of v012 and v027); indesign/large: Packaging (prove which file went to plate)",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent kernel provenance only. Spec 14.19 [STU-HIS-001..004]: per-file history stack, every promoted batch = StudioHistoryEntry backed by EventLedger events, content-addressed snapshots. [STU-LAY-066] names a 'document-states/history surface beyond linear undo.' 56-studio-handshake-integration-architecture.md: OperatorApprovalEvidence, PromotionGate, DCC Approval Inbox, STUDIO_EDIT_PROMOTED events = approval provenance for edits. But NO explicit deliverable-level version-of-record (named v1..vN mapped to feedback rounds), NO proof of which exact version was approved/printed/on-shelf, and branch-and-merge of comps is explicitly deferred in file 56 ('deep redo branch tree is the genuinely expensive part'). Searches: 'version of record','branch-and-merge','which version went to plate' return no feature card."
    },
    {
      "id": "NEED-018",
      "need": "Long-horizon archival: self-contained packaged archives keyed to jobs/SKUs/clients with fonts and links resolvable, plus long-term format stability or open-format export so masters reopen identically years later on any machine",
      "domain": "versioning",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: Brand identity program (future team on different software must reopen and edit); photoshop/large: Key art re-releases years later",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Spec 14 [STU-LAY-058] Package MUST collect document + linked resources + fonts + report into a portable folder 'for handoff to a printer or archive' (self-contained archive with fonts/links resolvable), operating over a book as well as single doc. Open-format export via 14.13 matrix: .idml/.svg/.psd/.ai round-trip + PDF/PDF-X + .ase swatches + structured XML [STU-LAY-064] (long-term/open-format stability). 03-indesign-feature-map indesign.package_output. But NO explicit long-horizon archival system keyed to jobs/SKUs/clients and NO guaranteed 'reopen identically years later on any machine' capability as a named feature; package is a handoff/archive folder, not a durable job-keyed archival vault. Searches for 'archive keyed to SKU/client' empty."
    },
    {
      "id": "NEED-019",
      "need": "Data merge / data-driven layout: CSV/spreadsheet/PIM/ERP binding to text and image frames, repeating record grids, multiple record-layout variants per page, re-sync on data change that preserves manual overrides, per-record error reporting, and data-diff verification against source before lock",
      "domain": "automation",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: 600-page retail catalog; affinity/solo: Business cards, badges, catalog pages from CSV",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.14 [STU-AUT-014/015] native data-driven graphics: typed StudioVariable bindings (layer visibility, pixel/placed-asset replacement, text-string replacement) + dataset binding imported from delimited text/CSV/XML (PIM/ERP export = CSV), row preview before commit, batch expansion (one doc/asset per row on batch runner with per-file receipts = per-record error reporting). 53-indesign-deep-feature-delta.md 'Data Merge panel' (CSV/TXT source, @-prefixed columns to text AND image placeholders, preview records, generate merged docs), 'Data merge multiple-records layout' (rows/columns-first grids, margins, spacing, per-record placement = repeating record grids + multiple record-layout variants), 'Data merge direct PDF export', 'Data merge QR fields'. Advanced re-sync-preserving-overrides and data-diff-verification-before-lock are not explicit, but the core CSV/spreadsheet binding to text+image frames, repeating grids, and multiple record layouts are explicit feature rows."
    },
    {
      "id": "NEED-020",
      "need": "Localization/versioning pipeline: copy-deck-driven text swap per market, locale-by-format deliverable matrix tracking, open text/structure interchange for CAT tools, cross-edition change propagation tracking, and localized text-in-graphics swap management",
      "domain": "localization",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: 20-language brochure/manual round-trip; photoshop/large: Campaign multi-market adaptation",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Building blocks only, no localization pipeline. Spec 14 [STU-LAY-064] structured-XML tagging + tagged-text plain-format round-trip + schema validation (open text/structure interchange for CAT tools); [STU-AUT-014] text-string-replacement variables + dataset (copy-deck-driven swap); [STU-LAY-049] conditional text with named conditions (language visibility); [STU-LAY-011/019] alternate layouts + linked stories with update-state propagation (cross-edition propagation primitive). But NO explicit localization/versioning pipeline: no copy-deck-per-market binding feature, no locale-by-format deliverable matrix tracking, no cross-edition change-propagation tracking, no localized text-in-graphics swap management as named capability. Corpus grep 'localiz|CAT tool|copy-deck|multilingual' hits only incidental Affinity UI-localization/settings references, not a workflow."
    },
    {
      "id": "NEED-021",
      "need": "Per-destination/channel export presets enforcing platform and vendor specs: pixel dims, color space, format, compression, file-size caps with automatic targeting, and compliance with platform validators (ad networks, marketplaces, KDP/IngramSpark)",
      "domain": "export",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/solo: Marketplace seller export presets (sRGB, dims, background check); illustrator/large: Per-vendor output spec profiles",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.13 [STU-IO-010] unified StudioExportRecipe (one primitive, many recipes): per-document/layer/artboard export, export-for-screens/slices with per-region format+scale sets and multi-scale variants (1x/2x/3x with suffix tokens), legacy web optimizer (GIF/JPEG/PNG-8/24 settings, dither, matte, image-size), PDF export with named reusable presets incl PDF/X standard, print output with color/marks/bleed. [STU-IO-014] each recipe is a stable-id model-steerable command. 45-source-distilled-tool-registry + 46-file-format-compatibility-registry document per-format targets. Per-destination presets enforcing pixel dims/color space/format/compression are explicit. File-size-cap auto-targeting and platform-validator compliance (ad-network/KDP/IngramSpark) are not explicitly named, but the per-destination reusable-preset export surface is a first-class primitive."
    },
    {
      "id": "NEED-022",
      "need": "Vendor proof/correction round-trip: export to converter/printer spec, ingest their annotated proof rejects, track which corrections were applied per round, tie resubmission to a new version through to final OK-to-print",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/small: Packaging vendor preflight exchange tracked to sign-off; illustrator/solo: Printer's annotated proof feedback loop",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Partial. Spec 14 [STU-LAY-066] native review surface (comments/annotations/markup anchored to layout positions) + line 1143 'Import of external review comments back into the layout, anchored to positions, MUST be supported through the interchange path' (ingest annotated proof rejects). Export-to-printer-spec via StudioExportRecipe PDF/X [STU-IO-010]. Approval/version tie via PromotionGate + OperatorApprovalEvidence + DCC Approval Inbox (56). But NO explicit vendor-proof correction-round-tracking feature (which corrections applied per round), NO resubmission-tied-to-new-version-through-to-OK-to-print sign-off workflow. Same version-of-record gap as NEED-017."
    },
    {
      "id": "NEED-023",
      "need": "Font fidelity and management: exact font versions travel/embed with jobs so copyfit is identical on the recipient's machine, team-wide licensed font sync, substitution reporting on import, licensing-aware collection in packages and embedding checks at export",
      "domain": "typography",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/solo: Agency overflow (line breaks identical on recipient machine); affinity/large: Team font management prevents per-artist missing-font errors",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.7 §9 Font Management: [STU-TYP-028] font sources incl OS-installed + Studio-managed local font library + org-shared/project-embedded local font set (team-wide licensed font sync, de-clouded); [STU-TYP-029] font picker preview/search/filter; [STU-TYP-030] missing-font handling on open MUST flag layers/stories using unavailable fonts + offer bulk replace-font mapping + document-wide find/replace-fonts (substitution reporting on import). [STU-LAY-058] Package collects fonts 'subject to font-licensing' (licensing-aware collection). Export options line 1101 font-subsetting threshold + line 1082 font-download policy (embedding checks at export). Exact-font-version pinning for identical copyfit is served by local-font-library + package + missing-font mapping; core font fidelity/management explicit."
    },
    {
      "id": "NEED-024",
      "need": "Multi-size/format derivation from one master: artboard size matrices, alternate/liquid layouts, master/variant governed relationships (not 40 orphan files), smart reflow/background extension, and content-change propagation across all aspect-ratio variants including batch resize/crop of placed imagery",
      "domain": "multi-doc-consistency",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: One key visual to 40+ sizes; figma/solo: Multi-size template sets for campaign creatives",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 [STU-RAS-006] multiple named StudioArtboard containers in one document each with own pixel dims + export path (no forking = not 40 orphan files); [STU-LAY-011] Alternate layouts (multiple named page-size/orientation variants coexist in one StudioDocument, side-by-side, stories linked back to source so edits propagate = master/variant governed) + flex/container layout; [STU-LAY-009/010] liquid rules + Adjust Layout (smart reflow on size change); [STU-LAY-019] linked stories with update-state/auto-update; [STU-VEC-037] Global Edit across artboards + content-change propagation; content-aware scale (14.4) = background extension; [STU-IO-010] export-for-screens multi-scale derivation. Multi-size derivation from one master with governed relationships is explicit."
    },
    {
      "id": "NEED-025",
      "need": "Freelancer/vendor working-file round-trip: package out with fonts/links/style guide/golden samples, track what is out and returned, clean reintegration of returned packages with automatic re-linking, and element hot-swap into composites without rebuilding masks",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Freelance overflow exchange; photoshop/small: Remote freelancer batch packaging with QC tracking",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14 [STU-LAY-058] Package collects document + linked resources + fonts + report into portable folder for handoff (package out with fonts/links); [STU-LAY-066] 'File-based assignment/check-in-check-out copy-editing workflow | Native' (track what is out and returned); [STU-RAS-012] placed_asset link health (up-to-date/modified/missing) as inspectable state with update-all command (clean reintegration + automatic re-linking); [STU-FX-011a] editing placed source re-flows the same effect stack over updated content + [STU-RAS-012] embedded/linked conversion (element hot-swap into composites without rebuilding masks). Style-guide/golden-samples inclusion in the package is not explicitly enumerated, but the freelancer/vendor working-file round-trip mechanics (package-out, check-in/check-out tracking, relink, hot-swap) are explicit."
    },
    {
      "id": "NEED-026",
      "need": "Production-volume performance: hundreds of pages, thousands of linked high-res images, proxy/low-res preview modes, lazy link loading, fast open/save/scroll/export and incremental save at deadline crunch",
      "domain": "performance",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "indesign",
        "illustrator",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Catalog studios split documents to dodge slowdown; cross-app/mixed: 500+ page retail catalog",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent pieces, no consolidated production-volume envelope. Corpus: 53-indesign-deep-feature-delta 'View > Display Performance' (Fast/Typical/High-Quality proxy display modes + per-object overrides = proxy/low-res preview), 07-indesign-leaf-index 'GPU performance in InDesign', 06-photoshop-leaf-index/51-deep-delta 'Save large documents / Large Document Format (PSB)', 05-studio-primitive-map proxy_render_cache/proxy artifact. Spec 14 [STU-FX-012b] tiled/progressive/region-of-interest re-rendering for bounded large-document interactive preview; [STU-FX-011a]/[STU-RAS-012] rendered proxy for placed assets + link health. But NO explicit production-volume performance requirement covering thousands of linked high-res images, lazy link loading, incremental save at deadline, or fast open/save/scroll/export at hundreds-of-pages scale as a named capability/requirement."
    },
    {
      "id": "NEED-027",
      "need": "High-fidelity editable interchange with the incumbent ecosystem: IDML/PSD/AI/SVG import AND export preserving styles, masters, spot names, layers and text flow, so mixed-tool collaborators, legacy takeovers, and screen-design tools round-trip without a one-way door",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/small: No IDML export blocks handoff to InDesign-based printers; cross-app/mixed: Adobe-to-Affinity migration survival",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.13 import/export matrix [STU-IO-006/010] with explicit bidirectional round-trip (I·EP·X·RT): .psd (layers/groups/masks/adjustment-layers/linked-embedded/blend modes preserved), .ai (paths/artboards/text/gradients; PDF-compatible; export via PDF-compatible), .svg/.svgz (paths/text/gradients/symbols preserved), .idml (pages/spreads/frames/text stories/styles = high-fidelity editable layout interchange), plus .fig local-copy round-trip. [STU-LAY-064] structured interchange (layout markup export/open for cross-version exchange, tagged-text round-trip, structured-XML with tag-to-style mapping). [STU-IO-005] preservation-blob re-emit so round-trip does not silently drop structure. 46-file-format-compatibility-registry.md 410 records. Two-way IDML/PSD/AI/SVG interchange preserving styles/masters/spot/layers/text-flow is explicit."
    },
    {
      "id": "NEED-028",
      "need": "Recordable actions/macros with headless batch processing over folders (droplet-class), per-file error reporting, conditional logic beyond simple recording, and batch operations across many files (swap a logo, re-export 300 PDFs)",
      "domain": "automation",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Thousands of SKUs/week headless batch; affinity/small: Current macro+batch system insufficient at volume",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.14: [STU-AUT-010/011] native action/macro system (record/stop/play command stream into named StudioMacro, per-step enable/exclude/reorder, per-step modal pause, insert path/tool-stroke steps, Conditional step branching on document condition, Conditional mode change, event-bound triggers, macro library import/export = conditional logic beyond simple recording); [STU-AUT-012/013] native batch runner on kernel Job Runtime running headless/bounded/quiet with per-file receipts + error log (headless batch over folders + per-file error reporting), portable re-runnable job artifact ('droplet' equivalent), format-conversion batch ('image-processor'), multi-format output, watched-folder export. Recordable actions/macros + headless batch over folders is explicit."
    },
    {
      "id": "NEED-029",
      "need": "Single color definition carrying paired CMYK/spot and RGB/HEX values as one source of truth across raster, vector, and layout surfaces — no forked files or manual transcription between print and digital",
      "domain": "color",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "indesign",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "cross-app/mixed: Brand studio color truth across all surfaces; indesign/small: Color defined once yields correct values in every tool",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.8 Color Management & Pipeline: ONE color model StudioColorProfile referenced by every fill/stroke/swatch/gradient/text-color/adjustment across raster/vector/layout ([STU-COL-001/032], no untagged device triples). [STU-COL-007] StudioSwatch kinds: process/global/spot/mixed_ink/tint; [STU-COL-033] global/spot swatch edit live-updates every referencing object; [STU-COL-034] spot swatch carries optional Lab definition for accurate screen/proof independent of CMYK alternate + per-use tint; [STU-COL-004] assign/convert-to-profile with rendering intent; value models incl RGB/RGB-hex/CMYK/Lab (line 1530). Single source-of-truth color defined once resolving correctly across all print/digital surfaces is explicit; a single swatch's paired CMYK/spot and RGB/HEX is served by the one-profile-tagged-value + deterministic convert pipeline + global propagation."
    },
    {
      "id": "NEED-030",
      "need": "Professional vector precision toolset: pen/node editing, boolean pathfinder, shape builder, stroke controls, expand strokes, align to grid, glyph outlining and custom letterform editing for identity-grade artwork",
      "domain": "editing-core",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/small: Brand identity precision construction; affinity/large: Identity-grade vector tooling",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec 14.5 Vector Graphics: [STU-VEC-002/003/005] anchor/handle/segment Bezier node editing with mirroring modes, live corner treatment, direct segment reshape; Pen tool (network-anchor connect), Bend/Anchor-convert, Node/Direct-Select tools (lines 459-463); [STU-VEC-012] boolean Pathfinder (Union/Trim/Merge/subtract/divide) + [STU-VEC-007/050] Shape Builder over shared VectorEngine boolean core; [STU-VEC-062] caps/joins/arrowhead controls incl branching endpoints; [STU-VEC-054] expand-appearance/expand strokes; [STU-VEC-029] pixel snapping + align/distribute to grid/artboard/key-object (line 678); [STU-VEC-045] zoom-independent precision geometry; [STU-VEC-057] measure distance/angle/area; text-to-outlines (fonts-as-outlines line 686). Identity-grade professional vector precision toolset is explicit."
    },
    {
      "id": "NEED-031",
      "need": "Bleed, trim, slug, and safe-margin geometry as first-class document settings with visual guides, honored on export — including exact physical-unit math and repositionable guides for spine/safe zones",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/solo: Print collateral bleed/trim/safe as first-class settings; illustrator/solo: Bleed model honored in PDF/X output",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14-studio-creative-suite.md: New Document/Document Setup carry page size, margins, bleed, slug (research 53-indesign-deep-feature-delta.md L497,637); STU-LAY-009/010 responsive layout on bleed change; crop-to 'bounding box, art, crop, trim, bleed, media' (L930); Marks & Bleed panel with crop/bleed/registration marks + bleed values + include-slug (L1080,1099); preflight bleed-zone hazards + document bleed/slug (L1063-1066); booklet imposition bleed-between-pages (L1089). Bleed/Slug/Preview screen modes (research 53 L1283). Margins + ruler guides as document settings honored on export/PDF."
    },
    {
      "id": "NEED-032",
      "need": "Developer handoff inspect surface: read-only mode exposing measurements, spacing, tokens, component props, copy-ready code hints, and self-serve asset export for internal and external dev teams without designer mediation or edit rights",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/small: Client website dev handoff; cross-app/mixed: Web agency inspect mode",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "research 38-figma-source-distilled-domain-ledger.md L126-128: 'Dev Mode inspect and handoff, measurements, code snippets, design tokens, Code Connect... CSS/iOS/Android snippets, self-serve export'; 55-figma-deep-feature-delta.md dev-mode section incl. seat-gating with 'Studio's analog is a role-scoped inspect mode'; spec 14 STU-VEC-057 typed measurement/inspection surfaces (distance/angle/area, document inventory) usable by operator and model; STU-RAS-012 placed-asset link health as inspectable state. Read-only role-scoped inspect + measurements + code hints + self-serve asset export addressed."
    },
    {
      "id": "NEED-033",
      "need": "Design tokens/variables with modes (theme, brand, density) as first-class exportable data (DTCG/Style Dictionary class) — the same source of truth designers use and engineers read, supporting multi-brand overrides and code sync",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Enterprise design-system release train; cross-app/mixed: Brand studio DTCG token export",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-DS-023 StudioVariableCollection with modes (light/dark, compact/comfortable = theme/density), one value per mode, no plan-gated mode-count cap; STU-TYP-027 token-driven typography via StudioVariable; STU-DS-049 design system (components/variables/collections/modes) are exportable local PostgreSQL authority rows, offline. research 55-figma-deep-feature-delta.md L1716 'Extended variable collections (multi-brand)' + variable-mode-inheritance; Code Connect (code sync) in 38 ledger. Note: exact DTCG/Style-Dictionary interchange format name not explicitly cited, but the exportable token/variable/mode/multi-brand source-of-truth system is covered."
    },
    {
      "id": "NEED-034",
      "need": "Physical units and work-at-scale: mm/inch canvases, DPI-aware export, documented scale factors carried as metadata into exports, very large canvas support without feature loss, and tiling/panelization with seam/overlap annotations for wide-format output",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "figma",
        "photoshop"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/solo: Design at 1:10 with scale metadata for trade-show graphics; figma/solo: Physical-unit page setup for print jobs",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-053 measurement systems: points, picas, inches, decimal inches, millimeters, centimeters, ciceros, agates, pixels, custom, per-axis; STU-DOC-003 Studio unit law (every length-bearing field carries explicit unit); preflight low placed-image DPI (L1060); raster export with resolution + bleed + overlap (STU-LAY-063 L1120); booklet/imposition N-up (STU-LAY-060); research 52-illustrator-deep-feature-delta print-tiling-tool, 07-indesign-leaf-index print-oversized-documents, 51-photoshop Image>Analysis measurement-scale (scale factor metadata). Physical units, DPI-aware export, large-canvas tiled raster, tiling/overlap covered."
    },
    {
      "id": "NEED-035",
      "need": "Dieline and technical-layer handling: locked non-printing layers for dieline/varnish/emboss with spot identity preserved, CAD-format dieline import (DXF/CF2-class), dieline version traceability against artwork, strippable at output",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "photoshop",
        "indesign",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: CPG dieline-to-press; cross-app/mixed: Packaging studio CAD dieline import and layer roles",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Constituent primitives present: InDesign hidden/nonprinting objects + Nonprinting flag (research 07-indesign-leaf-index L706, 53-indesign-deep-feature-delta L3912); spot colors/channels + ink manager per-ink spot-to-process (spec 14 L1112, STU-RAS spot channels); DWG/DXF import retaining layers/scale (research 02-affinity-suite-feature-map L58,75-76); PDF export non-printing-objects/layers toggles (research 53 L3448). NOT explicit: dedicated dieline/varnish/emboss technical-layer ROLES with spot identity strippable-at-output, CF2-class packaging CAD import (searches for 'dieline|cf2|varnish' return no dieline/CF2 hits; emboss appears only as layer FX bevel/emboss, not print finish), and dieline-version-traceability-against-artwork."
    },
    {
      "id": "NEED-036",
      "need": "Style system: paragraph/character/object/table styles where 100% of formatting can live in styles, enforced and synchronized across documents and contributors, with house-style adoption for external freelancers",
      "domain": "typography",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "indesign",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: Style-driven docs so translations inherit formatting; affinity/small: Styles enforced across an issue's multiple files",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-036/038/039 shared StudioStyleRegistry with normative style types: paragraph, character, object (full frame-formatting surface), table, cell, TOC style; nested/GREP/pattern styles; object style applies fitting/wrap/anchor/effects/export-tag in one op; table style cascades cell styles. research 03-indesign-feature-map paragraph/character/nested/GREP/object/table+cell styles; 35-indesign domain ledger 'paragraph, character, object, table, and cell styles'; TOC style 'reusable across documents and books' + book files shared style source (research 03 L23). Style-driven layout so translations inherit formatting addressed."
    },
    {
      "id": "NEED-037",
      "need": "Professional composition engine: threaded text flow across frames/pages, hyphenation/justification, keep options, widow/orphan control, baseline grids, optical kerning and margin alignment",
      "domain": "typography",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/solo: 200-400 page book typesetting; affinity/small: Multi-page layout engine",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-TYP-005 threaded/linked text with autoflow modes + smart reflow; STU-TYP-036 hyphenation record (min word length, letters-after-first/before-last, consecutive-hyphen limit, zone, better-spacing/fewer-hyphens bias); composer table paragraph/every-line composer (L1198); keep/flow model STU-TYP-037 (referenced), break/keep interaction (L1336); baseline grids (research 03-indesign-feature-map guides/layout/baseline grids, 04-affinity-leaf-index Baseline grids); optical margin/kerning via TextEngine (14.7 owns kerning/tracking, OpenType). Justification, keep options, widow/orphan-class controls, baseline grid, optical alignment addressed."
    },
    {
      "id": "NEED-038",
      "need": "Non-destructive editing stack: adjustment layers, per-layer masks, clipping, blend modes, live/smart filters, non-destructive transforms — so rejected or revised work never forces redo",
      "domain": "editing-core",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: E-commerce retouch corrections without redoing work; photoshop/small: Deep layer architecture with nested groups",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-RAS-035 StudioAdjustment usable as non-destructive adjustment-kind layer with built-in mask + clip-to-parent + re-editable params; STU-RAS-015 StudioMask (per-layer/clip forms); STU-RAS-038 canonical StudioBlendMode enum; STU-RAS-012 placed_asset (smart-object-class) embedded/linked; live/smart filters via StudioEffectStack (STU-FX). research 01-photoshop-feature-map adjustment layers/smart objects/smart filters non-destructive; 02-affinity live filters+adjustment layers, non-destructive warp; 13-layer-graph-vertical-slice dedicated non-destructive layer-graph slice; 05-studio-primitive-map non-destructive operation stack."
    },
    {
      "id": "NEED-039",
      "need": "Deep retouch/compositing toolset: frequency separation, clone/heal/inpaint with source control, dodge/burn, liquify, hair/edge-grade masking and selection, multi-frame compositing, grain/light matching",
      "domain": "editing-core",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Entertainment key art compositing; affinity/large: Pro retouch toolset for studio pipelines",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "research 02-affinity-suite-feature-map: Frequency Separation (L37), Inpainting/Patch/Healing (L38), Liquify Persona (L44); 01-photoshop-feature-map Healing/Patch/Clone family (L60), Warp/Liquify/Puppet Warp (L91); 04-affinity-leaf-index cloning-and-healing, inpainting, frequency-separation leaves. spec 14 Liquify with push/twirl/pinch + freeze/thaw masking (L262), on-device inpaint/content-aware (STU-RAS-044); edge/hair masking via StudioSelectionSet refine + StudioMask (STU-RAS-015/017). Dodge/burn + multi-frame compositing/grain-match within raster retouch domain."
    },
    {
      "id": "NEED-040",
      "need": "Very-large-file performance: multi-GB PSB-class documents, 100+ MP, hundreds of layers, 16-bit/wide gamut, responsive pan/zoom/brush, stable open-edit-save throughput (50-150 images/day per retoucher) and non-blocking long batch runs",
      "domain": "performance",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Multi-GB hundreds-of-layers key art files; photoshop/small: Hundreds of 50-100MP layered files daily",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-FX-012b tiled/progressive/region-of-interest re-rendering so large-document interactive preview stays bounded; StudioRasterTile tiled pixel storage at document bit depth (L138); STU-FX-006b document-level performance budget via render harness; performance limits documented in 14.23; STU-RAS-002 headless/quiet law = non-blocking batch/filter/transform runs. research 46-file-format-compatibility-registry + 51-photoshop-deep-feature-delta PSB Large Document Format (L5400), 06-photoshop-leaf 'Save large documents', 54-affinity 8/16/32-bit precision. Enabling architecture for multi-GB/100MP/hundreds-of-layers responsiveness present; note explicit throughput SLAs (50-150 img/day) are not quantified as requirements."
    },
    {
      "id": "NEED-041",
      "need": "Master/parent page system: stacked masters, running heads, auto folios, chapter-opener variants, automatic propagation of layout changes across all pages, and multi-page long-document layout with master consistency",
      "domain": "layout",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/solo: Master pages with automatic propagation; indesign/solo: Parent pages across a whole book",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-008 primary text frame on parent page (auto-adopt, re-thread on parent change); STU-LAY-015 auto page-number markers resolve to section number on parent/running-header/TOC + last-page 'page X of Y'; STU-LAY-016 content-derived running headers/footers pulling styled text. research 03-indesign-feature-map parent pages (shared headers/footers/furniture inherited), page numbers/sections/chapter numbers, text variables for running headers; 04-affinity-leaf-index master pages create/edit/apply/detach+link; 02-affinity smart master pages. Stacked masters, running heads, auto folios, chapter-opener variants, propagation covered."
    },
    {
      "id": "NEED-042",
      "need": "Long-document apparatus: auto TOC, index, section numbering, cross-references, footnotes/endnotes, running headers that update on edit and rebuild after repagination",
      "domain": "typography",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Corporate annual report; affinity/solo: Long-document automation (TOC, index, running heads)",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-043 TOC style stored/reusable across documents and books; STU-LAY-047 cross-references from building blocks + stale-reference preflight + named text anchors; running headers STU-LAY-016; section numbering markers STU-LAY-015; preflight out-of-date TOC / stale cross-references (L1066). research 03-indesign-feature-map footnotes/endnotes (create/format/span/wrap/convert), table of contents (auto-maintain page numbers), indexing (tag/manage/update/cross-reference), cross-references; 35-indesign domain ledger 'footnotes; endnotes; cross-references; table of contents; indexes'; 02-affinity Books sync TOC/indexes + footnotes/endnotes/sidenotes."
    },
    {
      "id": "NEED-043",
      "need": "Concurrent editing of one publication with story/frame-level check-out/check-in — text vs geometry ownership split so editors and designers work the same document in parallel without overwriting (InCopy-class)",
      "domain": "collaboration",
      "scales": [
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "indesign",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: Magazine via InDesign+InCopy workflow; cross-app/mixed: Parallel designer-editor model",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-066 collaboration table: 'File-based assignment/check-in-check-out copy-editing workflow | Native (operates on shared local/network storage)'; STU-OVR-003/STU-LAY-066 kernel CRDT co-edit as primary path for concurrent editing; story-editor + linked stories (STU-TYP-005/006). research 03-indesign-feature-map indesign.incopy_workflows 'assignment files... for designer/editor collaboration while preserving layout control'; 11-provider-posture-map 'About assignment files between InDesign and InCopy' + relink/unlink assignment files. Text vs geometry ownership split (InCopy-class) addressed as native."
    },
    {
      "id": "NEED-044",
      "need": "Zero-friction external reviewer access: free viewer/commenter roles with no install, no account hurdle, no per-reviewer seat cost — including universal access to native working files for clients and freelancers",
      "domain": "collaboration",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/solo: Cannot pay per client reviewer; affinity/large: Zero-cost viewer for external collaborators",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec 14 STU-LAY-066: 'Review and collaboration MUST be local-first. The native review surface — comments, annotations, and markup anchored to layout positions... requiring no external account or cloud service' is first-class native; hosted share-for-review link (pin/highlight/strike/insert/reply comments in browser) is adapter-backed/optional; STU-DS-049/STU-OVR-002 no-account offline. research 55-figma-deep-feature-delta seat model (full/dev/collab/view) recorded as provider-gated/omitted with Studio analog role-scoped; 03-indesign share_for_review browser review links. No-account/no-seat native review + optional browser reviewer link covers zero-friction external reviewer."
    },
    {
      "id": "NEED-045",
      "need": "DAM/PIM connectivity: check-out/check-in of assets against a canonical repository with version history, drag-and-drop placement preserving canonical links, published-asset registry for approved finals, and push of final+source packages to client DAM/brand portals",
      "domain": "asset-management",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "indesign",
        "affinity",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Magazine imaging desk DAM round-trip (no stale or forked copies); figma/large: Approved finals findable outside campaign files",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Local coverage present: spec 14 STU-RAS-012 placed_asset embedded/linked with link health (up-to-date/modified/missing) + update-all; native Studio asset library replacing vendor CC-Libraries, vendor-cloud sync as optional adapter (L383); STU-LAY-058 Package collects document+linked resources+fonts with report for printer/archive handoff, operates over a book; research 07-indesign Links panel/relink, 34-photoshop 'local-first equivalents for review packages, asset libraries, linked resource resolution'. NOT explicit: check-out/check-in against an external canonical DAM/PIM repository with version history, a published-asset registry for approved finals, and push of final+source packages to a client DAM/brand portal (searches 'DAM|PIM|brand portal|published.*registry' return no external-DAM-round-trip feature rows; only generic optional-adapter mentions)."
    },
    {
      "id": "NEED-046",
      "need": "Placed-asset fidelity across raster/vector/layout: clipping paths, alpha channels, spot channels, and transparency preserved through save, placement into other apps, and export into RIP workflows — with no unwanted color shifts",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "indesign",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/small: TIFF/PSD paths/channels accepted by layout and RIP; indesign/small: Placed spot-color vector art renders exactly at any scale",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14-studio-creative-suite.md STU-RAS-043 (MUST preserve alpha channels, spot channels, masks, placed-asset links across import/export, report loss rather than drop silently); STU-LAY-030/031 (placed graphics apply embedded clipping path, alpha-channel choice, per-image color profile + rendering intent; clipping/masking via embedded clipping path/alpha/frame-as-mask); STU-LAY output PDF/X-1a/X-3/X-4 output intents + color conversion 'none/convert-preserve-numbers' + ink-manager (no-color-shift/RIP path); STU-COL-014 spot-channel-as-StudioSwatch(spot). Research 03-indesign-feature-map separations/inks/overprint; 46-file-format-compatibility-registry color-separations/overprint records."
    },
    {
      "id": "NEED-047",
      "need": "Many-artboard/frame canvas performance: hundreds to 1,000+ artboards or image-heavy frames per document without per-operation stalls, editor slowdown, or hard memory ceilings that lock the file mid-production",
      "domain": "performance",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/solo: Second-long stalls at ~30 artboards with heavy content; figma/small: Memory ceiling locks working file mid-production",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Artboards/frames fully covered as features (52-illustrator-deep-feature-delta 'Layout and Artboards' section, Object>Artboards commands; 19-studio-local-first-rust-posture studio_layout owns artboards/boards/frames). Adjacent performance clauses exist: spec-modules/14 STU-FX-012b (tiled/progressive/ROI re-render so 'large-document interactive preview stays bounded') and STU-FX-006b ('document-level performance budget surfaced through the render harness'). But grep for 'many artboard / hundreds/thousands of artboards / memory ceiling / per-operation stall / scaling target' returned no explicit non-functional requirement for 1,000+ artboards/frames without stalls; the performance-at-production-volume need itself is not directly addressed."
    },
    {
      "id": "NEED-048",
      "need": "Book/multi-file publication assembly: chapter files owned by different people with synchronized styles/swatches, continuous page numbering, cross-file TOC/index, and whole-book batch export as one deliverable",
      "domain": "multi-doc-consistency",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/solo: 300+ page catalog split into chapter files; affinity/large: Book-file binding across separately-owned sections",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14 STU-LAY-042 (Books MUST bind multiple chapter StudioDocuments into one publication sharing numbering/output; add/remove/reorder, designate style-source document, per-document status, open from book list); STU-LAY-014/015 (sections restart/continue page numbering, page-x-of-Y markers); TOC style 'reusable across documents and books' (line 987); STU-LAY-047 cross-references across documents; STU-LAY-058 Package 'MUST operate over a book as well as a single document'; STU-LAY-060 booklet/imposition. Whole-book batch output covered."
    },
    {
      "id": "NEED-049",
      "need": "Style-mapped manuscript import (Word/RTF/ODT): incoming named styles map onto document styles, preserving footnotes, italics, tables, and special characters, with typographic cleanup — instead of pasting dead formatting",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/solo: Word manuscript to KDP book; affinity/large: Word import preserving footnotes and styles",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14 STU-TYP-007 (place TXT/RTF/DOC/DOCX into stories with encoding, style-map/preserve, and cleanup options); STU-LAY-040 ('incoming word-processor style mapping on import with saved presets', per-style conflict resolution use-incoming/auto-rename); footnotes/endnotes markers (STU-TYP-022 special-character catalog + 14.6 long-document); special-characters/white-space catalog (STU-TYP-022); tables (STU-LAY-039). ODT not named explicitly (list is TXT/RTF/DOC/DOCX) but the generic word-processor style-map + cleanup workflow covers the need's intent."
    },
    {
      "id": "NEED-050",
      "need": "Real-time multiplayer co-editing with presence, multi-cursor, and follow/spotlight observation for crits, walkthroughs, and distributed simultaneous work",
      "domain": "collaboration",
      "scales": [
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/small: Distributed freelancers and principals in one file; cross-app/mixed: Web agency multi-cursor collaboration",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 55-figma-deep-feature-delta collaboration-and-files: multiplayer-presence ('named live cursors, per-user selection highlights, avatar stack ... Handshake maps this onto its local-first CRDT layer'), observation-mode ('follow a collaborator ... locks local viewport to follow their cursor/navigation'), spotlight-me ('presenter spotlights themselves so participants are pulled into following'). spec-modules/14 STU-LAY-066 native local-first collaboration (comments/annotations/markup + CRDT co-edit); 14.16/14.17 CRDT collaborative editing; 01-vision-and-context CRDTs(Yjs) as human+AI concurrency fabric."
    },
    {
      "id": "NEED-051",
      "need": "Integrated culling/rating/catalog stage: browse, rate, filter thousands of frames without opening each document, with lossless round-trip to external catalog tools (open from catalog, save layered file back, stay linked and stacked)",
      "domain": "asset-management",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/small: The suite's biggest pipeline hole — every workflow bolts on Lightroom/C1; photoshop/solo: Lightroom round-trip hero finishing",
      "coverage_verdict": "NOT_COVERED",
      "coverage_evidence": "Grep across research corpus (excluding _source_snapshots) and spec-modules/14 for 'Lightroom|Capture One|cull|rating|star|flag pick/reject|catalog|DAM|browse thousands|keyword|contact sheet|thumbnail grid' returned no culling/rating/catalog-stage feature row and no lossless external-catalog round-trip (open-from-catalog / save-layered-back / stay-linked-and-stacked). Only tangential hits: 09-affinity-desktop-delta 'Integrating Affinity Photo into Apple/Windows Photos', a generic 'Studio asset library' for linked placed assets (STU-RAS-047/line 383), and 08-gap-resolution-notes acknowledging Camera-Raw/Develop control gaps. The scenario itself flags this as 'the suite's biggest pipeline hole'; no covering surface exists."
    },
    {
      "id": "NEED-052",
      "need": "Template slots for non-designers: locked structure with designated editable text/image fields, centrally-governed brand kits (logos, fonts, colors) enforced org-wide, so marketers and clients produce on-brand variants without breaking layouts",
      "domain": "templates",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "figma",
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: In-house brand studio template system; indesign/solo: Client-operable template output",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 38-figma-source-distilled-domain-ledger Figma Buzz domain: 'brand kits, templates, locked/editable regions, batch/bulk content, CSV/data import ... team collaboration' + red-team note 'Template locks and brand controls need structured validation'; mirrored in 43-figma-source-distilled-feature-rows and 25-figma-feature-use-cards ('On-brand asset production workflows and templates'). spec-modules/14 STU-DS-014 Slots (structural placeholder region for consumer-inserted content), STU-DS-015 instance-swap author-curated preferred-values (constrain non-designer edits), text variables/placeholders (STU-LAY-048), object/graphic styles + library. Locked-structure + editable-field + governed brand-kit workflow addressed."
    },
    {
      "id": "NEED-053",
      "need": "Org/workspace taxonomy: workspace-team-project-file-page hierarchy with permission boundaries, status lanes (WIP/in dev/in production), per-client scoping and library separation, draft/private space, and archive conventions that survive dozens of concurrent campaigns",
      "domain": "file-organization",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "illustrator",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Models the whole org, not one folder of files; illustrator/solo: Workspace/library scoping per client",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 55-figma-deep-feature-delta: file-hierarchy ('Teams/projects/files/drafts hierarchy ... personal drafts space; moving between spaces changes sharing defaults'), sharing-link-permissions (view/edit/dev role scoping + invited/password), seat-model (full/dev/collab/view role-capability matrix 'directly relevant to Studio's local permission model'), organization-admin console (members/teams/workspaces/shared resources), file-browser-search, trash-restore (archive). Status lanes only partially explicit (dev-mode DEV_MODE_STATUS_UPDATE status). Hierarchy + permission boundaries + per-client scoping (teams/projects) + drafts/private + archive conventions covered."
    },
    {
      "id": "NEED-054",
      "need": "Branch/merge on documents and libraries with reviewable visual diffs (added/edited/removed, side-by-side and overlay), request-review gates before merge, and automatic pre-merge checkpoints — accessible below enterprise pricing",
      "domain": "versioning",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Design-system library branch/merge with review gate; figma/solo: Gating branching to enterprise tiers forces file-duplication drift",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 55-figma-deep-feature-delta branch-model ('Branches fork a file for isolated exploration, request reviews, diff changes against main, merge back with conflict handling, and pull updates from main; branch/merge semantics local-mappable') and dev-mode diff row (line 2346: 'diff a frame's current state against earlier versions side-by-side or overlaid'); version-history rows (pre-merge checkpoints). Review gate = 'request reviews' before merge. Pricing gating noted 'provider-dependent' (Studio local storage has no enterprise-tier gate)."
    },
    {
      "id": "NEED-055",
      "need": "Deterministic expand/outline/flatten to a production-safe deliverable copy (strokes, effects, live type to plain filled paths; guides/hidden layers stripped) while the live-text editable master is preserved untouched",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/solo: Convert-to-outlines plus preserved live-text master; illustrator/small: RIP-safe expand for screen-print shops",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14 STU-VEC-054 (expand-appearance MUST bake current appearance stack—fills/strokes/per-row effects/brushes/live corners/live constructs—into concrete geometry/raster; 'single explicit boundary between non-destructive editing and destructive materialization', one history entry, MUST NOT occur implicitly on save/export); STU-VEC-070 flatten (destructive-merge over any selection incl. text outlines); outline-stroke (line 442/533); text stays live until explicit outline (STU-VEC-068 'MUST NOT bake text to outlines as a side effect'); STU-CON-005 determinism. Production-safe stripped copy via package/export (STU-LAY-058, export recipes exclude non-printing/guides). Live-text master preserved via non-destructive model + explicit-only expand."
    },
    {
      "id": "NEED-056",
      "need": "Unified cross-discipline environment: raster, vector, and layout personas in one document/runtime with in-context editing of placed content (StudioLink-class) — no export/reimport churn between apps",
      "domain": "architecture",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "affinity",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "cross-app/mixed: Solo generalist spanning web and print; affinity/small: Retouch a placed photo without round-trips",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 02-affinity-suite-feature-map StudioLink row + narrative ('raster, vector, and page-layout tools can operate inside one document model without launching separate applications ... should become a shared primitive architecture: same layer graph, vector path engine, layout frame engine, export system exposed through task-focused work modes'). spec-modules/14 STU-DOC-004 (shared-primitives product; a capability in one domain is the SAME primitive in another; single worksurface); STU-RAS-012 placed_asset with 'nested child-document editing' (in-context edit of placed content, no export/reimport churn)."
    },
    {
      "id": "NEED-057",
      "need": "Governed library publishing: explicit versioned releases (v1.0, v1.1) with downstream update notification and per-document accept/review instead of silent propagation, component relocation between libraries without breaking instances, deprecation/supersede signaling",
      "domain": "asset-management",
      "scales": [
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "illustrator",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Library publish as versioned release; illustrator/large: Versioned brand asset generations with deprecation",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Core covered: Research 55-figma-deep-feature-delta library-publishing ('publishing pushes selected components/styles/variables to a team library with a per-item change list and publish notes; consumers receive update review prompts; publish/subscribe model is a local concept') + library-update-review ('consuming files list pending library updates with previews and apply them selectively, keeping instance overrides intact') + LIBRARY_PUBLISH webhook event. Gaps: grep for 'deprecat|supersede|relocate|component move between libraries|explicit versioned release v1.0/v1.1' returned no evidence for semantic versioned releases, deprecation/supersede signaling, or component relocation between libraries without breaking instances; those named sub-requirements are not addressed by any feature row."
    },
    {
      "id": "NEED-058",
      "need": "Non-destructive RAW development (profiles, lens corrections, noise reduction, re-editable RAW layers) feeding directly into layered retouch, with 16-bit and raw-linked editing so heavy retouch never bands",
      "domain": "photo-editing",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "affinity",
        "photoshop"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/large: Develop-to-retouch in one tool; affinity/small: Pro-quality re-editable RAW layers",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14 section 14.12 Camera Raw/Develop pipeline (STU-RAW-*): STU-RAW-009 lens-profile correction (distortion/vignetting/chromatic aberration, auto+manual), demosaic/raw-details refinement + super-resolution (STU-RAW section 14, deterministic studio-engine), develop process version as first-class field, snapshots/default-settings, non-destructive parametric develop, STU-RAW-014a local-adjustment masks with per-mask develop sub-stack, STU-RAW-016 'developed raw opens flat or as a re-editable placed object' (feeds layered retouch). Raster masks 8/16-bit (line 170), HDR 32-bit. Research: 14-domain catalog 'Camera Raw/Develop pipeline (Photoshop Camera Raw, Affinity Develop persona)'."
    },
    {
      "id": "NEED-059",
      "need": "Flexible seat/role economics: lightweight restricted editor seats for text editors and marketers, guest external editors without full-member billing (with admin approval on upgrades), and dev-seat pricing that doesn't punish adding technical stakeholders",
      "domain": "collaboration",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/small: Guest editor model without full billing per short engagement; cross-app/mixed: Editor role that cannot break geometry and needs no full license",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Research 55-figma-deep-feature-delta: seat-model ('Seat types full/dev/collab/view permission model ... role-capability matrix directly relevant to Studio's local permission model'), dev-mode seat-gating ('Studio's analog is a role-scoped inspect mode'), sharing-link roles (view/edit/dev to invited/anyone-with-link = guest external editors), organization-admin ('seat assignments, teams' + admin approval on upgrades). Restricted-editor concept (editor that cannot break geometry) maps to collab/view/role-scoped modes. Billing/pricing itself noted provider-dependent, which for local-first Studio removes the per-seat-billing punishment the need targets."
    },
    {
      "id": "NEED-060",
      "need": "Professional table engine with table/cell styles: dense financial and spec/price tables that restyle globally across hundreds of instances and survive late numeric edits, usable inside generated and hand-built blocks",
      "domain": "typography",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: Financial tables in annual report; indesign/large: Spec/price tables applied by many hands",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14 STU-LAY-039 (Table and cell styles MUST format tables/cells declaratively; a table style references its component cell styles and applying it cascades through the table); table style references up to five cell styles (header/footer/body/left/right) + border/alternating patterns (line 985-986); table creation/structure—insert/delete/select rows+cols, merge/unmerge, split, sort, convert text↔table (line 958/968); tables usable in data-merge (line 964 graphic/data cells) and hand-built. Global restyle across instances + survival of late numeric edits achieved via style cascade."
    },
    {
      "id": "NEED-061",
      "need": "Per-project external access scoping and strict client isolation: guests see exactly one project, agency IP elsewhere stays invisible, single-file guest edit grants possible",
      "domain": "collaboration",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/small: Freelancer sees exactly one client's files; figma/large: Client-scoped guest access",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md explicitly captures the scoped-external-access model: `figma.deep.organization-admin.guest-management` (l.4713-4717) 'External guests receive scoped access to specific teams/files with an admin roster... scoped-external-access is the local-relevant concept'; `sharing-link-permissions` share links with view/edit/dev role scoping + password + invited-only (l.3643-3647); prototype-only share links keep working files private (l.2246-2249); org settings constrain default link sharing/guest access (l.4695). Single-file guest edit = edit-role share link on one file. Spec 14.16/14.17 collaboration posture (CRDT + role-scoped) underlies enforcement."
    },
    {
      "id": "NEED-062",
      "need": "Live copyfit feedback for editors: exact overset/fit, word/line counts, and overset depth rendered against real frame geometry while writing",
      "domain": "typography",
      "scales": [
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "indesign",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: Editors see fit against real geometry; cross-app/mixed: Copyfit rendered against actual layout",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "53-indesign-deep-feature-delta.md Story Editor 'text-only editing view with style column, depth ruler, overset text indicator, and inline display of notes, tracked changes' (l.1333, l.1571) gives live overset + overset-depth while writing; `View word and character counts in text frames` is an explicit feature-use card + source-distilled row + command-contract candidate (17-indesign-feature-use-cards.md l.892-950; 40-indesign-source-distilled-feature-rows.md l.492-508; 07-indesign-leaf-index.md l.140). Overset rendered against real layout geometry via preflight text-overset check (53 l.3358) and studio primitive `overset_text` diagnostic (05-studio-primitive-map.md l.78). Spec 14.7 native TextEngine + overset detection."
    },
    {
      "id": "NEED-063",
      "need": "Version-agnostic working-file compatibility: lossless exchange between different app versions and installs — no forward-only native format, no lossy downsave dance",
      "domain": "handoff",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "indesign",
        "illustrator"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: The IDML workaround exists because native files aren't backward compatible; illustrator/small: Freelancer and studio on different versions",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent coverage only. Spec 14.13 defines round-trip interchange posture and 14-studio-creative-suite.md format matrix rows 24/25 make IDML a round-trip target while binary INDD export routes via IDML/PDF; STU-LAY-064 (l.1124) requires layout markup interchange for cross-version exchange. Spec l.73 explicitly forbids inventing a new interchange format. BUT no explicit requirement guarantees Studio's OWN native StudioDocument working file is version-agnostic / backward+forward compatible across different Studio versions and installs (the actual need: lossless exchange when freelancer and studio run different versions without a downsave dance). Single-native-format + CRDT/EventLedger authority implies it but it is not stated as a compatibility guarantee. Searches: 'version.agnostic', 'backward.compat', 'forward.only', 'downsave', 'schema.version' returned only legacy-format IDML round-trip rows, no native-format version-compat clause."
    },
    {
      "id": "NEED-064",
      "need": "Clean project exit and ownership transfer: move whole files/projects/libraries across org boundaries at engagement end, with explicit link-severance behavior, self-contained package export, and the studio retaining an archival copy",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/solo: Transfer to client workspace without breaking component links; figma/large: File handover at project close",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec STU-LAY-058 (l.1072) Package collects document + linked resources + fonts into a portable self-contained folder with a report 'for handoff to a printer or archive... over a book as well as a single document'; format matrix row 29 'Packaged/collect output package folder' and row 44 linked assets 'collected on package' (l.2710, 2725). Link-severance: STU-RAS-012/013 placed_asset explicit embedded<->linked conversion in both directions detaches source references. Cross-org move: 55-figma-deep-feature-delta.md 'Files move between projects/teams via drag or dialog and duplicate in place, with permission implications surfaced on move' (l.3715). Studio retains archival copy = Package portable folder / duplicate-in-place. All handoff primitives are explicit."
    },
    {
      "id": "NEED-065",
      "need": "Pixel-grid precision for screen assets: snap-to-pixel, integer coordinates, preview at exact target raster size to avoid half-pixel blur in exported SVG/PNG",
      "domain": "precision-drawing",
      "scales": [
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "illustrator/large: 1,000+ icon design-system library; illustrator/small: Crisp SVGs at target sizes",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "52-illustrator-deep-feature-delta.md `Object > Make Pixel Perfect` aligns art to pixel grid for crisp screen rendering (l.1287-1290), `View > Snap to Pixel` + snap-on-draw/move/scale + align-to-pixel-grid for crisp raster output (l.1819-1821, l.3237); 55-figma-deep-feature-delta.md `snap-to-pixel-grid` '1px pixel grid at high zoom and a snap-to-pixel-grid preference rounds geometry to whole (or half) pixels during draw/move/resize' (l.373-377); 54-affinity-deep-feature-delta.md 2.3 pixel grid display (l.4907-4911); pixel preview across apps. Explicit rows for both illustrator and affinity."
    },
    {
      "id": "NEED-066",
      "need": "Reliable file locking and ownership signals over shared and cloud-synced storage — preventing the lock-file-on-Dropbox failure mode of conflicted copies and silently overwritten work",
      "domain": "collaboration",
      "scales": [
        "small"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/small: .idlk-on-Dropbox documented loss scenario; affinity/small: One-file-one-person convention needs real locking signals",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Partial. Explicit check-out/ownership feature exists for managed content: spec 14.10 posture 'File-based assignment/check-in-check-out copy-editing workflow | Native (operates on shared local/network storage)' (l.1140) and 11-provider-posture-map.md InCopy check-in-and-check-out-content + workflow-icons-for-managed-files leaves (l.671-726). Handshake's native answer to conflicted-copy is CRDT collaborative document state + PostgreSQL/EventLedger authority (spec STU-OVR-003 l.50, 14.16/14.17), which structurally removes the shared-file overwrite race. BUT there is NO explicit general file-lock / ownership-signal feature for the arbitrary shared/cloud-synced single-file 'one-file-one-person' convention, and nothing addressing the affinity scenario's need for real locking signals over Dropbox-style storage. Searches: 'file lock', '.idlk', 'conflicted copy', 'dropbox', 'ownership signal', 'lock file' returned only layer-locking and InCopy check-out, not a shared-storage file-lock primitive."
    },
    {
      "id": "NEED-067",
      "need": "Editorial change tracking inside text stories: text-edit suggestions, tracked changes, and notes visible to the layout artist for editorial review without PDF round-trips",
      "domain": "review-approval",
      "scales": [
        "large",
        "mixed"
      ],
      "apps": [
        "affinity",
        "indesign",
        "cross-app"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/large: In-document suggestions and change tracking; cross-app/mixed: Change tracking inside stories",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "53-indesign-deep-feature-delta.md `Type > Track Changes submenu` 'Enables tracking per story or all stories and accepts/rejects changes individually, by story, or document-wide' (l.974-981); `Preferences: Track Changes` (edit kinds tracked, per-user marking colors/styles, change-bar options) (l.4134-4141); editorial notes color-coded (03-indesign-feature-map.md l.80 `editorial_notes`); Story Editor inline display of notes + tracked changes (l.1571). InCopy assignment workflow for designer/editor review without PDF round-trip (03 l.79). Explicit tracked-changes-inside-stories feature rows."
    },
    {
      "id": "NEED-068",
      "need": "Crash-safe autosave and recovery that reliably restores in-progress work — documented work loss from exception crashes with no autosave is unacceptable for billable throughput",
      "domain": "reliability",
      "scales": [
        "solo"
      ],
      "apps": [
        "affinity",
        "illustrator"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/solo: Documented user work loss with no autosave; illustrator/solo: Retainer throughput is billable time",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "53-indesign-deep-feature-delta.md `Automatic document recovery` 'Auto-recovery data restores unsaved changes after a crash on next launch, with a configurable recovery folder' (l.2299-2301, l.4187); 52-illustrator-deep-feature-delta.md auto-save/data-recovery interval with backup-location + Save-in-Background (l.4336); 51-photoshop-deep-feature-delta.md autosave recovery interval / background save (l.5829); 37-affinity-source-distilled-domain-ledger.md document recovery (l.133); 55-figma-deep-feature-delta.md autosave checkpoints every ~30 min (l.3558-3561). Spec 14.19 unified per-document history/undo underpins recovery. Covers both named apps (affinity, illustrator)."
    },
    {
      "id": "NEED-069",
      "need": "CMYK and multichannel raster editing/export with control over black generation, rich black builds, and ink limits per printer spec",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/large: Magazine prepress CMYK control; photoshop/solo: CMYK conversion while layered RGB master stays editable",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "51-photoshop-deep-feature-delta.md color-mode switching among Bitmap/Grayscale/Duotone/Indexed/RGB/CMYK/Lab/Multichannel (l.1153); `Multichannel mode` channel-per-plate for specialized printing (l.3889-3892); `Duotone mode` mono/duo/tri/quadtone with per-ink transfer curves + overprint colors (l.3878-3881); `Edit > Color Settings` sets CMYK working space + advanced conversion options (l.3927-3930); `Convert to Profile` with rendering intent (l.3936-3939); Channels split/merge to multichannel (l.3786). InDesign separations/inks/overprint + `Preview color separations and ink coverage` (03 l.70; 11 l.951-957). Note: granular custom-CMYK black-generation (GCR/UCR) and total-ink-limit controls live inside Color Settings advanced conversion but are not broken out as a distinct named row; the CMYK/multichannel editing+export and ink-coverage need is explicitly addressed."
    },
    {
      "id": "NEED-070",
      "need": "Multi-script composition: RTL and CJK support, per-language hyphenation/justification dictionaries, multilingual font fallback, and layouts tolerant of 30-35% text expansion (auto-size frames, robust reflow)",
      "domain": "typography",
      "scales": [
        "large"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "indesign/large: 20-language localization program",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "03-indesign-feature-map.md `cjk_text` CJK composition incl. Japanese settings (l.42); 35-indesign-source-distilled-domain-ledger.md 'CJK, right-to-left, vertical type, ruby, kinsoku, mojikumi' (l.59); 07-indesign-leaf-index.md Arabic/Hebrew leaves (diacritics, ligatures, l.3474-3484) + hyphenation/word-breaks (l.2427) + smart text reflow (l.76-83); hyphenation/justification dictionaries (03 l.41). Multilingual font fallback = replace/substitute missing fonts (06-photoshop-leaf-index.md l.3758; 22-illustrator-leaf-index.md l.3897) + illustrator RTL/Indic/CJK/MENA + font substitution (36 l.81). Text-expansion tolerance = smart text reflow + Adjust Layout reflow (03 l.26) + overset detection. Spec 14.7 native shaping/line-breaking engine."
    },
    {
      "id": "NEED-071",
      "need": "Library-scale performance: lazy-load variants, amortized component memory across documents, and fast publish for large component sets — so teams are not forced to shard libraries",
      "domain": "performance",
      "scales": [
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/large: Figma's failure here forces library sharding",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Feature surface covered, performance NFR not. Library publish/subscribe is explicit: 55-figma-deep-feature-delta.md publish selected components/styles/variables with per-item change list + consumer update prompts (l.1594), figma.teamLibrary import APIs (l.3954); 23-figma-leaf-index.md Publish/Unpublish a library (l.1090-1144); variants/component-sets in spec 14.6. BUT the actual need is a performance-at-scale requirement — lazy-load variants, amortized component memory across documents, fast publish for large component sets so teams are NOT forced to shard libraries — and no feature row or non-functional requirement addresses lazy-loading, memory amortization, or large-set publish performance. Searches: 'lazy.load', 'amortiz', 'shard', 'component memory', 'library performance' returned only publish/unpublish feature rows, no scale/performance clause."
    },
    {
      "id": "NEED-072",
      "need": "Live, non-rasterized text through the whole production cycle so late copy changes never force rebuilds; vector type stays sharp at print resolution",
      "domain": "typography",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "photoshop/solo: Multi-size ad campaign late copy changes",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "51-photoshop-deep-feature-delta.md type layers remain live editable vector text until an explicit `Convert to Shape/work path` command that 'ends text editability' (l.4696), and clipping/type-glyph workflows operate on the live type layer (l.4782, l.4729 all-selected type edit). Spec STU-RAS-008 (l.146) mandates non-destructive re-editable layers with rasterize only on explicit command emitting a distinct history entry; text is a shared StudioTextStory primitive (14.7); STU-RAS-043 (l.367) preserves vector/text through export where format supports. So late copy changes never force rebuilds and vector type stays sharp — covered by live type layers + explicit-only rasterization."
    },
    {
      "id": "NEED-073",
      "need": "Deterministic CSS-mappable layout model: flexbox/grid semantics and first-class breakpoint/responsive variants so designs rebuild or export to web builders without reinterpretation",
      "domain": "handoff",
      "scales": [
        "solo"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "figma/solo: Figma-to-Webflow client site build",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "21-figma-feature-map.md `auto_layout` 'Auto layout, constraints, responsive sizing, grids — responsive layout engine and constraints' (l.45-47) provides flexbox-equivalent semantics; spec studio_layout engine owns figma_frames/auto_layout/responsive_constraints (19 l.37). Deterministic CSS mapping: 55-figma-deep-feature-delta.md Dev Mode built-in codegen 'emits CSS and iOS/Android platform snippets' extensible via plugins (l.2276), Code Connect maps real component code, per-platform variable code syntax (l.1707, l.2386). Variants/variant-properties (spec 14.6). Thin part: no dedicated named 'breakpoint' primitive was found (responsive is expressed via constraints + variants rather than an explicit breakpoint feature), but the core CSS-mappable flexbox layout + CSS export is explicitly covered."
    },
    {
      "id": "NEED-074",
      "need": "Multi-up/n-up imposition output with bleed and marks acceptable to trade printers",
      "domain": "prepress",
      "scales": [
        "solo"
      ],
      "apps": [
        "affinity"
      ],
      "max_criticality": "BLOCKER",
      "scenario_examples": "affinity/solo: Business cards and badges to trade printers",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "54-affinity-deep-feature-delta.md `Print dialog layout models (document/booklet/N-up/tiled)` — 'N-Up (multiple copies per sheet) and Booklet (fold/staple imposition)... two-sided/short-edge binding, duplex... print-to-PDF' (l.4171-4174), the explicit affinity/solo distilled row. InDesign booklet imposition set (impose documents, booklet types, creep, preview) captured in source snapshots (adobe-indesign-desktop-jina.md l.777-782). Bleed/marks covered by print/PDF export prepress. Directly matches the affinity trade-printer scenario."
    },
    {
      "id": "NEED-075",
      "need": "Structured client deliverable package generation: full format/size/color permutation matrix, organized folder trees per usage/channel, manifests and format cheat-sheets, repeatable — not manual export-and-rename assembly",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/small: Studios buy plugins for delivery packaging today; affinity/small: The most error-prone step of identity delivery",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Export primitives covered, structured-package generator not. Multi-format/scale/artboard export: 52-illustrator-deep-feature-delta.md `Export for Screens: artboards/assets, scales, formats` (l.3321); Affinity Export Persona + Slices (02 l.64, 04 l.1186-1810); Photoshop Export As/Quick Export (01 l.119) + batch/asset export lineage (34 l.47); InDesign Package collects document+links+fonts into a portable folder with report (spec STU-LAY-058; format matrix row 29). BUT the need's distinguishing ask — a repeatable structured deliverable-package generator producing the full format/size/color permutation matrix organized into per-usage/channel folder trees with manifests and format cheat-sheets (the plugin-level packaging studios buy today, 'not manual export-and-rename assembly') — is not an explicit feature. Export-for-Screens and Package are adjacent building blocks; no permutation-matrix/manifest/cheat-sheet deliverable-assembly row found. max_criticality IMPORTANT."
    },
    {
      "id": "NEED-076",
      "need": "Effective-resolution awareness: expose ppi of placed images at final output size with low-res warnings before preflight/RIP rejection",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign",
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Catch low-res before the RIP proof; photoshop/large: Imaging desk catches low-res before preflight",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "InDesign deep-feature-delta 53-indesign-deep-feature-delta.md:2532-2535 'Link Info metadata pane' explicitly captures per-link 'resolution (actual and effective PPI), scale' as inspectable diagnostics state. Preflight is a first-class resolution-checking surface: 03-indesign-feature-map.md:66 (indesign.preflight 'Check documents for output issues'), 10-studio-command-contracts.md:234 studio.prepress.run_preflight command + StudioPreflightProfile, spec-modules/14-studio-creative-suite.md preflight section (~1058-1066) and separations preview per-ink coverage readouts (14 line 1115). Effective-PPI awareness + preflight low-res gating both present."
    },
    {
      "id": "NEED-077",
      "need": "Extensibility surface: third-party plugin ecosystem (including prepress classes like trapping and shrink distortion), scripting API, event/automation hooks (new-version triggers proofs/notifications), and status sync integrations with PM/traffic tools",
      "domain": "automation",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "illustrator",
        "affinity",
        "figma",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Packaging teams will not adopt a tool that closes the plugin door; cross-app/mixed: Jira/Confluence-class integrations",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Dedicated studio_extensibility engine module (56-studio-handshake-integration-architecture.md:844 engine map 'studio_collaboration, studio_model_tools, studio_extensibility'; 19-studio-local-first-rust-posture.md). Scripting/plugin APIs: Photoshop UXP scripts/plugins/hybrid (01-photoshop-feature-map.md:116), InDesign UXP/DOM/event scripting + headless Server automation (03-indesign-feature-map.md:82-85), Figma REST API/plugins/widgets/webhooks with event catalog (FILE_UPDATE, FILE_VERSION_UPDATE, LIBRARY_PUBLISH, DEV_MODE_STATUS_UPDATE) mapped to local event-ledger triggers (55-figma-deep-feature-delta.md:4257-4291). Automation hooks = per-command eventledger_event_family + command contracts (56 line 44). Status sync to PM/traffic: dev-resources external links GitHub/Jira/Storybook (55:2316). Trapping prepress class covered (53:2883 trap presets in-RIP trapping, 52:2235, 51:1240)."
    },
    {
      "id": "NEED-078",
      "need": "Contact-sheet/gallery proofing: set-level client select and review views with rating, per-image accept/reject and retouch annotations traced back to the exact source file among thousands of frames",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "affinity",
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "affinity/small: Client select galleries replacing screenshot-markup email; photoshop/solo: Change requests traced among ~1,000 delivered frames",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Component coverage present but not consolidated as a set-level client-select proofing gallery. Native review surface with comments/annotations/markup anchored to positions (spec-modules/14 STU-LAY-066 line 1132). Photoshop Camera Raw filmstrip star ratings/color labels/mark-for-deletion + Presentation view with rating (51-photoshop-deep-feature-delta.md:4368-4451) and Contact Sheet II / PDF Presentation batch (51:5260). InDesign create-contact-sheets leaf (07-indesign-leaf-index.md:265). No explicit feature for a client-select gallery with per-image accept/reject traced back to the exact source among thousands of frames as one review view."
    },
    {
      "id": "NEED-079",
      "need": "Presentation/prototype mode: pitch and walk through from working files (flows, transitions, speaker view, artboard presentation) with recorded decisions, without exporting to a deck tool",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "figma",
        "illustrator",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/large: Pitch from prototype without a deck tool; illustrator/large: Presentation-mode artboard review",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Figma prototyping + present mode: 'Present prototypes offline' (26-illustrator-figma-provider-posture-map.md:1990), figma prototype triggers/actions/animations (23-figma-leaf-index.md:1299-1342), studio_interaction engine owns slide_presentations (19-studio-local-first-rust-posture.md:42), spec 14.11 'Figma prototyping + Motion, InDesign interactive/EPUB'. InDesign 'Preview and present interactive documents' (07-indesign-leaf-index.md:3402). Recorded decisions via native review surface (14 STU-LAY-066) + EventLedger receipts."
    },
    {
      "id": "NEED-080",
      "need": "Mockup/visualization generation for sign-off: 3D fold-up from dielines, garment/vehicle/product surface mockups, and composited context previews that update from working art without manual re-export per revision",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "photoshop",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/solo: Live 3D fold-up preview for client review; illustrator/solo: Artwork onto garment templates without another app",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Illustrator Mockup panel 'Window > Mockup (Beta)' + create-mockups-for-images (52-illustrator-deep-feature-delta.md:3997-4004; 26-...:408 'Save mockups as templates') applies working art onto product/garment surfaces with live update. 3D fold-up from dielines served by Illustrator 3D and Materials: Extrude and Bevel, Revolve, Inflate (52:1886-1909). Composited context previews update from source art via the mockup construct."
    },
    {
      "id": "NEED-081",
      "need": "Enforced naming conventions for files and layers plus manifest generation (name, size, qty, vendor, allocation) — vendors, traffic systems, and downstream artists key on names; layer conventions enable cross-file element reuse",
      "domain": "file-organization",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Vendors and traffic systems key on filenames; photoshop/large: Any retoucher can open any teammate's file",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Layer/object organization metadata as canonical fields — freeform tags (incl. export-semantic), color labels, named layer states, with a find/query surface (spec-modules/14 STU-RAS-009 line 148); slash-path naming convention for components/styles (14 line 2081). Export/package manifests exist (05-studio-primitive-map.md:99,113 export_manifest/package_manifest; 12-cross-app-parity-matrix.md:106). But no enforced file/layer naming-convention validation gate, and no delivery manifest with name/size/qty/vendor/allocation columns as an explicit feature."
    },
    {
      "id": "NEED-082",
      "need": "Ready-for-dev status marking on frames/sections with engineer-filterable annotations and measurement callouts, so handoff state is visible inside the file and devs know approved vs exploratory",
      "domain": "handoff",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/small: Developers know what is approved vs exploratory; figma/large: Per-frame 'Ready for dev' status",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "figma.deep.dev-mode.ready-for-dev VERIFIED row (55-figma-deep-feature-delta.md:2322-2331): 'Sections, frames, and components are markable ready-for-dev; a dedicated view filters marked designs and status changes emit notifications and webhook events.' Plus Dev Mode inspect/measurements, dev-resources links (55:2312), focus view (55:2332), and native review annotations (spec 14 STU-LAY-066). Engineer-filterable approved-vs-exploratory state is explicit."
    },
    {
      "id": "NEED-083",
      "need": "Controlled late-change workflow: edits to locked/approved documents flagged so only affected pages need re-verification and re-approval; reflow safety showing pagination impact of late text edits",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "indesign/small: Annual report late changes before hard print date; indesign/solo: Late edits don't silently break running heads or TOC",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Reflow-safety components present: overset text detection + threaded text (03-indesign-feature-map.md:29), Adjust Layout recompute (spec 14 STU-LAY-010 line 858), smart text reflow (07:76), layout_reflow_trace diagnostic (05-studio-primitive-map.md:78). Native review + document-states/history beyond linear undo (14 STU-LAY-013 line 864, STU-LAY-066). But no explicit controlled late-change/approval-lock workflow that flags edits to locked/approved documents so only affected pages need re-verification/re-approval; searches for 'locked approved / late change / re-verification' returned no dedicated feature."
    },
    {
      "id": "NEED-084",
      "need": "Watermarked/low-res client proof generation (spec-size, dated) as a distinct output decoupled from press files and working masters",
      "domain": "review-approval",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "photoshop",
        "illustrator"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/solo: Watermarked client proofs separate from press files; illustrator/solo: Lo-res watermarked proof links per round",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "No Studio feature generating watermarked/low-res spec-size dated client proofs decoupled from press files. Only adjacent/raw mentions: retired Photoshop Digimarc watermark filter (51-photoshop-deep-feature-delta.md:2738 'treat as retired third-party surface'), Affinity->Capture One roundtrip preserving watermarks (54:5236), and InDesign UXP WatermarkPreference enum in raw snapshot (_source_snapshots/indesign-uxp-dom-api-jina.md:1496). StudioExportRecipe could produce a proof but no watermarked/dated lo-res proof output is specced. Searches: 'watermark', 'low-res proof', 'dated proof', 'comp proof'."
    },
    {
      "id": "NEED-085",
      "need": "Headless/CLI/watched-folder export and delivery integration: build pipelines and cloud storage/schedulers pull assets without a human clicking Export",
      "domain": "automation",
      "scales": [
        "solo",
        "large"
      ],
      "apps": [
        "illustrator",
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Engineering build pipeline pulls icon assets; figma/solo: Watch-folder export to cloud storage",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Headless/quiet export path: all render-to-output as StudioExportRecipe executions on the quiet/headless output path, observable background task with progress/cancel (spec-modules/14 STU-LAY-054 line 1048, headless/quiet law 14.20). Headless render harness runs as a kernel scheduler job with leases/backpressure + REST/MCP API surface (56-studio-handshake-integration-architecture.md:159, 24, 263). InDesign Server headless/unattended automation (03-indesign-feature-map.md:85). Watch Folders auto-import product surface (spec-modules/10-product-surfaces.md:5371). Build pipelines/schedulers pull via REST/MCP without a human clicking Export."
    },
    {
      "id": "NEED-086",
      "need": "Clean semantic SVG export: group/layer structure and IDs intentionally mirror the document, minimal cruft, controllable precision, no editor metadata — fit for direct developer consumption",
      "domain": "export",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Groups = code structure engineers touch; illustrator/small: Clean minimal markup from named artboards",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "SVG is a first-class export format across Illustrator/Figma/Affinity (export domain incl. svg — 37-affinity-source-distilled-domain-ledger.md:147, 36-illustrator-source-distilled-domain-ledger.md:142 'SVG/CSS export'; verification PDF/SVG_export_checks 05-studio-primitive-map.md:188). Resolution-independent geometry precision, coordinate encode only at API boundary (spec 14 STU-VEC-045 line 449); export metadata policy (12-cross-app-parity-matrix.md:103). But no explicit clause on clean semantic SVG with intentional group/layer/ID structure, controllable precision, minimal cruft, and no editor metadata for direct developer consumption."
    },
    {
      "id": "NEED-087",
      "need": "Multi-target export from one source document: press PDF/X, accessible tagged PDF (WCAG/PDF-UA), styled EPUB, hyperlinked interactive/screen PDF — without maintaining forked documents",
      "domain": "export",
      "scales": [
        "solo",
        "small",
        "large"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "indesign/small: Print PDF/X + accessible tagged PDF + screen spreads from one source; indesign/solo: Print PDF/X and valid EPUB from the same styled document",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "One-source multi-target export via StudioExportRecipe over a single document. Press PDF/X: General preset/PDF-X choice + Output PDF/X output intent, ink manager (spec-modules/14 lines 1097,1100,1112). Tagged/accessible PDF from style-to-tag mapping, per-object alt text/roles, reading/tab order, document title (STU-LAY-041 line 997, STU-LAY-061 line 1105). Reflowable + fixed-layout EPUB with full option set (STU-PRO-043b line 2454). Interactive/screen PDF owned by 14.11 (03-indesign-feature-map.md:59). Cross-app parity export recipes (12-cross-app-parity-matrix.md:98-106)."
    },
    {
      "id": "NEED-088",
      "need": "Screen-print production aids: white underbase/overprint control with visual verification, deliberate color reduction to N inks, halftone/gradient conversion guidance for limited-capability shops",
      "domain": "prepress",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "illustrator"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/solo: Underbase under inks on dark garments; illustrator/solo: Cheaper-ink-count variants of one design",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Overprint control covered (Affinity overprinting leaves 04:704/2246/3848, InDesign separations/inks/overprint 03:70). Halftone/gradient conversion covered: bitmap halftone-screen conversion method (spec 14 line 1415) and InDesign 'Specify halftone frequency and resolution' (07-indesign-leaf-index.md:4428). Deliberate color reduction to N inks via Ink Manager spot-to-process / all-spots-to-process (spec 14 line 1112). But screen-print white underbase generation with visual verification for dark garments is absent — 'underbase' returned no matches corpus-wide."
    },
    {
      "id": "NEED-089",
      "need": "Ink coverage/TAC checks and rich-black control for large solids, plus barcode quality validation before delivery",
      "domain": "prepress",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "photoshop",
        "indesign",
        "illustrator",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "cross-app/mixed: TAC and barcode validation pre-delivery; photoshop/large: Total-ink-limit warnings per press spec",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Ink coverage/TAC covered: Separations preview 'ink-limit view with a configurable total-ink threshold, per-ink coverage readouts' (spec-modules/14 line 1115) + Affinity preflight ink checks 'ink density over thresholds' (54-affinity-deep-feature-delta.md:3677-3680); ink_report diagnostic (05-studio-primitive-map.md:113). Rich-black control covered: Affinity preflight 'rich black violations' (54:3680), InDesign/Illustrator appearance-of-black rich-vs-accurate policy (53:2903, 53:4177, 52:4358). But barcode quality validation before delivery is NOT covered — 'barcode' matches only unrelated nutrition/product-surface modules, none in Studio corpus."
    },
    {
      "id": "NEED-090",
      "need": "Brand governance in-canvas: digital style guide surfaced next to the canvas (reference overlays, margin/crop guides, target values) and locked style-guide constraints (approved palettes, fonts, clear-space) that production artists cannot silently violate",
      "domain": "templates",
      "scales": [
        "small",
        "large"
      ],
      "apps": [
        "photoshop",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/large: Style guide next to canvas for cross-SKU consistency; affinity/large: Constraints artists cannot violate silently",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent primitives only. Grid/margin/column/ruler guides with per-guide lock and view thresholds (spec-modules/14 STU-LAY-050 line 1036), layer/object lock (14 STU-LAY-005 line 835, STU-VEC-072 line 488), native Studio asset library + StudioStyleRegistry/swatches (14 line 383, 997), preflight profile-violation rules. But no explicit digital style-guide-next-to-canvas surface (reference overlays / target values) and no locked brand-governance constraints (approved palettes/fonts/clear-space) that production artists cannot silently violate — searches 'style guide', 'brand guideline', 'clear-space', 'locked palette', 'constraint violation' returned no matches under studio scope."
    },
    {
      "id": "NEED-091",
      "need": "Flatplan/pagination board linked to actual documents (page slots -> files -> statuses) with drag-reorder of spreads that preserves numbering, sections, and continued-on jumps",
      "domain": "collaboration",
      "scales": [
        "solo",
        "large"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "indesign/large: Flatplan linked to files and statuses; indesign/solo: Reorder spreads without breaking numbering",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "spec-modules/14-studio-creative-suite.md STU-LAY-042 Books (bind multiple chapter StudioDocuments; add/remove/REORDER documents, designate style-source, show per-document status, open from list) + STU-LAY-015 auto page-number markers resolving section numbering + indesign.adjust_layout reflow (03-indesign-feature-map.md). These give reorder + per-doc status + preserved numbering at BOOK level, but no page-slot/spread flatplan board linked to individual page files with drag-reorder preserving continued-on jumps. Real professional need confirmed: dedicated flatplan software (flat-plan.com, Magazine Manager, Renaissance) syncs page status to InDesign as a distinct pagination board (WebSearch: creativepro.com 'Creating a Flatplan in InDesign', magazinemanager.com). Adjacent primitives present, flatplan surface not a named feature."
    },
    {
      "id": "NEED-092",
      "need": "Late-copy reconciliation: update already-styled stories from revised manuscripts (editorial re-link/merge) without re-flowing and re-styling from scratch",
      "domain": "collaboration",
      "scales": [
        "solo"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "indesign/solo: Revised manuscript merges into styled story",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14-studio-creative-suite.md STU-LAY-019 Linked stories: 'place-and-link a child copy of a story MUST show update state and support auto-update or warn-on-parent-change' + STU-TYP-007 placing external text (TXT/RTF/DOC/DOCX) into stories with 'style-map/preserve, and cleanup options'. Together these update already-styled stories from revised manuscripts via link-update/style-preserving re-import without re-flowing/re-styling from scratch. indesign.links_panel relinking (03-indesign-feature-map.md) reinforces the re-link path."
    },
    {
      "id": "NEED-093",
      "need": "Overwrite-in-place versioning: keep history of an image without changing its filename, because renaming breaks placed links downstream",
      "domain": "versioning",
      "scales": [
        "small"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/small: History without filename change inside an InDesign-led cycle",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "spec-modules/14-studio-creative-suite.md STU-RAS-012 placed_asset linked mode surfaces link health (up-to-date/modified/missing) with update-all — the placed link is preserved by the container, decoupled from filename. STU-LAY-013 document-states surface + kernel per-file history/undo (14.19) + EventLedger keeps version history WITHOUT rename; row 224 History/snapshot brush paints from prior states. Overwrite-in-place history without filename change is provided by kernel history + placed_asset link semantics."
    },
    {
      "id": "NEED-094",
      "need": "Geometry QA tooling at set scale: find stray points, open paths, off-grid anchors, and inconsistent stroke weights across hundreds of artboards; overset/overflow detection across generated frames",
      "domain": "automation",
      "scales": [
        "small",
        "large"
      ],
      "apps": [
        "illustrator",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: QA across 1,000 icon artboards; affinity/small: Overset detection across generated merge frames",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "52-illustrator-deep-feature-delta.md 'Object > Path > Clean Up' (deletes stray points, unpainted objects, empty text paths document-wide, line 1512), 'Select > Object > ... Stray Points' (line 1701), join open path ends (line 473); 53-indesign-deep-feature-delta.md Open/Close Path topology; overset detection primitive (05-studio-primitive-map.md overset_text_detector, 12-cross-app-parity-matrix.md). Covers stray points, open paths, overset/overflow across generated frames. NOT explicitly covered: batch geometry-QA report of off-grid anchors and inconsistent stroke weights across hundreds of artboards (only raw snapshot mentions off-grid anchor bug in illustrator-release-notes-jina.md)."
    },
    {
      "id": "NEED-095",
      "need": "Document hygiene tooling: find and purge unused swatches/symbols/artboards, simplify paths, and report what makes a file heavy",
      "domain": "performance",
      "scales": [
        "solo"
      ],
      "apps": [
        "illustrator"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/solo: Large-format files kept lean for wide-format vendors",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "41-illustrator-source-distilled-feature-rows.md Auto/Manual/Advanced Simplify paths (path simplification distilled); 51-photoshop-deep-feature-delta.md Edit > Purge (memory purge, not asset purge). Purge unused swatches/symbols/artboards and 'report what makes a file heavy' (document-info/file-size report) appear ONLY in raw _source_snapshots (illustrator-tools-jina.md, illustrator-supported-file-formats-jina.md matched 'unused'), not as distilled feature rows — grep for 'unused swatch|delete all unused' returned zero distilled hits (only 52-illustrator-deep-feature-delta raw). Simplify covered; unused-purge and heaviness reporting are snapshot-only."
    },
    {
      "id": "NEED-096",
      "need": "High-bit-depth pipeline ingest: EXR/16-bit TIFF, alpha channels, linear-to-display color handling from CGI/photo pipelines",
      "domain": "color",
      "scales": [
        "small",
        "large"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/small: Element ingest from CGI pipelines for key art",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "46-file-format-compatibility-registry.md format.exr_hdr (EXR/HDR) and format.tiff registered for import/export; 02-affinity-suite-feature-map.md affinity_photo.hdr_32bit_ocio '32-bit HDR / OpenEXR / OpenColorIO' merge/tone-map; 01-photoshop-feature-map.md photoshop.color.ocio 'OCIO/ACES color management, HDR/profile handling'; 04-affinity-leaf-index.md 32-bit OpenEXR support + linear/tone-mapping. High-bit-depth EXR/16-bit TIFF ingest, alpha, and linear-to-display handling from CGI/photo pipelines is addressed via format registry + color-management primitive."
    },
    {
      "id": "NEED-097",
      "need": "Component-to-production-code mapping (Code-Connect class) and component property documentation visible at inspection time, so handoff shows real component APIs, not generated CSS",
      "domain": "handoff",
      "scales": [
        "solo",
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/large: Handoff shows real component APIs; figma/solo: Property docs attached to the component",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md / 43-figma-source-distilled-feature-rows.md 'Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP' + 'Explore component properties' / 'Edit instances with component properties' + 55 'Component playground' (flip variants/properties). 21-figma-feature-map.md domain 'Dev Mode, inspect, Code Connect, MCP, REST, plugin/widget APIs'. Component-to-production-code mapping (Code Connect) and component property docs at inspection time are explicitly represented."
    },
    {
      "id": "NEED-098",
      "need": "Cross-org connected projects: agency and client workspaces collaborate on shared projects while each side uses its own seats and plan",
      "domain": "collaboration",
      "scales": [
        "small"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/small: Agency workspace <-> client workspace connected projects",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md figma.deep.organization-admin.workspaces (enterprise org subdivisions grouping teams/members), guest-management (scoped external access + admin roster), sharing-policy-defaults; spec-modules/14 STU-LAY-066 local-first CRDT review + adapter-backed 'Invite-to-edit / shared projects'. But cross-org 'connected projects' where agency and client each use their OWN seats/plan on a shared project is a distinct Figma feature (WebSearch: help.figma.com 'Guide to connected projects', launched H1 2025) not represented as a distilled surface. Guest/workspace primitives present; dual-org connected-projects model not addressed."
    },
    {
      "id": "NEED-099",
      "need": "Draft-then-move workflow: work starts in personal/draft space and moves into shared projects preserving history and library links; cross-file copy/merge of approved frames preserves library bindings",
      "domain": "collaboration",
      "scales": [
        "small"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/small: Freelancer drafts move into studio project intact",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md figma.deep.collaboration-and-files.file-hierarchy 'Teams/projects/files/drafts hierarchy ... personal drafts space; moving between spaces changes sharing defaults' + branch-model 'Branching with review and merge' (fork, diff against main, merge back, pull updates — history preserved) + '360-day/full version history' retention row; STU-VEC-058/STU-LAY design-system domain provides library binding preserved on copy/merge. Draft-then-move preserving history and library links is represented."
    },
    {
      "id": "NEED-100",
      "need": "Offboarding audit workflow: revoke external access, reclaim seats, and verify nothing remains shared — auditable rather than memory-dependent",
      "domain": "collaboration",
      "scales": [
        "small"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/small: Freelancer network offboarding",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md figma.deep.organization-admin.guest-management 'External guests receive scoped access to specific teams/files with an admin ROSTER of all guests' + sharing-policy-defaults (constrain guest access). This is the roster/scoped-access primitive but NOT an explicit offboarding audit workflow (revoke external access, reclaim seats, verify nothing remains shared) — WebSearch (figma.com/blog billing-freelancers-agencies) confirms this is a real pain: 'onus on admins to keep track', audit logs only on Enterprise. Auditable offboarding not a named surface."
    },
    {
      "id": "NEED-101",
      "need": "Design-system contribution governance: proposal/playground files, request-and-review forum loops, and contribution rules so consuming teams feed changes back to the system team",
      "domain": "collaboration",
      "scales": [
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "figma/large: Enterprise release-train contribution loop",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md figma.deep.dev-mode.component-playground 'sandboxed playground lets developers flip variants/properties to explore without touching the file' + collaboration-and-files.branch-model 'Branching with review and merge' (request reviews, diff, merge). These cover playground + review/merge mechanics but NOT design-system contribution GOVERNANCE (proposal/playground contribution files, request-and-review forum loops, contribution rules for consuming teams feeding changes back). WebSearch confirms this is a distinct professional workflow (figma.com/community Design System Component Contribution Playground File; designsystems.com 'How to govern a design system'). Governance loop not represented."
    },
    {
      "id": "NEED-102",
      "need": "Living brand-guidelines deliverable: paginated, versioned, style/token-linked guideline documents authored beside the artwork that never drift from actual brand values — replacing static PDF style guides; includes clearspace/min-size spec artboards",
      "domain": "client-deliverables",
      "scales": [
        "solo",
        "small",
        "large",
        "mixed"
      ],
      "apps": [
        "figma",
        "illustrator",
        "affinity",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "cross-app/mixed: Always-current guidelines replace static PDFs; figma/small: Guideline pages style-linked to brand tokens",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Primitives present in spec-modules/14: STU-LAY-042 Books (paginated, versioned multi-document publications), StudioStyleRegistry/StudioSwatch/graphic styles (STU-VEC-024/051) + Figma variables collections 'multi-brand token systems' (55-figma-deep-feature-delta.md line 1719) for style/token-linking, STU-LAY-066 local-first versioned review. But a named living brand-guidelines DELIVERABLE (paginated token-linked guideline doc that never drifts, replacing static PDFs, with clearspace/min-size spec artboards) is not a feature row. WebSearch confirms real need (brandyhq.com 'Living Brand Guidelines vs Static Style Sheets'; medium phaidra 'design your brand guide in Figma'). Token/layout primitives present, deliverable concept not addressed."
    },
    {
      "id": "NEED-103",
      "need": "Git-friendly design source: meaningful diffs or reliable file-per-asset granularity so design changes are reviewable in engineering repos",
      "domain": "versioning",
      "scales": [
        "large"
      ],
      "apps": [
        "illustrator"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "illustrator/large: Icon library changes reviewable in the repo",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Adjacent surfaces only: 55-figma-deep-feature-delta.md branch-model (diff changes against main, merge with conflict handling) + SVG per-asset import/export (46-file-format-compatibility-registry.md) + kernel EventLedger/per-file history (spec-modules/14 14.19). None address git-friendly design SOURCE with meaningful diffs or reliable file-per-asset granularity so Illustrator icon-library changes are reviewable in an engineering repo. Grep for 'git|meaningful diff|file-per-asset' hit only GitHub source-repo URLs, not a versioning capability. Git-repo diff-review not represented."
    },
    {
      "id": "NEED-104",
      "need": "Template marketplace and third-party template import: purchased template packs usable as production starting points, restylable to a brand without degradation",
      "domain": "templates",
      "scales": [
        "solo"
      ],
      "apps": [
        "indesign",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "indesign/solo: Buy/import a template and restyle it; affinity/solo: Purchased .aftemplate packs as starting points",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "41-illustrator-source-distilled-feature-rows.md 'Create documents using templates' (blank / Adobe Stock / Adobe Express templates) + 54-affinity-deep-feature-delta.md 'Documents save/open as reusable templates (.aftemplate) across the suite' + restyle via StudioStyleRegistry/StudioSwatch (spec-modules/14 STU-VEC-024/051). Template open/import and restyling-to-brand are covered. NOT covered: a template MARKETPLACE and purchased third-party template-pack import as a governed production starting point (marketplace is provider commerce; no distilled row for buying/importing packs restylable without degradation)."
    },
    {
      "id": "NEED-105",
      "need": "Campaign/master-to-template promotion: promote last cycle's structure to a reusable template with clear separation of evergreen template vs per-campaign content, enabling event-to-event and month-over-month content-swap reuse",
      "domain": "templates",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "affinity"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/solo: Reuse last campaign's structure next cycle; illustrator/solo: Next year's show is a content swap, not a rebuild",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "55-figma-deep-feature-delta.md 'Spreadsheet import maps columns onto named template fields and generates one asset per row for campaign-scale production' + 'channel-sized variants (social formats) for campaign delivery'; spec-modules/14 master/parent pages (STU-LAY-008), custom-text reusable placeholders (STU-LAY, single-edit updates all instances), template save (NEED-104 evidence). Data-merge content-swap reuse is represented. NOT covered: explicit promote-last-cycle-master-to-reusable-template with clear evergreen-template vs per-campaign-content separation for event-to-event / month-over-month reuse as a named promotion workflow."
    },
    {
      "id": "NEED-106",
      "need": "Migration tooling from incumbent suites: template/style/swatch migration so studio scaffolding is rebuilt once, font substitution reporting and mapping, and documented per-format fidelity limits so studios can quote migration risk",
      "domain": "migration",
      "scales": [
        "mixed"
      ],
      "apps": [
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "cross-app/mixed: Adobe-to-Affinity cost-driven migration",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "The migration substrate is covered but not framed as an incumbent-suite migration/onboarding workflow. Corpus: 00-preamble.md (implementation_gate + mitigation require round-trip expectations, unsupported-feature diagnostics, lossy-conversion diagnostics per import/export), 46-file-format-compatibility-registry.md and 50-proprietary-format-fixture-plan.md (native/import/export/round-trip/feature-level fidelity records, unsupported-feature receipts, .sketch fidelity limits e.g. 55-figma-deep-feature-delta.md 'styles not retained, must recreate'), 39/41 source rows + 06/22 leaf indexes cover 'Replace missing fonts' / 'Preview, add, or replace missing fonts' (font substitution). Spec: STU-LAY-058 Package. What is missing as an explicit feature: a bundled template/style/swatch migration path so studio scaffolding is rebuilt once, a font-substitution mapping/report deliverable, and a documented per-format fidelity ledger positioned so studios can quote migration risk. Fidelity limits exist as round-trip receipts; migration-as-adoption-workflow does not."
    },
    {
      "id": "NEED-107",
      "need": "Predictable licensing/business model: perpetual, one-time, or self-hosted options adoptable by freelance networks and small studios without per-seat subscription bleed",
      "domain": "business-model",
      "scales": [
        "small",
        "mixed"
      ],
      "apps": [
        "affinity",
        "cross-app"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "cross-app/mixed: Licensing as explicit adoption driver; affinity/small: Perpetual license drove small-studio adoption",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Handshake's own licensing/business model is not specified in the search surfaces; the adjacent posture is. Spec module 14 has no business-model/perpetual/self-host/pricing section (grep for 'business model|perpetual|self-host|pricing|per-seat|freelance|adoption' = No matches), but the local-first, no-account, no-subscription-DEPENDENCY posture (STU-OVR-002, referenced by STU-COL-006, STU-COL-020, STU-DS-049, STU-IO-013, STU-AUT-023) removes per-seat subscription bleed at runtime. Corpus captured competitor pricing as observation only: 54-affinity-deep-feature-delta.md rows '3.0 free-forever pricing (no paid license)' replacing '~USD 70-per-app one-time purchase model'. No corpus/spec row defines Handshake's perpetual/one-time/self-hosted commercial model as an explicit adoption driver."
    },
    {
      "id": "NEED-108",
      "need": "Job/XMP-class metadata read/write carried through the pipeline (SKU, style-guide ref, status) so files travel with job context and no human retypes it",
      "domain": "asset-management",
      "scales": [
        "large"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "IMPORTANT",
      "scenario_examples": "photoshop/large: Files carry SKU and status metadata through the retouch queue",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Metadata primitives exist but XMP/IPTC job-metadata read/write carried as first-class job context is not an explicit feature. Spec: STU-LAY line 1028 'Document Fields | Author and other document metadata fields, plus user-defined custom variables', STU-TYP-023 live computed text variables, line 1025/1355 'Image Name (Metadata Caption)' pulling metadata from placed images into live captions, format table (lines 2686-2695) 'metadata/ICC preserved'. Corpus: 01-photoshop-feature-map.md Content Credentials (PS-S16), 09/11 Affinity Metadata panel (provider-posture compatibility shim), 10-studio-command-contracts.md diagnostics 'asset_metadata'/'include_metadata'. Missing: explicit XMP/IPTC read-write of SKU/style-guide-ref/status job fields that travel through the retouch/pipeline with no retype — custom variables + metadata preservation are adjacent, not a dedicated job-metadata pipeline feature."
    },
    {
      "id": "NEED-109",
      "need": "Licensed-asset tracking: which stock images and fonts a deliverable embeds, with license scope (media, territory, duration) attached for legal handoff to the client",
      "domain": "asset-management",
      "scales": [
        "solo"
      ],
      "apps": [
        "photoshop",
        "illustrator",
        "indesign"
      ],
      "max_criticality": "NICE_TO_HAVE",
      "scenario_examples": "photoshop/solo: Stock/font license scope for book-cover handoff; illustrator/solo: Usage/licensing record attached to deliverables",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Licensed-asset tracking with license scope (media/territory/duration) attached for legal handoff is not covered as a feature; only adjacent asset-dependency and font-licensing awareness exist. Spec: STU-LAY-058 Package 'collect the document, its linked resources, and its fonts (subject to font-licensing) into a portable folder with a report' (font-licensing acknowledged, not tracked with scope); STU-AUT-025 asset browser with bulk metadata/keyword editing. Corpus: 10-studio-command-contracts.md / 14-feature-use-card-schema.md placed-asset dependency records (link status, asset_metadata, export_manifest inclusion); 'Use Adobe Stock images' rows (07/11/40) are a stock-USE provider adapter, not license-scope tracking. No row records stock/font license scope (media/territory/duration) as a deliverable-attached legal record."
    },
    {
      "id": "NEED-110",
      "need": "Secure exchange of unreleased-property assets: watermarking and access control on shared working files",
      "domain": "collaboration",
      "scales": [
        "large"
      ],
      "apps": [
        "photoshop"
      ],
      "max_criticality": "NICE_TO_HAVE",
      "scenario_examples": "photoshop/large: Unreleased entertainment key art exchange",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Access control is partially represented as a local permission model and PDF export protection; watermarking on working files is explicitly NOT shipped. Spec: STU-FX-022 and table row (line 1847) 'Watermark (retired) ... NOT a native Studio effect. Provenance-only row'. Access control: corpus 55-figma-deep-feature-delta.md 'sharing-link-permissions' (view/edit/dev-mode roles, password-protected) and 'Seat types permission model' captured as 'the role-capability matrix is directly relevant to Studio's local permission model'; 54-affinity-deep-feature-delta.md 'PDF passwords and permission restrictions' (open/modify/print passwords) — but that is PDF-export protection, not access control on shared working files. No watermarking or DRM on unreleased-property working files; secure-exchange-of-working-files as a workflow is not covered."
    },
    {
      "id": "NEED-111",
      "need": "Multi-GB file transfer/delivery to vendors with integrity verification and delivery confirmation",
      "domain": "handoff",
      "scales": [
        "solo",
        "small"
      ],
      "apps": [
        "photoshop",
        "illustrator"
      ],
      "max_criticality": "NICE_TO_HAVE",
      "scenario_examples": "photoshop/small: Multi-GB delivery with integrity check; illustrator/solo: Large-file transfer path to wide-format vendors",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Integrity verification of delivered files is covered; multi-GB transport/transfer with delivery confirmation is not. Spec: STU-LAY-058 Package (portable handoff folder with report); STU-VAL-001/STU-VAL-004 exported-file integrity checks. Corpus: 56-studio-handshake-integration-architecture.md line 520 'exported-file integrity (hash + format validation, round-trip receipts...)' and lines 75/364/386 document-integrity checks. What is missing: a multi-GB file transfer/delivery path to vendors and delivery-confirmation/receipt of transport — the integrity (hash) piece exists but the transfer/delivery-confirmation transport concern has no feature row. 54-affinity-deep-feature-delta.md notes '.af export ... watermarks and metadata preserved' (unrelated transport)."
    },
    {
      "id": "NEED-112",
      "need": "Cross-deliverable dependency awareness: linked parameters between deliverables (e.g., cover spine width depends on interior page count and stock) flag each other on change",
      "domain": "prepress",
      "scales": [
        "solo"
      ],
      "apps": [
        "indesign"
      ],
      "max_criticality": "NICE_TO_HAVE",
      "scenario_examples": "indesign/solo: Spine width flags when interior page count changes",
      "coverage_verdict": "PARTIALLY_COVERED",
      "coverage_evidence": "Reactive-dependency primitives exist but cross-deliverable parametric linkage that flags on change (spine width vs interior page count/stock) is not an explicit feature. Spec: STU-LAY-042 Books 'bind multiple chapter StudioDocuments into one publication unit that shares numbering and output ... per-document status'; indesign.cross_references (03-indesign-feature-map.md) 'update references as targets move or change'; STU-TYP-023 computed text variables update every instance when edited; STU-DS-028/STU-PRO-025/STU-PRO-026 variable expressions, conditionals, and expression/alias cycle validation errors; 09-affinity-desktop-delta.md Cross-References panel + Data merge. These give within/cross-document reactive updates and computed variables, but no row models a linked parameter between separate deliverables (cover spine width derived from interior page count + stock) that flags each deliverable on change."
    },
    {
      "id": "NEED-113",
      "need": "Component usage documentation export/sync so guidance lives next to (or is generated from) the library itself",
      "domain": "handoff",
      "scales": [
        "large"
      ],
      "apps": [
        "figma"
      ],
      "max_criticality": "NICE_TO_HAVE",
      "scenario_examples": "figma/large: Docs generated from the design-system library",
      "coverage_verdict": "COVERED",
      "coverage_evidence": "Spec module 14 explicitly binds usage documentation to the library itself. STU-DS-004: 'Every StudioComponent and StudioComponentInstance MUST carry an optional description and documentation-link field, surfaced in the operator asset browser, the instance inspector, and the model/UserManual surfaces. Components, styles ([STU-DS-024]), and variables ([STU-DS-013]) share this description contract.' STU-DS-050: design-system analytics (component/variable/style usage counts, adoption, detachment rates, orphaned-instance reports) 'available as a locally computed report over the authority rows', with cloud/team aggregation as optional adapter. STU-DS-002/019 library publish-consume loop works offline. Corpus provenance: 38-figma-source-distilled-domain-ledger.md design-systems/team-libraries. Guidance living next to (and generated from) the library — description/documentation-link field surfaced across browser/inspector/UserManual plus locally computed usage report — is directly specified."
    }
  ]
}
```
