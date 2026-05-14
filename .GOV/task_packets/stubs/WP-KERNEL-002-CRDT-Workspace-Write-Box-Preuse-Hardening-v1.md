# TASK_PACKET_STUB_TEMPLATE

Activation note:
- This stub is superseded by the active machine-readable packet at `.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json`.
- Activation signature: `ilja140520260455`.
- The active packet replaces the pre-activation validator-gate wording with Kernel Builder consolidated implementation plus separate Integration Validator batch review.
- Preserved planning content below remains historical fold evidence, not executable authority.

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` or the active lane-owner protocol: Technical Refinement Block, USER_SIGNATURE, refinement, and official packet creation.
- If a Base WP later gains multiple packets or revisions, record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.
- This stub may be activated only after `WP-KERNEL-001-Event-Ledger-Session-Broker-v1` is accepted as the event authority substrate or the Operator explicitly authorizes a parallel activation path.

---

# Work Packet Stub: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1

## STUB_METADATA
- WP_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- BASE_WP_ID: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening
- CREATED_AT: 2026-05-14T00:00:00Z
- STUB_STATUS: ACTIVATED_TO_PACKET
- ACTIVE_PACKET: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- ACTIVE_PACKET_CONTRACT: .GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json
- ACTIVATED_AT: 2026-05-14T10:20:22.552Z
- ACTIVATION_SIGNATURE: ilja140520260455
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-1-Global-Silent-Edit-Guard, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Artifact-System-Foundations, WP-1-Role-Mailbox, WP-1-Micro-Task-Executor, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Postgres-Primary-Control-Plane-Foundation
- BUILD_ORDER_BLOCKS: WP-KERNEL-003-Sandbox-Validation-Promotion, WP-KERNEL-004-Local-Model-Memory-Runtime, WP-1-Software-Delivery-Runtime-Truth, WP-1-Workflow-Transition-Automation-Registry, WP-1-Dev-Command-Center-MVP, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-Session-Spawn-Tree-DCC-Visualization, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Visual-Debugging-Loop
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: Kernel reset brief Week 2 CRDT workspace + promotion; pre-use kernel hardening for action catalog, write boxes, projection drift prevention, software-delivery runtime truth, DCC/runtime projection, Role Mailbox handoff, FEMS checkpoint safety, and no-context model operation.
- ROADMAP_ADD_COVERAGE: RESET_BRIEF=handshake-v2-kernel-reset-brief.md; WEEK=2; TOPICS=CRDT workspace, promotion gate, Postgres authority, no SQLite authority, action catalog, write boxes, Markdown mirror drift guard, DCC projection, software-delivery runtime truth, Role Mailbox coordination, FEMS checkpoints, visual debugging.
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: N/A
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH

## COMPLETE_STUB_FOLD_POLICY
- This WP is a complete-stub fold, not a selective reset excerpt.
- Every source stub listed under `SOURCE_STUBS_FOLDED_COMPLETE` is folded as a full obligation: identity, intent, scope, out-of-scope, acceptance criteria, dependencies, risks, UI notes, research notes, and activation cautions all transfer into Kernel002 unless explicitly superseded by a stricter reset invariant.
- Stricter reset invariants are:
  - Postgres-backed authority is the Kernel V1 authority path.
  - CRDT is live collaborative workspace state, not final workflow truth.
  - CRDT edits are not authoritative until a PromotionGate action accepts them into EventLedger-backed authority state.
  - SQLite is not an authority store, offline replica, fallback authority, or Kernel V1 scaffold.
  - Markdown, Task Board prose, Role Mailbox prose, and generated mirrors are projections, advisory notes, or legacy references unless backed by machine-readable authority records.
  - Models must work through registered actions and write boxes. Direct mutation of authority artifacts must be rejected or converted into an advisory proposal with denial evidence.
- Source hashes below are pre-fold SHA-256 hashes of the source Markdown files. After this fold, source files may gain supersession metadata; these hashes preserve the exact source content that was folded.

## SOURCE_STUBS_FOLDED_COMPLETE

| Source Stub | Source Path | Pre-Fold SHA-256 | Fold Status |
|-------------|-------------|------------------|-------------|
| WP-1-Postgres-Control-Plane-Shift-Bundle-v1 | `.GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.md` | `f160424f7dd05647fec455d6eee7acbd0f1774d58d4b948963d4af9c58cce5a7` | FULL_STUB_FOLDED |
| WP-1-Postgres-Dev-Test-Container-Matrix-v1 | `.GOV/task_packets/stubs/WP-1-Postgres-Dev-Test-Container-Matrix-v1.md` | `fa7d06125f95851335964035492d67096e8e3051ebd80bab9557fca7eccbfb5e` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | `.GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Leases-Backpressure-v1.md` | `a7483370b3e309abcbee0b1e5c18615410c2ada92db22c57f1246f0ede802f93` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-ModelSession-Postgres-Queue-Workers-v1 | `.GOV/task_packets/stubs/WP-1-ModelSession-Postgres-Queue-Workers-v1.md` | `42b20ed2dd8c520a019032edb51973ab49b57c67c90212bc1efd47b79c6ff745` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-FEMS-Postgres-Memory-Store-v1 | `.GOV/task_packets/stubs/WP-1-FEMS-Postgres-Memory-Store-v1.md` | `f3d77fc67144ccfd3e6aeee01a683318f1cfdf69f07ed3c4dccc9de8e5b21a29` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | `.GOV/task_packets/stubs/WP-1-Workflow-Engine-Postgres-Durable-Execution-v1.md` | `2abaaf481d34acade35ea172b26db947d1a9525f77dde0cd42493832af01f84c` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-DCC-Postgres-Control-Plane-Projections-v1 | `.GOV/task_packets/stubs/WP-1-DCC-Postgres-Control-Plane-Projections-v1.md` | `bcc5adb8d8cb0cdabf20bdb50fa49ea1d3ae40ca56cff7ecf22e4c53e980d018` | FULL_STUB_FOLDED_TRANSITIVE |
| WP-1-SQLite-Cache-Offline-Boundaries-v1 | `.GOV/task_packets/stubs/WP-1-SQLite-Cache-Offline-Boundaries-v1.md` | `b0377704e89922c158720fe9bf32237f8358ef6847e4339726cbaeacb8ccccb1` | FULL_STUB_FOLDED_TRANSITIVE_WITH_RESET_OVERRIDE |
| WP-1-Software-Delivery-Runtime-Truth-v1 | `.GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.md` | `85906f631d2d45d6ac946d63cf627fe32be5081b3f085b79379a5258c57e3e78` | FULL_STUB_FOLDED |
| WP-1-Workflow-Transition-Automation-Registry-v1 | `.GOV/task_packets/stubs/WP-1-Workflow-Transition-Automation-Registry-v1.md` | `31e8380e93c8dff47d2ac1b8d93a89984ac9db32e6d9d08ba8112f15caa65b59` | FULL_STUB_FOLDED |
| WP-1-Dev-Command-Center-MVP-v1 | `.GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.md` | `c08b0372b562475a0f45dd9374c1b21de90cb8dc0c0adc1053bf4821364e78b6` | FULL_STUB_FOLDED |
| WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1 | `.GOV/task_packets/stubs/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.md` | `48d658515ebc042061fff0b2b72eee868e66af5b836a61d6bac424c5530953c4` | FULL_STUB_FOLDED |
| WP-1-Dev-Command-Center-Layout-Projection-Registry-v1 | `.GOV/task_packets/stubs/WP-1-Dev-Command-Center-Layout-Projection-Registry-v1.md` | `35a4dad6c2a6463b0ba2bf28309a9f410c2eef8e7c45c92463bb76c60b464c71` | FULL_STUB_FOLDED |
| WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1 | `.GOV/task_packets/stubs/WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1.md` | `3d334db27543a1209895d7865644ad7cd7ea78aba022e3ab6b76f35b3cd08c75` | FULL_STUB_FOLDED |
| WP-1-FEMS-Write-Time-Safeguards-v1 | `.GOV/task_packets/stubs/WP-1-FEMS-Write-Time-Safeguards-v1.md` | `478b61d701afed3d04e4ac4224bc6299c998f74271adb4fe170ac9be577a4701` | FULL_STUB_FOLDED_WITH_STORAGE_OVERRIDE |
| WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1 | `.GOV/task_packets/stubs/WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1.md` | `81d08167ecd52d00b4057a3d898392cc890596c94eced1c99844f7c66b2c7776` | FULL_STUB_FOLDED |
| WP-1-FEMS-MT-Handoff-Memory-Context-v1 | `.GOV/task_packets/stubs/WP-1-FEMS-MT-Handoff-Memory-Context-v1.md` | `2fcf8539ce8fdeafe983c4585f04c8001e00896849d7dc90421c5a4edb2dfed3` | FULL_STUB_FOLDED |
| WP-1-Session-Spawn-Tree-DCC-Visualization-v1 | `.GOV/task_packets/stubs/WP-1-Session-Spawn-Tree-DCC-Visualization-v1.md` | `4ee8fcb66087a2a03bb37e2699ff3f5aa29ec175839e16786cea9501ab2517f0` | FULL_STUB_FOLDED |
| WP-1-Session-Spawn-Conversation-Distillation-v1 | `.GOV/task_packets/stubs/WP-1-Session-Spawn-Conversation-Distillation-v1.md` | `e5cd2f010cd1ed02307792079a556526c37af673c8f857d47ffa176cf89f36f0` | FULL_STUB_FOLDED |
| WP-1-Visual-Debugging-Loop-v1 | `.GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.md` | `e7c521856929e4d96a23046d1f3d10ee2793010d61785cf7e69ce4db8ecf542a` | FULL_STUB_FOLDED |
| WP-1-Product-Screenshot-Visual-Validation-v1 | `.GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.md` | `aa410c75ca5c0a85bbae90b3451d388579a1d033a5aee260bd2c7c5be5d94bcf` | FULL_STUB_FOLDED |
| WP-1-Markdown-Mirror-Sync-Drift-Guard-v1 | `.GOV/task_packets/stubs/WP-1-Markdown-Mirror-Sync-Drift-Guard-v1.md` | `aa5ad6b368a1777ef33d4b41e78ac1bc0f5a3a1df9cadaf03863722be98ffa0b` | FULL_STUB_FOLDED |
| WP-1-Software-Delivery-Governance-Overlay-Boundary-v1 | `.GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.md` | `949cb0143adb074ec46d5c84bfe200987103e23c442a4b5e7530a5e07e06ed09` | FULL_STUB_FOLDED |
| WP-1-Software-Delivery-Overlay-Coordination-Records-v1 | `.GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Coordination-Records-v1.md` | `34cb80503244efcc8d71f552feec11a806ffea9d30acf39cfd508c4b56a88527` | FULL_STUB_FOLDED |
| WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1 | `.GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1.md` | `4c9c9de3ebea742c40fd3072cd26d219de0a86faf9bcaf0ad6009df18b54df58` | FULL_STUB_FOLDED |
| WP-1-Role-Turn-Isolation-v1 | `.GOV/task_packets/stubs/WP-1-Role-Turn-Isolation-v1.md` | `0d390abb2fb0460fcc5debb01406c613b926ca0e591be9b02a14ac5e9105fcd4` | FULL_STUB_FOLDED |
| WP-1-LocalFirst-Agentic-MCP-Posture-v1 | `.GOV/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.md` | `eaaf9ea2f49e7baab0deec309750fbd8d9c48a155ee06774ae19d4d8de37a7cf` | FULL_STUB_FOLDED |
| WP-1-Git-Engine-Decision-Gate-v1 | `.GOV/task_packets/stubs/WP-1-Git-Engine-Decision-Gate-v1.md` | `edfb98b8ca3bcc137ecfcc2766396ada0776d971a3d080616894b91005d4e7b8` | FULL_STUB_FOLDED |
| WP-1-Session-Anti-Pattern-Registry-v1 | `.GOV/task_packets/stubs/WP-1-Session-Anti-Pattern-Registry-v1.md` | `207ae7c5a6f3080155c1ad1a7f6ab3a484f2618d202c37c2ef9fb616020aaa36` | FULL_STUB_FOLDED |
| WP-1-Work-Profiles-v1 | `.GOV/task_packets/stubs/WP-1-Work-Profiles-v1.md` | `4ac98b03e6af6d5ba41980575b98391f5c6f5e70b846a3dce84b283711a7b36f` | FULL_STUB_FOLDED |
| WP-1-Governance-Pack-v1 | `.GOV/task_packets/stubs/WP-1-Governance-Pack-v1.md` | `f443c20ffc9895c12a9896c385e9fcfb6581f1aa179f459402d45acaef417d3e` | FULL_STUB_FOLDED |
| WP-1-Locus-Work-Tracking-System-Phase1-v1 | `.GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.md` | `6a4fa49233f7d36102c4b585bdbaa67703e12f14aef1a4dc1841fc49ff11900d` | FULL_STUB_FOLDED_WITH_STORAGE_OVERRIDE |
| WP-1-Inbox-Role-Mailbox-Alignment-v1 | `.GOV/task_packets/stubs/WP-1-Inbox-Role-Mailbox-Alignment-v1.md` | `4af0d85eb3693f4f33a4de1917422831d342dce89c3860fe53e8520a41578031` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1.md` | `95e912f9df5267e96150b7b8045b9b3f403f16ad165f10671ce10332a80ec6a1` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Message-Thread-Contract-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Message-Thread-Contract-v1.md` | `bf325c9033b1daa42d68d433e1c56ef39efb05e36e9eef9cbd3ea6f593d9be00` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1.md` | `d3bb810783b291489c0ac14a2ddd2448fe410b2cfe6e324b929d6585ee5a72c0` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Triage-Queue-Controls-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Triage-Queue-Controls-v1.md` | `3206b9b86e84af4e8bcc431bc342adb9b4f40f3818db5c03d71bad4a4eca8f9f` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1.md` | `87f289e590265545d732edca5e4faf41f10697ababbabbe990bac647e820fb98` | FULL_STUB_FOLDED |
| WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1 | `.GOV/task_packets/stubs/WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1.md` | `7bc0a81a45d5ddd5d60bca63fc7e56f959bf641b2de8a7b10abfac8ce6f9557f` | FULL_STUB_FOLDED |

