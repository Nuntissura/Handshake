---
schema: handshake.indexed_spec.module@1
spec_version: "v02.197"
bundle_id: "master-spec-v02.197"
module_id: "00"
section_id: "0"
title: "Lines before section 1"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "8f9c13f0cdf9b9004395c842d5ba21a335cb6d978b0c5ab73a5ca9a01df096fa"
body_sha256: "dea2f6dd101000884187e953a72bf368683ebea732b372a8a3041161148e78cb"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
          .%:                                                                         .#@-          
        .:@@@@%:..                                                                ..#@@@@@+.        
       .:@@@@@@@@@-.                                                         ...:%@@@@@@@@@*..      
      .+@@@@@@@@@@@=....             ........       ...:-=++=-:....        ..=@::@@@@@@@@@@@#.      
     .*@@@@@@@@@@@:.=@@@@+:....:-+%@@@@@@@@@@@@=..:%@@@@@@@@@@@@@@@@@%##%@@@@@@@-.%@@@@@@@@@@@.     
   ..#@@@@@@@@@@@:.#@@@@@@@@@@@@@@@@@@@@@@@@#:.:*@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@-.#@@@@@@@@@@@:.   
  ..%@@@@@@@@@@%..%@@@@@@@@@@@@@@@@@@@@@@%-..=@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@+.*@@@@@@@@@@@:.  
 .:@@@@@@@@@@@# .%@@@@@@@@@@@@@@@@@@@@@= .=@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@#.=@@@@@@@@@@@=. 
.:@@@@@@@@@@@#.:@@@@@@@@@@@@@@@@@@@@*..-%@@@@@@@@@@@@@%-...=@@@@@@@@@@@@@@@@@@@@@@@@#.-@@@@@@@@@@@+.
-@@@@@@@@@@@+.-@@@@@@@@@@@@@@@@@@@* .#@@@@@@@@@@@@@*...=@@%:.=@@@@@@@@@@@@@@@@@@@@@@@%.-@@@@@@@@@@@*
.*@@@@@@@@@-.-@@@@@@@@@@@@@@@@@@@@.:@@@@@@@@@@@%=..-#@@@@@@@%..=@@@@@@@@@@@@@@@@@@@@@@@:.%@@@@@@@#:.
  ..*@@@@@-.+@@@@@@@@@@@@@@@@@@@@@.:@@@@@@@@*...=@@@@@@@@@@@@@#. =@@@@@@@@@@@@@@@@@@@@@@:.#@@@#:.   
     ..+@:.*@@@@@@@@@@@@@@@@@@@@@@*..*@@#=..-*@@@@@@@@@@@@@@@@@@#..+@@@@@@@@@@@@@@@@@@@@@-.+:.      
          +@@@@@@@@@@@@@@@@@@@@@@@@@+-..:=%@@@@@@@@@@@@@@@@@@@@@@@#..+@@@@@@@@@@@@@@@@@@@=          
          ..%@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@#..+@@@@@@@@@@@@@@@+.           
            .:@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@*..+@@@@@@@@@@@#.             
             ..=@@@*++*@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@*..+@@@@@@@%:               
                ...-++-.:%@@@%++#@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@*..*@@@@-.                
                .+@@@@@@.:@-..=+:.:%@@@@@@@@@@@@@@@@@@@@@@@@@@@@%..+@@@@@@@@*..*=..                 
               .%@@@@@@@:..-%@@@@@.:@%++*@@@@@@@@@@@@@@@@@@@@@@@@@#..=@@@@@@@@*.                    
               .*@@@@@+..=@@@@@@@@:...=+:.:%@@@@@@@@@@@@@-.:%@@@@@@@#..=@@@@@@@@=.                  
                .:==-..=@@@@@@@@#:.:%@@@@@:.%@@@@@@@@@@@@@@-.:%@@@@@@@#..=@@@@@@%.                  
                      .@@@@@@@*..=@@@@@@@@-.%@@@@*..#@@@@@@@@:.:%@@@@@@@*..=@@@#..                  
                      ..%@@@-..*@@@@@@@@%:.+@@@@@@@+..#@@@@@@@%-.:@@@@@@@. .....                    
                         ....#@@@@@@@@*.....=@@@@@@@@+..#@@@@@@@@:.:%@@@-.                          
                           .-@@@@@@@-..*@@@%..=@@@@@@@@*..*@@@@@@*.......                           
                           ..#@@@@:..#@@@@@@* ..=@@@@@@@@=..*@@@*..                                 
                            ....... :@@@@@@@:  ...=@@@@@@%. .....                                   
                                    .+@@@@:.       .=@@@@:.                                         







# Handshake Master Specification v02.186

**Status:** LIVING  
**Version:** v02.186
**Date:** 2026-05-18
**Authority:** [CX-001] (The Master Spec is the Source of Truth)

**Purpose:** Complete reference combining product vision, Diary governance, extraction pipeline, Phase 1 closure requirements, and technical supply-chain gate specs.

---

## CHANGELOG (Recent)

Machine-readable changelog authority for this versioned bundle: `../spec-changelog.jsonl`.

