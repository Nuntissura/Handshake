---
file_id: studio-app-feature-research-gap-resolution-notes
topic_id: SFR-GAP-NOTES
title: "Gap Resolution Notes"
status: draft
summary: "Resolved, mitigated, and residual evidence gaps for the Studio app feature research corpus."
sources: 61
updated_at: "2026-07-09"
---

## [SFR-GAP-NOTES] Gap Resolution Notes

### [SFR-GAP-NOTES.resolved] Gaps Resolved In This Pass

```yaml
resolved_gaps:
  - id: "SFR-GAP-004"
    original_issue: "Adobe Photoshop and InDesign broad desktop index pages timed out from direct shell downloads."
    resolution: "Fetched local markdown snapshots through Jina Reader using the official Adobe Help URLs, then generated leaf indexes from those snapshots."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/adobe-photoshop-desktop-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/adobe-indesign-desktop-jina.md"
      - ".GOV/reference/studio_app_feature_research/06-photoshop-leaf-index.md"
      - ".GOV/reference/studio_app_feature_research/07-indesign-leaf-index.md"
    counts:
      photoshop_total_help_leaves: 461
      photoshop_feature_leaves: 441
      photoshop_support_context_leaves: 20
      indesign_total_help_leaves: 603
      indesign_feature_leaves: 542
      indesign_support_context_leaves: 61
  - id: "SFR-GAP-007"
    original_issue: "The corpus did not yet encode the operator requirement that Studio is built in, local-first, no-cloud-required, and Rust-forward."
    resolution: "Added an explicit posture file and wired it into the index, preamble, feature maps, parity lanes, and Feature Use Cards."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md"
      - ".GOV/reference/studio_app_feature_research/index.yaml"
      - ".GOV/reference/studio_app_feature_research/00-preamble.md"
  - id: "SFR-GAP-008"
    original_issue: "Illustrator and Figma were not covered by the earlier Photoshop/Affinity/InDesign corpus."
    resolution: "Added source-backed feature maps, leaf indexes, Feature Use Cards, provider posture, and parity lanes for Illustrator and Figma-family products."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/20-illustrator-feature-map.md"
      - ".GOV/reference/studio_app_feature_research/21-figma-feature-map.md"
      - ".GOV/reference/studio_app_feature_research/22-illustrator-leaf-index.md"
      - ".GOV/reference/studio_app_feature_research/23-figma-leaf-index.md"
      - ".GOV/reference/studio_app_feature_research/24-illustrator-feature-use-cards.md"
      - ".GOV/reference/studio_app_feature_research/25-figma-feature-use-cards.md"
      - ".GOV/reference/studio_app_feature_research/26-illustrator-figma-provider-posture-map.md"
      - ".GOV/reference/studio_app_feature_research/27-illustrator-figma-parity-matrix.md"
    counts:
      illustrator_total_help_leaves: 532
      illustrator_feature_cards: 515
      figma_feature_cards: 200
      illustrator_figma_provider_rows: 344
      total_feature_use_cards_all_apps: 2730
  - id: "SFR-GAP-011"
    original_issue: "Adobe product card counts were being read as if they were true all-tools/all-features counts."
    resolution: "Corrected the framing: online sources are sufficient for source-distilled feature/tool inventory. Added methodology, per-app online-source ledgers, optional installed enrichment scripts, and a unified online-source feature ledger."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/28-adobe-count-methodology.md"
      - ".GOV/reference/studio_app_feature_research/29-photoshop-expanded-count-ledger.md"
      - ".GOV/reference/studio_app_feature_research/30-indesign-expanded-count-ledger.md"
      - ".GOV/reference/studio_app_feature_research/31-illustrator-expanded-count-ledger.md"
      - ".GOV/reference/studio_app_feature_research/32-adobe-installed-ui-export-playbook.md"
      - ".GOV/reference/studio_app_feature_research/33-online-source-distilled-feature-ledger.md"
      - ".GOV/reference/studio_app_feature_research/34-photoshop-source-distilled-domain-ledger.md"
      - ".GOV/reference/studio_app_feature_research/35-indesign-source-distilled-domain-ledger.md"
      - ".GOV/reference/studio_app_feature_research/36-illustrator-source-distilled-domain-ledger.md"
      - ".GOV/reference/studio_app_feature_research/37-affinity-source-distilled-domain-ledger.md"
      - ".GOV/reference/studio_app_feature_research/38-figma-source-distilled-domain-ledger.md"
      - ".GOV/reference/studio_app_feature_research/39-photoshop-source-distilled-feature-rows.md"
      - ".GOV/reference/studio_app_feature_research/40-indesign-source-distilled-feature-rows.md"
      - ".GOV/reference/studio_app_feature_research/41-illustrator-source-distilled-feature-rows.md"
      - ".GOV/reference/studio_app_feature_research/42-affinity-source-distilled-feature-rows.md"
      - ".GOV/reference/studio_app_feature_research/43-figma-source-distilled-feature-rows.md"
  - id: "SFR-GAP-012"
    original_issue: "Affinity coverage must be included without creating confusing duplicate Adobe-labeled implementation scope."
    resolution: "Added a generated cross-app overlap and Affinity dedupe map. It groups shared source behavior under Handshake-native Studio primitives while retaining Affinity source rows as source-specific variants."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/44-cross-app-overlap-and-affinity-dedupe-map.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-cross-app-dedupe-map.py"
    counts:
      total_source_distilled_feature_rows: 2730
      primitive_overlap_domains: 21
      affinity_dedupe_domains: 10
  - id: "SFR-GAP-013"
    original_issue: "The corpus had feature rows and domain ledgers, but no dedicated cross-app tool/surface registry for tool-level rebuild planning."
    resolution: "Added a generated source-distilled tool registry from the app domain ledgers. Rows preserve source app provenance, source domain, tool/surface kind, Studio primitive grouping, Affinity overlap guard, and implementation-readiness status."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/45-source-distilled-tool-registry.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-source-distilled-tool-registry.py"
    counts:
      tool_registry_rows: 1219
      unique_normalized_labels: 876
      cross_app_name_overlaps: 125
  - id: "SFR-GAP-014"
    original_issue: "The corpus needed a dedicated file-format compatibility registry so Studio preserves existing creative formats instead of inventing a replacement interchange format."
    resolution: "Added a generated source-distilled format compatibility registry covering native, import, export, round-trip, domain-level, and feature-level compatibility records."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/46-file-format-compatibility-registry.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-file-format-compatibility-registry.py"
    counts:
      compatibility_records: 410
      format_families: 38
      native_format_records: 15
      domain_format_records: 23
      feature_format_records: 372
  - id: "SFR-GAP-015"
    original_issue: "The corpus needed an implementation-facing bridge from research records to future local-first Rust Studio module work."
    resolution: "Added a generated Studio Rust implementation backlog grouping feature rows, tool registry rows, and compatibility records into primitive lanes and build slices."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/47-studio-rust-implementation-backlog.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-studio-rust-implementation-backlog.py"
    counts:
      backlog_items: 28
      build_slices: 5
      feature_rows: 2730
      tool_registry_rows: 1219
      compatibility_records: 410
  - id: "SFR-GAP-016"
    original_issue: "Provider, cloud, AI, collaboration, hosted API, runtime, automation, and compatibility-adjacent rows needed one cross-app offline parity registry so Studio stays local-first while preserving source behavior."
    resolution: "Added a generated provider/offline parity registry that classifies selected source-distilled rows into local-first primitive, optional provider adapter with offline fallback, compatibility shim with receipts, local collaboration/model surface, local runtime/automation surface, and local model/provider-adapter postures."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/48-provider-offline-parity-registry.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-provider-offline-parity-registry.py"
    counts:
      provider_offline_parity_rows: 1061
      source_feature_rows: 2730
      source_apps: 5
      offline_parity_classes: 6
  - id: "SFR-GAP-017"
    original_issue: "The corpus needed an auditable coverage matrix before claiming that all source-distilled rows have behavior text, use intent, source refs, provider/file-format posture, manual handoff, command-contract refs, verification refs, and cross-registry linkage."
    resolution: "Added a generated source coverage verification matrix with one row per source-distilled feature row."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/49-source-coverage-verification-matrix.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-source-coverage-verification-matrix.py"
    counts:
      coverage_rows: 2730
      missing_required_fields: 0
      url_and_local_snapshot_rows: 2730
      url_only_rows: 0
      provider_offline_registry_selected_rows: 1061
      provider_posture_or_runtime_adjacent_rows: 693
  - id: "SFR-GAP-018"
    original_issue: "Illustrator and Figma generated feature rows had source URLs but no local source snapshot path references in their per-row source_refs."
    resolution: "Added snapshot-path resolution to the source-distilled row generator, added explicit Figma manual-row URL overrides, patched the upstream Illustrator/Figma research generator to emit stable snapshot names, captured the missing Figma snapshots, and regenerated the dependent ledgers."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/_tools/generate-source-distilled-feature-rows.py"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-illustrator-figma-research.py"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-figjam-guide-to-figjam-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-figjam-import-export-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-figjam-spreadsheet-data-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-figjam-media-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-ai-section-jina.md"
      - ".GOV/reference/studio_app_feature_research/_source_snapshots/figma-draw-section-jina.md"
      - ".GOV/reference/studio_app_feature_research/49-source-coverage-verification-matrix.md"
    counts:
      illustrator_rows_with_local_snapshot_path: 515
      figma_rows_with_local_snapshot_path: 200
      url_only_rows_remaining: 0
  - id: "SFR-GAP-019"
    original_issue: "Proprietary/native creative file formats were identified as compatibility targets but did not yet have a concrete fixture, round-trip, unsupported-feature receipt, and Rust lane plan."
    resolution: "Added a generated proprietary format fixture plan from the file-format compatibility registry."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/50-proprietary-format-fixture-plan.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-proprietary-format-fixture-plan.py"
    counts:
      format_fixture_plan_rows: 15
      source_format_families: 38
      source_compatibility_records: 410
      required_support_directions:
        - import
        - edit_preserve
        - export
        - round_trip
  - id: "SFR-GAP-020"
    original_issue: "Help-TOC leaf indexes missed the sub-article surface (complete tool sets, menu trees, filter/effect catalogs, adjustment/blend-mode enumerations, Camera Raw and Develop control sets, panel inventories, dialog option catalogs, preferences panes, scripting/plugin/REST API domains), and the Figma family was covered by only 200 rows. The Operator judged the corpus incomplete for all-tools/all-features rebuild planning."
    resolution: "Ran a per-app deep-delta pass below the help-TOC level with per-row dedupe status against the existing leaf indexes, added the Studio-Handshake integration architecture (pillar wiring, model visibility, visual steerability, parallel multi-file/multi-model workflows, propose-work system, per-file history/undo with revert-of-undo, visual inspection duty, headless/quiet law, operator unification surface, dual-audience UserManual strategy), and generated a cross-app overlap map over the deep rows for no-double-features grouping."
    artifacts:
      - ".GOV/reference/studio_app_feature_research/51-photoshop-deep-feature-delta.md"
      - ".GOV/reference/studio_app_feature_research/52-illustrator-deep-feature-delta.md"
      - ".GOV/reference/studio_app_feature_research/53-indesign-deep-feature-delta.md"
      - ".GOV/reference/studio_app_feature_research/54-affinity-deep-feature-delta.md"
      - ".GOV/reference/studio_app_feature_research/55-figma-deep-feature-delta.md"
      - ".GOV/reference/studio_app_feature_research/56-studio-handshake-integration-architecture.md"
      - ".GOV/reference/studio_app_feature_research/57-deep-delta-cross-app-overlap-map.md"
      - ".GOV/reference/studio_app_feature_research/_tools/generate-deep-delta-overlap-map.py"
    counts:
      deep_delta_rows_total: 2304
      photoshop_deep_rows: 576
      illustrator_deep_rows: 447
      indesign_deep_rows: 416
      affinity_deep_rows: 440
      figma_deep_rows: 425
      integration_architecture_records: 66
      deep_delta_overlap_groups: 160
      corpus_feature_rows_total_after_pass: 5034
      verification_pass: "All deep rows source-confirmed VERIFIED except 4 documented-unreachable residuals (3 InDesign menu enumerations, 1 Figma Motion .fig posture); each carries a residual_reason."
      completeness_audit: "Two rounds. Round 1 (Adobe-centric top-level surface classes) found 3 Photoshop ecosystem-posture holes + missing Figma Motion. Round 2 (app-unique features with no cross-app equivalent + cross-cutting capability depth + whole-product surfaces, 130+ classes) found only InDesign Balance Ragged Lines. All findings filled and verified; round 2 otherwise returned dry across all five apps."
```