## VALIDATED_PREREQUISITES_REUSED_NOT_FOLDED
- WP-1-Global-Silent-Edit-Guard: reuse existing direct/silent edit guard posture as prior product evidence; Kernel002 must add action/write-box enforcement rather than reopen that validated packet.
- WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Contract-Hardening, WP-1-Structured-Collaboration-Governed-Next-Action-Alignment, and WP-1-Structured-Collaboration-Schema-Registry-v4: reuse the schema/projection discipline and governed next-action constraints.
- WP-1-Artifact-System-Foundations: reuse artifact storage, materialization, retention, and evidence bundle rules.
- WP-1-Role-Mailbox-v1 and WP-1-Micro-Task-Executor-v1: reuse validated baseline capabilities, then extend them through the folded Role Mailbox and MT-loop stubs.
- WP-1-Dev-Command-Center-Control-Plane-Backend-v1: reuse as the DCC backend substrate if current implementation remains valid under Kernel001/Kernel002 authority.

## RESEARCH_BASIS (DRAFT)
- Sources checked:
  - Yjs documentation, including document updates/state-vector model: https://docs.yjs.dev/ and https://docs.yjs.dev/api/document-updates
  - Loro documentation: https://loro.dev/docs/tutorial/get_started
  - Automerge documentation: https://automerge.org/docs/
  - Reset brief: `.GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md`
  - Folded source stubs listed above.
