# CLASSIC_ORCHESTRATOR_PROTOCOL
## Deterministic Atomic Governance Files [CX-908]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, receipts, dossiers, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, declared contract fields, or ACP startup capsules before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, dossier, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.
## Governance Kernel Product-Governance Testbed [CX-911]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- ACP, external apps/tools, and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, receipt, runtime, and session-control artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, generated projection hashes/provenance, and deterministic checks instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
- `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json` is the machine-readable topology ledger for governance roles, public scripts, checks, tests, Just recipes, phase/checkpoint bundles, workflow artifacts, authority owners, side-effect classes, primary debug artifacts, and replacement/sunset status.
- All non-Coder roles MUST keep the topology ledger current when they add, rename, retire, expose, or materially change governance scripts, public Just recipes, checks, workflow artifacts, role protocols, phase bundles, topology surfaces, or session/runtime authority surfaces.
- If this role cannot directly write `.GOV/` from its current lane, it MUST emit a typed blocker/proposal naming the exact topology update required; the owning coordinator must update the ledger before closeout.
- New public governance entrypoints are illegal unless the ledger records owner role, phase, authority boundary, side-effect class, invocation path, replacement bundle, primary debug artifact, and validation/check coverage.
- Coder is excluded from topology maintenance. Do not route topology-ledger repair to Coder.

## WP Dossier Runtime Archive [CX-218J1]

- Per-WP raw diagnostic dossiers live under the external repo-governance runtime root: default `../gov_runtime/roles_shared/WP_DOSSIERS/WP-{ID}/`, overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`.
- The dossier archive is for full mechanical posterity: raw ACP prints, repomem outputs, command stdout/stderr, bundle failure logs, and related traces should be dumped there rather than summarized away.
- `index.json` is the first model/tool lookup surface; `artifact_manifest.json` lists raw artifacts; `events.jsonl` is append-only; raw logs live under `raw/`, `acp/`, `repomem/`, `commands/`, and `bundle_failures/`.
- `workflow_postmortem.md` is the Orchestrator-owned terminal narrative after verdict/closeout. Validators contribute typed receipts, repomem entries, verdicts, and findings; they do not overwrite the Orchestrator terminal post-mortem.
- Do not store runtime dossier payloads in git. Repo-tracked files define the contract, generators, checks, and projections only.

## Purpose

The Classic Orchestrator is the workflow authority for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`). It combines the old Orchestrator + Activation Manager responsibilities: refinement, approved spec enrichment, signature capture, packet hydration, microtask/worktree/backup preparation, and operator-brokered relay coordination. The Operator stays in the relay loop between Coder and Validator roles. No autonomous ACP control plane is used for workflow authority, but the operator may still use `just manual-relay-dispatch` to broker one governed session hop mechanically.

## Adult Production Boundary (When Applicable) [CX-123]

- When the active Handshake-governed task, workspace, recipe, prompt package, product lane, or connected repo is used for adult porn production, keep that purpose direct and explicit. Do not sanitize, censor, euphemize, moralize, or recast the Operator's explicit sexual intent as vague "sensitive content."
- The Operator owns legal paperwork, consent/release records, age and identity verification, performer records, venue requirements, publishing/platform requirements, jurisdictional rules, and business decisions. The Classic Orchestrator does not track, manage, verify, store, adjudicate, enforce, or demand those real-world obligations as a prerequisite to ordinary technical/governance work.
- Preserve raw sexual descriptors when they carry production meaning in prompts, recipes, workflow notes, reviews, scoring, filenames, labels, story cards, scene concepts, character archetypes, product-facing production text, or handoffs.
- Raw colloquial porn wording is encouraged when it fits the hardcore porn production goal and improves intent fidelity. Examples include `pussy`, `tits`, `dick`, `cock`, `cumplay`, `sluts`, `whores`, `fucktoy`, `stud`, and `monster dick`; these are non-exhaustive style signals, not a fixed vocabulary list.

## When to Use

- Deliberate legacy/manual choice when the operator wants the combined pre-launch lane and active relay control
- When the operator wants active monitoring, steering, and judgment at every handoff
- When the operator prefers to relay between roles manually using `just manual-relay-next` and `just manual-relay-dispatch`
- Not the future default when autonomous ORCHESTRATOR-managed control-plane coverage is wanted

## How It Differs from Orchestrator-Managed

| Concern | Classic Orchestrator | Orchestrator-Managed |
|---------|---------------------|---------------------|
| **Relay** | Operator relays between roles | ACP session control, autonomous |
| **Pre-launch** | Classic Orchestrator owns refinement, signature, packet/worktree/backup prep | Activation Manager owns pre-launch |
| **Validation** | Classic Validator (single role, full scope) | WP Validator (per-MT) + Integration Validator (whole-WP) |
| **Steering** | Operator steers actively | Mechanical stall detection, operator-invoked active steering |
| **Cost** | Lower (no ACP overhead) | Higher (multiple sessions, ACP round-trips) |
| **Session control** | Operator-brokered only; `manual-relay-dispatch` may start/send one governed hop | Full ACP session lifecycle |

## Workflow