### [SFR-GAP-NOTES.unresolved] Remaining Unresolved Gaps

```yaml
remaining_gaps:
  - id: "SFR-REMAINING-GAP-001"
    title: "Leaf rows are not implementation contracts."
    status: "mitigated"
    detail: "A reusable command-contract schema and 12 seed commands now exist, but individual vendor leaves still require promotion before implementation."
    mitigation: "Use 10-studio-command-contracts.md as the promotion gate for high-priority leaves."
    residual_verification_needed: "For each promoted leaf, inspect the source page and map behavior to Rust engine module, state model, deterministic tests, and model-facing command receipt."
  - id: "SFR-REMAINING-GAP-002"
    title: "Affinity desktop-specific leaf parity remains partial."
    status: "mitigated"
    detail: "Direct desktop Affinity contents.xml paths still return 403, but desktop index.html pages were reachable and parsed into 1,035 raw desktop leaf rows, 1,032 stable unique feature IDs, and 234 path-based desktop-only delta records."
    mitigation: "Use 09-affinity-desktop-delta.md for desktop delta planning."
    residual_verification_needed: "Capture desktop contents.xml later only if exact vendor-TOC fidelity is required; page-level desktop-vs-iPad behavior differences still need inspection when promoting a leaf."
  - id: "SFR-REMAINING-GAP-006"
    title: "Feature purpose and use coverage is generated, not behavior-inspected."
    status: "mitigated"
    detail: "Generated Feature Use Cards now cover every stable source-backed leaf feature ID in the current Photoshop, Affinity, InDesign, Illustrator, and Figma inventories: 441 Photoshop cards, 1,032 Affinity cards, 542 InDesign cards, 515 Illustrator cards, and 200 Figma cards."
    mitigation: "Use 15-photoshop-feature-use-cards.md, 16-affinity-feature-use-cards.md, 17-indesign-feature-use-cards.md, 24-illustrator-feature-use-cards.md, 25-figma-feature-use-cards.md, and 18-feature-use-card-manual-handoff-index.md as the planning bridge into command contracts and internal Studio UserManual topics."
    residual_verification_needed: "Each implemented feature still needs exact source-page or app-behavior inspection, a typed Rust command contract, fixtures/tests, receipts/diagnostics, and a same-change internal Studio UserManual update."
  - id: "SFR-REMAINING-GAP-007"
    title: "Figma category crawl depth is uneven."
    status: "mitigated"
    detail: "Largely closed by the SFR-GAP-020 deep-delta pass: 55-figma-deep-feature-delta.md adds 400 rows covering design core/auto layout, vector networks and Draw, typography, components/variables, full prototyping, Dev Mode/Code Connect/MCP server, FigJam, Slides/Sites/Buzz/Make, collaboration, the Plugin API node catalog, REST API domains, AI surface, and org/admin posture."
    mitigation: "Use 55-figma-deep-feature-delta.md alongside 21/23/25/38/43 as the current Figma inventory."
    residual_verification_needed: "156 Figma deep rows remain UNVERIFIED (leaf-URL-anchored but not individually fetched); inspect at command-contract promotion time."
  - id: "SFR-REMAINING-GAP-008"
    title: "Proprietary AI and Figma-family file schemas are compatibility targets, not implementation contracts."
    status: "mitigated"
    detail: "Official sources identify supported/importable/exportable formats, but do not provide stable full implementation schemas for proprietary .ai, .fig, .jam, .deck, .buzz, .site, .make, native Affinity, native InDesign, and related native/local-copy formats."
    mitigation: "Use 50-proprietary-format-fixture-plan.md. It defines fixture families, support directions, unsupported-feature receipt fields, round-trip assertions, Rust implementation lanes, and manual-topic requirements for 15 native/proprietary/local-copy targets."
    residual_verification_needed: "Collect the actual fixture corpus during implementation, run import/export/edit-preserve/round-trip tests, and document every unsupported or shimmed feature in receipts and the Studio UserManual."
  - id: "SFR-REMAINING-GAP-009"
    title: "Source-distilled ledgers need generation into per-feature rows."
    status: "mitigated"
    detail: "Generated per-app source-distilled feature rows now exist for all current Feature Use Cards: 441 Photoshop rows, 542 InDesign rows, 515 Illustrator rows, 1,032 Affinity rows, and 200 Figma rows."
    mitigation: "Use 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md as the source-distilled per-feature row ledgers. Regenerate them with _tools/generate-source-distilled-feature-rows.py after card or domain-ledger updates."
    residual_verification_needed: "Before product implementation, promote each selected row through exact source-page behavior inspection, command-contract acceptance criteria, file-format fixtures where applicable, provider/offline posture tests, and same-change Studio UserManual entries."
  - id: "SFR-REMAINING-GAP-010"
    title: "Illustrator and Figma row-level local source snapshot paths are absent."
    status: "resolved"
    detail: "The source coverage matrix now shows 2,730 URL-and-local-snapshot rows and zero URL-only rows. Illustrator has 515 rows with local snapshot paths; Figma has 200 rows with local snapshot paths."
    mitigation: "Regenerate 39 through 49 after changing source refs to keep the coverage matrix current."
    residual_verification_needed: "None for row-level local snapshot path coverage. Exact behavior inspection is still required before product implementation."
  - id: "SFR-REMAINING-GAP-003"
    title: "Adobe direct-shell snapshots remain blocked."
    status: "mitigated"
    detail: "Direct curl/Invoke-WebRequest to helpx.adobe.com timed out, while the same official URLs were readable through web reader and Jina Reader."
    mitigation: "Keep the local Jina snapshots and official URLs together. Treat Jina as the local evidence copy, not a separate source of product truth."
    verification_needed: "If source fidelity is challenged, capture the same TOCs through a real browser export and diff against the Jina snapshots."
  - id: "SFR-REMAINING-GAP-004"
    title: "AI/cloud/collaboration behavior is provider-dependent."
    status: "mitigated"
    detail: "Provider-affected source rows across Photoshop, InDesign, Illustrator, Affinity, and Figma have been classified into provider/offline parity rows."
    mitigation: "Use 48-provider-offline-parity-registry.md as the current cross-app implementation gate for AI/cloud/collaboration/provider-adjacent behavior. Use 11-provider-posture-map.md and 26-illustrator-figma-provider-posture-map.md as source posture inputs."
    residual_verification_needed: "Provider mocks, offline behavior tests, receipt schemas, and fallback UX are still required when implementing each provider-backed feature."
  - id: "SFR-REMAINING-GAP-005"
    title: "Research corpus is not product authority."
    status: "open"
    detail: "The corpus now contains contracts, parity lanes, provider posture, and a vertical slice, but those artifacts remain reference/planning material until accepted into a Work Packet, spec, or implementation authority."
    mitigation: "Promote the selected vertical slice into the appropriate Handshake work packet/spec authority before coding."
    residual_verification_needed: "Work packet acceptance criteria, test commands, and product-code proof gates."
```