- Patterns found:
  - Mature CRDT libraries model collaboration as operation/update streams and snapshots; this maps cleanly to a Postgres append log plus compaction snapshots.
  - CRDT merge success is not authority success. It solves collaborative convergence, not workflow legality, schema validity, operator approval, validation, or promotion.
  - Presence/awareness should be treated as ephemeral coordination state unless promoted into a typed authority event.
  - Field-driven projections and action previews are safer than treating board movement, Markdown edits, or mailbox replies as workflow mutation.
- Selected approach:
  - Use `WP-KERNEL-001` EventLedger, ToolGate, ArtifactProposal, ValidationRunner, PromotionGate, and TraceProjection as the authority substrate.
  - Add a CRDT-backed workspace for live collaborative drafts.
  - Add a typed `KernelActionCatalogV1` and `WriteBoxV1` family so no-context models work through mechanical action envelopes rather than editing authority files directly.
  - Persist CRDT updates, promotion requests, action requests, denials, receipts, and projections through Postgres-backed product authority.
  - Keep Markdown and board surfaces as generated projections or advisory edit capture surfaces.
- Rejected options:
  - Raw file editing as the model interface: rejected because it reintroduces surface drift and bypasses action legality.
  - CRDT as final authority: rejected because CRDT convergence cannot prove validation, approval, role eligibility, or workflow legality.
  - SQLite as Kernel V1 scaffold or fallback authority: rejected by reset brief and by the no-split-brain authority requirement.
  - A UI-only DCC fix without action contracts: rejected because it would only make drift easier to see, not mechanically prevent.