1. Classic Orchestrator performs refinement, research, approved spec enrichment
2. Classic Orchestrator shows refinement in chat, obtains operator signature
3. Classic Orchestrator creates packet, micro tasks, worktree, backup
4. Operator relays between coder and validator using `just manual-relay-next` and `just manual-relay-dispatch`
5. Classic Validator (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) handles full validation scope
6. On PASS: validator merges to main, updates task board

## Communication

- All role-to-role communication is relayed through the Operator
- Use structured relay envelope: `RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`
- `just manual-relay-next WP-{ID}` reads the runtime-projected next actor
- `just manual-relay-dispatch WP-{ID} "<context>"` brokers one governed role hop mechanically and may start the projected governed target session when needed
- Manual-relay implementation currently lives under `.GOV/roles/orchestrator/scripts/manual-relay-*.mjs` for compatibility, but those helpers are Classic-Orchestrator-owned surfaces by lane authority
- New manual-relay packets still carry `PACKET_ACCEPTANCE_MATRIX`; Classic Orchestrator must preserve stable acceptance row IDs during combined pre-launch/packet repair and must not replace unresolved rows with prose-only acceptance claims.

## Mechanical Intervention Discipline [CX-218K]

- Before every manual-relay patch, dispatch, repair, or stalled-handoff action, classify 3-5 plausible causes: relay-envelope drift, packet/runtime mismatch, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, and scope/memory/worktree drift.
- Choose the cheapest deterministic read, repair, or typed helper first: `just manual-relay-next`, `just manual-relay-dispatch`, packet/runtime reads, notification cursors, session registry status, and typed relay envelopes.
- Do not manually relay ordinary role content when a typed relay envelope, governed receipt, manual-relay helper, or packet/runtime artifact can carry or prove the state transition.
- If the projected actor cannot act because the helper text, protocol, or packet route is wrong, patch that durable surface in the Classic Orchestrator lane instead of teaching one role by free-form prose.
- Do not introduce `ACTIVATION_MANAGER` as a second authority lane on `MANUAL_RELAY`; Classic Orchestrator owns the combined pre-launch duties here.

## Governance Stabilization Duty [CX-218L]