| Version | Date | Author | Changes | Approval |
|---------|------|--------|---------|----------|
| v02.197 | 2026-06-28 | Orchestrator | Clarified Stage/Studio boundary: Stage is the governed browser/webviewer, trusted HTML/Stage App host, lightweight tab/session backlog, model-operated front-end test surface, capture/import/provenance lane, and visual/3D review handoff surface; Studio Module owns Figma/Spline-like collaboration, canvas, photo/painting/compositing, Affinity/Photoshop-class tools, and future full visual authoring. Refined native Rust rule to forbid a webview-hosted app shell while allowing controlled Stage WebView/browser islands. [ADD v02.197] | Operator proposal approved in chat, 2026-06-28 |
| v02.186 | 2026-05-18 | KERNEL_BUILDER | KERNEL-004 4-cluster max-fold enrichment: SandboxAdapter trait + 3 adapters (Â§3.5/3.6); Â§4.2.4 rewrite removing Ollama-as-primary; ModelRuntime + LocalModelAdapter (Â§4.6); Inference Research Lab boundary + 8 PRODUCTION techniques (Â§4.7); Distillation opt-in + Memory V0+ self-improvement loop (Â§4.8); HBR build+handoff gate (Â§5.6); ProcessOwnershipLedger (Â§5.7); visual debugger (Â§6.4), backend inspector plane (Â§6.5), non-hijacking GUI invariants (Â§6.6), swarm-agent harness (Â§6.7), MoD preliminary research (Â§6.8); product surfaces Â§10.12-10.15 (Diagnostics panel, ModelRuntime control panel, Inference Lab UI, ModelManual); Appendix 12 primitive + interaction edge additions; new Codex CX-131 HARD_HBR_BUILD_HANDOFF_GATE anchor. [ADD v02.186] | Operator-locked KERNEL-004 scope; v02.186 enrichment proposal approved 2026-05-18 |
| v02.185 | 2026-05-14 | Kernel Builder | Added Kernel002 authority law: KernelActionCatalogV1, WriteBoxV1, direct-edit denial, advisory edit normalization, CRDT workspace draft persistence, and CRDT-to-EventLedger promotion bridge with DCC projection requirements. [ADD v02.185] | ilja140520260455 |
| v02.184 | 2026-05-13 | Kernel Builder | Added Kernel V1 authority law: Postgres EventLedger as product runtime authority, SessionBroker/ContextBundle/ModelAdapter boundary, no-SQLite Kernel V1 authority/cache/offline/fallback/test permission, and projection-only Flight Recorder/DCC/diagnostics posture for kernel replay and promotion. [ADD v02.184] | Operator approval in chat, 2026-05-13; WP activation signature pending |
| v02.183 | 2026-05-13 | Orchestrator | Migrated the active indexed Master Spec into the copy-first versioned bundle `.GOV/spec/master-spec-v02.183/`, moved the previous indexed bundle to `.GOV/spec/spec_archive/master-spec-v02.182/`, added uniform module `spec_version` metadata, added a manifest-declared machine-readable changelog module, updated `.GOV/spec/SPEC_CURRENT.md`, and refreshed internal references away from latest-monolith/version-file wording. [ADD v02.183] | Operator approval in chat, 2026-05-13 |
| v02.182 | 2026-05-05 | Activation Manager | Added PostgreSQL-primary control-plane foundation law: explicit storage modes, PostgreSQL-authoritative self-hosted runtime records, fail-closed behavior when PostgreSQL is required, SQLite cache/offline boundaries, downstream split for queue workers, leases/backpressure, FEMS memory store, workflow durable execution, DCC projections, SQLite fallback boundaries, and developer/test container setup; updated Appendix 12 feature, primitive, and interaction metadata for the pivot. [ADD v02.182] | APPROVE REFINEMENT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 |
| v02.181 | 2026-04-17 | Orchestrator | Added software-delivery governance overlay law: product-owned runtime truth over imported repo `/.GOV/**`, validator-gate convergence on top of Governance Check Runner, projection-only Dev Command Center / Task Board / Role Mailbox posture, derived closeout semantics, overlay claim/lease and queued-instruction extension records, explicit overlay lifecycle constraints, and workflow-backed start/steer/cancel/close/recover control-plane law; updated Appendix 12 / roadmap follow-through for the affected feature families. [ADD v02.181] | pending |
| v02.180 | 2026-04-07 | Orchestrator | Added 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) -- typed CheckResult contract (PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED), tool surface governance.check.run, FR events FR-EVT-GOV-CHECK-001..003, additive overlay rule. [ADD v02.180] | pending |
| v02.179 | 2026-03-28 | Orchestrator | **Workflow-correlation bundle-scope pass:** patched Debug Bundle export law so `workflow_run` and `workflow_node_execution` become first-class bounded scopes, added workflow-node execution inventory plus manifest-count rules, extended exporter and exportable-inventory posture, deepened FEAT-DEBUG-BUNDLE UI guidance for workflow-scoped export, and kept roadmap/cov-matrix scheduling aligned with the existing Workflow Projection Correlation backlog. | ilja280320262308 |
| v02.178 | 2026-03-11 | Orchestrator | **RAG mode and no-RAG cross-pillar pass:** clarified that RAG is one governed retrieval mode rather than the default context strategy; added retrieval-mode and non-hybrid-reason law across AI-Ready Data, ACE Runtime, Project Brain, Prompt-to-Spec Router, Loom, Work Packets, and Micro-Task Executor; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for authoritative direct-load, graph-first, and bounded local-model retrieval posture; and materialized a dedicated retrieval-mode policy stub. | ilja110320261228 |
| v02.177 | 2026-03-11 | Orchestrator | **Role Mailbox handoff-bundle and announce-back provenance pass:** defined structured handoff bundles, announce-back provenance, note-transcription duties, and compact handoff summaries across Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for durable handoffs; and materialized a dedicated mailbox handoff/transcription/announce-back stub. | ilja110320260813 |
| v02.176 | 2026-03-11 | Orchestrator | **Role Mailbox executor-routing and claim-lease pass:** defined mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture across Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Work Packet System, and Task Board; deepened Appendix 12 ownership, coverage, UI guidance, and interaction notes for claimant-aware parallel work; and materialized a dedicated mailbox executor-routing and claim-lease stub. | ilja110320260021 |
| v02.175 | 2026-03-11 | Orchestrator | **Role Mailbox triage and queue-control pass:** defined mailbox triage queue state, reminder schedules, snooze and expiry posture, dead-letter remediation, and operator-facing remediation controls across Role Mailbox, Dev Command Center, Task Board, Work Packet System, and Locus Work Tracking; deepened Appendix 12 ownership, coverage, UI guidance, and interaction edges for queue aging and recovery; and materialized a dedicated mailbox-triage-and-queue-controls stub. | ilja110320260002 |
| v02.174 | 2026-03-10 | Orchestrator | **Role Mailbox and Micro-Task loop-control pass:** defined verifier-driven mailbox loop checkpoints, structured verifier outcomes, bounded retry and escalation posture, and completion-report transcription duties across Role Mailbox, Micro-Task Executor, Locus Work Tracking, Work Packet System, Task Board, and Dev Command Center; and deepened Appendix 12 ownership, coverage, UI guidance, and stub mapping while materializing a dedicated mailbox-loop-control stub. | ilja100320262233 |
| v02.173 | 2026-03-10 | Orchestrator | **Role Mailbox message-contract pass:** defined typed Role Mailbox message families, thread-lifecycle states, delivery states, allowed-response envelopes, and Micro-Task collaboration message groundwork; clarified that mailbox actions request governed mutations instead of mutating linked work directly; and deepened Appendix 12 ownership, coverage, UI guidance, and stub mapping across Locus Work Tracking, Work Packet System, Task Board, Micro-Task Executor, and Dev Command Center. | ilja100320261756 |
| v02.172 | 2026-03-10 | Orchestrator | **Workflow transition matrix, queue automation, and executor eligibility pass:** defined portable workflow transition rules, queue automation rules, and executor eligibility policy contracts for Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked waits, and Dev Command Center action previews; clarified automatic versus approval-gated transitions; strengthened local-small-model, cloud-model, reviewer, and operator eligibility boundaries; and deepened Appendix 12 ownership, coverage, and stub mapping for transition-automation registry implementation gaps. | ilja100320261658 |
| v02.171 | 2026-03-10 | Orchestrator | **Project-agnostic workflow-state and governed-action contract pass:** defined one shared workflow-state family, queue-reason vocabulary, and governed-action descriptor for Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked work, and Dev Command Center queues; clarified how project-profile extensions may customize labels without forking base semantics; strengthened operator and local-small-model routing law around explicit state reasons instead of board position or prose; and deepened Appendix 12 ownership, coverage, and stub mapping for workflow-state registry implementation gaps. | ilja100320261443 |
| v02.170 | 2026-03-10 | Orchestrator | **Dev Command Center typed-viewer, board-layout, and queue-projection pass:** defined view presets, lane definitions, and governed action bindings for board, queue, list, roadmap, inbox-triage, and execution-queue layouts; clarified local-small-model readiness queues and drag or quick-action semantics; strengthened Dev Command Center field-provenance and projection-behavior law; and deepened Appendix 12 ownership, coverage, and stub mapping for layout-projection implementation gaps. | ilja100320260238 |
| v02.169 | 2026-03-10 | Orchestrator | **Canonical-to-mirror synchronization and drift-governance pass:** defined mirror authority modes, reconciliation actions, and mirror contracts for structured collaboration artifacts; clarified how canonical JavaScript Object Notation records, compact summaries, Markdown mirrors, and note sidecars synchronize without creating silent second authorities; strengthened Dev Command Center mirror-drift, regeneration, and normalization posture; and deepened Appendix 12 ownership, coverage, and stub reuse for Markdown mirror sync and typed viewer implementation gaps. | ilja100320260217 |
| v02.168 | 2026-03-10 | Orchestrator | **Base structured schema and project-profile contract pass:** defined the shared base envelope, compact summary contract, and mirror-state semantics for structured collaboration artifacts; clarified project-profile extension boundaries for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports; strengthened Dev Command Center field provenance and summary ingestion rules; and deepened Appendix 12 ownership, coverage, and stub mapping for schema-registry and profile-extension implementation gaps. | ilja100320260150 |
| v02.167 | 2026-03-10 | Orchestrator | **Canonical structured artifact backend pass:** defined the versioned JSON file standard for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports; clarified Markdown mirrors as readable derivatives plus note sidecars; added projected board and queue rules for Dev Command Center; introduced project-profile extensibility so structured collaboration artifacts do not stay repository-specific; and deepened Appendix 12 ownership, coverage, and stub mapping for the structured artifact family. | ilja100320260112 |
| v02.166 | 2026-03-10 | Orchestrator | **Structured collaboration-substrate backend pass:** clarified canonical structured records versus human-readable Markdown mirrors for Work Packets, Micro-Tasks, Task Board, and Role Mailbox; strengthened append-only note, handoff, and routing contracts for local small models and cloud models; added Dev Command Center structured record and collaboration-inbox viewing posture; and deepened Appendix 12 ownership, UI guidance, and interaction edges for structured work-state projection while reusing the existing Dev Command Center, Locus, Role Mailbox, and workflow backlog. | ilja100320260032 |
| v02.165 | 2026-03-09 | Orchestrator | **Dev Command Center operating-surface backend pass:** sharpened Dev Command Center as the governed operating surface for run history, tool infrastructure, workspace runtime, and promotion gates; patched Main Body law for replay-safe run history, tool/server health visibility, and protected-branch readiness; deepened Appendix 12 ownership for replay chronology, tool-health status, workspace readiness, and merge-readiness snapshots; and added Dev Command Center matrix growth for workflow replay and tool-infrastructure health while reusing the existing Dev Command Center, workflow projection, unified tool surface, workspace safety, and repository-decision backlog. | ilja090320262346 |
| v02.164 | 2026-03-09 | Orchestrator | **Dev Command Center resilience and governed repository-decision backend pass:** patched recovery, provider-readiness, anti-pattern, and repository-engine decision law in place; deepened Appendix 12 ownership for session checkpoints, span-linked recovery state, provider capability readiness, and governed version-control policy; and added Dev Command Center matrix edges for session recovery and repository-engine policy while reusing the existing Dev Command Center, model-session, provider-coverage, and repository-decision backlog. | ilja090320262320 |
| v02.163 | 2026-03-09 | Orchestrator | **Dev Command Center planning and coordination backend pass:** patched Dev Command Center planning-and-coordination law in place; backfilled Task Board and Work Packet System appendix ownership; deepened planning projection across Dev Command Center, Locus Work Tracking, Workflow Engine, Model Session Orchestration, and Micro-Task Executor; and added backend matrix edges for Dev Command Center -> Task Board, Dev Command Center -> Work Packet System, Task Board -> Locus Work Tracking, and Work Packet System -> Workflow Engine while reusing existing Dev Command Center, Locus, and Spec Session Log backlog. | ilja090320261949 |
| v02.162 | 2026-03-09 | Orchestrator | **Dev Command Center work orchestration backend pass:** patched Dev Command Center work-orchestration law in place; deepened Appendix 12 ownership for tracked work packets, task board sync freshness, micro-task summaries, ready-query state, and parallel model session occupancy; and added backend matrix edges for Dev Command Center -> Locus Work Tracking, Dev Command Center -> Micro-Task Executor, and Model Session Orchestration -> Locus Work Tracking while reusing the existing Dev Command Center, Locus, and multi-session backlog. | ilja090320261642 |
| v02.161 | 2026-03-09 | Orchestrator | **Dev Command Center evidence and replay backend pass:** patched Dev Command Center evidence-and-replay law in place; deepened Appendix 12 ownership for governance export, workspace bundle export, diagnostics query state, and bounded evidence packaging; and added backend matrix edges for Dev Command Center -> Governance Pack, Dev Command Center -> Workspace Bundle, and Dev Command Center -> Diagnostics Schema while reusing the existing governance, workspace bundle, and diagnostics backlog. | ilja090320261600 |
| v02.160 | 2026-03-09 | Orchestrator | **Dev Command Center control-plane backend pass:** patched Dev Command Center control-plane law in place; deepened Appendix 12 ownership for workflow runs, artificial intelligence jobs, capabilities and consent, model session orchestration, and work packet or worktree binding; and added backend matrix edges for Dev Command Center -> Workflow Engine, Dev Command Center -> Artificial Intelligence Job Model, Dev Command Center -> Capabilities and Consent, and Dev Command Center -> Model Session Orchestration while reusing the existing Dev Command Center, workflow-correlation, and consent-audit backlog. | ilja090320261423 |
| v02.159 | 2026-03-09 | Orchestrator | **Correlation/projection backend pillar pass:** patched Dev Command Center, Operator Consoles, Role Mailbox, and backend evidence-projection law in place; clarified full-name wording discipline for touched additions; deepened Appendix 12 ownership for control-vs-evidence surfaces; and added backend matrix edges for Dev Command Center -> Flight Recorder, Dev Command Center -> Debug Bundle, and Role Mailbox -> Dev Command Center while keeping weaker bridges stub-backed. | ilja090320261125 |
| v02.158 | 2026-03-09 | Orchestrator | **Stage/Studio/Media/ASR backend pillar pass:** patched ASR, Stage, Media Downloader, and Stage Studio backend law in place; deepened Appendix 12 ownership for ASR/media artifact portability and Stage capture lineage; added backend matrix edges for ASR -> Flight Recorder, ASR -> Storage Portability, Media Downloader -> ASR, and Stage -> Storage Portability; and materialized unresolved Stage -> ASR transcript lineage as a dedicated stub-backed Phase 1 track. | ilja090320260940 |
| v02.157 | 2026-03-09 | Orchestrator | **Distillation/context/spec-router backend pass:** patched Skill Bank, Context Packs, ACE Runtime, Micro-Task Executor, and Spec Router laws in place; incorporated current distillation research on teacher/student lineage, adapter-only late-stage training, prompt/context reuse, and checkpoint/eval gating; deepened Appendix 12 ownership; added backend matrix edges for ACE Runtime -> Context Packs, Micro-Task Executor -> Skill Bank, Context Packs -> Flight Recorder, Skill Bank -> Storage Portability, and Spec Router -> Context Packs; and materialized unresolved Context Pack recorder visibility as a dedicated stub. | ilja090320260633 |
| v02.156 | 2026-03-09 | Orchestrator | **Knowledge/retrieval pillar backend pass:** patched Project Brain, Semantic Catalog, Context Packs, and Loom portability law in place; deepened Appendix 12 ownership for retrieval notebooks, deterministic routing registry, portable ContextPack artifacts, and Loom library storage contracts; added backend matrix edges for Project Brain -> AI-Ready Data, Semantic Catalog -> Spec Router, Context Packs -> Storage Portability, and Loom -> Storage Portability while reusing existing backlog and adding only a dedicated Loom portability stub. | ilja090320260528 |
| v02.155 | 2026-03-09 | Orchestrator | **Calendar-centered backend force-multiplier pass:** patched Calendar backend law in place so sync state, export mode, capability profiles, AI-job mutation discipline, and ACE scope-hint routing remain explicit backend contracts; deepened Appendix 12 Calendar ownership and added Calendar storage-portability / consent / AI-job / Spec Router matrix edges while keeping mailbox/Locus/export bridges stub-backed. | ilja090320260324 |
| v02.154 | 2026-03-09 | Orchestrator | **Backend governance/export reciprocity pass:** patched Governance Pack and Workspace Bundle backend export law in place, backfilled missing appendix ownership for Governance Pack + Workspace Bundle, added Governance Pack workflow/capability/recorder/storage edges, and reused existing backlog stubs for unresolved transfer/instantiation work. | ilja090320260125 |
| v02.153 | 2026-03-09 | Orchestrator | **Backend capability/diagnostics evidence pass:** patched Main Body capability/diagnostic backend evidence law in place, deepened Capabilities / Workflow / Spec Router / MCP / Diagnostics backend projection posture, added Appendix 12.6 capability-recorder and diagnostics-bundle edges, and materialized unresolved cloud-consent portability as a stub-backed Phase 1 track. | ilja090320260021 |
| v02.152 | 2026-03-08 | Orchestrator | **Backend orchestration/projection/replay pass:** patched Main Body backend evidence law in place, deepened Spec Router / Locus / MCP / MEX backend evidence posture, added Appendix 12.6 spec-router/locus/mcp/mex projection edges, and materialized unresolved evidence-export bridges as stub-backed Phase 1 tracks. | ilja080320262335 |
| v02.151 | 2026-03-08 | Orchestrator | **Backend export/evidence/portability pass:** patched Main Body backend evidence law in place, deepened Role Mailbox / AI-Ready Data / Workflow Engine export and recorder posture, added Appendix 12.6 mailbox/AI-ready/workflow portability edges, and materialized unresolved debug-bundle and calendar-mailbox bridges as stub-backed Phase 1 tracks. | ilja080320262258 |
| v02.150 | 2026-03-08 | Orchestrator | **Backend-heavy matrix expansion pass:** patched backend-first combo growth into the Main Body, deepened workflow/consent/calendar/stage-media projection contracts, added explicit Appendix 12.6 backend force-multiplier edges, and materialized unresolved combos as stub-backed Phase 1 tracks. | ilja080320262147 |
| v02.149 | 2026-03-08 | Orchestrator | **Refinement reciprocity + research-rubric hardening:** hardcoded Main Body<->Appendix reciprocity, roadmap/cov-matrix coupling, `[ADD v<version>]` packet visibility, primitive exposure/creation reporting, and new mandatory MATRIX_RESEARCH_RUBRIC + GUI_IMPLEMENTATION_ADVICE_RUBRIC with carry-over/hidden-requirement rules. | ilja080320261910 |
| v02.148 | 2026-03-08 | Orchestrator | **Ownership-reduction deepening pass:** attached Stage/media session-auth contracts, multi-session runtime substrate, and debug/export/retention contracts to owning feature rows; added explicit Stage?Media Downloader, session?AI job, and debug?storage interaction edges. | ilja080320261744 |
| v02.147 | 2026-03-08 | Orchestrator | **Orphan-ownership and projection-contract pass:** attached high-signal orphan primitives to owning feature rows, deepened capability/consent, AI job, debug-bundle, storage portability, and operator projection contracts, and added explicit export/projection interaction edges. | ilja080320261623 |
| v02.146 | 2026-03-08 | Orchestrator | **Deepening pass over seeded runtime rows:** attached missing AI-ready artifact/status contracts, AI Job and Flight Recorder UI/operator surfaces, Role Mailbox/Locus/Loom query contracts, and explicit jobâ†”consent / MEXâ†”Flight Recorder interaction edges. | ilja080320261546 |
| v02.145 | 2026-03-08 | Orchestrator | **Third-pass runtime/data/operator coverage sweep:** added model-session orchestration, cloud escalation consent, and MEX runtime feature coverage; deepened runtime/export/filter/session contracts across Appendix 12; and scaffolded execution-path interaction edges for later matrix expansion. | ilja080320261408 |
| v02.144 | 2026-03-08 | Orchestrator | **Second-pass primitive/feature coverage sweep:** expanded Appendix 12.3/12.4/12.5/12.6 with named feature families, richer runtime visibility rows, high-signal primitives/tools/tech, and stub-linked backlog for unresolved embodiments. | ilja080320260600 |
| v02.143 | 2026-03-08 | Orchestrator | **Primitive index coverage replay:** Added `6.0.2.11 Primitive Index Coverage Contract (MUST)`, normalized Appendix 12.4 into coverage-driven feature rows, backfilled runtime/job/tool/frontend/operator primitives, and synced roadmap + stub backlog pointers for unresolved gaps. | ilja080320260320 |
| v02.142 | 2026-03-08 | Orchestrator | **Runtime visibility contract replay:** Added `6.0.2.10 Runtime Visibility Contract (MUST)`, expanded Appendix 12 Feature Registry with capability slices + runtime visibility rows, extended Interaction Matrix with runtime-linked force multipliers, and patched Phase 1 roadmap/runtime visibility guidance in place. | pending |
| v02.141 | 2026-03-04 | Orchestrator | **EOF appendix backfill:** Populated Â§12 appendix blocks (Feature Registry, Primitive/Tool/Tech Matrix, UI Guidance per feature, Interaction Matrix) with a Phase 1 inventory and initial cross-feature interaction edges. | ilja040320262011 |
| v02.140 | 2026-03-04 | Orchestrator | **End-of-file appendices system:** Added Â§12 defining mandatory end-of-file appendix blocks (Feature Registry, Primitive/Tool/Tech Matrix, UI Guidance per feature, Interaction Matrix), plus maintenance rules. These appendices keep the spec self-contained while scaling UI guidance and cross-feature interaction tracking. | ilja040320261813 |
| v02.139 | 2026-02-26 | Orchestrator | **Promptâ†’Spec hardening quartet:** Added SpecPromptPack + SpecPromptCompiler (deterministic PromptEnvelope compilation + ContextSnapshot logging), added CapabilitySnapshot artifact + enforcement, added SpecLint mechanical preflight (SPEC-VAL-*). Updated Spec Router schemas/job profile and updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix with [ADD v02.139] bullets. | pending |
| v02.138 | 2026-02-25 | Orchestrator | **Front End Memory System (FEMS) merge:** Integrated FEMS into ACE Runtime tiered memory (Â§2.6.6.7.6.2), added FEMS job profile (Â§2.6.6.6.6) and profile registry row (Â§2.6.6.6.2), ModelSession integration (Â§4.3.9.12.7), DCC Memory Panel (Â§10.11.5.14), Flight Recorder event family (Â§11.5.13), and evaluation suite (Â§5.4.8). Updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix with [ADD v02.138] bullets. | pending |
| v02.137 | 2026-02-22 | Orchestrator | **Multi-Session Orchestration & Spawn Lifecycle:** Added Â§4.3.9.12â€“Â§4.3.9.21 (ModelSession, session scheduler `model_run`, spawn contract, cloud consent gate, provider tool-calling/streaming adapters, workspace isolation, crash recovery, ModelSessionSpan observability + FR event families). Updated Flight Recorder with `model_session_id` correlation + registered FR-EVT-SESS-* families; added Locus MT occupancy (`active_session_ids` + bind/unbind ops); made HTC tool calls require `session_id` for ModelSessions + added session-scoped capability intersection in Tool Gate; updated Â§7.2.0.5 to normative; added DCC Sessions panel; updated Stage Prompt Playground requirements; extended Â§7.6 roadmap. | pending |
| v02.136 | 2026-02-22 | Orchestrator | **Roadmap + Coverage Matrix update:** Updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix to schedule Unified Tool Surface Contract (HTC v1.0, Â§6.0.2) implementation (Tool Registry + Tool Gate + MCP schema generation) and to phase the Design Studio shell/IA recontextualization (per `Handshake_Design_Studio_Overhaul_v0.1.md`) into Phase 2+ to avoid Phase 1 rework. Added Phase 1/2 bullets tagged [ADD v02.136] (no new phase fields). | pending |
| v02.135 | 2026-02-22 | Orchestrator | **Tool surface unification:** add Â§6.0.2 Unified Tool Surface Contract (HTC-1.0) including `assets/schemas/htc_v1.json` SSoT, require Tool Gate routing for all tool calls (local + MCP), add DCC Tool Call Ledger UX, add MCP binding rules (Â§11.3.0), and add Flight Recorder `tool_call` event schema (FR-EVT-007). Also add product framing: worksurfaces/modules naming guardrail (Design Studio is additive). | pending |
| v02.134 | 2026-02-20 | Orchestrator | **Spec enrichment:** Add OutputRootDir (default materialization root) + add Â§10.14 Media Downloader unified archiving surface (YouTube/Instagram/forum crawler/generic video) with progress, Stage Sessions auth, and Export/materialize routing. | ilja200220260830 |
| v02.133 | 2026-02-20 | Orchestrator | **Spec enrichment:** Cloud escalation Flight Recorder event alignment: declare 11.5.8 FR-EVT-CLOUD-001..004 canonical; align 9.1.4 mirror table; remove FR-EVT-CLOUD-005 and consent-presented/received event types. | ilja200220260027 |
| v02.132 | 2026-02-19 | Orchestrator | **Spec enrichment:** Canonicalize AutomationLevel + GovernanceDecision across Main Body and the 10.13 Stage import; define AutoSignature schema; align self-approval to FR-EVT-GOV-001..005; pin LOCKED semantics. | ilja190220261426 |
| v02.131 | 2026-02-19 | Orchestrator | **Stage merge:** Integrated handshake-stage-spec_v0.6.md v0.6 into Master Spec: added Â§10.13 Handshake Stage (verbatim import); updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix; registered Stage job profiles in Â§2.6.6.6.2; added merge alignment notes (Stageâ†”Docling timing; Stage job family mapping). | pending |
| v02.130 | 2026-02-18 | Orchestrator | **Loom merge:** Integrated Loom_Integration_Spec_Handshake.md v1.1.0 into Master Spec: LoomBlock/LoomEdge + Loom views; added Â§10.12 Loom (verbatim import); extended relationship taxonomy, storage portability examples, AI JobKinds/profiles, and Flight Recorder events; updated Â§7.6 Roadmap + Coverage Matrix (tagged [ADD v02.130]). | pending |
| v02.129 | 2026-02-18 | Orchestrator | **Roadmap normalization:** Converted legacy `**ADD v02.xxx â€” â€¦**` atomic blocks into inline phase-field patches (Phase 1: multi-model orchestration, DCC MVP, layer-wise inference foundations; Phase 3: layer-wise experiments; Phase 4: layer-wise productization). Preserved all items/version tags; restored Phase 1 **Vertical slice** field by extracting the inline core-loop chunk; added governance rule: no permanent Addendum section. | pending |
| v02.128 | 2026-02-18 | Orchestrator | **Merge:** Embedded `Spec_Creation_System_v2.2.1_merged.md` into Â§2.6.8.13 as verbatim import (heading-level shifted only). No per-spec logging system added; respects v2.2.1 boundary. | pending |
| v02.127 | 2026-02-17 | Orchestrator | **Sidecar/DCC merge:** Added Â§10.11 Dev Command Center (Sidecar Integration) incl. DCC wiring + inline schemas (`.handshake/workspace.json`, `devcc.db`), plus Locus integration pointer and updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix. | pending |
| v02.126 | 2026-02-12 | Coder | **Spec-only consistency correction (runtime governance paths):** Replace stale work-tracking path examples with runtime governance root `.handshake/gov/` for Task Board, Task Packets, and related refs. | ilja120220260342 |
| v02.125 | 2026-02-06 | Orchestrator | **Governance snapshot definition:** Added `#### 7.5.4.10 Product Governance Snapshot (HARD)` defining a deterministic, leak-safe JSON snapshot derived ONLY from canonical `.GOV/**` inputs, with default output `.GOV/roles_shared/runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`, explicit `schema_version`, and list-based validator gate summaries (no timestamps; no raw logs). | ilja060220260754 |
| v02.124 | 2026-02-05 | Validator | **Spec enrichment (governance boundary + pack path update):** define `/.GOV/` as canonical governance workspace and `docs/` as temporary compatibility bundle only; require hard enforcement that product code MUST NOT read/write `/.GOV/`; set default runtime governance state dir to `.handshake/gov/` (configurable; runtime governance state only). Updated Governance Pack sections 7.5.4.7-7.5.4.9 to reflect `.GOV/` canonical layout and boundary rules. | ilja050220260910 |
| v02.122 | 2026-01-29 | Orchestrator | **v02.122 merge:** merged Multi-Model Parallelism Addendum (UPDATED) and Handshake_Layerwise_Inference_SpecDraft_v0.3.md into Master Spec main body: RuntimeMode/ExecutionMode + invariants (DOCS_ONLY/AI_ENABLED, min_ready_models, strict file-scope locks), RoleExecutionIdentity + ParameterClass + largest-first routing + performance telemetry scoring, MailboxKind taxonomy, HSK_STATUS lifecycle marker, SwapRequest escalation + CX-MM code registry, plus reserved `settings.exec_policy` + Work Profile compute/approximate-waiver hooks and new FR `llm_exec_policy` + `hsk.layerwise_trace@0.1`. Updated Â§7.6 coverage matrix + roadmap (Phase 0 closed; new [ADD v02.122] entries). | pending |
| v02.121 | 2026-01-28 | Orchestrator | **ANS-001 enrichment:** defined frontend session chat log persistence in `{APP_DATA}` (one file per session; JSONL), UI presentation rules (hidden-by-default + per-message expand + global show-inline + side-panel timeline), leak-safe runtime chat telemetry events `FR-EVT-RUNTIME-CHAT-101..103`, and clarified `EXEC-060` compliance logging semantics. | ilja260120261908 |
| v02.120 | 2026-01-27 | Orchestrator | **Runtime Integration merge:** merged Handshake_Runtime_Integration_Addendum_v0_5 into master spec: model resource management (ModelSwap protocol + FR-EVT-MODEL-*), autonomous governance protocol (AutomationLevel + GovernanceDecision/self-approval + FR-EVT-GOV-*), Work Profiles (role-based model assignment + FR-EVT-PROFILE-*), Role Mailbox (â€œInboxâ€) alignment (body schema + runtime telemetry FR-EVT-RUNTIME-MAILBOX-*), cloud escalation consent artifacts + FR-EVT-CLOUD-*, promptâ†’macroâ†’micro pipeline + tooling profile selection, and conformance tests. Updated roadmap (Phase0 unchanged/closed; new [ADD v02.120] entries only). | ilja270120260001 |
| v02.119 | 2026-01-26 | Validator | **Non-normative AI UX notes (Command Palette + Jobs UI):** Recorded the current UX preferences that (a) Command Palette uses Ctrl/Cmd+K as primary shortcut with Ctrl/Cmd+Shift+P as a fallback, (b) "Summarize" opens the palette so instructions can be tweaked before creating the job, and (c) the backend remains the queue/source-of-truth while the frontend acts as a global job tracker UI (poll queued/running only to avoid storms). (Non-normative; expected to evolve.) | ilja260120260248 |
| v02.118 | 2026-01-26 | Validator | **AI-Ready Data Architecture (Tree-sitter + workspace root clarifications):** Clarified the Phase 1 Shadow Workspace root mapping for `workspace/raw|derived|indexes|graph` under the app-managed `data/workspaces/{workspace_id}/workspace/` tree, required a dedicated parser (Tree-sitter) for AST-aware code chunking determinism, and clarified FR-EVT-DATA-015 to log `query_hash` only (never plaintext). | ilja260120260102 |
| v02.117 | 2026-01-25 | Orchestrator | **AI-Ready Data Architecture (FR-EVT-DATA schema completion):** Added missing DATA event schemas for FR-EVT-DATA-003/005/006/007/008/010/013/014 in Â§11.5.5 so the "Flight Recorder MUST reject DATA events that do not match schemas above" requirement is fully enforceable; no requirement changes. | ilja250120261843 |
| v02.116 | 2026-01-23 | Orchestrator | **Locus Work Tracking System Integration:** Added complete Â§2.3.15 Locus Work Tracking System (governance-aware work tracking from macro Work Packets through micro Micro-Task execution): Â§2.3.15.1 Overview and Scope (unified tracking from Promptâ†’Specâ†’Gatesâ†’MTâ†’Done); Â§2.3.15.2 Core Schemas (TrackedWorkPacket with governance/gates/task_packets/micro_tasks, TrackedMicroTask with iterations/escalation/validation, TrackedDependency with 10 types); Â§2.3.15.3 Mechanical Operations (18 operations: locus_create_wp, locus_update_wp, locus_gate_wp, locus_register_mts, locus_start_mt, locus_record_iteration, locus_complete_mt, locus_add_dependency, locus_query_ready, locus_search, locus_sync_task_board, etc.); Â§2.3.15.4 Integration Points (Spec Router auto-creates WPs via locus_create_wp, MT Executor records iterations via locus_record_iteration, Task Board bidirectional sync, Task Packet linking, Calendar policy integration, Knowledge Graph dependencies); Â§2.3.15.5 Storage Architecture (Bronze/Silver/Gold medallion, SQLite Phase 1 local-first, PostgreSQL Phase 2 multi-user, CRDT conflict resolution, vector clocks); Â§2.3.15.6 Event Sourcing (21 Flight Recorder events: FR-EVT-WP-001..005 for Work Packets, FR-EVT-MT-001..006 for Micro-Tasks, FR-EVT-DEP-001..002 for dependencies, FR-EVT-TB-001..003 for Task Board, FR-EVT-SYNC-001..003 for sync, FR-EVT-QUERY-001); Â§2.3.15.7 Query Interface (ready work detection with dependency blocking, hybrid search vector+keyword+graph, dependency tree traversal); Â§2.3.15.8 Multi-User Architecture (workspace model, real-time WebSocket collaboration, CRDT op-based merge); Â§2.3.15.9 Performance Targets (locus_create_wp <50ms, locus_query_ready <100ms, locus_search <200ms, 10K WPs Phase 1, 100K WPs Phase 2); Â§2.3.15.10 Conformance Requirements (MUST/SHOULD/MAY RFC 2119 requirements). Updated Coverage Matrix Â§7.6.1 (added Â§2.3.15 row: P1, P2, P3, P4). Added Phase 1-4 roadmap items tagged [ADD v02.116]: Phase 1 (SQLite backend, core operations, Spec Router integration, MT Executor integration, Task Board sync, basic queries, Flight Recorder events), Phase 2 (hybrid search, Calendar policy, dependency graph queries, migration tools), Phase 3 (PostgreSQL backend, CRDT implementation, WebSocket real-time, workspace multi-tenancy), Phase 4 (advanced analytics, auto-archival, AI-powered insights). **Integration touchpoints:** Spec Router (Â§2.6.8), MT Executor (Â§2.6.6.8), Task Board (.handshake/gov/TASK_BOARD.md), Task Packets (.handshake/gov/task_packets/), Flight Recorder (Â§11.5), Shadow Workspace (Â§2.3.8), Knowledge Graph (Â§2.3.7), Calendar (Â§11.9), Mechanical Tool Bus (Â§6.3), Capability System (Â§11.1). | ilja230120262345 |
| v02.115 | 2026-01-22 | Orchestrator | **AI-Ready Data Architecture FULL Integration (2,350+ lines):** Added complete Â§2.3.14 AI-Ready Data Architecture with 22 major sections and 5 appendices: Â§2.3.14.1 Motivation and Scope (problem statement, research citations: Anthropic 35-67% retrieval improvement, Databricks 37% LLM improvement, 87% vs 50% semantic-aware chunking accuracy); Â§2.3.14.2-3 Normative References and Terminology (30+ defined terms); Â§2.3.14.4 Design Principles (8 principles: semantic coherence MUST, contextual enrichment SHOULD, hybrid indexing MUST, rich metadata MUST, content-aware processing MUST, event-driven freshness SHOULD, two-stage retrieval SHOULD, validation automation MUST); Â§2.3.14.5 Content Storage Architecture (Bronze/Silver/Gold medallion pattern with full TypeScript schemas: BronzeRecord, SilverRecord, ProcessedContent, EmbeddingRecord, ProcessingRecord, ValidationRecord); Â§2.3.14.6 Chunking Strategies (AST-aware code chunking with Python implementation, header-recursive document chunking, semantic chunking for prose, validation requirements); Â§2.3.14.7 Embedding Architecture (model registry with version tracking, model comparison tables for text/code/vision, selection function, migration plan schema); Â§2.3.14.8 Indexing Architecture (HNSW vector index, BM25 keyword index, Knowledge Graph with 20 relationship types, RRF fusion algorithm); Â§2.3.14.9 Retrieval Pipeline (two-stage with reranking, context assembly, "lost in middle" mitigation); Â§2.3.14.10 Metadata Schema (core metadata, content-type extensions for code/image/email/calendar, agent context annotations); Â§2.3.14.11 Multimodal Data Organization (unified schema, cross-modal queries); Â§2.3.14.12 Anti-Patterns and Mitigations (fixed-size chunking, orphan embeddings, context pollution, stale indexes); Â§2.3.14.13 Context Management (pollution scoring, budget management, fresh context pattern); Â§2.3.14.14 Validation and Quality Metrics (SLOs: MRRâ‰¥0.6, Recall@10â‰¥0.8, NDCG@5â‰¥0.7, p95â‰¤500ms, validationâ‰¥95%, completenessâ‰¥99%; mechanical validation jobs); Â§2.3.14.15 Integration Mapping (Master Spec section touchpoints); Â§2.3.14.16 Security and Privacy (embedding access controls, index encryption, audit trail); Â§2.3.14.17 Conformance Requirements (MUST/SHOULD lists, performance baselines); Â§2.3.14.A Complete Schema Definitions (full TypeScript types); Â§2.3.14.B Embedding Model Comparison (benchmark tables); Â§2.3.14.C Chunking Algorithm Implementations (Python reference code); Â§2.3.14.D Validation Job Profiles (mechanical job specs); Â§2.3.14.E Flight Recorder Event Schemas (FR-EVT-DATA-001..015). Extended Â§2.3.7 Knowledge Graph, Â§2.3.8 Shadow Workspace, Â§2.3.13 Storage Traits. Updated Coverage Matrix Â§7.6.1. Added Phase 1-4 roadmap items. **Cross-cutting principle:** "everything can use everything" - all tools produce Bronzeâ†’Silverâ†’Gold, all features consume via unified retrieval. | ilja220120262330 |
| v02.114 | 2026-01-21 | Orchestrator | Added Â§2.6.6.8 Micro-Task Executor Profile: auto-generated MT decomposition from Work Packets, iterative execution loop with fresh-context-per-iteration, model/LoRA escalation chain, completion signal protocol with anti-gaming rules, crash recovery via run ledger, Skill Bank distillation integration (Â§9), 17 Flight Recorder events (FR-EVT-MT-001..017); updated Coverage Matrix (Â§2.6 row covers new subsection); added Phase 1 roadmap items (Mechanical Track: MT Loop Controller + validation engine wiring; Distillation Track: escalation candidate capture); added Phase 2/3/4 items for LoRA training automation and parallel wave execution. | ilja210120262100 |
| v02.113 | 2026-01-17 | Orchestrator | Governance workflow hardening: Validator gate state is stored per WP in `.handshake/gov/validator_gates/{WP_ID}.json` (merge-safe) with `docs/VALIDATOR_GATES.json` as a legacy read-only archive; stub activation MUST update `.handshake/gov/WP_TRACEABILITY_REGISTRY.md` Baseâ†’Active mapping and move Task Board entry out of STUB; define Flight Recorder events for gate transitions and WP activation mirroring. | ilja170120260225 |
| v02.112 | 2026-01-15 | Orchestrator | Role Mailbox hardening: define FR-EVT-GOV-MAILBOX-001/002/003 event schemas, require schema validation at Flight Recorder ingestion, forbid inline message bodies in Flight Recorder or repo exports, and require a RoleMailboxExportGate mechanical gate. | ilja150120260214 |
| v02.111 | 2026-01-13 | Orchestrator | Inline missing high-signal governance docs into the Governance Pack Template Volume: role rubrics (`docs/CODER_RUBRIC.md`, `docs/ORCHESTRATOR_RUBRIC.md`), migration law (`docs/MIGRATION_GUIDE.md`), and legacy shim pointers for moved templates (`docs/*_TEMPLATE.md`). | ilja130120260459 |
| v02.110 | 2026-01-13 | Orchestrator | Fix Governance Pack template drift: `docs/VALIDATOR_GATES.json` now uses the `validation_sessions` + `archived_sessions` schema (matches `scripts/validation/validator_gates.mjs`). | ilja130120260438 |
| v02.109 | 2026-01-13 | Orchestrator | Inlined the full Governance Pack Template Volume (codex + role protocols + governance artifacts + mechanical hard-gate tooling) as project-agnostic templates and required a PROJECT_INVARIANTS section for project-specific naming/layout/tool paths. | ilja130120260124 |
| v02.108 | 2026-01-12 | Orchestrator | Added Role Mailbox (always-on repo export + transcription), Spec Authoring Rubric, fixed Trinity required roles in Spec Router policy, and defined Governance Pack project identity + conformance requirements. | ilja120120262149 |
| v02.107 | 2026-01-12 | Orchestrator | Integrated the project-agnostic Governance Kernel (mechanical gates + artifacts + small-context handoff) into the Master Spec; added a local-first agentic/MCP positioning note; clarified cross-tool interaction expectations; updated roadmap determinism pointers to reference the kernel additions. | ilja120120260452 |
| v02.106 | 2026-01-11 | Orchestrator | Migration governance: clarified heavy per-file (tracking-independent) replay-safe migrations and required concrete down migrations in Phase 1; updated migration acceptance criteria accordingly. | ilja110120262355 |
| v02.105 | 2026-01-11 | Orchestrator | Roadmap determinism: updated the Roadmap Coverage Matrix rules (no Phase 0 allocations, no UNSCHEDULED), fully phase-allocated all section rows (P1-P4), and updated the Roadmap text to reference and enforce the matrix. | ilja110120260038 |
| v02.104 | 2026-01-10 | Orchestrator | Roadmap determinism: added Â§7.6.1 Roadmap Coverage Matrix (section-level) and hard rules to prevent drift; aligned Codex + role protocols to enforce. | ilja100120262214 |
| v02.103 | 2026-01-08 | Orchestrator | Intent audit vs roadmap: added Phase 1 roadmap pointer for Diary ANS-001 Response Behavior Contract; clarified phase closure rule in roadmap preamble; queued TASK_BOARD + stub updates. | ilja080120262313 |
| v02.102 | 2026-01-08 | Orchestrator | Roadmap vs Master Spec audit: added missing Phase 1 roadmap items for storage portability closure WPs (CX-DBP-030), CapabilityRegistry SSoT (WP-1-Capability-SSoT), and Global Silent Edit Guard (WP-1-Global-Silent-Edit-Guard); queued Task Board + stub backlog sync. | ilja080120262305 |
| v02.101 | 2026-01-04 | Orchestrator | Clarified LLM completion trace_id transport and defined FR-EVT-006 llm_inference event shape. | ilja040120260108 |
| v02.100 | 2026-01-01 | Orchestrator | Added sync/async bridge requirement for TokenizationService telemetry emission (metric.accuracy_warning must be non-blocking). | ilja010120260602 |
| v02.99 | 2025-12-31 | Orchestrator | Expanded AI Job Model JobKind/JobState lists, added canonical JobKind strings, and defined FR-EVT-WF-RECOVERY. | ilja311220251755 |
| v02.98 | 2025-12-29 | Orchestrator | Added normative Debug Bundle schemas, DebugBundleExporter trait (HSK-TRAIT-005), API endpoints, job profile, redactor integration, determinism rules, and frontend UI spec (Â§10.5.6.5-12) | ilja291220250100 |
| v02.97 | 2025-12-28 | Orchestrator | Added normative DuckDB schema and DiagnosticsStore trait signatures (Â§11.4.2+) | ilja281220252016 |
| v02.96 | 2025-12-28 | Orchestrator | Reconciled Â§11.3.4 signatures to use `&dyn Database` instead of `SqlitePool` | ilja281220250525 |
| v02.95 | 2025-12-28 | Orchestrator | Enriched Â§4.6.1 with Tokenizer Trait definition (unified interface) | ilja281220250435 |
| v02.94 | 2025-12-28 | Orchestrator | Enriched Â§2.3.13.5 with Mandatory Storage Audit (sqlx/SqlitePool leakage check) | ilja281220250353 |
| v02.93 | 2025-12-26 | Orchestrator | Enriched Â§2.6.1 with Mandatory Startup Recovery (non-blocking loop in main.rs) | ilja271220250057 |
| v02.92 | 2025-12-26 | Orchestrator | Enriched Â§2.6.6.2.8 with AI Job Model Hardening (Strict Enum Mapping, Metrics Integrity) | ilja261220252215 |
| v02.91 | 2025-12-26 | Orchestrator | Enriched Â§2.6.6.7.11 with Hardened Security Enforcement (Content Awareness, Atomic Poisoning, NFC Normalization) | ilja261220252202 |
| v02.89 | 2025-12-26 | Orchestrator | Enriched Â§2.6.6.7.11 with ACE Security Guard Normative Requirements | ilja261220250201 |

