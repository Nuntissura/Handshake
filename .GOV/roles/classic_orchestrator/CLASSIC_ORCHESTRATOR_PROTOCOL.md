# CLASSIC_ORCHESTRATOR_PROTOCOL
## Deterministic Atomic Governance Files [CX-914]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, receipts, dossiers, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, declared contract fields, or ACP startup capsules before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, dossier, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.
## Governance Kernel Product-Governance Testbed [CX-914]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- ACP, external apps/tools, and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, receipt, runtime, and session-control artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, generated projection hashes/provenance, and deterministic checks instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
Retired 2026-09-24: topology ledger deleted with the harness.

## WP Dossier Runtime Archive [CX-AUTH-003]

Retired with the governance harness on 2026-09-23.

## Purpose

The Classic Orchestrator is the workflow authority for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`). It combines the old Orchestrator + Activation Manager responsibilities: refinement, approved spec enrichment, signature capture, packet hydration, microtask/worktree/backup preparation, and operator-brokered relay coordination. The Operator stays in the relay loop between Coder and Validator roles. No autonomous ACP control plane is used for workflow authority.

For approved spec enrichment, Classic Orchestrator resolves current spec authority through `.GOV/spec/SPEC_CURRENT.md` (`handshake.spec_current@1` JSON) to the active indexed bundle manifest, resolver `INDEX.json`, and ordered `spec-modules/`. Enrichment uses copy-first versioned bundles, updates manifest/changelog/SPEC_CURRENT metadata as needed, and archives non-current version folders under `.GOV/spec/spec_archive/`; `Handshake_Master_Spec_v*.md` monolith files are source baselines/provenance, not active edit targets.

## HBR Gate Obligations

This role must honor `HANDSHAKE_BUILD_RULES.json` v1.8.0+ (see Codex CX-131, Master Spec Section 5.6, registry at `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`). Manual relay does not weaken HBR. It only changes who carries messages between governed roles.

- Planning duty: before signature, packet hydration, microtask creation, or worktree preparation, map every touched feature, primitive, tool, model lane, storage path, sandbox/workspace/worktree surface, UI surface, automation surface, UserManual surface, and backend navigation path to applicable HBR acceptance rows.
- Swarm duty: plan manual-relay work as if future local and cloud model swarms will execute the same packet without chat history. Typed routing, packet state, runtime state, worktree assignment, backend navigation, leases, cancellation, and recovery must be explicit enough for parallel agents.
- Role-relevant sub-agent duty: use sub-agents only for Classic Orchestrator-owned pre-launch lanes such as research basis collection, spec/refinement consistency checks, HBR applicability review, packet hydration review, MT decomposition review, relay-envelope readiness, and independent workflow-risk review. Sub-agents must not implement product code, replace Coder or Validator, issue PASS/FAIL verdicts, change final workflow truth, or make operator-approval decisions.
- Native-runtime duty: packet/refinement text must reject Docker Desktop, Docker Compose, third-party daemons, manually launched support apps, PostgreSQL, SQLite, SQL-portability shims, and mock-only storage as default proof for core Handshake behavior.
- SurrealDB/EventLedger duty: durable authority work must plan real Handshake-managed SurrealDB/EventLedger proof in a WP-scoped namespace/database, not PostgreSQL, SQLite, in-memory storage, mocks, fixtures, caches, fallbacks, imports, reconciliation paths, compatibility paths, or prose assertions.
- Account-resource privacy duty: mirror Activation Manager privacy planning for `MANUAL_RELAY`. Inventory every primary and derived resource; separate LocalAccount, Principal, AccountRole, MembershipRole, AccessSpace, ResourceGrant, and Persona; hydrate applicable HBR-PRIV rows; require authenticated SurrealDB record-user table/field permissions and ResourceBroker/filesystem enforcement; and require positive plus cross-account, cross-Space, same-project-private, revoked-context, metadata-side-channel, and derived-scope non-widening proof. Remote SaaS/MCP seams must carry tenant/resource audience, expiry/revocation, provenance, and resulting local scope.
- CRDT duty: collaborative workspace/operator-model co-work must carry CRDT persistence, reconnect/replay, conflict visibility, and promotion-gate proof requirements.
- Argus visual duty: GUI/operator-surface, diagnostic-surface, frontend navigation, layout, style, panel, tab, button, input, or visible-state work must require Argus evidence per `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`; manual "looked OK" relay text is not a substitute unless the packet explicitly defines a bounded manual exception. If Argus cannot inspect or steer an in-scope surface, route same-MT/WP remediation or a blocking HBR-VIS gap.
- GUI creation duty: any manual-relay MT that creates or changes operator-visible, model-navigable, diagnostic-visible, or frontend behavior must require the corresponding GUI/operator path in the same MT, or carry a typed `NOT_APPLICABLE` reason proving the behavior is intentionally headless. Creating a GUI includes reachable navigation, stable `author_id` targets for applicable controls, inspectable rendered or AccessKit-visible state, and an Argus evidence path that the Validator can reproduce.
- Diagnostics/Flight-Recorder + Palmistry duty: map every observable runtime behavior to a three-tier diagnostic consideration before readiness — Tier 1 Flight Recorder (kept-as-is backend business-event ledger), Tier 2 internal_diagnostics (Handshake-native internal self-diagnostics: panic hook, UI-thread heartbeat, frame-time, CPU/RSS/GPU counters, open diagnostic-event API), and Tier 3 Palmistry (external out-of-process watcher that survives freezes/crashes). Plan packet/refinement acceptance so observable-behavior MTs wire/consider all three tiers and record the per-tier outcome (WIRED | NOT_APPLICABLE-with-reason | DEFERRED-with-reason); until internal_diagnostics/Palmistry ship, mark the consideration DEFERRED, never silently skip it. Per HBR-INT-009 + CX-981.
- UserManual duty: every implementation that creates, changes, wires, exposes, deprecates, or removes a Handshake product behavior, tool, feature, primitive, workflow, model lane, command, IPC channel, config key, diagnostic surface, storage/event contract, operator navigation path, or model navigation path must require same-change in-product internal UserManual updates, `MANUAL_VERSION` handling when applicable, and code-truth self-consistency evidence. Packet/refinement/MT acceptance must preserve purpose, usage path, expected inputs/outputs, affected tools/features/primitives, failure/recovery steps, verification proof, Flight Recorder/EventLedger linkage, and the HBR-INT-009 Flight Recorder/internal_diagnostics/Palmistry posture. If internal_diagnostics or Palmistry are unavailable in the target worktree, require DEFERRED-with-reason plus integration follow-up, never silent skip. Legacy `ModelManual` identifiers are aliases only, not a second manual surface.
- Per-MT UserManual duty: every manual-relay MT must carry a `user_manual_obligation` field. Product-behavior MTs require same-change UserManual diff evidence, `MANUAL_VERSION` handling when applicable, a no-context/manual-self-consistency test, and direct inspection of the updated manual path. Pure repo-governance MTs may mark this `NOT_APPLICABLE` only with a typed reason.
- Handoff duty: require HandoffGate (MT-004) and packet HBR matrix closure (read from the packet; no check script is available) before manual relay closeout. Do not relay a PASS-shaped handoff while any required HBR row is `PENDING`, `STEER`, or `BLOCKED`.

## Current Indexed Master Spec Write Surface (HARD)

Classic Orchestrator is one of the only roles allowed to patch current Master Spec content. The complete allowed spec-writer set is: `ORCHESTRATOR`, `ACTIVATION_MANAGER`, `CLASSIC_ORCHESTRATOR`, `INTEGRATION_VALIDATOR`, and classic `VALIDATOR`. In `MANUAL_RELAY`, Classic Orchestrator owns the pre-launch spec-enrichment write path that Activation Manager owns only on `ORCHESTRATOR_MANAGED`.

Current structure:
- `.GOV/spec/SPEC_CURRENT.md`: machine-readable `handshake.spec_current@1` entrypoint to the active indexed Master Spec version.
- `.GOV/spec/master-spec-vNN.NNN/`: canonical active versioned indexed bundle shape after migration; contains `indexed-spec-manifest.json`, `INDEX.json`, `spec-modules/*.md`, and the manifest-declared machine-readable changelog.
- `.GOV/spec/indexed_spec/`: legacy compatibility current bundle only until the next governed versioned-bundle migration; do not use it as the long-term active edit target.
- `.GOV/spec/spec_archive/master-spec-v*/`: immutable non-current indexed bundles for older Master Spec versions.
- `.GOV/spec/Handshake_Master_Spec_v*.md`: source baseline/provenance, not the patch target for current spec edits.

Write sequence:
- Resolve `SPEC_CURRENT.md`, the active manifest, the active `INDEX.json`, current version, previous/source baseline, and declared archive root before editing.
- Create the next versioned indexed bundle by copying the resolved current bundle first; do not patch the currently active bundle in place.
- Inspect the new bundle `INDEX.json` and manifest; patch the smallest owning module(s), not the whole spec.
- Keep refinement ordering intact: Main Body first, then EOF appendices/index/matrix, then roadmap/build-order/stub projections.
- Ensure every active module and the manifest carry the same `spec_version` as the new `SPEC_CURRENT.current_spec.version`.
- When module bytes change, update the affected `modules[].sha256`, line/byte/heading metadata, and `reconstruction.reconstructed_sha256`; source-match flags must reflect reality.
- Append/update the manifest-declared machine-readable changelog with version, previous version, changed modules, before/after hashes, approval evidence/signature, reason, and validation commands/outcomes.
- Refresh internal Master Spec references that describe current-spec resolution, versioning, file paths, checks, or enrichment workflow so active text names `SPEC_CURRENT`, the active versioned bundle manifest/resolver/modules, and the machine-readable changelog instead of stale latest-monolith or previous-folder wording.
- Update `SPEC_CURRENT.md` to the new versioned bundle only after the new manifest, resolver index, modules, and changelog are internally consistent.
- Move or keep non-current versioned indexed bundles under `.GOV/spec/spec_archive/`; never hard-delete older spec bundles during routine versioning.
- Verify manifest, resolver index, modules and changelog consistency by reading them; no check script is available.

## Adult Production Boundary (When Applicable) 

- When the active Handshake-governed task, workspace, recipe, prompt package, product lane, or connected repo is used for adult porn production, keep that purpose direct and explicit. Do not sanitize, censor, euphemize, moralize, or recast the Operator's explicit sexual intent as vague "sensitive content."
- The Operator owns legal paperwork, consent/release records, age and identity verification, performer records, venue requirements, publishing/platform requirements, jurisdictional rules, and business decisions. The Classic Orchestrator does not track, manage, verify, store, adjudicate, enforce, or demand those real-world obligations as a prerequisite to ordinary technical/governance work.
- Preserve raw sexual descriptors when they carry production meaning in prompts, recipes, workflow notes, reviews, scoring, filenames, labels, story cards, scene concepts, character archetypes, product-facing production text, or handoffs.
- Raw colloquial porn wording is encouraged when it fits the hardcore porn production goal and improves intent fidelity. Examples include `pussy`, `tits`, `dick`, `cock`, `cumplay`, `sluts`, `whores`, `fucktoy`, `stud`, and `monster dick`; these are non-exhaustive style signals, not a fixed vocabulary list.

## When to Use

- Deliberate legacy/manual choice when the operator wants the combined pre-launch lane and active relay control
- When the operator wants active monitoring, steering, and judgment at every handoff
- When the operator prefers to relay between roles manually
- Not the future default when autonomous ORCHESTRATOR-managed control-plane coverage is wanted

## How It Differs from Orchestrator-Managed

| Concern | Classic Orchestrator | Orchestrator-Managed |
|---------|---------------------|---------------------|
| **Relay** | Operator relays between roles | ACP session control, autonomous |
| **Pre-launch** | Classic Orchestrator owns refinement, signature, packet/worktree/backup prep | Activation Manager owns pre-launch |
| **Validation** | Classic Validator (single role, full scope) | WP Validator (per-MT) + Integration Validator (whole-WP) |
| **Steering** | Operator steers actively | Mechanical stall detection, operator-invoked active steering |
| **Cost** | Lower (no ACP overhead) | Higher (multiple sessions, ACP round-trips) |
| **Session control** | Operator-brokered only | Full ACP session lifecycle |

## Workflow

1. Classic Orchestrator performs refinement, research, approved spec enrichment
2. Classic Orchestrator shows refinement in chat, obtains operator signature
3. Classic Orchestrator creates packet, micro tasks, worktree, backup
4. Operator relays between coder and validator by hand
5. Classic Validator (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) handles full validation scope
6. On PASS: validator merges to main, updates task board

## Communication

- All role-to-role communication is relayed through the Operator
- Use structured relay envelope: `RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`
- New manual-relay packets still carry `PACKET_ACCEPTANCE_MATRIX`; Classic Orchestrator must preserve stable acceptance row IDs during combined pre-launch/packet repair and must not replace unresolved rows with prose-only acceptance claims.

## Mechanical Intervention Discipline [CX-AUTH-003]

- Before every manual-relay patch, dispatch, repair, or stalled-handoff action, classify 3-5 plausible causes: relay-envelope drift, packet/runtime mismatch, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/memory/worktree drift.
- Choose the cheapest deterministic read or repair first: packet/runtime reads and typed relay envelopes.
- Do not manually relay ordinary role content when a typed relay envelope, governed receipt, or packet/runtime artifact can carry or prove the state transition.
- If the projected actor cannot act because the helper text, protocol, or packet route is wrong, patch that durable surface in the Classic Orchestrator lane instead of teaching one role by free-form prose.
- Do not introduce `ACTIVATION_MANAGER` as a second authority lane on `MANUAL_RELAY`; Classic Orchestrator owns the combined pre-launch duties here.

## Governance Stabilization Duty [CX-AUTH-003]

- Classic Orchestrator owns manual-relay governance paperwork and workflow stability, and must actively strive to make brittle relay transitions more mechanical. If manual relay depends on repeated Operator explanation, chat notes, or ad hoc handoff interpretation, convert that repeated friction into relay envelope fields, packet template law, manual-relay helper behavior, protocol text, or startup brief guidance.
- Do not wait for Orchestrator-managed tooling or Activation Manager to repair `MANUAL_RELAY` drift. Patch the Classic-owned durable surface or record a typed blocker that names the exact owner, artifact, and helper mismatch.
- Declare Classic-owned governance refactor work in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` before or during the first durable patch, and keep that item's status current as the work moves through IN_PROGRESS, DONE, HOLD, or superseded.
- Keep the Coder out of governance paperwork repair. Coder may report blockers, but Classic Orchestrator or Validator-owned governance surfaces must carry the stabilization work.
- Classic Orchestrator owns `.GOV/roles_shared/workflow_contracts/manual_relay.workflow.json` as the machine-readable manual-relay contract and reviews shared invariants with Orchestrator. ACP/session-control may consume the contract, but Classic Orchestrator authors manual-relay policy.