## INTENT (DRAFT)
- What: Build the second kernel follow-up as a single pre-use hardening WP that folds the complete kernel-adjacent backlog stubs and adds the missing mechanical layer: CRDT workspace state, action catalog, write boxes, direct-edit denial, advisory normalization, Markdown mirror drift prevention, software-delivery runtime truth, DCC projection, Role Mailbox coordination, FEMS checkpoint safety, visual debugging, no-context model operation, and a contract-first stub-to-work-packet-to-microtask lifecycle.
- Why: Kernel001 creates the first event/promotion substrate, but it intentionally does not make models stop editing files directly, does not implement full CRDT workspace/promotion, and does not close the folded software-delivery, DCC, Role Mailbox, memory, and projection gaps needed before the kernel is safe to use as a harness.
- Why one WP: Splitting these stubs again would recreate the original cost: repeated authority choices, repeated projection repair, hidden Markdown/mirror truth, duplicated action semantics, and drift between Locus, Task Board, Role Mailbox, DCC, CRDT workspace, and EventLedger.
- Operator intent preserved: future models with no context must see a catalogue of lawful actions, tools, and write boxes and be forced to use that interface. They may draft artifacts, propose changes, and create task outputs, but they must not mutate authority artifacts by hand.
- Detail preservation intent: the activated work packet must keep the full technical detail itself, even when it mechanically emits smaller microtasks. Microtasks are executable slices for smaller/local models, not the only place where implementation detail lives.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Full preservation and activation integration of every folded source stub listed in `SOURCE_STUBS_FOLDED_COMPLETE`.
  - Contract-first lifecycle:
    - `StubContractV1`: inactive planning object for future features, operator ideas, refinement outputs that do not fit the current WP, and deferred scope. Stubs can carry enough detail to seed a future WP, but they are not executable work authority.
    - `WorkPacketContractV1`: active execution authority that carries full implementation detail, technical context, constraints, source imports, scope edges, acceptance criteria, verification, risks, and an embedded microtask source plan.
    - `MicroTaskContractV1`: generated execution unit derived from the work packet source plan, small enough for weaker/smaller/local models to execute with fresh context and bounded verification.
    - Stub-to-WP promotion and WP-to-MT extraction must be mechanical operations with receipts, ids, provenance hashes, and projection refreshes.
  - Work packet detail preservation:
    - Microtasks may duplicate and specialize work-packet detail, but must not become the only technical authority.
    - The official work packet must remain independently usable by a no-context strong model without opening each MT file.
    - The official work packet must include enough structured detail to regenerate MT files, MT contracts, and human projections deterministically.
  - Local-model microtask loop preparation:
    - Define the future Locus-compatible loop contract for one fresh model context per MT attempt, retry budget, receipt capture, verifier outcome, memory checkpoint input, and requeue/fail rules.
    - Smaller/local model MT loops must consume machine-readable MT contracts, not prose-only instructions.
  - Validator-mediated remediation loop:
    - Handshake, not the coder or validator model, owns MT/WP status transitions after work handoff.
    - Coders submit structured work status and evidence to Handshake; they do not manually advance packet or MT status.
    - Handshake mechanically requests Integration Validator batch review for the completed MT bundle against the MT contract, parent WP contract, allowed action catalog, write boxes, code diff, receipts, and test evidence.
    - Integration Validator submits structured pass/fail verdicts, mediation instructions, issue reports, bug reports, gap reports, out-of-scope findings, and evidence references; it does not manually edit packet/MT status.
    - Handshake records verdicts mechanically, updates projections, and creates remediation MTs or remediation packets when a fail, gap, bug, or out-of-scope finding requires follow-up work.
    - Handshake loops back before forward progress: failed or mediation-required MTs must generate a bounded remediation path and a coder dispatch before dependent MTs advance.
    - When the current coder has finished its active MT, Handshake may dispatch the next eligible coder session with the generated remediation MT or next ready MT, subject to leases, actor eligibility, retry budget, and dependency state.
  - Generated documentation/status maintenance:
    - Task Board rows, packet status, microtask status, DCC work views, mirror docs, and operator summaries must be generated projections from contracts, receipts, runtime state, and validation outputs.
    - A model must not manually update documentation status when a mechanical status receipt/projection path exists.
  - A `KernelActionCatalogV1` registry that defines every lawful model/operator/tool action by stable action id, input schema, output schema, authority effect, allowed actor/profile, required approvals, validation hooks, receipt events, write boxes, and promotion path.
  - A `WriteBoxV1` family for bounded work surfaces:
    - `DraftBox`: freeform or structured draft that is not authority.
    - `CRDTWorkspaceBox`: live collaborative state backed by CRDT updates and Postgres persistence.
    - `ProposalBox`: proposed authority mutation with target ids, patch intent, provenance, and validator requirements.
    - `PatchBox`: typed candidate patch against an allowed target, never applied outside PromotionGate.
    - `ArtifactBox`: materialized generated artifact with hash, retention, and evidence refs.
    - `MirrorAdvisoryBox`: human/model edit against a generated mirror captured as advisory input.
    - `MemoryBox`: typed checkpoint or memory proposal subject to FEMS validation.
    - `ExecutionBox`: command/test/sandbox execution request and result envelope.
    - `PromotionBox`: accepted proposal emitted through PromotionGate into EventLedger-backed authority.
  - Direct-edit prevention:
    - No model-facing product workflow may instruct a model to edit authority files directly.
    - ToolGate must reject direct authority-file edits when a registered action/write box exists.
    - If an unavoidable external tool returns a raw diff, Handshake must wrap it as `ProposalBox` or `PatchBox`, not apply it directly.
    - Denials emit replayable evidence with actor id, attempted target, matching lawful action, and remediation instructions.
  - CRDT workspace:
    - Live collaborative project documents/workspaces with actor/session attribution, CRDT update persistence, snapshots, state vectors or equivalent sync cursors, compaction, and restart-safe replay.
    - CRDT update records linked to EventLedger, ArtifactProposal, PromotionGate, Role Mailbox, Locus/work item, and DCC projections by stable ids.
    - Context slicing for models: field summaries, change digests, selection/range pulls, and schema-aware extracts instead of dumping whole CRDT documents into prompt context.
    - Promotion path from CRDT change/draft to authority event through validation and approval.
  - Software-delivery runtime truth and overlay control:
    - Product-owned runtime records for work items, actions, workflow state, claims, leases, queued steering, recovery posture, and imported repo-governance overlay evidence.
    - Repo `.GOV/**` content is overlay/evidence/import material, not live product runtime truth.
  - DCC and projection surfaces:
    - DCC MVP, structured artifact viewer, layout/preset registry, runtime-state projections, mailbox/queue projections, action previews, visual debugging evidence, and screenshot capture surface.
    - DCC remains a projection/control surface over product authority, not authority itself.
  - Role Mailbox and handoff:
    - Typed thread lifecycle, allowed responses, action requests, micro-task loop checkpoints, triage queues, claim/lease, handoff bundle, announce-back provenance, inbox label alignment, and debug-bundle evidence bridge.
    - Mailbox prose and thread chronology cannot become workflow truth.
  - FEMS checkpoint safety:
    - Working-memory checkpoint schema, write-time safeguards, memory poisoning/drift guardrails, and MT handoff memory context integrated as typed state and action requests.
  - No-context model manual:
    - Durable project-local and product-facing model manual that explains purpose, startup, allowed actions, write boxes, safety constraints, common failure modes, recovery, DCC paths, CRDT promotion, and validation.
- OUT_OF_SCOPE:
  - Editing `WP-KERNEL-001-Event-Ledger-Session-Broker-v1` files or changing its 27 microtasks.
  - Kernel003 sandbox depth beyond the action/write-box hooks needed for later sandbox promotion.
  - Kernel004 full local-model/memory-runtime depth beyond the FEMS checkpoint/write-safety items folded here.
  - Broad non-kernel Phase 1 feature work unrelated to the folded source stubs.
  - Treating CRDT state, Markdown mirrors, board drag/drop, or mailbox replies as final authority without PromotionGate.