---

## Table of Contents

- [1 Vision & Context](#1-vision-context)
  - [1.1 Executive Summary](#11-executive-summary)
  - [1.2 The Diary Origin Story](#12-the-diary-origin-story)
  - [1.3 The Four-Layer Architecture](#13-the-four-layer-architecture)
  - [1.4 LLM Reliability Hierarchy](#14-llm-reliability-hierarchy)
  - [1.5 What Gets Ported from the Diary](#15-what-gets-ported-from-the-diary)
  - [1.6 Design Philosophy: Self-Enforcing Governance](#16-design-philosophy-self-enforcing-governance)
  - [1.7 Success Criteria](#17-success-criteria)
  - [1.8 Introduction](#18-introduction)
- [2 System Architecture](#2-system-architecture)
  - [2.1 High-Level Architecture](#21-high-level-architecture)
  - [2.2 Data & Content Model](#22-data-content-model)
  - [2.3 Content Integrity (Diary Part 5: COR-700)](#23-content-integrity-diary-part-5-cor-700)
    - [2.3.12 Storage Backend Portability Architecture](#2312-storage-backend-portability-architecture-cx-dbp-001)
    - [2.3.14 AI-Ready Data Architecture](#2314-ai-ready-data-architecture-add-v02115)
      - [2.3.14.1 Motivation and Scope](#23141-motivation-and-scope-add-v02115)
      - [2.3.14.2 Normative References](#23142-normative-references-add-v02115)
      - [2.3.14.3 Terminology](#23143-terminology-add-v02115)
      - [2.3.14.4 Design Principles](#23144-design-principles-add-v02115)
      - [2.3.14.5 Content Storage Architecture](#23145-content-storage-architecture-add-v02115)
      - [2.3.14.6 Chunking Strategies](#23146-chunking-strategies-add-v02115)
      - [2.3.14.7 Embedding Architecture](#23147-embedding-architecture-add-v02115)
      - [2.3.14.8 Indexing Architecture](#23148-indexing-architecture-add-v02115)
      - [2.3.14.9 Retrieval Pipeline](#23149-retrieval-pipeline-add-v02115)
      - [2.3.14.10 Metadata Schema](#231410-metadata-schema-add-v02115)
      - [2.3.14.11 Multimodal Data Organization](#231411-multimodal-data-organization-add-v02115)
      - [2.3.14.12 Anti-Patterns and Mitigations](#231412-anti-patterns-and-mitigations-add-v02115)
      - [2.3.14.13 Context Management](#231413-context-management-add-v02115)
      - [2.3.14.14 Validation and Quality Metrics](#231414-validation-and-quality-metrics-add-v02115)
      - [2.3.14.15 Integration Mapping](#231415-integration-mapping-add-v02115)
      - [2.3.14.16 Security and Privacy Considerations](#231416-security-and-privacy-considerations-add-v02115)
      - [2.3.14.17 Conformance Requirements](#231417-conformance-requirements-add-v02115)
      - [2.3.14.A Appendix A: Complete Schema Definitions](#2314a-appendix-a-complete-schema-definitions-add-v02115)
      - [2.3.14.B Appendix B: Embedding Model Comparison](#2314b-appendix-b-embedding-model-comparison-add-v02115)
      - [2.3.14.C Appendix C: Chunking Algorithm Implementations](#2314c-appendix-c-chunking-algorithm-implementations-add-v02115)
      - [2.3.14.D Appendix D: Validation Job Profiles](#2314d-appendix-d-validation-job-profiles-add-v02115)
      - [2.3.14.E Appendix E: Flight Recorder Event Schemas](#2314e-appendix-e-flight-recorder-event-schemas-add-v02115)
    - [2.3.15 Locus Work Tracking System](#2315-locus-work-tracking-system-add-v02116)
      - [2.3.15.1 Overview and Scope](#23151-overview-and-scope-add-v02116)
      - [2.3.15.2 Core Schemas](#23152-core-schemas-add-v02116)
      - [2.3.15.3 Mechanical Operations](#23153-mechanical-operations-add-v02116)
      - [2.3.15.4 Integration Points](#23154-integration-points-add-v02116)
      - [2.3.15.5 Storage Architecture](#23155-storage-architecture-add-v02116)
      - [2.3.15.6 Event Sourcing](#23156-event-sourcing-add-v02116)
      - [2.3.15.7 Query Interface](#23157-query-interface-add-v02116)
      - [2.3.15.8 Multi-User Architecture](#23158-multi-user-architecture-add-v02116)
      - [2.3.15.9 Performance Targets](#23159-performance-targets-add-v02116)
      - [2.3.15.10 Conformance Requirements](#231510-conformance-requirements-add-v02116)
  - [2.4 Extraction Pipeline (The Product)](#24-extraction-pipeline-the-product)
  - [2.5 AI Interaction Patterns](#25-ai-interaction-patterns)
  - [2.6 Workflow & Automation Engine](#26-workflow-automation-engine)
    - [2.6.6 AI Job Model (Global)](#266-ai-job-model-global)
      - [2.6.6.2.8 Normative Rust Types](#26628-normative-rust-types)
      - [2.6.6.8 Micro-Task Executor Profile](#2668-micro-task-executor-profile)
  - [2.7 Response Behavior Contract (Diary ANS-001)](#27-response-behavior-contract-diary-ans-001)
  - [2.8 Governance Runtime (Diary Parts 1-2)](#28-governance-runtime-diary-parts-1-2)
  - [2.9 Deterministic Edit Process (COR-701)](#29-deterministic-edit-process-cor-701)
  - [2.10 Session Logging (LOG-001)](#210-session-logging-log-001)
- [3 Local-First Infrastructure](#3-local-first-infrastructure)
  - [3.1 Local-First Data Fundamentals](#31-local-first-data-fundamentals)
  - [3.2 CRDT Libraries Comparison](#32-crdt-libraries-comparison)
  - [3.3 Database & Sync Patterns](#33-database-sync-patterns)
  - [3.4 Conflict Resolution UX](#34-conflict-resolution-ux)
- [4 LLM Infrastructure](#4-llm-infrastructure)
  - [4.1 LLM Infrastructure](#41-llm-infrastructure)
  - [4.2 LLM Inference Runtimes](#42-llm-inference-runtimes)
  - [4.3 Model Selection & Roles](#43-model-selection-roles)
  - [4.4 Image Generation (Stable Diffusion)](#44-image-generation-stable-diffusion)
  - [4.5 Model Orchestration Policy](#45-model-orchestration-policy)
- [5 Security & Observability](#5-security-observability)
  - [5.1 Plugin Architecture](#51-plugin-architecture)
  - [5.2 Sandboxing & Security](#52-sandboxing-security)
  - [5.3 AI Observability](#53-ai-observability)
  - [5.4 Evaluation & Quality](#54-evaluation-quality)
    - [5.4.6 Governance Compliance Tests](#546-governance-compliance-tests)
  - [5.5 Benchmark Harness](#55-benchmark-harness)
- [6 Mechanical Integrations](#6-mechanical-integrations)
  - [6.1 Document Ingestion: Docling Subsystem](#61-document-ingestion-docling-subsystem)
  - [6.2 Speech Recognition: ASR Subsystem](#62-speech-recognition-asr-subsystem)
  - [6.3 Mechanical Extension Engines](#63-mechanical-extension-engines)
- [7 User Experience & Development](#7-user-experience-development)
  - [7.1 User Interface Components](#71-user-interface-components)
  - [7.2 Multi-Agent Orchestration](#72-multi-agent-orchestration)
  - [7.3 Collaboration and Sync](#73-collaboration-and-sync)
  - [7.4 Reference Application Analysis](#74-reference-application-analysis)
  - [7.5 Development Workflow](#75-development-workflow)
  - [7.6 Development Roadmap](#76-development-roadmap)
- [8 Reference](#8-reference)
  - [8.1 Risk Assessment](#81-risk-assessment)
  - [8.2 Technology Stack Summary](#82-technology-stack-summary)
  - [8.3 Gap Analysis & Open Questions](#83-gap-analysis-open-questions)
  - [8.4 Consolidated Glossary](#84-consolidated-glossary)
  - [8.5 Sources Referenced](#85-sources-referenced)
  - [8.6 Appendices](#86-appendices)
  - [8.7 Version History & Subsection Versioning](#87-version-history--subsection-versioning)
- [9 Continuous Local Skill Distillation (Skill Bank & Pipeline)](#9-continuous-local-skill-distillation-skill-bank-pipeline)
  - [9.1 Canonical Specification (verbatim import)](#91-canonical-specification-verbatim-import)
- [10 Product Surfaces](#10-product-surfaces)
  - [10.1 Terminal Experience](#101-terminal-experience)
  - [10.2 Monaco Editor Experience](#102-monaco-editor-experience)
  - [10.3 Mail Client](#103-mail-client)
  - [10.4 Calendar](#104-calendar)
  - [10.5 Operator Consoles: Debug & Diagnostics](#105-operator-consoles-debug-diagnostics)
  - [10.6 Canvas: Typography & Font Packs](#106-canvas-typography-font-packs)
  - [10.7 Charts & Dashboards](#107-charts--dashboards)
  - [10.8 Presentations (Decks)](#108-presentations-decks)
  - [10.9 Future Surfaces](#109-future-surfaces)
  - [10.10 Photo Studio](#1010-photo-studio)
  - [10.11 Dev Command Center (Sidecar Integration)](#1011-dev-command-center-sidecar-integration)
  - [10.12 Loom (Heaper-style Library Surface)](#1012-loom-heaper-style-library-surface-add-v02130)
  - [10.13 Handshake Stage (Built-in Browser + Stage Apps)](#1013-handshake-stage-built-in-browser--stage-apps-add-v02131)
  - [10.14 Media Downloader (Unified Web Media Archiving Surface)](#1014-media-downloader-unified-web-media-archiving-surface-add-v02134)
- [11 Shared Dev Platform & OSS Foundations](#11-shared-dev-platform-oss-foundations)
  - [11.1 Capabilities & Consent Model](#111-capabilities-consent-model)
  - [11.2 Sandbox Policy vs Hard Isolation](#112-sandbox-policy-vs-hard-isolation)
  - [11.3 Auth/Session/MCP Primitives](#113-authsessionmcp-primitives)
  - [11.4 Diagnostics Schema (Problems/Events)](#114-diagnostics-schema-problemsevents)
  - [11.5 Flight Recorder Event Shapes & Retention](#115-flight-recorder-event-shapes-retention)
  - [11.6 Plugin/Matcher Precedence Rules](#116-pluginmatcher-precedence-rules)
  - [11.7 OSS Component Choices & Versions](#117-oss-component-choices-versions)
    - [11.7.1 Terminal Engine / PTY / Sandbox](#1171-terminal-engine--pty--sandbox)
    - [11.7.2 Monaco Bundling / LSP Bridges](#1172-monaco-bundling--lsp-bridges)
    - [11.7.3 Mail / Calendar Engines](#1173-mail--calendar-engines)
    - [11.7.4 OSS Licensing, Compliance, Isolation, and Determinism (Baseline Policy Addendum)](#1174-oss-licensing-compliance-isolation-and-determinism-baseline-policy-addendum)
    - [11.7.5 Industry Modules & OSS Foundations Spec (Embedded Snapshot)](#1175-industry-modules--oss-foundations-spec-embedded-snapshot)
    - [11.7.6 Photo Stack OSS Component Matrix (Photo Studio)](#1176-photo-stack-oss-component-matrix-photo-studio)
  - [11.8 Mechanical Extension Specification v1.2 (Verbatim)](#118-mechanical-extension-specification-v12-verbatim)
  - [11.9 Future Shared Primitives](#119-future-shared-primitives)
    - [11.9.1 ActivitySpan and SessionSpan](#1191-activityspan-and-sessionspan)
    - [11.9.2 Calendar Range as a Query Surface](#1192-calendar-range-as-a-query-surface)
    - [11.9.3 CalendarEvent and ActivitySpan Join Semantics](#1193-calendarevent-and-activityspan-join-semantics)
    - [11.9.4 Minimum Slice for Calendar and Flight Recorder](#1194-minimum-slice-for-calendar-and-flight-recorder)
  - [11.10 Implementation Notes: Phase 1 Final Gaps](#1110-implementation-notes-phase-1-final-gaps)
- [12 End-of-File Appendices (Feature Index + Matrix + UI Guidance)](#12-end-of-file-appendices)
- [13 Tailor -- Cloth/Garment Engine](#13-tailor-cloth-garment-engine)
  - [13.1 Overview, Scope & Model-First Differentiator](#131-overview-scope-model-first-differentiator)
  - [13.2 Architecture: tailor-solver Crate + handshake_core::tailor Module](#132-architecture-tailor-solver-crate-handshake-core-tailor-module)
  - [13.3 XPBD Solver Core (WGSL/wgpu)](#133-xpbd-solver-core-wgsl-wgpu)
  - [13.4 Collision: Body, Self, Multi-Layer, Exaggerated Proportions](#134-collision-body-self-multi-layer-exaggerated-proportions)
  - [13.5 Fabric & Material Models](#135-fabric-material-models)
  - [13.6 Garment Authoring: Patterns, Seams, Parametric/Model-First](#136-garment-authoring-patterns-seams-parametric-model-first)
  - [13.7 Auto-Fit & Retargeting Across Body Morphs](#137-auto-fit-retargeting-across-body-morphs)
  - [13.8 Trims & Cloth-Rigid Coupling](#138-trims-cloth-rigid-coupling)
  - [13.9 UV-from-Pattern & Texturing](#139-uv-from-pattern-texturing)
  - [13.10 Animation & Keyframe Timeline](#1310-animation-keyframe-timeline)
  - [13.11 Kernel Integration (Authority, CRDT, Sandbox, Promotion, Model Lanes)](#1311-kernel-integration-authority-crdt-sandbox-promotion-model-lanes)
  - [13.12 Viewport, Visual Debug & Render/Export Handoff](#1312-viewport-visual-debug-render-export-handoff)
  - [13.13 Model-First API & LLM Steering](#1313-model-first-api-llm-steering)
  - [13.14 Canonical Tailor Authority Contracts](#1314-canonical-tailor-authority-contracts)
  - [13.15 Validation, Promotion Equivalence & HBR](#1315-validation-promotion-equivalence-hbr)

---


---

<a id="1-vision-context"></a>