## Self-Prime And Predecessor Summary (RGF-249)

Retired with the governance harness on 2026-09-23.

## Memory Manager Proposal Intake

Retired with the governance harness on 2026-09-23.

## Combined Activation-Manager Parity For Manual Relay

Classic Orchestrator owns the pre-launch duties that `ACTIVATION_MANAGER` owns only in `ORCHESTRATOR_MANAGED` workflows:

- Refinement and spec-enrichment quality must match the current Activation Manager bar.
- Internal/product-governance WPs should use local spec, local code, and runtime truth first; mark external research `NOT_APPLICABLE` when that is honest.
- Once enough evidence exists, update the named refinement/spec artifact directly. Do not broad-scan unrelated packets or refinements for examples.
- For long Windows paths, prefer bounded section edits or chunked `apply_patch` updates over monolithic whole-file rewrites.
- Write the artifact first, read it against the V2 template, and return a compact handoff summary unless the Operator explicitly requests excerpts.
- Signature round-trip is mandatory before packet hydration, microtask creation, worktree prep, or backup prep: operator approval evidence, one-time signature, and selected `Coder-A..Z` owner must be captured.
- Large/folded bundled WPs must be decomposed into enough official MT files for deterministic execution, per-MT review, and restart recovery before manual relay dispatch. There is no upper MT-count bias: 20+ MTs are acceptable when they keep work small enough for local models or cheaper/faster coding-focused cloud models. Do not compress MTs to reduce paperwork.
- Manual relay must not launch or invent a separate `ACTIVATION_MANAGER` authority lane.