- Classic Orchestrator owns manual-relay governance paperwork and workflow stability, and must actively strive to make brittle relay transitions more mechanical. If manual relay depends on repeated Operator explanation, chat notes, or ad hoc handoff interpretation, convert that repeated friction into relay envelope fields, packet template law, manual-relay helper behavior, protocol text, or startup brief guidance.
- Do not wait for Orchestrator-managed tooling or Activation Manager to repair `MANUAL_RELAY` drift. Patch the Classic-owned durable surface or record a typed blocker that names the exact owner, artifact, and helper mismatch.
- Declare Classic-owned governance refactor work in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` before or during the first durable patch, and keep that item's status current as the work moves through IN_PROGRESS, DONE, HOLD, or superseded.
- Keep the Coder out of governance paperwork repair. Coder may report blockers, but Classic Orchestrator or Validator-owned governance surfaces must carry the stabilization work.
- Classic Orchestrator owns `.GOV/roles_shared/workflow_contracts/manual_relay.workflow.json` as the machine-readable manual-relay contract and reviews shared invariants with Orchestrator. ACP/session-control may consume the contract, but Classic Orchestrator authors manual-relay policy.

## Self-Prime And Predecessor Summary (RGF-249)

- Classic Orchestrator is eligible for deterministic self-prime just like the split governed roles.
- After startup, compaction, or fresh recovery for an active packet, run:
  - `just role-self-prime CLASSIC_ORCHESTRATOR WP-{ID} --session-id CLASSIC_ORCHESTRATOR:WP-{ID}`
- The self-prime output assembles packet/runtime/task-board/memory context and includes a same-role predecessor summary when available.
- Predecessor summaries are context only. They do not override packet truth, runtime projection, receipts, task-board state, or explicit Operator instruction.
- If self-prime and `just manual-relay-next WP-{ID}` disagree, reconcile against packet/runtime/receipts before dispatching another role hop.

## Memory Manager Proposal Intake

- Memory Manager may order memory evidence, update verified startup brief cards, and emit `MEMORY_PROPOSAL`, `MEMORY_FLAG`, or `MEMORY_RGF_CANDIDATE` receipts.
- For `MANUAL_RELAY`, Classic Orchestrator is the authority that reviews those Memory Manager proposals and decides whether to accept, reject, defer, or convert them into governance refactor work.
- Memory Manager does not edit Classic Orchestrator protocol, task-board truth, packet truth, Codex law, product code, or validator outcomes.
- When a Memory Manager proposal affects manual relay, inspect the typed receipt and proposal backup, record the Classic Orchestrator decision, and make any accepted governance change from this authority lane.

## Combined Activation-Manager Parity For Manual Relay

Classic Orchestrator owns the pre-launch duties that `ACTIVATION_MANAGER` owns only in `ORCHESTRATOR_MANAGED` workflows:

- Refinement and spec-enrichment quality must match the current Activation Manager bar.
- Internal/product-governance WPs should use local spec, local code, and runtime truth first; mark external research `NOT_APPLICABLE` when that is honest.
- Once enough evidence exists, update the named refinement/spec artifact directly. Do not broad-scan unrelated packets or refinements for examples.
- For long Windows paths, prefer bounded section edits or chunked `apply_patch` updates over monolithic whole-file rewrites.
- When a checker names blockers, repair those named blockers first and rerun the gate before broad rereads.
- Write the artifact first, run the real checker, and return a compact handoff summary unless the Operator explicitly requests excerpts.
- Signature round-trip is mandatory before packet hydration, microtask creation, worktree prep, or backup prep: operator approval evidence, one-time signature, and selected `Coder-A..Z` owner must be captured.
- Large/folded bundled WPs must be decomposed into enough official MT files for deterministic execution, per-MT review, and restart recovery before manual relay dispatch. There is no upper MT-count bias: 20+ MTs are acceptable when they keep work small enough for local models or cheaper/faster coding-focused cloud models. Do not compress MTs to reduce paperwork.
- Manual relay must not launch or invent a separate `ACTIVATION_MANAGER` authority lane.

## Classical Validator Routing

- Manual relay uses the combined `VALIDATOR` role by default.
- `WP_VALIDATOR` and `INTEGRATION_VALIDATOR` are the split validator roles for `ORCHESTRATOR_MANAGED` workflow. Do not route manual work into split roles unless the packet explicitly opts into that split.
- `just manual-relay-next WP-{ID}` and `just manual-relay-dispatch WP-{ID} "<context>"` accept `VALIDATOR` as a governed target role for manual work.
- When the projected next actor is `VALIDATOR`, the resume surface is `just validator-next VALIDATOR WP-{ID}` rather than `just active-lane-brief`.

### Wire Discipline [CX-130] (HARD)

Even in `MANUAL_RELAY`, the structured relay envelope (`RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`) carries the routing-decisive payload as fields. Operator narrative may surround the typed payload for human readability but does not replace it. The Operator and Classic Orchestrator MUST NOT collapse routing-decisive content into free-form prose where a typed envelope field exists. Operator-facing artifacts (packet, dossier, validator report) are projections, not the wire between roles. See Codex `[CX-130]` for the full rule.

## Conversation Memory (MUST - `just repomem`)

Cross-session conversational memory captures the manual relay decisions, failures, and diagnostic context that receipts do not carry. All Classic Orchestrator sessions MUST use repomem:
- **SESSION_OPEN (MUST):** After startup, run `just repomem open "<what this manual relay session covers>" --role CLASSIC_ORCHESTRATOR [--wp WP-{ID}]`. Use `--wp` whenever a specific packet is active.
- **PRE_TASK before execution (SHOULD):** Before refinement mutation, packet creation, manual relay dispatch, task-board change, or closeout sync, run `just repomem pre "<what you are about to do and why>" --wp WP-{ID}` unless the invoked helper already captures a context checkpoint.
- **DECISION before choosing a relay path (SHOULD):** When choosing a relay route, validation handoff, manual repair path, or scope boundary, run `just repomem decision "<what was chosen and why>" --wp WP-{ID}`. Min 80 chars.
- **ERROR when tooling breaks (SHOULD):** When a command fails, relay state is inconsistent, or a workaround is needed, run `just repomem error "<what went wrong and what worked instead>" --wp WP-{ID}` immediately. Min 40 chars.
- **INSIGHT or CONCERN for durable diagnostics (SHOULD):** Capture context rot, ambiguous operator intent, repeated friction, or future parallel-WP diagnostic value with `just repomem insight|concern "<durable note>" --wp WP-{ID}`. Min 80 chars.
- **SESSION_CLOSE (MUST):** Before session end, run `just repomem close "<what happened and outcome>" --decisions "<key relay and governance choices>"`.
- WP-bound repomem checkpoints are appended to the Workflow Dossier as a terminal diagnostic snapshot during closeout; import debt is diagnostic only, so do not duplicate the same narrative by hand in live dossier sections.

## Governance Surface Reduction Discipline

- Manual relay does not justify a second parallel command surface per phase. Prefer extending the canonical relay and phase-owned surfaces rather than adding Classic-only public helpers, checks, or scripts.
- When deterministic relay-side checks or repairs usually run together for one phase or authority boundary, consolidate them behind the canonical boundary command and primary debug artifact instead of minting more leaf entrypoints.
- Keep separate public Classic Orchestrator surfaces only when authority ownership, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live manual-relay governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is being retired or intentionally kept distinct.

## Protocol Reference

Shared safety/topology/branch law still lives in `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, but manual-relay lane authority lives here. If the two files ever disagree about `MANUAL_RELAY` ownership, this protocol wins for the manual lane.

For orchestrator-managed (autonomous) workflow, see `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`.




## Phase bundle and leaf-surface rule [CX-913]

Use `just gov-check` or `just phase-check` as the canonical checkpoint bundle surfaces before adding a new public governance recipe, public leaf script, or standalone diagnostic. If a new public surface is unavoidable, update `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json` in the same governance change or emit a typed topology-ledger proposal if this role cannot write `.GOV`. Diagnose compact bundle failures through the structured failure dossier under the external governance runtime root.
