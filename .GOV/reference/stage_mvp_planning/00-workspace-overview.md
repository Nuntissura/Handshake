---
file_id: stage-mvp-planning-overview
file_kind: reference-planning-note
updated_at: "2026-07-20"
status: full-feature-planning-hardening
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-planning-overview" status="full-feature-planning-hardening" version="v0.3" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# Handshake Stage MVP planning workspace

## Purpose

This workspace supports iterative expansion, detailing, consolidation, and refactoring of the Stage module plan. The target is one large `WP-1-Handshake-Stage-MVP-v1` contract with a dependency-aware microtask graph exceeding 100 microtasks.

## Working product direction

The operator's core framing is deliberately simple (`STAGE-DEC-020`): Stage is a normal browser. Its one defining performance requirement is that 3,000+ tabs do not tax the CPU. Everything else a normal browser does stays the same unless another Handshake module's contract explicitly overrides it. Downloads are never stopped, frozen, or made dormant by any lifecycle mechanism (`STAGE-DEC-019`). All planning detail below serves that core; complexity is only justified by the tab-scale requirement, a Handshake-module integration, or a proven security need.

- The primary operator workflow is a high-volume web research and asset-intake queue: discover videos, images, and other assets; download or capture them through governed jobs; and ingest the resulting artifacts into Handshake projects for Lens/Atelier and related consumers.
- Stage must treat thousands of tabs as lightweight durable records rather than thousands of continuously live webviews. CPU and memory use should scale mainly with the small active working set, not the saved-tab count.
- Servo is the strategic and eventual default Stage renderer.
- Chromium is a minimal first-usable bootstrap, not a co-equal feature backend.
- Net-new Stage feature development targets Servo.
- Headless-agent and restricted-document operation are planned as Servo execution profiles rather than separate browser engines.
- The transition must not create silent fallback, hidden engine switching, or shared profile-state assumptions.
- A complete production-qualified WebView2 Windows slice remains the truthful near-term release path only while it passes its viability gate: authenticated YouTube through governed session import must work in WebView2, or the Chromium bootstrap is abandoned and all effort focuses on Servo (`STAGE-DEC-021`). Whole-WP closure requires everything proven and Chromium retired or demoted (`STAGE-DEC-022`).
- The current Stage plan supersedes all older Stage-specific code, schemas, APIs, adapters, connectors, routes, panes, and mockups, even when already built. Those assets may be inspected or salvaged only when they independently satisfy a current requirement; they do not define compatibility or architecture.
- Shared Handshake systems outside Stage remain dependencies where the current plan deliberately selects them. Real operator data receives an explicit retain/export/import/delete disposition, but data protection does not require retaining old Stage implementations.
- Full-feature planning includes ordinary browser workflows, authentication compatibility, agent prompt-injection/data-flow security, production persistence/events, migration, backup/recovery, packaging/updates/supply chain, accessibility/localization, support, and future-authoring schemas.

## Initial consolidation boundary

The verified Stage-owned source stubs are:

- `.GOV/task_packets/stubs/WP-1-Handshake-Stage-MVP-v1.contract.json`
- `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.contract.json`
- `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.contract.json`

Media Downloader, Media Downloader Loom Bridge, ASR Transcribe Media, Video Archive/Loom, Artifact System Foundations, Storage Trait Purity, and Atelier/Lens remain separate product or foundation work. Their Stage-facing obligations must be represented without absorbing their full scope.

## Full-feature planning completion conditions

Initial planning is concluded only when:

1. Every requirement from the three Stage-owned stubs has a current-plan disposition; source lineage is retained without granting old Stage behavior or implementation continuing authority.
2. The Servo-primary and Chromium-bootstrap boundaries are explicit.
3. Current research supports the renderer, security, compatibility, privacy, and agent-control decisions.
4. Risks, failure scenarios, mitigations, acceptance gates, and test surfaces are defined.
5. Stable full-feature requirements map to release slices, lanes, gates, and evidence.
6. Product topology, shared-system reuse, active-WP supersession, and any real-data disposition are inspected and future revalidation requirements are explicit.
7. Persistence, events, concurrency, migration, backup, recovery, packaging, update, supply-chain, support, accessibility, localization, and manual contracts are defined.
8. The expanded future 100+ MT decomposition has stable lanes, allocator rules, conflict groups, and future contract schemas without prematurely freezing official MT IDs.
9. Proposed master-spec enrichments are assembled but remain deferred.
10. The operator locks or corrects the architecture, release slices, authentication path, worksurface, packaging baseline, and measured targets before authority authoring begins.

## Non-goals for this workspace phase

- Editing the master spec.
- Activating a work packet for implementation.
- Creating product code.
- Treating the Chromium bootstrap as Stage completion.
- Editing or deleting superseded source stubs before the later coordinated archival/supersession transaction is approved.

</topic>