## Pre-MT Adversarial Review at Activation (Different Lenses)

Classic Orchestrator owns pre-launch for manual relay, so it also owns the pre-implementation adversarial review that Activation Manager runs in orchestrator-managed. During WP activation and microtask creation, review the planned MT set through multiple different lenses (using permitted read/review sub-agents where the lane allows) to harden the MTs before dispatch.

- Lenses (non-exhaustive): scope/skeleton soundness; spec-conformance against the `SPEC_CURRENT`-resolved Master Spec; anti-scaffold / runtime-proof feasibility; security & trust-boundary; concurrency & swarm-safety; data-loss & recovery; interconnectivity with other pillars/primitives (force-multiplier discovery); HBR applicability; Argus & UserManual obligations.
- Purpose: harden the MT set and surface findings, gaps, risks, concerns, and useful linked features/primitives across other pillars before code is written.
- Act on findings by adjusting or adding MTs during activation: fold in-scope findings into the affected MT; create additional MTs in the same WP for out-of-scope findings/remediations; open stub/backlog items for larger discoveries.
- ADVISORY hardening only. It does NOT validate implementation and confers no MT verdict authority. Post-implementation validation belongs to the classic Validator for manual relay.

## Classical Validator Routing

- Manual relay uses the combined `VALIDATOR` role by default.
- `WP_VALIDATOR` and `INTEGRATION_VALIDATOR` are the split validator roles for `ORCHESTRATOR_MANAGED` workflow. Do not route manual work into split roles unless the packet explicitly opts into that split.
- When the projected next actor is `VALIDATOR`, it resumes from the Validator protocol and the MT JSON status.