### [SFR-GAP-NOTES.next] Recommended Next Closure Unit

```yaml
next_closure_unit:
  id: "studio-layer-graph-work-packet"
  base_scope: "Promote 13-layer-graph-vertical-slice.md into a formal Handshake work packet/spec change before coding."
  high_roi_additions:
    - item: "Create a command-contract schema shared by raster, vector, layout, typography, export, automation, and AI primitives."
      why_high_roi: "Prevents the feature inventory from turning into one-off app clones."
      gap_closed: "Mitigated by 10-studio-command-contracts.md."
      reuse: "Use 05-studio-primitive-map.md, 10-studio-command-contracts.md, and the existing Handshake EventLedger/CRDT/state-authority concepts."
      validation: "Work-packet schema lint plus vertical-slice command acceptance gates."
    - item: "Add provider posture fields for AI/cloud/collaboration leaves."
      why_high_roi: "Makes local-vs-provider scope explicit before engineering starts."
      gap_closed: "Mitigated by 11-provider-posture-map.md."
      reuse: "Use source_ids, primitive_domain fields, and provider_posture rows already present in the corpus."
      validation: "Every implemented provider-backed command has posture, receipt, offline behavior, and fallback tests."
    - item: "Generate a parity matrix view over all category and leaf indexes."
      why_high_roi: "Lets humans and models compare Photoshop, Affinity, and InDesign without rereading long files."
      gap_closed: "Mitigated by 12-cross-app-parity-matrix.md."
      reuse: "Use parity_id rows as future work-packet seed IDs."
      validation: "Every generated work packet links to one parity_id and one command-contract set."
```