## ACCEPTANCE_CRITERIA (DRAFT)
- The official activated packet preserves the full folded-source manifest with source paths, pre-fold hashes, and a rule that activation must import full source intent/scope/acceptance/risks, not selectively excerpt reset-relevant lines.
- The official activated packet defines `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` with lifecycle states, authority rules, required fields, provenance fields, projection rules, validation hooks, and receipt events.
- The official work packet carries the full technical implementation detail and an embedded microtask source plan; generated MT files cannot be the only location that contains technical detail needed to execute the WP.
- A mechanical operation can promote a stub to a draft work packet/refinement candidate without losing stub detail, source hashes, operator intent, or folded-source obligations.
- A mechanical operation can extract or regenerate MT contracts/files from a work packet and refresh packet, MT, Task Board, traceability, DCC, and mirror projections from the same authority data.
- Microtasks contain enough scoped context, constraints, acceptance criteria, verification steps, allowed actions, write boxes, dependencies, and handoff rules for smaller/local models to execute one MT per fresh context loop.
- `CoderHandoffContractV1`, `ValidatorVerdictContractV1`, `MediationInstructionContractV1`, `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, `OutOfScopeReportContractV1`, and `RemediationMicroTaskContractV1` are defined as machine-readable contracts with stable ids, source refs, evidence refs, severity, affected acceptance criteria, affected files/actions, and required follow-up state.
- A coder handoff mechanically triggers Integration Validator batch review when the MT contract requires review; validator verdicts are recorded through Handshake and cannot be represented by manual packet/MT status edits.
- Fail, mediation-required, bug, gap, and out-of-scope verdicts mechanically create one of: a remediation MT, a remediation packet/stub, a blocked-state escalation, or an operator-decision request with evidence and dependency impact.
- Handshake prevents dependent MTs from advancing past failed or mediation-required prerequisites until remediation has passed or an authorized escalation resolves the block.
- The loop supports fresh-context local model execution: coder MT attempt -> handoff receipt -> validator review -> verdict receipt -> generated remediation/next-MT dispatch -> repeat until pass, blocked, or escalated.
- Documentation and status surfaces are updated from receipts/runtime contracts/projection rebuilds. Manual model edits to status fields, sidecars, task-board rows, or mirror docs are denied or normalized as advisory proposals.
- All source stubs folded directly by Kernel002 are marked superseded or folded in their own files and Task Board entries after this stub is created.
- The Task Board contains one active Kernel002 stub entry and no independent active backlog entries for directly folded source stubs.
- `WP_TRACEABILITY_REGISTRY.md` maps Kernel002 and records folded source stubs as superseded/folded, so no orchestrator can accidentally activate them separately.
- A no-context model can identify:
  - allowed action catalog location and action ids,
  - allowed write boxes,
  - which surfaces are authority,
  - which surfaces are projections/advisory,
  - how to create work output without direct file mutation,
  - how to request promotion,
  - how to recover from a denied edit.
- Raw direct mutation of an authority artifact is mechanically rejected or transformed into an advisory proposal with denial/proposal evidence.
- CRDT workspace edits persist through restart, carry actor/session attribution, project into DCC, and remain non-authoritative until promotion.
- Promotion from CRDT or write-box proposal to authority requires validation, approval where required, EventLedger entry, and projection refresh.
- Markdown mirrors, Task Board projections, Role Mailbox summaries, and DCC views expose synchronized, stale, advisory, normalization-required, and conflict states without relying on raw file diff inspection.
- Software-delivery runtime truth can answer current work posture by stable records/actions, not by packet prose, mailbox chronology, or mirror freshness.
- Role Mailbox action requests, claim/lease posture, handoff bundles, and transcription state are queryable without replaying full threads.
- FEMS checkpoints and memory handoff/write safeguards protect no-context and resumed sessions from hidden memory drift.
- Visual/screenshot evidence can be attached to GUI-bearing actions and validator loops before the kernel is used for frontend/governed UI work.
- No Kernel002 implementation path introduces SQLite as an authority store, offline replica, fallback authority, or convenience database for Kernel V1.

## MICRO_TASK_PLAN (DRAFT MINIMUM DECOMPOSITION)

Activation Manager must convert this draft into official packet microtask files during activation. The list below is a minimum decomposition guide, not a cap. Split any item further when files, test proof, or reviewer ownership would cross unrelated authority boundaries.

### MT-001 Fold Preservation Manifest and Source Import
- Focus: materialize the complete folded-source manifest in the official packet/refinement.
- Acceptance: every listed source stub has path, pre-fold hash, direct/transitive fold classification, and source-scope import instructions. Activation cannot proceed if any source file is missing or hash mismatch is unexplained.

### MT-002 Reset Invariant Reconciliation
- Focus: reconcile folded legacy assumptions with reset invariants.
- Acceptance: every source obligation that mentions SQLite, Markdown authority, mailbox chronology, or UI-local truth is explicitly converted to Postgres authority, projection/advisory status, or promotion-gated action semantics.

### MT-003 CRDT Library and Storage ADR
- Focus: compare Yjs, Loro, Automerge, and existing product dependencies against Handshake runtime needs.
- Acceptance: ADR selects a CRDT approach, rejected options, sync/storage model, Rust/TypeScript integration boundary, schema compatibility, and validation plan.

### MT-004 Kernel Action Envelope
- Focus: define `KernelActionRequestV1`, `KernelActionResultV1`, `KernelActionDenialV1`, and receipt/event mappings.
- Acceptance: action requests carry actor/session/profile, target ids, input schema id, expected write boxes, authority effect, approval posture, validation requirements, and trace id.

### MT-005 Action Catalog Registry
- Focus: implement the durable `KernelActionCatalogV1` registry.
- Acceptance: every model-facing action has stable id, schemas, role eligibility, capability requirements, write boxes, promotion path, validation hooks, and DCC preview metadata.

### MT-006 Write Box Schema Family
- Focus: define `DraftBox`, `CRDTWorkspaceBox`, `ProposalBox`, `PatchBox`, `ArtifactBox`, `MirrorAdvisoryBox`, `MemoryBox`, `ExecutionBox`, and `PromotionBox`.
- Acceptance: each write box has lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules.

### MT-007 Direct Edit Denial Path
- Focus: route model/tool attempts to mutate authority artifacts through ToolGate denial or proposal wrapping.
- Acceptance: tests prove raw authority-file edit attempts fail with actionable denial evidence and lawful replacement action ids.

### MT-008 Advisory Edit Normalization
- Focus: convert manual/model edits against generated mirrors into `MirrorAdvisoryBox` records.
- Acceptance: advisory edits do not mutate authority until a registered normalization/promotion action validates and accepts them.

### MT-009 No-Context Model Manual
- Focus: create durable model-facing instructions for using Handshake mechanically.
- Acceptance: the manual explains purpose, startup, action catalog, write boxes, DCC paths, CRDT workflow, safety constraints, failure modes, denial recovery, and validation evidence for a model with no conversation history.

### MT-010 CRDT Document Identity and Workspace Model
- Focus: define document/workspace ids, actor ids, site/client ids, schema ids, and authority links.
- Acceptance: CRDT records can be linked to work item, action request, artifact proposal, Role Mailbox thread, DCC projection, and EventLedger ids.

### MT-011 CRDT Update Persistence
- Focus: persist CRDT updates in Postgres with ordering, hash, actor/session attribution, and replay metadata.
- Acceptance: a workspace can be reconstructed from persisted updates after restart without file-system authority assumptions.

### MT-012 CRDT Snapshot and Compaction
- Focus: add snapshot/state-vector or equivalent sync cursor support.
- Acceptance: update replay is bounded by snapshots, old updates remain auditable or compacted according to policy, and compaction never drops promotion evidence.

### MT-013 CRDT Context Slicing for Models
- Focus: expose summaries, selected ranges, field digests, and operation deltas.
- Acceptance: model prompts can request bounded CRDT context without loading entire documents, and extract outputs cite workspace/version/source ids.

### MT-014 CRDT Schema and Validity Guard
- Focus: validate CRDT materialized state before promotion.
- Acceptance: structurally invalid, unauthorized, or schema-drifted CRDT state cannot be promoted into authority.

### MT-015 CRDT Promotion Bridge
- Focus: convert CRDT edits/drafts into ArtifactProposal and PromotionGate inputs.
- Acceptance: accepted promotions emit EventLedger authority events; rejected promotions keep CRDT/draft state as non-authoritative evidence.

### MT-016 Conflict and Presence Projection
- Focus: expose presence, pending conflicts, actor attribution, and merge/proposal state.
- Acceptance: DCC can show who changed what, which edits are merely merged CRDT state, and which changes are pending promotion.

### MT-017 Software-Delivery Runtime Truth Records
- Focus: fold `WP-1-Software-Delivery-Runtime-Truth-v1`.
- Acceptance: current software-delivery posture is queryable from product-owned stable records and governed actions, not packet prose, mailbox order, or Markdown freshness.

### MT-018 Workflow Transition Automation Registry
- Focus: fold `WP-1-Workflow-Transition-Automation-Registry-v1`.
- Acceptance: every workflow mutation has a registered transition rule, eligible actor, action trigger, approval boundary, and DCC preview.

### MT-019 Governance Overlay Boundary
- Focus: fold `WP-1-Software-Delivery-Governance-Overlay-Boundary-v1`.
- Acceptance: imported repo `.GOV/**` artifacts are evidence/source overlays, not runtime truth, and import/export cannot bypass gates.

### MT-020 Overlay Coordination Records
- Focus: fold `WP-1-Software-Delivery-Overlay-Coordination-Records-v1`.
- Acceptance: claim/lease, queued steering, follow-up, takeover, and actor eligibility are queryable by stable ids without mailbox chronology.

### MT-021 Overlay Lifecycle and Recovery Control Plane
- Focus: fold `WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1`.
- Acceptance: start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture are record-backed and projection-safe.

### MT-022 Postgres Control-Plane Residual Scope
- Focus: fold `WP-1-Postgres-Control-Plane-Shift-Bundle-v1` plus its transitive folded stubs.
- Acceptance: residual live Postgres service proof, leases/backpressure, ModelSession queues, FEMS memory store, durable workflow execution, DCC projections, and SQLite boundary obligations are carried into Kernel002 or explicitly mapped to Kernel003/Kernel004 without reopening the old bundle.

### MT-023 Locus Work Tracking Reset Migration
- Focus: fold `WP-1-Locus-Work-Tracking-System-Phase1-v1`.
- Acceptance: WP/MT tracking, dependencies, occupancy, query, Task Board projection, and Flight Recorder obligations are preserved, but SQLite authority is replaced with Postgres/EventLedger/CRDT-compatible authority.

### MT-024 DCC MVP Runtime Surface
- Focus: fold `WP-1-Dev-Command-Center-MVP-v1`.
- Acceptance: DCC can select work, view worktree/session/action/proposal state, inspect diffs/evidence, preview approvals, and trigger governed actions through the catalog.

### MT-025 DCC Structured Artifact Viewer
- Focus: fold `WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1`.
- Acceptance: DCC renders canonical fields before mirrors, exposes mirror state, and provides raw structured drilldown as advanced view.

### MT-026 DCC Layout Projection Registry
- Focus: fold `WP-1-Dev-Command-Center-Layout-Projection-Registry-v1`.
- Acceptance: board, queue, list, roadmap, inbox-triage, and execution-queue views derive from registered presets and action bindings.

### MT-027 Role Mailbox Message and Action Request Contract
- Focus: fold `WP-1-Role-Mailbox-Message-Thread-Contract-v1`.
- Acceptance: mailbox lifecycle, delivery state, allowed responses, due/dead-letter posture, and action requests are typed and authority-bounded.

### MT-028 Role Mailbox Micro-Task Loop Control
- Focus: fold `WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1`.
- Acceptance: retry budget, verifier outcome, escalation, completion report, dead-letter, and loop checkpoint state are compact and replayable.

### MT-029 Role Mailbox Triage Queue Controls
- Focus: fold `WP-1-Role-Mailbox-Triage-Queue-Controls-v1`.
- Acceptance: reminder, snooze, expiry, dead-letter, retry/reroute/archive, and Task Board pressure overlays are field-backed projections.

### MT-030 Role Mailbox Claim and Lease
- Focus: fold `WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1`.
- Acceptance: claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility are explicit and queryable.

### MT-031 Role Mailbox Handoff and Announce-Back
- Focus: fold `WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1`.
- Acceptance: handoff bundles, transcription targets, recommended next actor, announce-back provenance, and advisory/completion distinction are typed.

### MT-032 Role Mailbox Inbox Alignment and Evidence Bridge
- Focus: fold `WP-1-Inbox-Role-Mailbox-Alignment-v1` and `WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1`.
- Acceptance: Inbox labels map to Role Mailbox only, mailbox telemetry is leak-safe, and debug bundle exports preserve stable evidence/provenance.

### MT-033 FEMS Working-Memory Checkpoints
- Focus: fold `WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1`.
- Acceptance: SESSION_OPEN, PRE_TASK, INSIGHT, TASK_COMPLETE, SESSION_CLOSE, memory extract, repeated insight promotion, and GC are typed and quality-gated.

### MT-034 FEMS Write-Time Safeguards
- Focus: fold `WP-1-FEMS-Write-Time-Safeguards-v1`.
- Acceptance: novelty scoring, supersession, contradiction detection, dedup, state validation, and audit trail run mechanically; SQLite/FTS5 references are reworked to reset-approved storage/search primitives.

### MT-035 FEMS Memory Poisoning and Drift Guardrails
- Focus: fold `WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1`.
- Acceptance: trust gates, pack budget, deterministic reduction, proposal/approval/denial events, and effective pack hashes prevent untrusted long-lived memory drift.

### MT-036 FEMS MT Handoff Memory Context
- Focus: fold `WP-1-FEMS-MT-Handoff-Memory-Context-v1`.
- Acceptance: escalated or handed-off MTs carry typed memory context with source/target sessions, failed attempts, recommended items, provenance, and bounded scoring.

### MT-037 Role Turn Isolation
- Focus: fold `WP-1-Role-Turn-Isolation-v1`.
- Acceptance: role turns default to isolated context, replay pins are recorded, and cross-role bleed is mechanically prevented.

### MT-038 Work Profiles
- Focus: fold `WP-1-Work-Profiles-v1`.
- Acceptance: profile storage, selection, immutable profile ids, per-role routing, autonomy knobs, and profile receipts are wired into action requests.

### MT-039 Local-First Agentic MCP Posture
- Focus: fold `WP-1-LocalFirst-Agentic-MCP-Posture-v1`.
- Acceptance: local-first execution remains default; MCP/cloud paths are capability-gated adapters with cached artifacts and fallback behavior.

### MT-040 Git Engine Decision Gate
- Focus: fold `WP-1-Git-Engine-Decision-Gate-v1`.
- Acceptance: one repo engine path is recorded/enforced, dangerous git actions remain gated, and DCC/action catalog expose only lawful git affordances.

### MT-041 Session Anti-Pattern Registry
- Focus: fold `WP-1-Session-Anti-Pattern-Registry-v1`.
- Acceptance: scheduler/trust/capability/session orchestration anti-patterns have machine-readable detections and deny/downgrade/consent/stop outcomes.

### MT-042 Governance Pack Instantiation
- Focus: fold `WP-1-Governance-Pack-v1`.
- Acceptance: project identity, pack manifest, instantiation, naming/path policy, conformance harness, and imported-overlay boundaries are compatible with Kernel002 action/write-box law.

### MT-043 Session Spawn Tree DCC Visualization
- Focus: fold `WP-1-Session-Spawn-Tree-DCC-Visualization-v1`.
- Acceptance: DCC shows spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from runtime records.

### MT-044 Session Spawn Conversation Distillation
- Focus: fold `WP-1-Session-Spawn-Conversation-Distillation-v1`.
- Acceptance: parent-child request/summary pairs and spawn metadata feed distillation artifacts without making conversation text authority.

### MT-045 Product Screenshot Capture
- Focus: fold `WP-1-Product-Screenshot-Visual-Validation-v1`.
- Acceptance: governed sessions can capture full app, panel, and module screenshots with metadata and artifact refs.

### MT-046 Visual Debugging Loop
- Focus: fold `WP-1-Visual-Debugging-Loop-v1`.
- Acceptance: post-commit or post-action screenshot capture, baseline comparison, visual evidence storage, threshold config, and validator steering are available for GUI work.

### MT-047 Markdown Mirror Sync Drift Guard
- Focus: fold `WP-1-Markdown-Mirror-Sync-Drift-Guard-v1`.
- Acceptance: deterministic mirror regeneration, drift states, manual advisory handling, reconciliation, DCC mirror queue, and projection banners are implemented.

### MT-048 Direct-Edit Regression Harness
- Focus: prove future models cannot bypass write boxes through common edit paths.
- Acceptance: tests simulate model raw patch, generated file write, mirror edit, CRDT edit, mailbox reply, DCC quick action, and git action; each path either uses registered action/write box or fails with evidence.

### MT-049 Projection Rebuild and Task Board Sync
- Focus: regenerate projections and sync Task Board, traceability registry, build order, and stub contracts.
- Acceptance: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` pass or produce a concrete blocker.

### MT-050 Pre-Use Kernel Acceptance Run
- Focus: prove Kernel001 + Kernel002 are usable before real kernel operation.
- Acceptance: a no-context model follows the manual to draft in CRDT, submit a proposal, trigger validation, receive a promotion/denial, view DCC projections, and inspect evidence without direct authority-file edits.

### MT-051 Stub, Work Packet, and Microtask Contract Lifecycle
- Focus: define the machine-readable lifecycle from inactive stub to active work packet to generated microtask contracts.
- Acceptance: `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` schemas define states, authority rules, required fields, provenance hashes, source imports, lifecycle transitions, receipt events, projection hooks, validation hooks, and failure states.

### MT-052 Work Packet Full-Detail Authority and Microtask Source Plan
- Focus: ensure the activated work packet itself carries full implementation detail while also containing a structured MT source plan.
- Acceptance: a no-context strong model can execute from the work packet alone; the same packet can regenerate MT contracts/files without relying on manually maintained sidecars or hidden chat context.

### MT-053 Mechanical Stub Promotion and Microtask Extraction
- Focus: implement deterministic commands or action-catalog entries for stub-to-WP promotion and WP-to-MT extraction.
- Acceptance: promotion/extraction preserves operator intent, source hashes, folded details, dependencies, constraints, acceptance criteria, verification, and status provenance; every generated artifact records its source contract id and hash.

### MT-054 Local-Model Fresh-Context Microtask Loop Contract
- Focus: define the Locus-compatible execution loop for smaller/local models working one MT at a time.
- Acceptance: the loop contract covers fresh-context input bundle, allowed actions, write boxes, retry budget, verifier handoff, failure requeue, memory checkpoint input, receipt emission, and final MT outcome without requiring the model to inspect unrelated WP scope.

### MT-055 Generated Documentation and Status Projection
- Focus: replace manual status/docs maintenance with projections from contracts, receipts, runtime state, and validation outputs.
- Acceptance: packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries regenerate from machine-readable authority; direct manual status edits are denied or captured as advisory normalization input.

### MT-056 Coder Handoff and Validation Request Contract
- Focus: define the structured handoff from coder execution to Handshake-owned validation.
- Acceptance: `CoderHandoffContractV1` records MT id, parent WP id, actor/session, claimed scope, touched files/actions, receipts, tests, evidence, known blockers, and requested review; Handshake can generate a validator review request from it without a model editing status fields.

### MT-057 Validator Verdict and Mediation Contract
- Focus: define structured pass/fail/mediation verdicts from Integration Validator batch review.
- Acceptance: `ValidatorVerdictContractV1` and `MediationInstructionContractV1` encode verdict, failed acceptance criteria, evidence refs, severity, reproducibility, exact remediation instructions, dependency impact, and whether the MT may advance, must loop back, or must escalate.

### MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports
- Focus: define machine-readable reports for validator findings that are not simple pass/fail.
- Acceptance: `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` preserve validator reasoning, source refs, affected surfaces, reproduction or proof, proposed destination, and routing outcome without becoming manual prose-only reports.

### MT-059 Remediation Microtask and Packet Generation
- Focus: generate follow-up work from failed verdicts and reports.
- Acceptance: Handshake can create `RemediationMicroTaskContractV1` or a remediation packet/stub from verdict/report contracts, preserving parent WP/MT links, dependency state, acceptance criteria, allowed actions, write boxes, evidence refs, retry budget, and validator recheck requirements.

### MT-060 Loop Scheduler and Next-Coder Dispatch
- Focus: define the mechanical loop that dispatches coders after validation outcomes.
- Acceptance: Handshake only dispatches a new coder when leases, current coder completion, dependency state, retry budget, and verdict state allow it; failed prerequisites loop to remediation before dependent MTs can advance.

### MT-061 Locus Work Graph Projection for MT Validation Loops
- Focus: connect the validation/remediation loop to Locus work tracking semantics from the Master Spec.
- Acceptance: Locus can project MT nodes, validator verdicts, remediation edges, blocked/escalated states, actor leases, and pass/fail history without treating prose reports or chat messages as truth.

## RISKS / UNKNOWNs (DRAFT)
- Risk: this WP is large enough to hide cross-surface drift. Mitigation: one microtask per authority boundary, explicit file ownership at activation, one write-box/action contract reused everywhere, and validator review per MT.
- Risk: fold-by-reference could make future models skip source details. Mitigation: preserve source paths and hashes, require activation to import full source scope/acceptance/risks, and keep old stubs retained as superseded evidence.
- Risk: CRDT convergence may be mistaken for workflow authority. Mitigation: name CRDT state as workspace state only and require PromotionGate/EventLedger for authority.
- Risk: technical detail may drift between the work packet and generated microtasks. Mitigation: make the work packet the full-detail authority, make MT files generated projections/execution slices, and verify source hashes/projection freshness before MT claim.
- Risk: local/smaller models may act outside scope when given too much packet context or too little task context. Mitigation: generate one fresh-context MT bundle with explicit allowed actions, write boxes, dependencies, acceptance criteria, retry limits, and verifier handoff.
- Risk: generated documentation could become another stale mirror. Mitigation: define docs/status as projections with source ids, source hashes, freshness checks, and direct-edit denial/advisory normalization.
- Risk: validator prose could become another manual truth surface. Mitigation: require structured verdict, mediation, issue, bug, gap, and out-of-scope contracts; prose is projection or explanatory evidence only.
- Risk: remediation loops could run forever or block the WP invisibly. Mitigation: per-MT retry budgets, blocked/escalated terminal states, operator-decision requests, dependency-impact projection, and Integration Validator escalation.
- Risk: active coder and validator sessions could race on MT state. Mitigation: Handshake-owned state transitions, actor leases, one active owner per MT state, validator verdict receipts, and generated next-dispatch decisions.
- Risk: legacy stubs mention SQLite as implementation substrate. Mitigation: preserve the work intent but override storage authority with Postgres/EventLedger/CRDT per reset.
- Risk: DCC scope creep could turn Kernel002 into a broad UI project. Mitigation: DCC work is limited to projection/control surfaces needed for no-context operation and action previews.
- Risk: direct edit prevention at product level may conflict with current repo-governance authoring practices. Mitigation: treat Kernel002 as product harness behavior; repo `.GOV` editing remains governed by current repo authority until imported into product runtime.
- Unknown: final CRDT library choice. Decision deferred to MT-003 ADR, but implementation must use a mature CRDT library rather than hand-rolling merge logic.
- Unknown: exact Postgres schema layout after Kernel001 lands. Activation must inspect Kernel001's final product code before finalizing schemas.

## MACHINE_READABLE_GOVERNANCE_ARTIFACT_STANCE
- CONTRACT_FIRST_TARGET: YES
- STUB_CONTRACT_REQUIRED: YES
- WORK_PACKET_MACHINE_CONTRACT_REQUIRED: YES
- MICRO_TASK_MACHINE_CONTRACT_REQUIRED: YES
- STUB_TO_WORK_PACKET_PROMOTION_MECHANICAL: YES
- WORK_PACKET_TO_MICRO_TASK_EXTRACTION_MECHANICAL: YES
- WORK_PACKET_RETAINS_FULL_TECHNICAL_DETAIL: YES
- MICRO_TASKS_ARE_GENERATED_EXECUTION_SLICES: YES
- LOCAL_MODEL_MT_LOOP_CONTRACT_REQUIRED: YES
- CODER_HANDOFF_CONTRACT_REQUIRED: YES
- VALIDATOR_VERDICT_CONTRACT_REQUIRED: YES
- MEDIATION_INSTRUCTION_CONTRACT_REQUIRED: YES
- ISSUE_BUG_GAP_OUT_OF_SCOPE_REPORT_CONTRACTS_REQUIRED: YES
- REMEDIATION_MICRO_TASK_GENERATION_MECHANICAL: YES
- HANDSHAKE_OWNS_MT_STATUS_TRANSITIONS: YES
- CODER_VALIDATOR_MANUAL_STATUS_EDIT_ALLOWED: NO
- LOOPBACK_BEFORE_DEPENDENT_FORWARD_PROGRESS_REQUIRED: YES
- CURRENT_STUB_MARKDOWN_ROLE: PLANNING_STUB_UNTIL_ACTIVATION
- GENERATED_CONTRACT_SIDECAR_REQUIRED: YES
- GENERATED_CONTRACT_FILE: `.GOV/task_packets/stubs/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1.contract.json`
- GENERATED_STATUS_PROJECTIONS_REQUIRED: YES
- STATUS_DOCS_MANUAL_EDIT_ALLOWED_WHEN_MECHANICAL_PATH_EXISTS: NO
- MARKDOWN_IS_AUTHORITY_AFTER_ACTIVATION: NO
- SOURCE_STUBS_RETAINED_AS_SUPERSEDED_EVIDENCE: YES
- SOURCE_STUB_HASHES_REQUIRED_FOR_ACTIVATION: YES
- DIRECT_FILE_EDIT_ALLOWED_FOR_PRODUCT_AUTHORITY_AFTER_IMPLEMENTATION: NO
- CRDT_IS_FINAL_AUTHORITY: NO
- PROMOTION_GATE_REQUIRED_FOR_AUTHORITY: YES
- SQLITE_AUTHORITY_ALLOWED: NO

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm Kernel001 final accepted packet/code has landed or obtain explicit Operator approval to activate Kernel002 in parallel.
- [ ] Recompute and compare source hashes for every folded source stub. Any mismatch must be explained as supersession metadata added by this Kernel002 stub creation pass or as a deliberate operator-approved update.
- [ ] Produce a Technical Refinement Block that imports the complete folded-source obligations and reset invariants.
- [ ] Obtain USER_SIGNATURE for Kernel002 activation.
- [ ] Create a signed refinement for `WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- [ ] Create the official task packet via governed packet creation tooling.
- [ ] Define the mechanical stub-to-WP and WP-to-MT contract generation path before MT files are materialized.
- [ ] Define the mechanical coder-handoff, validator-verdict, mediation, report, remediation-MT, and next-coder dispatch loop before MT execution begins.
- [ ] Materialize MT files from `MICRO_TASK_PLAN` through the mechanical extraction path, splitting further where needed.
- [ ] Prove a failed MT produces a validator verdict, generated remediation MT, blocked/dependency projection, coder redispatch, and later pass verdict without manual status editing.
- [ ] Prove packet, MT, Task Board, traceability, DCC, and mirror status projections are regenerated from contracts/receipts rather than manually edited.
- [ ] Move this stub from Stub Backlog to Ready for Dev only after official packet creation.