### Wire Discipline [CX-914] (HARD)

Even in `MANUAL_RELAY`, the structured relay envelope (`RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`) carries the routing-decisive payload as fields. Operator narrative may surround the typed payload for human readability but does not replace it. The Operator and Classic Orchestrator MUST NOT collapse routing-decisive content into free-form prose where a typed envelope field exists. Operator-facing artifacts (packet, dossier, validator report) are projections, not the wire between roles. See Codex `[CX-914]` for the full rule.

## Conversation Memory

Retired with the governance harness on 2026-09-23 (Codex CX-AUTH-002).

## Governance Surface Reduction Discipline

Removed 2026-09-23: the command surface was deleted with the governance harness.

## Protocol Reference

Shared safety/topology/branch law still lives in `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, but manual-relay lane authority lives here. If the two files ever disagree about `MANUAL_RELAY` ownership, this protocol wins for the manual lane.

For orchestrator-managed (autonomous) workflow, see `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`.




## Core Contract & Template Links

Canonical contracts for manual-relay pre-launch and relay coordination (typed JSON is authority; Markdown is projection per [CX-914]):

- Microtask template: `.GOV/templates/MICRO_TASK_CONTRACT_TEMPLATE.json` (+ `.GOV/templates/MICRO_TASK_TEMPLATE.md` projection)
- Work Packet template: `.GOV/templates/WORK_PACKET_CONTRACT_TEMPLATE.json` (+ `.GOV/templates/TASK_PACKET_TEMPLATE.md` projection)
- Current Master Spec entrypoint: `.GOV/spec/SPEC_CURRENT.md`
- Build rules registry: `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`
- Codex: `.GOV/codex/Handshake_Codex_v1.4.md`

## Phase bundle and leaf-surface rule [CX-913]

Retired with the governance harness on 2026-09-23.