### [SFR-GAP-NOTES.sources] Sources

```yaml
sources:
  - { id: GAP-S01, url: "https://helpx.adobe.com/photoshop/desktop.html", note: "Official Adobe Photoshop desktop help TOC." }
  - { id: GAP-S02, url: "https://helpx.adobe.com/indesign/desktop.html", note: "Official Adobe InDesign desktop help TOC." }
  - { id: GAP-S03, path: "_source_snapshots/adobe-photoshop-desktop-jina.md", note: "Local Jina Reader snapshot of GAP-S01." }
  - { id: GAP-S04, path: "_source_snapshots/adobe-indesign-desktop-jina.md", note: "Local Jina Reader snapshot of GAP-S02." }
  - { id: GAP-S05, path: "_source_snapshots/*2ipad-contents.xml", note: "Official Affinity V2 iPad help XML snapshots used for generated Affinity leaf coverage." }
  - { id: GAP-S06, path: "09-affinity-desktop-delta.md", note: "Affinity desktop index.html delta artifact." }
  - { id: GAP-S07, path: "10-studio-command-contracts.md", note: "Command-contract schema and seed commands." }
  - { id: GAP-S08, path: "11-provider-posture-map.md", note: "Provider posture classifications." }
  - { id: GAP-S09, path: "12-cross-app-parity-matrix.md", note: "Primitive-centered cross-app parity matrix." }
  - { id: GAP-S10, path: "13-layer-graph-vertical-slice.md", note: "First vertical slice proof contract." }
  - { id: GAP-S11, path: "14-feature-use-card-schema.md", note: "Feature Use Card schema and seed examples." }
  - { id: GAP-S12, path: "15-photoshop-feature-use-cards.md", note: "Generated Photoshop Feature Use Cards." }
  - { id: GAP-S13, path: "16-affinity-feature-use-cards.md", note: "Generated Affinity Feature Use Cards." }
  - { id: GAP-S14, path: "17-indesign-feature-use-cards.md", note: "Generated InDesign Feature Use Cards." }
  - { id: GAP-S15, path: "18-feature-use-card-manual-handoff-index.md", note: "Generated Studio UserManual handoff grouping." }
  - { id: GAP-S16, path: "19-studio-local-first-rust-posture.md", note: "Local-first, no-cloud-required, Rust-forward posture for Studio." }
  - { id: GAP-S17, path: "20-illustrator-feature-map.md", note: "Illustrator feature map." }
  - { id: GAP-S18, path: "21-figma-feature-map.md", note: "Figma feature map." }
  - { id: GAP-S19, path: "22-illustrator-leaf-index.md", note: "Illustrator generated leaf index." }
  - { id: GAP-S20, path: "23-figma-leaf-index.md", note: "Figma generated leaf index." }
  - { id: GAP-S21, path: "24-illustrator-feature-use-cards.md", note: "Generated Illustrator Feature Use Cards." }
  - { id: GAP-S22, path: "25-figma-feature-use-cards.md", note: "Generated Figma Feature Use Cards." }
  - { id: GAP-S23, path: "26-illustrator-figma-provider-posture-map.md", note: "Illustrator/Figma provider posture classifications." }
  - { id: GAP-S24, path: "27-illustrator-figma-parity-matrix.md", note: "Illustrator/Figma parity matrix." }
  - { id: GAP-S25, path: "28-adobe-count-methodology.md", note: "Adobe expanded count methodology." }
  - { id: GAP-S26, path: "29-photoshop-expanded-count-ledger.md", note: "Photoshop expanded count ledger." }
  - { id: GAP-S27, path: "30-indesign-expanded-count-ledger.md", note: "InDesign expanded count ledger." }
  - { id: GAP-S28, path: "31-illustrator-expanded-count-ledger.md", note: "Illustrator expanded count ledger." }
  - { id: GAP-S29, path: "32-adobe-installed-ui-export-playbook.md", note: "Optional installed UI enrichment playbook." }
  - { id: GAP-S30, path: "33-online-source-distilled-feature-ledger.md", note: "Unified online-source feature/tool ledger across Photoshop, InDesign, Illustrator, Affinity, and Figma." }
  - { id: GAP-S31, path: "34-photoshop-source-distilled-domain-ledger.md", note: "Photoshop online-source-distilled domain ledger." }
  - { id: GAP-S32, path: "35-indesign-source-distilled-domain-ledger.md", note: "InDesign online-source-distilled domain ledger." }
  - { id: GAP-S33, path: "36-illustrator-source-distilled-domain-ledger.md", note: "Illustrator online-source-distilled domain ledger." }
  - { id: GAP-S34, path: "37-affinity-source-distilled-domain-ledger.md", note: "Affinity online-source-distilled domain ledger." }
  - { id: GAP-S35, path: "38-figma-source-distilled-domain-ledger.md", note: "Figma online-source-distilled domain ledger." }
  - { id: GAP-S36, path: "39-photoshop-source-distilled-feature-rows.md", note: "Photoshop source-distilled feature rows." }
  - { id: GAP-S37, path: "40-indesign-source-distilled-feature-rows.md", note: "InDesign source-distilled feature rows." }
  - { id: GAP-S38, path: "41-illustrator-source-distilled-feature-rows.md", note: "Illustrator source-distilled feature rows." }
  - { id: GAP-S39, path: "42-affinity-source-distilled-feature-rows.md", note: "Affinity source-distilled feature rows." }
  - { id: GAP-S40, path: "43-figma-source-distilled-feature-rows.md", note: "Figma source-distilled feature rows." }
  - { id: GAP-S41, path: "_tools/generate-source-distilled-feature-rows.py", note: "Generator for source-distilled feature row ledgers." }
  - { id: GAP-S42, path: "44-cross-app-overlap-and-affinity-dedupe-map.md", note: "Cross-app overlap and Affinity dedupe map." }
  - { id: GAP-S43, path: "_tools/generate-cross-app-dedupe-map.py", note: "Generator for the overlap and Affinity dedupe map." }
  - { id: GAP-S44, path: "45-source-distilled-tool-registry.md", note: "Source-distilled cross-app tool registry." }
  - { id: GAP-S45, path: "_tools/generate-source-distilled-tool-registry.py", note: "Generator for the source-distilled tool registry." }
  - { id: GAP-S46, path: "46-file-format-compatibility-registry.md", note: "Source-distilled file-format compatibility registry." }
  - { id: GAP-S47, path: "_tools/generate-file-format-compatibility-registry.py", note: "Generator for the file-format compatibility registry." }
  - { id: GAP-S48, path: "47-studio-rust-implementation-backlog.md", note: "Source-distilled Studio Rust implementation backlog." }
  - { id: GAP-S49, path: "_tools/generate-studio-rust-implementation-backlog.py", note: "Generator for the Studio Rust implementation backlog." }
  - { id: GAP-S50, path: "48-provider-offline-parity-registry.md", note: "Provider/offline parity registry." }
  - { id: GAP-S51, path: "_tools/generate-provider-offline-parity-registry.py", note: "Generator for the provider/offline parity registry." }
  - { id: GAP-S52, path: "49-source-coverage-verification-matrix.md", note: "Source coverage verification matrix." }
  - { id: GAP-S53, path: "_tools/generate-source-coverage-verification-matrix.py", note: "Generator for the source coverage verification matrix." }
  - { id: GAP-S54, path: "_source_snapshots/figma-figjam-guide-to-figjam-jina.md", note: "Local Figma Guide to FigJam snapshot." }
  - { id: GAP-S55, path: "_source_snapshots/figma-figjam-import-export-jina.md", note: "Local Figma FigJam import/export snapshot." }
  - { id: GAP-S56, path: "_source_snapshots/figma-figjam-spreadsheet-data-jina.md", note: "Local Figma FigJam spreadsheet-data snapshot." }
  - { id: GAP-S57, path: "_source_snapshots/figma-figjam-media-jina.md", note: "Local Figma FigJam media snapshot." }
  - { id: GAP-S58, path: "_source_snapshots/figma-ai-section-jina.md", note: "Local Figma AI section snapshot." }
  - { id: GAP-S59, path: "_source_snapshots/figma-draw-section-jina.md", note: "Local Figma Draw section snapshot." }
  - { id: GAP-S60, path: "50-proprietary-format-fixture-plan.md", note: "Proprietary/native/local-copy format fixture plan." }
  - { id: GAP-S61, path: "_tools/generate-proprietary-format-fixture-plan.py", note: "Generator for the proprietary/native/local-copy format fixture plan." }
```
