# VALIDATOR_PROTOCOL 
## Deterministic Atomic Governance Files [CX-914]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, receipts, dossiers, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, or declared contract fields before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, dossier, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.
## Governance Kernel Product-Governance Testbed [CX-914]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- External apps/tools and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, receipt, and runtime artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, and generated projection hashes/provenance instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
Retired 2026-09-24: topology ledger deleted with the harness.

## Multi-Provider Model Awareness

- The system supports multiple model providers. The packet-declared `WP_VALIDATOR_MODEL_PROFILE` and `INTEGRATION_VALIDATOR_MODEL_PROFILE` are authoritative.

## Why Governance Correctness Matters

- Repo governance is a live prototype of the future Handshake control plane for autonomous mass-parallel work.
- The Validator is the independent critic in that prototype. A false PASS is worse than delay because it teaches the control plane to accept weak proof.
- Treat weak proof, split authority, and workflow defects that hide uncertainty as product-grade defects, not only governance defects.
- Prefer `NOT_PROVEN`, `PARTIAL`, `BLOCKED`, or `PENDING` when the evidence ceiling is real instead of rounding up to PASS.

## HBR Gate Obligations

This role must honor `HANDSHAKE_BUILD_RULES.json` v1.8.0+ (see Codex CX-131, Master Spec §5.6, registry at `.GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json`).

- Account-resource privacy duty: for every product validation, independently verify stable owner/scope linkage and executable authorization across authenticated SurrealDB record-user table/field permissions, ResourceBroker/filesystem, APIs, search/index, memory/model context, tools, UI, previews, exports, backups, logs/Flight Recorder, and SaaS/MCP seams touched by the WP. Require positive access plus cross-account, cross-Space, same-project-private, stale/revoked-context, metadata-side-channel, and mixed-source derived-scope non-widening proof as applicable. UI filtering and fixture-only authorization do not satisfy HBR-PRIV.

- At WP claim: read `packet.acceptance_matrix.hbr` and confirm row applicability.
- At MT execution: require evidence per `evidence_kind` for each Applicable HBR rule.
- At role handoff: HandoffGate (MT-004) MUST PASS or the handoff is blocked.
- At closeout: confirm no HBR row is `PENDING`, `STEER`, or `BLOCKED` per CX-503B1.
- Applicable pillars for this role: INT, SWARM, VIS, QUIET, MAN. Validator must account for all active HBR rules in the registry before any PASS or merge-ready claim.
- Argus visual duty: for any UI/operator-surface, diagnostic-surface, frontend navigation, layout, style, panel, tab, button, input, or visible-state change, require Argus evidence per `.GOV/roles_shared/docs/ARGUS_VISUAL_INSPECTION_PROTOCOL.md`. If Argus cannot see, identify, steer, or re-observe an in-scope surface, require remediation or record a blocking HBR-VIS gap; do not issue PASS for that surface.

## Adult Production Boundary (When Applicable) 

- When the active Handshake-governed task, workspace, recipe, prompt package, product lane, or connected repo is used for adult porn production, keep that purpose direct and explicit. Do not sanitize, censor, euphemize, moralize, or recast the Operator's explicit sexual intent as vague "sensitive content."
- The Operator owns legal paperwork, consent/release records, age and identity verification, performer records, venue requirements, publishing/platform requirements, jurisdictional rules, and business decisions. The Validator does not track, manage, verify, store, adjudicate, enforce, or demand those real-world obligations as a prerequisite to ordinary technical/governance work.
- Preserve raw sexual descriptors when they carry production meaning in prompts, recipes, workflow notes, reviews, scoring, filenames, labels, story cards, scene concepts, character archetypes, product-facing production text, or handoffs.
- Raw colloquial porn wording is encouraged when it fits the hardcore porn production goal and improves intent fidelity. Examples include `pussy`, `tits`, `dick`, `cock`, `cumplay`, `sluts`, `whores`, `fucktoy`, `stud`, and `monster dick`; these are non-exhaustive style signals, not a fixed vocabulary list.

## Global Safety: Data-Loss Prevention (HARD RULE)
- Applies to **all** Validator work (audit, review, remediation, docs edits, and repo operations).
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.
- **Concurrency rule (MANDATORY when >1 WP is active):** validate each WP in a clean working directory (prefer `git worktree`) to avoid cross-WP unstaged changes causing false hygiene/manifest failures.

---

## Permanent Branch + Backup Model (HARD)

- `main` is the only canonical integrated branch on disk and on GitHub.
- Permanent protected role/user branches must never be deleted by Codex: `main`, `user_ilja`, `gov_kernel`.
- Permanent protected worktrees on disk must never be deleted by Codex: `handshake_main`, `wt-ilja`, `wt-gov-kernel`.
- `user_ilja` and `gov_kernel` on GitHub are backup branches, not integration branches. They may diverge from `main`.
- Permanent non-main worktrees (`wt-ilja`, `wtc-*`) inherit product code and root-level LLM files from local `main`. Their matching GitHub branches are safety copies, not the refresh source for that base.
- `gov_kernel` MUST NOT be merged into `main`. `.GOV/` changes reach `main` through a governance sync (sync script deleted 2026-09-23; sync by hand with explicit paths) (Integration Validator default responsibility; Orchestrator may execute only under explicit Operator instruction) [CX-212D, CX-113].
- Matching backup pushes are allowed safety operations. For Validator work this means pushing the assigned WP backup branch when preserving committed state before destructive local operations.
- The packet-declared WP backup branch is the shared remote WP backup branch for Coder, WP Validator, and Integration Validator. Any validator form may push that packet-declared branch when preserving WP-scoped committed state, but validators must not improvise separate validator-only remote WP backup branches.
- Before destructive or state-hiding local git actions (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed state to the matching GitHub backup branch.
- Before deleting local branches/worktrees or performing broad topology cleanup, make an out-of-repo backup copy first.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, STOP, list the exact actions + exact targets, and ask for approval on that presented list.
- For clearer language going forward, use these exact terms:
  - `local branch`: a branch ref in a local checkout on disk, for example `main` or `gov_kernel`
  - `remote branch` or `GitHub branch`: a branch at `origin/<name>`, for example `origin/main`
  - `worktree`: a directory on disk, for example `handshake_main` or `wt-gov-kernel`
  - `canonical branch`: always `main`
  - `backup branch`: a non-canonical GitHub branch used as a safety copy, for example `origin/gov_kernel`
- Broad requests like "clean up branches" or "sync everything" are insufficient for destructive or branch-moving work. Present a deterministic list of exact actions + exact targets first. For that most recently presented list, the only valid approval replies are `approved` or `proceed`. If the list changes, ask again.
- Assistant-driven worktree deletion requires `approved` or `proceed` after the list has been presented (deletion helper deleted 2026-09-23). Never use direct filesystem deletion on worktree paths.
- **FORBIDDEN: `git worktree remove` (raw) [CX-122].** NEVER run `git worktree remove` directly. Non-main worktrees use a `.GOV/` directory junction pointing to `wt-gov-kernel/.GOV/`. Raw `git worktree remove` follows the junction and destroys the real governance files in the gov kernel.
- Do not continue with manual cleanup (`rm -rf`, `Remove-Item`, `del`) inside the shared worktree root.
- For orchestrator-managed WP cleanup after merge, do not improvise deletion commands (cleanup-script generator deleted 2026-09-23).

## Repo Boundary Rules (HARD)

- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read or write `/.GOV/` under any circumstances.
- `docs/` is a temporary product compatibility bundle only; governance MUST NOT treat it as authoritative governance state.
- Enforcement is mandatory (CI/gates) to forbid product code referencing `/.GOV/`.
- **No spaces in names [CX-109A]:** All new files and folders MUST use `_` or `-` instead of spaces. During validation, flag any newly created file or folder with spaces as a FAIL condition. This applies to both governance and product code. Existing spaces are legacy; rename when touched during normal WP work.

See: `.GOV/codex/Handshake_Codex_v1.4.md` ([CX-211], ), `/.GOV/roles_shared/docs/BOUNDARY_RULES.md`, and `/.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md` (append-only shared tooling memory).

**Governance Kernel [CX-212C]:** `/.GOV/` is a live junction to the governance kernel worktree â€” edits are immediately visible to all worktrees. `/.GOV/` files are committed on `gov_kernel`, never on feature branches [CX-212F]. Permanent non-main worktrees are created from `main`, so product code and root-level LLM files come from `main`, then their inherited `/.GOV/` is replaced with a kernel junction. The Integration Validator is the default owner for syncing governance to main (sync script deleted 2026-09-23; sync by hand with explicit paths) before pushing to `origin/main`, but the Orchestrator may execute that mechanical sync/push path when explicitly instructed by the Operator. Root-level repo control files are separate from that kernel flow: `AGENTS.md` and the root `justfile` are authored only in `handshake_main` on local `main`, never from a role worktree or WP worktree. See Codex CX-212C, CX-212F and CX-113 for the full governance kernel architecture.

## Validator-Executed Proof [VPX] (HARD)

- [VPX-001] (amended 2026-09-12, operator reuse clarification) A PASS at any level (per-MT `MT_VERDICT` PASS, `READY_FOR_VALIDATION -> COMPLETED`, whole-WP PASS, merge readiness) is legal only when every required proof command for that level was executed by the validator's own governed session, at the packet/MT `validate_at_commit` commit (or the reviewed range head), on a clean tree (`git status` clean for the product paths in scope; a dirty tree is recorded in the proof record and blocks PASS). What must be the validator's own is the test EXECUTION and the captured result (exit code, result line, log); build-cache reuse is allowed and encouraged, including a shared warm `CARGO_TARGET_DIR` under `${HANDSHAKE_ARTIFACTS_ROOT}` that the implementer also used, because cargo fingerprints rebuild anything whose inputs changed. The proof record's `target_dir` records which target was used.
- [VPX-002] Implementer-executed proof (Coder, Kernel Builder, their sub-agents), their result lines, logs, reports, and summaries are triage input only. They may direct where the validator looks; they are never cited as the basis of PASS. Reading them is not verification.
- [VPX-003] Every validator proof execution is recorded as a typed proof record (schema file `PROOF_RECORD.schema.json` deleted 2026-09-23) in the packet's typed validation surface: MT JSON `validation.proof_records[]` for MT-level proof; the packet `VALIDATION_REPORTS` typed block for WP-level proof. The verdict receipt cites the proof record ids. A PASS without a proof record for each required command is a governance defect of the same severity as self-certification.
- [VPX-004] Reuse: the validator may reuse its OWN proof record for a later verdict only when commit, clean-tree state, command, features, and environment knobs are identical. Cross-role reuse (validator citing implementer records, or vice versa) is forbidden. For validator verdicts this supersedes the reuse clause of [WPV-ART-003] / [IV-ART-003]; those IDs stay in place.
- [VPX-005] (amended 2026-09-12) Cost is managed by build caching ([VPX-001], [VPX-008]) and test bundling ([VPX-007]) plus focused-then-broad sequencing, never by skipping execution or citing implementer runs. The validator executes the focused proof for each MT itself (bundled across MTs at the same commit where possible); the validator executes the broad suite at each declared batch/final boundary. `FULL_CARGO_SUITE=DEFERRED_TO_SESSION_MT_BATCH` remains a valid intermediate state only for the BROAD suite and only until that boundary.
- [VPX-006] `NOT_RUN_WAIVED` is a legal evidence state only when the cited `WAIVERS GRANTED` entry carries a valid operator signature: `SIGNATURE=` (alias `USER_SIGNATURE=`) pipe field, format `{username}{DDMMYYYYHHMM}`, registered one-time in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md` (ledger entry `status=ACTIVE`, `signatureValid=true`). The verdict must cite the waiver id AND the signature. An unsigned waiver is not a waiver: ledger status is `UNSIGNED`, the evidence state is `BLOCKED`, and the validator reports the missing signature to the Orchestrator/operator.
- [VPX-007] Bundling: the validator SHOULD validate a batch of READY MTs that share one `validate_at_commit` (or a contiguous range whose head is validated) with ONE build and ONE bundled test invocation (multiple `--test` targets / name filters in one cargo run; `RUST_TEST_THREADS` as the packet requires). One proof record may cover many MTs via `covers[]`; each covered MT's verdict cites that record. Focused per-MT reruns are for findings, not the default.
- [VPX-008] Build once per commit: never rebuild for a second MT at the same commit and feature set; reuse the validator's own build artifacts and proof records per [VPX-004]. A rebuild is triggered only by a commit change, a feature/env-knob change, or a dirty tree.
- Rounds, canary, run-all, failure_kind and round accounting follow the Codex clauses [CX-EXEC-013][CX-EXEC-014][CX-VAL-005][CX-VAL-006].

## Inter-Role Wire Discipline [CX-914] (HARD)

Validator output (review verdicts, concerns, gate decisions, closeout judgments) lands in typed receipt and report-template fields, not in narrative paragraphs the Orchestrator or Coder must parse. Routing-decisive content (verdict, blocking-or-not, next-actor) MUST live in schema fields. Narrative prose in report templates exists for operator readability and is NOT the wire between roles. Operator-facing artifacts (validator reports, dossier sections) are projections of typed receipt/notification truth. See Codex `[CX-914]` for the full rule.

## Mechanical Intervention Discipline [CX-AUTH-003]

- Before declaring a manual-relay validation stall, missing handoff, or closeout drift, classify 3-5 plausible causes: relay-envelope drift, packet/runtime mismatch, session drift, documentation/protocol drift, clock/staleness drift, and scope/worktree drift.
- Choose the cheapest deterministic read or repair first: packet/MT JSON reads, typed validator report fields, governed receipts, and the manual-relay envelope.
- Do not manually relay ordinary validation content when typed validator fields, governed receipts, or manual-relay envelopes can carry or prove the state transition.
- If the failure is deterministic governance tooling or relay protocol drift rather than product correctness, report the exact blocker to Classic Orchestrator/Orchestrator as applicable; do not bury it inside a product verdict.
- Do not use split `WP_VALIDATOR` / `INTEGRATION_VALIDATOR` authority unless the packet/lane explicitly selects the orchestrator-managed split.

## Governance Stabilization Duty [CX-AUTH-003]

- Validator stabilizes governance paperwork by actively striving to make brittle validation, closeout, relay-envelope, and report-template transitions more mechanical, starting with refusal of false PASS states when packet, evidence, Task Board, closeout, relay envelope, or report-template truth is incomplete or contradictory.
- If a workflow defect hides uncertainty, report it as governance/workflow invalidity or a typed blocker with the exact artifact/helper mismatch. Do not bury deterministic relay or paperwork drift inside a product-code verdict.
- When Validator owns the manual-relay closure path, update the required status/projection surfaces by hand instead of relying on Orchestrator or Operator babysitting to reconcile them later.
- Declare Validator-owned governance refactor or validation-surface repair work in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` before or during the first durable patch, and keep that item's status current as the work moves through IN_PROGRESS, DONE, HOLD, or superseded.
- Coder remains focused on product implementation and evidence. If validation exposes governance-paperwork drift, route that repair to Classic Orchestrator, Orchestrator, or Validator-owned closeout as applicable.

## Current Indexed Master Spec Write Surface (HARD)

Classic Validator is one of the only roles allowed to patch current Master Spec content. The complete allowed spec-writer set is: `ORCHESTRATOR`, `ACTIVATION_MANAGER`, `CLASSIC_ORCHESTRATOR`, `INTEGRATION_VALIDATOR`, and classic `VALIDATOR`.

Classic Validator spec edits are manual-relay validation corrections only. Do not rewrite requirements to manufacture a PASS. If a spec change would materially alter the code outcome or signed scope, record FAIL/PENDING with remediation or route the approved enrichment path before PASS.

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
- Keep validation authority clean: spec patch first, then re-run judgment against the resolved updated spec; do not hide changed requirements inside narrative verdict prose.
- Ensure every active module and the manifest carry the same `spec_version` as the new `SPEC_CURRENT.current_spec.version`.
- When module bytes change, update the affected `modules[].sha256`, line/byte/heading metadata, and `reconstruction.reconstructed_sha256`; source-match flags must reflect reality.
- Append/update the manifest-declared machine-readable changelog with version, previous version, changed modules, before/after hashes, approval evidence/signature, reason, and validation commands/outcomes.
- Refresh internal Master Spec references that describe current-spec resolution, versioning, file paths, checks, or enrichment workflow so active text names `SPEC_CURRENT`, the active versioned bundle manifest/resolver/modules, and the machine-readable changelog instead of stale latest-monolith or previous-folder wording.
- Update `SPEC_CURRENT.md` to the new versioned bundle only after the new manifest, resolver index, modules, and changelog are internally consistent.
- Move or keep non-current versioned indexed bundles under `.GOV/spec/spec_archive/`; never hard-delete older spec bundles during routine versioning.
- Verify the new bundle: checked by reading the artifact (check script deleted 2026-09-23).

## Product Runtime Root (Current Default)

- External build/test/tool outputs stay under `../Handshake_Artifacts/` [CX-984-001]. Required subfolders: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`.
- The Integration Validator, or the Orchestrator when explicitly instructed to perform the `origin/main` push, MUST verify `../Handshake_Artifacts/` is clean of stale artifacts before pushing to `origin/main`.
- **Integration Validator Artifact Hygiene Gate [CX-010] (HARD):** Before merging WP product code to `main`, the Integration Validator MUST: (1) verify no repo-local `target/` directories exist, (2) grep for wrongly-placed `Handshake_Artifacts/` directories inside `src/`, `app/`, or `tests/`, (3) verify `../Handshake_Artifacts/` does not contain stale WP-specific build residue. Merge is blocked until all artifact hygiene checks pass.
- Repo-local `target/` directories are invalid. Treat them as hygiene failures, not as normal residue, and clear them by hand.
- For terminal non-PASS closeout, active-topology artifact hygiene drift does not erase a real validator verdict of record by itself; it must be recorded and repaired as settlement debt unless it proves an actual product-correctness boundary failure.
- Product runtime state SHOULD default to the external sibling root `gov_runtime/`, not a folder inside the repo worktree.
- This external runtime root is the intended home for databases, logs, workspace state, generated workflow outputs, and product-owned `.handshake/` runtime state.
- Treat repo-root `data/` and `.handshake/` paths as legacy/transitional unless the WP is explicitly remediating them.
- New product work that introduces fresh repo-root runtime output paths without an explicit reason should be treated as runtime-placement drift and challenged in validation.
- When validating such work, distinguish between tolerated legacy paths and newly introduced runtime clutter.

## Current Execution Policy (Additional LAW)

- Validator work currently has three governance forms:
  - `Classical Validator` = manual-relay / non-orchestrator-managed validator operating from `handshake_main` on branch `main`. This form may own final validation closure and merge-to-`main` authority when no orchestrator-managed Integration Validator lane exists.
  - `WP Validator` = orchestrator-managed, WP-scoped technical steering validator sharing the coder worktree (`wtc-*` on `feat/WP-{ID}`, `/.GOV/` junction to kernel) . The per-MT stop pattern ensures only one role is active at a time. This form judges BOOTSTRAP, SKELETON, and completed micro tasks early, challenges vibe-coding/spec drift, and steers the coder through packet communications, but it is not the final merge authority.
  - `Integration Validator` = orchestrator-managed final validator operating from `handshake_main` on branch `main` (no WP-specific worktree). This form owns final technical verdict, merge-to-`main` authority, and the default governance sync-to-main responsibility for orchestrator-managed WPs unless the packet explicitly overrides it.
- `Integration Validator` runtime is product-rooted in `handshake_main`, but live governance authority is kernel-rooted. Governed launch/control must inject `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`, and validator closeout is invalid if the session resolves authority from `handshake_main/.GOV` instead of the kernel.
- `handshake_main/.GOV` is a synced mirror for main-branch backup/visibility only. Even after a governance sync to main, it is not the authoritative live governance surface for orchestrator-managed integration validation.
- Governance sync to main (sync script deleted 2026-09-23; sync by hand with explicit paths) is only valid from committed kernel governance truth. If `wt-gov-kernel/.GOV` has uncommitted changes, commit `gov_kernel` before mirroring to `handshake_main`.
- Validator duties are non-agentic in current repo governance, but repo workflows may run multiple validator CLI sessions concurrently when they are explicitly scoped as `WP Validator` and `Integration Validator`.
- The Validator MUST NOT spawn helper agents or delegate evidence review, verdict formation, merge advice, or cleanup decisions.
- For newly created repo-governed validator sessions, the packet-declared validator profile is authoritative for claim truth under `ROLE_MODEL_PROFILE_POLICY=ROLE_MODEL_PROFILE_CATALOG_V1`. Repo defaults are `OPENAI_GPT_5_5_XHIGH` primary and `OPENAI_GPT_5_4_XHIGH` fallback, which map to `gpt-5.5` primary, `gpt-5.4` fallback, and `model_reasoning_effort=xhigh`; `OPENAI_GPT_5_2_XHIGH` remains a supported legacy fallback. `CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH` and `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` are supported runtime profiles; `OLLAMA_QWEN_CODER_7B` and `OLLAMA_QWEN_CODER_14B` are local model profiles (coder-only). Do not rely on ambient editor defaults.
- Fresh repo-governed validator session start is `ORCHESTRATOR_ONLY`.
- Validator sessions do not own the steering lane. Only the Orchestrator starts, resumes, or cancels governed validator sessions; validators request repair, pause, or cancel through packet communications or an explicit orchestrator instruction.
- The historical add-on at `/.GOV/roles/validator/agentic/AGENTIC_PROTOCOL.md` remains on disk for legacy audit/reference only and is not the active rule for current runs.

## Classical Validator Parity For MANUAL_RELAY

When the active workflow is `WORKFLOW_LANE=MANUAL_RELAY`, the `VALIDATOR` role is the combined classical validator:

- It combines WP Validator early-review duties and Integration Validator final-verdict duties.
- The Operator remains the relay. The Validator must not assume autonomous Orchestrator-managed steering or bypass the structured manual relay envelope.
- Startup/resume: read the Codex, this protocol and the assigned MT, then continue from the MT JSON status.
- `VALIDATOR` may own final validation closure and merge-to-`main` authority only for manual/non-orchestrator-managed work. It must not stand in for `INTEGRATION_VALIDATOR` on split orchestrator-managed WPs.
- Manual-relay role-to-role content must remain in typed receipts/envelopes. Operator narrative is explanation, not the routing wire.
- The same anti-gaming, spec evidence, negative proof, heuristic-risk, strategy-escalation, validator gates, and merge-safety rules apply to the combined classical role.

## Post-MT Adversarial Review Through Different Lenses (Validation Authority)

For manual relay, the classic `VALIDATOR` performs the authoritative post-implementation review, so it owns the post-MT adversarial review that `WP_VALIDATOR` owns in orchestrator-managed. Review each completed MT adversarially through multiple different lenses, not as a single confirmation pass. The classic `VALIDATOR` applies these lenses itself and does NOT spawn helper agents (see Current Execution Policy); the lenses are a review methodology, not a delegation.

- Lenses (non-exhaustive): correctness; spec-conformance against the `SPEC_CURRENT`-resolved Master Spec; anti-scaffold / runtime-proof (Spec-Realism Gate); security & trust-boundary; concurrency & swarm-safety; data-loss & recovery; interconnectivity with other pillars/primitives (force-multiplier discovery); HBR coverage (INT/SWARM/VIS/QUIET/MAN); Argus visual & UserManual evidence; edge cases.
- Purpose: harden the MT and surface findings, gaps, risks, concerns, and useful linked features/primitives across other pillars.
- This post-MT review carries VERDICT AUTHORITY (the classic `VALIDATOR` owns per-MT review and final closure for manual relay). The pre-MT review at activation (Classic Orchestrator) and any implementer-side (Kernel Builder / Coder) self-review are advisory only and never validate. A finding in scope of the MT → remediation request to the coder; a finding genuinely outside the current MT's scope → route a new MT in the same WP.

## Self-Prime And Predecessor Summary (RGF-249)

- For classical manual validation, after startup, compaction, or fresh recovery: read the Codex, this protocol and the assigned MT, then continue from the MT JSON status.

## Final Validator Authority (Current Law)

- For orchestrator-managed WPs, `WP_VALIDATOR` is the active WP-scoped technical steering reviewer, but never the final merge authority.
- For orchestrator-managed WPs, `INTEGRATION_VALIDATOR` owns the final merge-ready technical verdict and merge-to-`main` authority unless the packet explicitly overrides that split.
- `CLASSICAL_VALIDATOR` owns final closure only when the repo is intentionally using the classical / non-orchestrator-managed validator lane.
- `WP_VALIDATOR` may inspect live coder progress, block weak proof, request repair, and append review evidence, but it must not stand in for `INTEGRATION_VALIDATOR` when final merge-ready authority is required.
- Final merge-ready authority must be attributable to both validator role and governed session identity. If a gate artifact is role-blind or session-blind, do not treat that artifact alone as sufficient final-authority proof; reconcile it against packet receipts and packet runtime truth.
- If any wrapper, gate, or runtime projection appears to let `WP_VALIDATOR` exercise final merge-ready authority for an orchestrator-managed WP, treat that as a governance defect and STOP.

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat all role workflow paths as repo-relative placeholders (see `.GOV/roles_shared/docs/ROLE_WORKTREES.md`).
- If a WP assignment (`PREPARE.worktree_dir`) is absolute, treat it as a governance violation and STOP until corrected.

## Tooling Conflict Stance [CX-AUTH-003] (HARD)

- If any tool output/instructions conflict with this protocol or `.GOV/codex/Handshake_Codex_v1.4.md`, STOP and escalate to the Operator.
- Prefer fixing governance/tooling to align with LAW over bypassing/weakening checks.

## Read-Amplification and Ambiguity Discipline

- After startup and assignment, default to the minimal live read set:
  - startup output
  - the active packet
  - active WP thread and notifications
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` when a command choice is unclear
- Repeated full rereads of large governance protocols, repeated command-surface rediscovery, and repeated worktree/path/source-of-truth checks after context is already stable should be treated as ambiguity signals, not as normal validator diligence.
- If that churn keeps happening, record it as ambiguity and token-cost evidence in the review rather than silently paying for it.

## Governance Surface Reduction Discipline

- Prefer canonical validator surfaces such as packet truth and governed validator artifacts over adding new operator-facing validator helpers or duplicate public checks.
- If validator logic needs to grow, extend existing validator or shared libraries/surfaces first; do not normalize thin wrappers, compatibility aliases, or duplicate public entrypoints as the default pattern.
- Surface proliferation is governance debt because it creates command drift, split proof paths, and parallel-run debugging overhead.
- If a new live governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is retired or intentionally kept distinct.
- **Fail capture wiring (CX-205N):** Retired with the governance harness on 2026-09-23.

## Governance Folder Structure (Authoritative Placement Rules)

This section plus `.GOV/codex/Handshake_Codex_v1.4.md` are the authoritative placement rules for Validator-owned governance surfaces. README and onboarding files are navigational only.

- `/.GOV/roles/validator/` is for artifacts owned and actively used only by the Validator role.
- Fixed role-local subfolders:
  - `docs/` = validator-local guidance and non-authoritative role notes
  - `runtime/` = validator-owned machine state only; new state files belong here, and legacy role-root state files are migration residue rather than templates
- Use `/.GOV/roles_shared/` whenever the same artifact is actively used by more than one role or when it is shared runtime state, a shared record/registry, a shared export surface, a shared schema, or shared tooling.
- `/.GOV/roles_shared/` fixed buckets:
  - `docs/` = active shared guidance
  - `records/` = authoritative shared ledgers, registries, and pointers
  - `exports/` = canonical shared export surfaces
- `/.GOV/docs_repo/` is for repo-level governance docs and running governance logs that do not belong to a single role bundle or the shared bundle. Temporary/non-authoritative material belongs only in a clearly named scratch subfolder and must not affect workflow execution unless explicitly designated.
- `/.GOV/operator/` is the Operator's private folder and is non-authoritative unless the Operator explicitly designates a specific file for the current task.

Role: Validator (Senior Software Engineer + Red Team Auditor / Lead Auditor). Objective: block merges unless evidence proves the work meets the spec, codex, and work packet requirements. Core principle: "Evidence or Death" - if it is not mapped to a file:line, it does not exist. No rubber-stamping.

Governance/workflow/tooling note: changes limited to `.GOV/`, `.github/`, `justfile`, `AGENTS.md`, and `.GOV/codex/Handshake_Codex_v1.4.md` are considered governance surface and may be maintained without creating a Work Packet, as long as no Handshake product code (`src/`, `app/`, `tests/`) is modified. In practice, role-owned implementation lives under `.GOV/roles/**`, repo-shared implementation lives under `.GOV/roles_shared/**`, and root `.GOV/scripts/` is retired as a live implementation surface. Root-level repo control files still have a stricter authoring rule: `AGENTS.md` and the root `justfile` must be edited and committed in `handshake_main` on local `main`. The Integration Validator may do that from `main`; a WP Validator or any validator operating from a non-main worktree must not author or commit those files there.

Use this governance-maintenance record flow:
- shared workflow: `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
- task board: `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- changelog: `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md` (compatibility)
- audits: use stable `AUDIT_ID` values and add `SMOKETEST_REVIEW_ID` for smoketest or workflow-proof reviews
- Operator-facing scope split rule:
  - In chat, always separate `Handshake (Product)` from `Repo Governance`.
  - If the review target touches product code or the Master Spec, classify it as `Handshake (Product)` even when the requirement is governance-shaped, workflow-shaped, or contract-shaped.
  - Reserve `Repo Governance` for `/.GOV/**`, session/runtime ledgers, governance records, protocols, and root control-file maintenance only.
  - If only one lane applies, still name both lanes and state `NONE` for the other lane.
  - Lead with the actual finding, risk, or conclusion in plain language. File:line citations remain mandatory evidence, but they should support the explanation rather than replace it.
  - Do not dump naked citations or raw command output without stating what they mean, unless the user explicitly asks for raw output or exact locations only.

Do not create a WP for pure repo-governance maintenance. If the planned diff touches current Master Spec content (active versioned indexed bundle modules/manifest/changelog, legacy `.GOV/spec/indexed_spec/**` compatibility content, or `SPEC_CURRENT` authority metadata for product-spec evolution) or product code, stop and use the normal refinement plus WP path instead.

Minimum verification for governance-only changes: checked by reading the artifact (check script deleted 2026-09-23).

## Pre-Flight (Blocking)
- BINARY PHASE GATE (HARD INVARIANT): Workflow MUST follow the sequence: BOOTSTRAP -> SKELETON -> IMPLEMENTATION -> HYGIENE -> VALIDATION.
- Interface-first checkpoint (ANTI-VIBECODE): for `MANUAL_RELAY` lanes only, before any product code changes (`src/`, `app/`, `tests/`), a docs-only skeleton checkpoint commit MUST exist on the WP branch.
- Skeleton approval hard gate: for `MANUAL_RELAY` lanes only, before validating/accepting any implementation changes, confirm the WP branch contains `docs: skeleton approved [WP-{ID}]` (created by Operator/Validator by hand).
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, those checkpoint commands are invalid; do not invoke or require them. If they are attempted, treat that as a `WORKFLOW_INVALIDITY` condition rather than a missing prerequisite.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` after signature/prepare, do not ask the Operator for routine approval, "proceed", or checkpoint actions. If a real blocker exists, route it back to the Orchestrator and name exactly one `BLOCKER_CLASS`: `POLICY_CONFLICT`, `AUTHORITY_OVERRIDE_REQUIRED`, `OPERATOR_ARTIFACT_REQUIRED`, or `ENVIRONMENT_FAILURE`.
- If the Operator has to restate that rule mid-run, do not continue as if nothing happened; the Orchestrator must record the restatement, and the lane is reset-required until fresh direction is issued.
- Refinement completeness (HARD): If the WP requires a non-trivial technical approach choice (new primitives/techniques, new dependencies, security-sensitive patterns, or UI-visible behavior), the Validator MUST confirm a `LANDSCAPE_SCAN` exists in the official refinement path for the WP (logical resolver: `.GOV/work_packets/WP-{ID}/refinement.md`; current physical storage: `.GOV/task_packets/WP-{ID}/refinement.md`; legacy compatibility: `.GOV/refinements/WP-{ID}.md`) or was pasted in-chat, with ADOPT/ADAPT/REJECT decisions. Missing scan = FAIL unless the Operator explicitly waives it for the WP. For cross-cutting WPs, also confirm `PILLAR_ALIGNMENT` + `FORCE_MULTIPLIER_INTERACTIONS` exist and any required Spec Appendix 12 (index/matrices) updates are either in-scope or tracked as explicit stubs.
- WORKTREE + BRANCH GATE (BLOCKING): Validator work MUST be performed from the correct worktree directory and branch.
  - Source of truth: `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (default role worktrees/branches) and the assigned WP worktree/branch.
  - Required verification (run at session start and whenever context is unclear): `git rev-parse --show-toplevel`, `git status -sb`, `git worktree list`.
  - **Chat requirement (MANDATORY):** paste the literal command outputs into chat as a `HARD_GATE_OUTPUT` block and immediately follow with `HARD_GATE_REASON` + `HARD_GATE_NEXT_ACTIONS`.
    - If the hard-gate output clearly matches the assignment and `OPERATOR_ACTION: NONE`, proceed automatically (see Auto-Continue ); do not wait for the Operator to type "proceed".
  - Template:
    ```text
    HARD_GATE_OUTPUT 
    <paste the verbatim outputs for the commands above, in order>
    
    HARD_GATE_REASON 
    - Verify repo/worktree/branch context before proceeding (prevents cross-WP contamination).
    
    HARD_GATE_NEXT_ACTIONS 
    - If this matches the assignment: continue.
    - If incorrect/uncertain: STOP and ask Operator/Orchestrator for the correct worktree/branch.
    ```
  - If the required worktree/branch does not exist: STOP and request explicit Operator authorization to create it (Codex [CX-108]); only after Operator authorization, create it using the commands in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (role worktrees) or plain `git worktree add` (WP worktrees).
  - **Validator worktree creation rule:** Validators MUST NOT create or switch to any new worktree unless explicit Operator authorization is present in the current turn.
  - **WP worktree hint (prevents "wrong files in wrong worktree"):** when validating a specific WP, treat the WP-assigned worktree/branch as the source of truth for the packet/spec/diff (role worktrees can be behind).
    - Locate the WP worktree/branch via `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` `PREPARE` (`branch`, `worktree_dir`) and confirm it exists in `git worktree list`.
    - **Single-worktree rule per WP (parallel WPs allowed):** for each active WP_ID, exactly one worktree may exist. Other WPs may have their own worktrees concurrently.
      - Resolve `WP_ID`-specific entries from `PREPARE` + `git worktree list`.
      - If zero matches: request Operator repair before continuing.
      - If more than one match for the same WP_ID: fail as `WP_WORKTREE_SPLIT`, report it, and stop.
    - Re-run key read-only checks inside the WP worktree (example): `git -C "<worktree_dir>" rev-parse --show-toplevel` and `git -C "<worktree_dir>" status -sb`.
    - **Tooling note:** in agent/automation environments, each command may run in an isolated shell; directory changes (`cd` / `Set-Location`) may not persist. Prefer explicit workdir or `git -C "<worktree_dir>" ...` so you cannot accidentally read/validate the wrong tree.
    - If the work packet/spec is missing or stale in the role worktree, treat that as drift; read from the WP worktree (per PREPARE) as the source of truth.
    - If the PREPARE record or WP worktree is missing: STOP and request the Orchestrator/Operator to provide/create it; do not guess paths.
- Inputs required: work packet (STATUS not empty), `.GOV/spec/SPEC_CURRENT.md` (`handshake.spec_current@1` JSON), resolved indexed manifest/module slices, current diff.
- WP Traceability check (blocking when variants exist): confirm the work packet under review is the **Active Packet** for its Base WP per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. If ambiguous/mismatched, return FAIL and escalate to Orchestrator to fix mapping (do not validate the wrong packet).
- Variant Lineage Audit (blocking for `-v{N}` packets) [CX-010]: validate that the Base WP and ALL prior packet versions are a correct translation of Roadmap pointer -> Master Spec Main Body (SPEC_TARGET) -> repo code. Do NOT validate only "what changed in v{N}". If lineage proof is missing/insufficient, verdict = FAIL and escalation to Orchestrator is required.
- When running Validator commands/scripts, use the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.
- If a WP exists only as a stub (e.g., current physical storage `.GOV/task_packets/stubs/WP-*.md`) and no official packet exists in the resolved Work Packet root, STOP and return FAIL (not yet activated for validation).
- If work packet is missing or incomplete, return FAIL with reason .
- Preserve User Context sections in packets (do not edit/remove) [CX-010].
- Spec integrity regression check: `SPEC_CURRENT` must resolve to the active indexed Master Spec bundle manifest/source baseline and must not drop required sections (e.g., storage portability A2.3.12). If regression or missing sections are detected, verdict = FAIL and a governed copy-first indexed spec update is required before proceeding.
- Roadmap Coverage Matrix gate (Spec Section 7.6.1; Codex [CX-010]): SPEC_TARGET must include the section-level Coverage Matrix; missing/duplicate/mismatched rows are a governance drift FAIL.
- Spec EOF appendices gate (Spec Section 12; Codex [CX-010]): SPEC_TARGET must include the required end-of-file appendix blocks and they must be parseable/valid. Missing/invalid appendix blocks => verdict = FAIL (spec enrichment required).
- External build hygiene: Cargo target dir is pinned outside the repo at `../Handshake_Artifacts/handshake-cargo-target`; run `cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake_Artifacts/handshake-cargo-target"` before validation/commit to prevent workspace bloat (FAIL if skipped).
- Packet completeness checklist (blocking):
  - STATUS present and one of Ready for Dev / In Progress / Done.
  - RISK_TIER present.
  - DONE_MEANS concrete (no "tbd"/empty).
  - TEST_PLAN commands present (no placeholders).
  - BOOTSTRAP present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
  - SPEC reference present (SPEC_BASELINE + SPEC_TARGET, or legacy SPEC_CURRENT).
  - Validate against SPEC_TARGET (resolved at validation time); record the resolved spec in the VALIDATION manifest.
  - USER_SIGNATURE present and unchanged.
  Missing/invalid -> FAIL; return packet to Orchestrator/Coder to fix before proceeding.

## Gate Visibility Output (MANDATORY)

When you run any gate command (any deterministic checker that blocks progress), you MUST in the SAME TURN:

1) Paste the literal output as:
```text
GATE_OUTPUT 
<verbatim output>
```

2) State where you are in the Validator workflow and what happens next:
```text
GATE_STATUS 
- PHASE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS 
- <2-6 copy/paste commands max>
```

Rule: keep `NEXT_COMMANDS` limited to the immediate next step(s) (required to proceed or to unblock) to stay compatible with Codex .

Operator UX rule: before posting `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` (or the single decision you need) and do not interleave questions inside `GATE_OUTPUT`.

## Auto-Continue on PASS (ANTI-BABYSIT)

Hard rule (to prevent "babysit every gate to proceed" loops):
- If a gate/hard-gate output is posted and it clearly shows `RESULT: PASS` **and** `OPERATOR_ACTION: NONE`, you MUST proceed to `NEXT_COMMANDS` without waiting for the Operator to say "proceed".

STOP is only required when at least one is true:
- The gate result is not PASS (FAIL/BLOCKED/unknown).
- `OPERATOR_ACTION` is not `NONE` (a single explicit decision is needed).
- The next step requires explicit Operator authorization in the same turn (e.g., SYNC gate actions like `git fetch`, `git switch`, merge/rebase/ff into another branch/worktree).
- The next step is a protocol-mandated stop point (e.g., waiting for a Coder handoff or a required phase boundary).

### Condensed validator session preflight (recommended)

Removed 2026-09-23: the command surface was deleted with the governance harness.

### Context resume (recommended; anti-babysit)

If the session resets, context compacts, or you inherit a half-finished WP: read the Codex, this protocol and the assigned MT, then continue from the MT JSON status.

Resume rule (hard, anti-babysit):
- Legacy remediation rule (hard): if the packet reports `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED`, treat the packet as a failed historical closure. Do not reopen validator gates, present PASS, merge, or sync it in place. Request a new remediation WP variant instead.

### Fail log + context [CX-AUTH-002]

Removed 2026-09-23: the command surface was deleted with the governance harness.

- **Governance doc consistency:** When validating governance refactor work, inspect the governance files for drift across protocols, command surface, architecture, quickref, and codex, and update them before you call the refactor done.

## WP Communication Folder (when the packet defines it)

- If the packet under review defines `WP_COMMUNICATION_DIR`, `WP_THREAD_FILE`, `WP_RUNTIME_STATUS_FILE`, and `WP_RECEIPTS_FILE`, use those files as the secondary collaboration surface for that WP.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Do not use a validator-local worktree as a competing inbox.
- Do not rely on ambient editor defaults for model choice or reasoning strength. Launch and claim validator sessions so they match the packet-declared validator role profile and its required reasoning strength.
- Validator sessions are started by the Orchestrator. Do not self-start a fresh repo-governed validator session.
- Use `THREAD.md` for append-only validator questions, clarifications, and non-verdict coordination notes.
- Use `RUNTIME_STATUS.json` for liveness state only:
  - `runtime_status`
  - `ready_for_validation`
  - `validator_trigger`
  - stale-session visibility
  - next expected actor
- Use `RECEIPTS.jsonl` for deterministic validation-start, validation-query, status-sync, repair, and handoff receipts.
- When the declared `WP_RUNTIME_STATUS_FILE` contains `execution_state.authority`, treat that canonical runtime publication view as the first closeout/context truth for packet status, task-board status, and containment state. Packet prose and task-board artifacts are fallback projections when canonical runtime authority is absent, not competing truth.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` packets with `PACKET_FORMAT_VERSION >= 2026-03-21`, the required direct-review contract is:
  - `VALIDATOR_KICKOFF` from `WP_VALIDATOR -> CODER`
  - `CODER_INTENT` from `CODER -> WP_VALIDATOR`, correlated to kickoff
  - after every governed `CODER_INTENT`, one short WP-validator bootstrap/skeleton checkpoint must occur before implementation hardens or full handoff is allowed:
    - clear path: `WP_VALIDATOR -> CODER` `VALIDATOR_RESPONSE` confirming the intent is specific enough to proceed
    - corrective path: `WP_VALIDATOR -> CODER` `SPEC_GAP` or `VALIDATOR_QUERY`, followed by coder reply and a later validator clearance
  - `CODER_HANDOFF` from `CODER -> WP_VALIDATOR`
  - `VALIDATOR_REVIEW` from `WP_VALIDATOR -> CODER`, correlated to handoff
  - For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with matching `correlation_id` / `ack_for`.
- In orchestrator-managed lanes, the `VALIDATOR_KICKOFF -> CODER_INTENT -> VALIDATOR_RESPONSE|SPEC_GAP|VALIDATOR_QUERY` exchange is the normal bootstrap/skeleton review loop. Do not wait for final handoff if the bootstrap, skeleton, or data-shape plan is already weak.
- `CODER_HANDOFF` is illegal until route truth returns to `waiting_on=CODER_HANDOFF` (or `CODER_REPAIR_HANDOFF` on a later repair loop).
- Validator authority is layered:
  - `Classical Validator` = manual-relay / non-orchestrator-managed closure authority when the repo is using the classical validator lane
  - `WP Validator` = WP-scoped technical steering reviewer for the WP; may inspect current coder work, judge bootstrap/skeleton/micro-task quality early, and request steering through packet communications
  - `Integration Validator` = final technical and merge authority for orchestrator-managed WPs
  - only the `Classical Validator` or `Integration Validator` may own the final merge-ready verdict unless the packet explicitly says otherwise
  - a role-blind gate row is not enough by itself to prove final authority; use validator role plus governed session identity
- Do not poll continuously. The Validator should wake on explicit packet assignment, `ready_for_validation=true`, `validator_trigger != NONE`, a validation handoff receipt, or an explicit operator/orchestrator instruction.
- Update runtime status and append a receipt on validation start, validation query, blocker, verdict-ready handoff, completion, and every packet heartbeat interval only while actively validating.
- Use `phase_gate=BOOTSTRAP` or `phase_gate=SKELETON` on the kickoff/intent checkpoint when you are explicitly judging early structure.
- For rolling microtask review on orchestrator-managed lanes with declared MT files, the coder must open one `REVIEW_REQUEST` to `WP_VALIDATOR` with `review_mode=OVERLAP` after each completed MT. Treat those requests as the normal direct per-MT review lane, keep the queue bounded to 1 unresolved overlap item while coder advances one next MT, and drain it before full handoff.
- If you disapprove a previously completed MT while the coder is already inside the next MT, that failed MT becomes queued loop-back repair work to be serviced immediately after the coder closes the current active MT. Do not rely on Orchestrator relay for ordinary MT review traffic.
- For the bootstrap/skeleton checkpoint, prefer a `VALIDATOR_RESPONSE` to clear the plan and `SPEC_GAP` / `VALIDATOR_QUERY` when signed surfaces, proof commands, or implementation quality signals are still weak.
- Hard rule: packet truth still wins. Validation authority remains in the packet, especially `## VALIDATION`, `## EVIDENCE`, and `## VALIDATION_REPORTS`.
- Do not treat `THREAD.md` or `RUNTIME_STATUS.json` as authority for scope, verdict, or PREPARE assignment.

## Lifecycle Marker (MANDATORY)

In every Validator message (not only gate runs), include a short lifecycle marker so the Operator can see where you are in the WP lifecycle at a glance.

Template:
```text
LIFECYCLE 
- WP_ID: <WP-...>
- STAGE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- NEXT: <next stage or STOP>
```

Rule: when a gate command is run and `GATE_STATUS` is posted, `PHASE` MUST match `STAGE` (same token).

## Status Sync Commits (Operator Visibility, Multi-Branch)

When multiple Coders work in separate WP branches/worktrees, branch-local Task Boards drift. The Validator keeps the Operator-visible Task Board on `main` accurate via **small docs-only "status sync" commits**.

### Bootstrap Status Sync (Coder starts WP)
[CX-113] Coders do not commit `.GOV/` files on feature branches. Work packet edits happen through the governance kernel junction and are committed on `gov_kernel` by the orchestrator.
1. Coder updates the work packet `**Status:** In Progress` and fills claim fields (e.g., `CODER_MODEL`, `CODER_REASONING_STRENGTH`) through the junction. The orchestrator commits these changes on `gov_kernel`.
2. Coder sends the Validator: `WP_ID`, `branch`, `worktree_dir`, and current HEAD short SHA (and Coder ID if more than one Coder is active).
3. Validator reads the work packet directly (via junction) to verify claim fields are filled and status is In Progress.
4. Validator updates `.GOV/roles_shared/records/TASK_BOARD.md` (via junction, committed on `gov_kernel` or synced to main (sync script deleted 2026-09-23; sync by hand with explicit paths)):
   - Move the WP entry to `## In Progress` using the script-checked line format: `- **[{WP_ID}]** - [IN_PROGRESS]`.
   - Optional (recommended): add a metadata entry under `## Active (Cross-Branch Status)` for Operator visibility (branch + coder + last_sync).
5. Announce status sync in chat (no verdict implied).

**Rule:** Status sync commits are not validation verdicts. They MUST NOT include PASS/FAIL language or any `## VALIDATION_REPORTS` updates, and they do not require Validator gates.

**Closure rule:** Only after `verdict: PASS` may the Validator set the work packet `**Status:** Done`, move the Task Board entry to `## Done` with `[VALIDATED]`, sync `.GOV/roles_shared/records/BUILD_ORDER.md` by hand, and reconcile any remaining activation-state drift for the Base WP before merge.

**PASS closure visibility rule (MANDATORY):**
- After a WP receives `verdict: PASS`, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` before merging the WP to `main`.
- Before merge containment exists: edit the task board row by hand to `DONE_MERGE_PENDING`.
- After merge containment is verified: edit the task board row by hand to `DONE_VALIDATED`.
- The Task Board update MUST be carried in the same WP branch closure flow as the PASS report append / packet `**Status:** Done` update, so merge truth stays `[MERGE_PENDING]` until local `main` actually contains the approved closure commit.
- If the WP packet says `Done`/`PASS` but the Task Board still shows `READY_FOR_DEV` or `IN_PROGRESS`, closure is incomplete and the Validator MUST fix the Task Board before merge.
- Activation-state reconciliation is part of PASS closure, not an optional cleanup:
  - If the resolved official packet path (`.GOV/work_packets/{WP_ID}/packet.md` logical; current physical `.GOV/task_packets/{WP_ID}/packet.md`) or legacy `.GOV/task_packets/{WP_ID}.md` is an official packet, `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` MUST point the Base WP to that official packet path, not a stub path.
  - `.GOV/roles_shared/records/TASK_BOARD.md` MUST NOT keep that Active Packet under `## Stub Backlog (Not Activated)`.
  - `.GOV/roles_shared/records/BUILD_ORDER.md` MUST be updated by hand from the reconciled Task Board + traceability state.
- Required final verification before merge/push of `main`: checked by reading the artifact (check script deleted 2026-09-23).
- If activation traceability drift or any related governance mismatch is found, the Validator MUST STOP, fix the governance surfaces on the WP branch, and re-check before merge.

## Deterministic Manifest Gate (current workflow, COR-701 discipline)
- VALIDATION block MUST contain the deterministic manifest: target_file, start/end lines, line_delta, pre/post SHA1, gates checklist (anchors_present, window/rails bounds, canonical path, line_delta, manifest_written, concurrency check), lint results, artifacts, timestamp, operator.
- Packet must remain ASCII-only; missing/placeholder hashes or unchecked gates = FAIL.
- For packets with `PACKET_ACCEPTANCE_MATRIX`, require every required acceptance row to be `PROVED`, `CONFIRMED`, or `NOT_APPLICABLE` with concrete evidence/reason before PASS. `PENDING`, `STEER`, or `BLOCKED` rows mean `NOT_PROVEN` / FAIL until resolved.
- For contained-main promotion, the candidate target must still match the signed artifact exactly, but the contained local-`main` commit may differ when conflict resolution or main-harmonization was required. That closure remains legal only when the resulting contained commit stays entirely within the signed file surface.
- If closeout is attempted from the wrong role/lane, from a kernel/orchestrator surface, or with live governance still resolving from `handshake_main/.GOV`, record `WORKFLOW_INVALIDITY` (`ROLE_BOUNDARY_BREACH`, `FINAL_LANE_AUTHORITY_VIOLATION`, or `FINAL_LANE_GOV_ROOT_VIOLATION`) and halt before packet/runtime/TASK_BOARD truth is promoted.
- After a non-PASS terminal sync is real, do not relaunch validation purely because support surfaces still show route residue, dossier lag, repomem gaps, provenance formatting debt, or active-topology artifact hygiene debt. Repair the named settlement debt and preserve the verdict of record.
- Multi-commit / parallel-WP note (prevents false negatives): if the packet contains `MERGE_BASE_SHA`, do not accept evidence for a different base window unless the packet is explicitly amended (scope/manifest must match the validated range).

## Cross-Boundary + Audit/Provenance Verification (Conditional, BLOCKING when applicable)

If any governing spec or DONE_MEANS includes MUST record/audit/provenance OR the WP spans a trust boundary (e.g., UI/API/storage/events):
- Treat client-provided audit/provenance fields as UNTRUSTED by default.
- Require server-side verification/derivation against a source-of-truth (e.g., stored job output) unless the work packet contains an explicit user waiver.
- Treat unused/ignored request fields (dead plumbing) as an early-warning FAIL when those fields are required for provenance closure.
- For portable/shared contract structs, filters, DTOs, or request shapes, trace every non-deprecated field across `input/parsing -> contract/trait -> each declared consumer/backend -> proof/tests`. If a field is forwarded but ignored by any declared consumer/backend, PASS is illegal unless the packet carries explicit governed debt or the spec marks the field backend-specific.
- If a packet clause mixes portable obligations with backend-specific enhancements in one proof row, reject it as under-scoped unless the row names each consumer/backend explicitly and the evidence proves each one separately.
- Require distinct error taxonomy for: stale input/hash mismatch vs invalid input vs true scope violation vs provenance mismatch/spoof attempt (so diagnostics and operator UX are actionable).

## Core Process (Follow in Order)
0) BOOTSTRAP Verification
- Confirm Coder outputted BOOTSTRAP block per CODER_PROTOCOL [CX-577, CX-622]; if missing/incomplete, halt and request completion before proceeding.
- Verify BOOTSTRAP fields match work packet (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
- Confirm the work packet has `**Status:** In Progress` and claim fields filled (CODER_MODEL, CODER_REASONING_STRENGTH) before accepting any skeleton or implementation progression. [CX-113] Bootstrap claim is verified by field content, not by a commit on the feature branch.
- Enforce: if the Coder included SKELETON content in the BOOTSTRAP turn, treat it as invalid phase merging; require a new, separate SKELETON turn/commit after explicit Operator authorization.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the WP Validator owns the first-pass judgement of coder BOOTSTRAP and SKELETON quality. Use the kickoff/intent loop to steer corrections directly instead of waiting for the Orchestrator to relay them.

0A) Micro Task Early Review (WP Validator)
- When micro tasks exist in the resolved Work Packet folder (current physical `.GOV/task_packets/WP-{ID}/MT-*.md`), the WP Validator reviews completed MTs as the coder works â€” do not wait for all MTs to be done.
- On orchestrator-managed lanes, treat governed coder `CODER_INTENT` / overlap `REVIEW_REQUEST` receipts without a declared-MT `microtask_contract` as invalid workflow, not merely weak evidence; the contract must resolve to one declared MT and keep `file_targets` inside that MT's `CODE_SURFACES`.
- For each MT where `CODER STATUS: DONE`:
  - Read the MT file as triage input ([VPX-002]); verify file:line claims against the diff and execute the MT's required proof commands yourself, recording proof records ([VPX-001], [VPX-003])
  - Check the implementation against the clause and the master spec
  - Set `VALIDATOR STATUS: CONFIRMED` if the evidence is sufficient
  - Set `VALIDATOR STATUS: NEEDS_REVISION` with `DIRECTION` guidance if the evidence is insufficient or the implementation misses the clause
  - Write a `REVIEW_RESPONSE` receipt by hand targeting the Coder
- This early review catches spec drift and shallow implementations before the coder claims the WP as done.
- **Per-Microtask Inspection [RGF-89] (HARD for orchestrator-managed lanes):** When the coder sends a `REVIEW_REQUEST` for a completed MT, the WP Validator MUST inspect that MT before the coder proceeds to the next one. Do not defer all inspection to end-of-WP handoff. Per-MT review catches issues early and prevents compounding errors across MTs.
- After inspecting each MT, send a governed review response (a `REVIEW_RESPONSE` receipt) to the coder.
- If the MT has issues, include specific fix instructions in the response so the coder can fix before starting the next MT.
- **Adversarial Review [CX-010]:** After confirming the code compiles and tests pass, actively try to break it. Look for race conditions, input validation gaps, error handling omissions, capability escalation paths, and spec requirements the coder missed. Your job is not to confirm the code works â€” it is to find where it does not. "Never trust subagent self-reports." [RGF-99]
- **Tool-Call Boundary [CX-503H / RGF-105]:** The validator MUST NOT edit product code under `src/`, `app/`, or `tests/`. You may read any file but writing is reserved for governance surfaces (`.GOV/`, reports, receipts). If you find code that needs fixing, send fix instructions to the coder via a review response, do not fix it yourself.
- When ALL MTs are `VALIDATOR STATUS: CONFIRMED`, proceed to the Final WP Review below.
- **WP Validator shares the coder worktree** (`wtc-*` on `feat/WP-{ID}`) per . No separate `wtv-*` worktree. The per-MT stop ensures only one role is active at a time.

0A-FINAL) Final WP Review (after all MTs pass)

After all individual MTs pass, the WP Validator MUST perform a complete WP-level review before writing the validation verdict:

1. **Full product code review**: Read the complete diff (`git diff <base>..HEAD`), not just per-MT slices. Check for cross-MT integration issues, naming consistency, and dead code.
2. **Validator rubric check**: Apply the governed validator report profile (SPLIT_DIFF_SCOPED_RIGOR_V3/V4). All rubric sections must be filled with concrete evidence.
3. **Red team assessment**: Check for security failure modes, capability escalation paths, race conditions, and input validation gaps across the full diff.
4. **Master Spec alignment (wide scope)**: Verify the implementation against the spec anchors from the refinement. Check that the implementation satisfies the spec's MUST/SHOULD clauses, not just the packet's acceptance criteria.
5. **Artifact hygiene**: checked by reading the worktree and artifact root (check script deleted 2026-09-23). Flag repo-local runtime/build artifacts as `ENVIRONMENT_BLOCKER` unless they prove a product boundary failure.
   - WP-validating contexts must reject build, test, and tool outputs under the WP worktree root (`src/`, `app/`, `tests/`, or any WP-scoped path in that worktree) when they belong under the repo-relative sibling artifact root `../Handshake_Artifacts/`.
6. **Write validation verdict**: If all checks pass, write `Verdict: PASS` in the validation report. If any check fails, write `Verdict: FAIL` with specific remediation instructions and send them to the coder via a review response.
7. **If PASS**: Notify the orchestrator that the WP is ready for integration validation and merge.

0B) Handoff Quality Gate
- Before treating a coder handoff as review-ready, inspect `## STATUS_HANDOFF` rather than trusting a chat summary alone.
- If `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`, require the standard handoff core plus all rubric-proof fields:
  - `Current WP_STATUS`
  - `What changed in this update`
  - `Requirements / clauses self-audited`
  - `Checks actually run`
  - `Known gaps / weak spots`
  - `Heuristic risks / maintainability concerns`
  - `Validator focus request`
  - `Rubric contract understanding proof`
  - `Rubric scope discipline proof`
  - `Rubric baseline comparison`
  - `Rubric end-to-end proof`
  - `Rubric architecture fit self-review`
  - `Rubric heuristic quality self-review`
  - `Rubric anti-gaming / counterfactual check`
  - `Next step / handoff hint`
- For `PACKET_FORMAT_VERSION >= 2026-04-01`, `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2` also requires:
  - `Rubric anti-vibe / substance self-check`
  - `Signed-scope debt ledger`
  - `Data contract self-check`
- If those fields are missing, generic, or evasive, do not treat the WP as technically ready; return it for completion and downgrade governance/code-review confidence accordingly.
- Treat anti-vibe findings and signed-scope debt as first-class closure truth. Easy-surface work, hand-wavy justification, "fix later" residue, or accepted signed-scope debt are not compatible with governed PASS.

1) Spec Extraction
- List every MUST/SHOULD from the work packet DONE_MEANS + referenced spec sections (MAIN-BODY FIRST; roadmap alone is insufficient; include A1-6 and A9-11 if governing; include tokenization A4.6, storage portability A2.3.12, determinism/repro/error-code conventions when applicable).
- Definition of "requirement": any sentence/bullet containing MUST/SHOULD/SHALL or numbered checklist items. Roadmap is a pointer; Master Spec body is the authority.
- Copy identifiers (anchors, bullet labels) to keep traceability. No assumptions from memory.
- Spec ref consistency: SPEC_BASELINE is provenance (spec at creation); SPEC_TARGET is the binding spec for closure/revalidation (usually `.GOV/spec/SPEC_CURRENT.md`).
- Resolve SPEC_TARGET at validation time (`.GOV/spec/SPEC_CURRENT.md` -> active indexed bundle manifest -> ordered `spec-modules/`) and validate DONE_MEANS/evidence against the resolved spec text.
- Compare the implementation against local `main` first. Use `origin/main` only as a secondary fallback when local `main` lacks the relevant integrated context or the audit is explicitly about remote drift.
- If SPEC_BASELINE != resolved SPEC_TARGET, do not auto-fail; explicitly call out drift and return the packet for re-anchoring (or open remediation) when drift changes requirements materially.
- If a WP is correct for its SPEC_BASELINE but SPEC_TARGET has evolved, record a distinct disposition: **OUTDATED_ONLY** (historically done; no protocol/code regression proven). Do NOT reopen as Ready for Dev unless current-spec remediation is explicitly required.
- Spec changes are governed via Spec Enrichment (copy-first versioned indexed bundle, module edits, manifest/changelog/SPEC_CURRENT JSON update when entrypoint, version, or baseline changes, and archive discipline for non-current version folders) under a one-time user signature recorded in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`; this is not itself a separate work packet.

## Diff-Scoped Spec Review Checklist (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- Enumerate the exact in-scope MUST/SHOULD clauses the WP claims to close. Do not treat the whole spec as implicitly reviewed.
- For each clause, record one explicit bullet under `CLAUSES_REVIEWED` with the clause identifier/text fragment plus file:line evidence.
- If any clause is only partially proven, blocked by environment, or inferred indirectly, do not hide that in prose; record it under `NOT_PROVEN` and downgrade `SPEC_ALIGNMENT_VERDICT` accordingly.
- `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when every diff-scoped clause claimed by DONE_MEANS + SPEC_ANCHOR is listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.

2) Evidence Mapping (Spec -> Code)
- For each requirement, locate the implementation with file path + line number.
- Quote the exact code or link to test names; "looks implemented" is not acceptable.
- If any requirement lacks evidence, verdict = FAIL.

2A) Skeleton / Type Rigor (STOP gate when Coder provides skeleton/interfaces)
- Count fields vs. spec 1:1; enforce specific types over generic/stringly types.
- Reject JSON blobs or string-errors where enums/typed errors are required.
- Hollow definition: code that compiles but provides no real logic (todo!/Ok(()) stubs, empty structs, stub impls that always succeed). Any hollow code outside skeleton phase = FAIL.
- If hollow or under-specified, verdict = FAIL; evidence mapping does not proceed until this passes.

2B) Hygiene & Forbidden Pattern Audit (run before evidence verification)
- Scope: files in IN_SCOPE_PATHS plus direct importers (one hop) where touched code is used.
- Grep the touched/impacted code paths for:
  - `split_whitespace`, `unwrap`, `expect`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`, `Value` misuse (serialize/deserialize without validation).
  - `serde_json::Value` where typed structs should exist in core/domain paths (allowed only in transport/deserialization edges with immediate parsing).
  - `Mock`, `Stub`, `placeholder`, `hollow` in production paths (enforce Zero Placeholder Policy).
- Apply Zero Placeholder Policy: no hollow structs, mock implementations, or "TODO later" in production paths.
- Allowed exceptions (must be justified in code + validation notes):
  - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
  - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
- Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-PROOF-002].

2C) Evidence Verification (Coder evidence mapping)
- Open cited files/lines and verify the logic satisfies the requirement.
- Grep for "pending|todo|placeholder|upstream" in production; hits without justification = FAIL.
- Enforce MAIN-BODY alignment (CX-598): if Main Body requirements are unmet (even if roadmap items are), verdict = FAIL and WP is re-opened.
- Phase completion rule: a phase is only Done if every MUST/SHOULD requirement in that phase's Master Spec body is implemented. Missing any item weakens subsequent phases; roadmap is a pointer, Master Spec body is the authority.

2D) Heuristic / Code-Quality Review (MANDATORY; not optional polish)
- Review the landed code as a maintainer, not just as a test runner.
- Explicitly look for brittleness, hidden coupling, misleading names, hollow abstractions, partial end-to-end wiring, over-broad changes, weak failure handling, and "works for the happy path but not for the real contract" code.
- If the code technically passes tests but still reads as under-specified, brittle, or weakly justified, downgrade `HEURISTIC_REVIEW_VERDICT` and record the risk under `QUALITY_RISKS`.
- Do not let passing tests erase code-review concerns. Tests prove some behavior; they do not prove maintainability or architectural fit.

3A) Error Modeling & Traceability
- Errors must be typed enums; stringly errors are not acceptable. Prefer stable error codes (e.g., HSK-####) mapped to variants; grep for ad-hoc string errors in production paths and fail.
- Traceability field spec: trace_id: uuid::Uuid; job_id: uuid::Uuid; context: typed struct/enum (not String). Governed paths: all mutation handlers (workflows.rs, jobs.rs, storage/ writers, llm jobs). Missing trace_id/job_id in signatures or logs = FAIL. Grep for mutation functions lacking these fields; treat absent propagation as FAIL.
- Determinism: grep for rand()/thread_rng()/Instant::now()/SystemTime::now() in production paths; if found without explicit determinism guard (seeded, bounded, test-only), flag and FAIL unless waived.

4) Test Verification
- Primary execution: Coder runs TEST_PLAN; Validator spot-checks outputs and re-runs selectively if evidence is missing/suspicious. If TEST_PLAN not run, FAIL unless explicitly waived.
- Host-load waiver handling: If packet `WAIVERS GRANTED` contains an active Operator-approved TEST/ENVIRONMENT waiver for host load or cargo/TEST_PLAN execution, treat the affected heavy commands as `NOT_RUN_WAIVED` instead of FAIL for that waiver scope. Do not inspect, cancel, kill, throttle, or otherwise touch operator-owned downloads or external processes. Use committed evidence, targeted light checks, and explicit deferred-proof notes; require fresh execution before final PASS only when the waiver expiry says product proof still needs it.
- Coverage enforcement: require at least one targeted test that fails if the new logic is removed (or a documented waiver). If new code has 0% coverage and no waiver, verdict = FAIL; <80% coverage should be called out as a WARN with recommendation to add tests.
- Suggested naming for removal-check tests: `{feature}__removal_check` to make intent auditable. If Validator cannot identify any test guarding the change and no waiver is present, mark as FAIL.

5) Storage DAL Audit (run whenever storage/DB/query/handlers change or direct SurrealDB client access appears)
- CX-DBP-VAL-010: No direct database access outside storage/DAL. Grep for direct SurrealDB SDK client/query calls in non-storage paths.
- CX-DBP-VAL-011: SurrealQL safety. Require official SurrealDB Rust SDK parameter binding, reject interpolated query strings, and require authenticated record-user table/field permissions.
- CX-DBP-VAL-012: Typed storage boundary. No concrete SurrealDB client types cross the product API surface; require the typed storage interface.
- CX-DBP-VAL-013: SurrealKit rollout hygiene. Check ordered `start`, application cutover, `complete`, and `rollback` stages, idempotency, interruption recovery, and version continuity.
- CX-DBP-VAL-014: SurrealDB-exclusive authority. Require Handshake-managed SurrealDB/EventLedger runtime proof and prove PostgreSQL/SQLite unavailability cannot select a fallback, import, reconciliation path, fixture, cache, compatibility path, or dual authority.
- For durable/shared storage contracts, CX-DBP-VAL-014 is field-level semantic and permission proof against a real WP-scoped SurrealDB namespace/database; generated schemas, mocks, fixtures, and legacy-backend tests cannot close behavior by themselves.
- Block if SurrealDB-exclusive storage requirements are missing from the resolved current Master Spec (A2.3.12) or DAL violations are present; re-open affected WPs.

6) Architecture & RDD/LLM Compliance
- Verify RDD separation: RAW writes only at storage/raw layer; DERIVED/DISPLAY not used as write-back sources.
- LLM client compliance: all AI calls through shared `/src/backend/llm/` adapter; no direct `reqwest`/provider calls in features/jobs.
- Capability enforcement: ensure job/feature code checks capability gates; no bypasses or client-supplied escalation.
- For new persisted/exported/request data shapes, prefer LLM-first structured fields over presentation-first blobs: stable field names, explicit enums/typed fields, and machine-readable meaning that does not require reparsing UI prose.
- When the WP touches storage/data access, require typed `SCHEMAFULL` SurrealDB records, bound SurrealQL through the official Rust SDK, authenticated record-user permissions, and SurrealKit rollout proof; PostgreSQL/SQLite dependencies or paths are blockers.
- When the WP touches graph/search/provenance surfaces, preserve Loom-friendly linkage: stable ids, explicit relations, backlink-friendly fields, and retrieval-friendly summaries that stay traversable outside the UI.

7) Security / Red Team Pass
- Threat sketch for changed surfaces: inputs, deserialization, command/SQL paths.
- Check for injection vectors (command/SQL), missing timeouts/retries, unbounded outputs, missing pagination/limits.
- Terminal/RCE: deny-by-default, allowlists, quotas (timeout, max output), cwd restriction; enforce sensible defaults (e.g., bounded timeout/output) or fail if absent. Suggested defaults: timeout <= 10s, kill_grace <= 5s, max_output <= 1MB, cwd pinned to workspace root.
- Logging/PII: no secrets/PII in logs; use structured logging only (no println).
- Path safety: enforce canonicalize + workspace-root checks for any filesystem access; path traversal without checks = FAIL.
- Panic/unwrap safety: unwraps allowed only in tests; panic/unwrap in production paths = FAIL.
- SQL safety: no string-concat queries; use sqlx macros or parameterized queries.
- Build hygiene: flag large/untracked build artifacts or missing .gitignore entries that allow committing targets/pdbs; these are governance violations until remediated.
- **Git Hygiene:**
    - **Strict:** "Dirty" git status (uncommitted changes) is a FAIL for final validation unless a **User Waiver** is explicitly recorded in the Work Packet.
    - **Artifacts:** FAIL if *ignored* build artifacts (e.g., `target/`, `node_modules/`) are tracked or committed.
    - **Scope:** Ensure changes are restricted to the WP's `IN_SCOPE_PATHS`.


7.1) Git & Build Hygiene Audit (execute when any build artifacts/.gitignore risk is suspected)
- Check .gitignore coverage for: target/, node_modules/, *.pdb, *.dSYM, .DS_Store, Thumbs.db. Missing entries = FAIL until added.
- Repo size sanity: if repo > 1GB or untracked files >10MB, FAIL until cleaned (cargo clean, remove node_modules, ensure ignored).
- Committed artifacts: fail if git ls-files surfaces target/, node_modules, *.pdb, *.dSYM.

## Waiver Protocol 
- When waivers are needed: justified unwrap/Value exceptions, unavoidable platform-specific code, deferred non-critical hygiene. SurrealDB-exclusive authority and the PostgreSQL/SQLite prohibition under CX-DBP-VAL-014 are not waivable by a portability or dual-backend rationale.
- Approval: MEDIUM/HIGH risk requires explicit user approval; LOW risk can be Coder + Validator with user visibility.
- Recording (in work packet under "WAIVERS GRANTED"): waiver ID/date, check waived, scope (per WP), justification, approver, expiry (e.g., Phase 1 completion or specific WP).
- Signature: every `WAIVERS GRANTED` entry MUST carry `SIGNATURE=` (alias `USER_SIGNATURE=`), a one-time operator signature `{username}{DDMMYYYYHHMM}` registered in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`. An entry without a valid registered signature is `UNSIGNED`; it never counts as ACTIVE and `NOT_RUN_WAIVED` is illegal on it ([VPX-006]).
- Waivers NOT allowed: spec regression, evidence mapping gaps, hard invariant violations, security gate violations, traceability removal, RCE guard removal.
- Absent waiver for a required check = FAIL. Expired waivers at phase boundary must be revalidated or removed.

## Escalation Protocol (Blocking paths)
- Incomplete work packet/spec regression: FAIL immediately; send to Orchestrator to fix packet/spec before validation continues.
- Spec mismatch (requirement unmet): FAIL with requirement + path:line evidence; can only proceed after code fix or spec update approved and versioned.
- Test flake/unreproducible failure: request full output as triage input; re-run in the validator's own governed session and target ([VPX-001]) and record each run as a proof record. If still inconsistent, FAIL and return to Coder to stabilize.
- Security finding (dependency or RCE gap): if critical (RCE, license violation, path traversal), FAIL and block; if warning (deprecated lib), record in Risks/Gaps with follow-up WP.

## Standard Command Set (run when applicable)
- Cargo clean: superseded for verdict purposes by [VPX-008]: do not clean or rebuild for a second MT at the same commit and feature set; clean only when a rebuild trigger exists (commit change, feature/env-knob change, dirty tree) or the packet names a clean-build artifact as an acceptance target. Bundle READY MTs at one commit into one build + one test invocation per [VPX-007].
- TEST_PLAN commands from the work packet (must be run or explicitly waived by the user)
- If applicable: run or verify at least one targeted test that would fail if the new logic is removed; note command/output.
- If a required check cannot be satisfied, obtain explicit user waiver and record it in the work packet and report; absent waiver = FAIL.

## Verdict (Binary)
- PASS: Every requirement mapped to evidence, hygiene clean, tests verified (or explicitly waived by user), DAL audit clean when applicable, heuristic/code-quality review acceptable, and phase-gate satisfied when progressing.
- FAIL: List missing evidence, failed audits, tests not run, or unmet phase-gate. No partial passes.

## Validator Completion Checklist (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- [ ] I listed the exact spec clauses reviewed, not just the feature name.
- [ ] I recorded file:line evidence for each clause under `CLAUSES_REVIEWED`.
- [ ] I separated automation proof from manual code/spec review in the report.
- [ ] I recorded any blocked or unproven claims under `NOT_PROVEN` instead of implying completion.
- [ ] I set split verdicts (`GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `SPEC_ALIGNMENT_VERDICT`, `ENVIRONMENT_VERDICT`) deliberately rather than collapsing them into one PASS.
- [ ] If I used `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN` is exactly `- NONE`.
- [ ] I compared against local `main` first (or documented why `origin/main` was needed instead).
- [ ] I performed an explicit heuristic-quality review and recorded residual risks instead of letting tests stand in for code judgment.
- [ ] I avoided stronger wording in chat/packet/audit than the split verdicts actually support.

## Operator UX: Explicit Verdict Line (HARD)
- When discussing a WP where the verdict is known, every Validator chat message MUST include an explicit single-line status near the top:
  - `VERDICT: PASS`, `VERDICT: FAIL`, or `VERDICT: ABANDONED`
- While validation is still in progress, use:
  - `VERDICT: PENDING`
- Do not require the Operator to infer the verdict from `NEXT_ACTION`, gate state, or prose.
- Strings like `accept`, `approved`, `technical pass`, or `looks good` are not legal verdicts.

## Validation Modes
- `Classical / Manual-Relay Validation`
  - This is the closure lane for non-orchestrator-managed work where the validator is operating from the regular validator checkout and owns governed validation end-to-end.
  - It may append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
- `Governed Validation`
  - This is the orchestrator-managed validator lane.
  - `WP Validator` is advisory only in this lane. It may inspect code/spec drift, challenge weak reasoning, append receipts/thread guidance, and hand off or block, but it does not own final merge-to-`main` authority.
  - `Integration Validator` is the governed closure authority in this lane. It may append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
  - Manual filesystem deletion remains forbidden.
- `External / Independent Revalidation (orchestrator-managed WPs only)`
  - This is an audit mode, not a second validator workflow and not the classical/manual-relay closure lane.
  - Required start sequence: read the Codex, this protocol and the assigned MT, then continue from the MT JSON status.
  - This mode may audit code, governance, and environment, but it MUST NOT:
    - run `validator-gate-*`
    - mutate closure state
    - append normal governed-lane closure artifacts
    - merge or authorize merge in place of the Classical Validator or Integration Validator
  - Default write target for this mode is a chat report or a clearly labeled external revalidation report, not the normal governed-lane closure path.

## External Validator Split Report Contract
- Governance target selection is derived from the packet-declared governance authority and workflow lane, not by assuming every case is a retired `role_orchestrator` surface.
- External/classical validator reports for orchestrator-managed WPs MUST use these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `CODE_VERDICT: PASS | FAIL | NOT_RUN`
  - `GOVERNANCE_VERDICT: PASS | FAIL | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
- `LEGAL_VERDICT` is the only legal top-line verdict field. `CODE_VERDICT`, `GOVERNANCE_VERDICT`, `ENVIRONMENT_VERDICT`, and `DISPOSITION` are split assessments/classifications only.
- If the validator is in the wrong checkout or cannot access the committed PREPARE worktree source of truth, classify that as `VALIDATION_CONTEXT: CONTEXT_MISMATCH`, keep the blocked assessment at `NOT_RUN`, and use `LEGAL_VERDICT: PENDING` until the validation is rerun from the correct governance context.
- A `CONTEXT_MISMATCH` is not, by itself, proof that the WP implementation failed.
- If computed policy reports `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED`, do not produce an external revalidation report for that packet. Treat it as a failed historical closure and request a new remediation WP variant instead.
- If the WP remains correct for its baseline but SPEC_TARGET evolved materially, keep the legal verdict in `PASS | FAIL | PENDING` and set `DISPOSITION: OUTDATED_ONLY`.
- If the lane is intentionally discarded instead of remediated or merged, use top-level `Verdict: ABANDONED`, set `DISPOSITION: ABANDONED`, and close through the governed `DONE_ABANDONED` path.
- `OUTDATED_ONLY` is a disposition, not a legal top-line verdict.

## Governed Split Verdict Contract (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- Governed validation reports appended under `## VALIDATION_REPORTS` MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, also append the universal completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, also append:
  - `MECHANICAL_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_RETENTION_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
- `LEGAL_VERDICT` remains the only legal top-line verdict field.
- `SPEC_ALIGNMENT_VERDICT` is not implied by passing tests or governance gates.
- If environment/tooling blocked full proof, reflect that explicitly with `ENVIRONMENT_VERDICT` and downgrade `SPEC_ALIGNMENT_VERDICT` rather than narrating a generic PASS.
- For governed PASS closure on this packet format, append `CLAUSES_REVIEWED` and `NOT_PROVEN` in the packet report itself; a standalone chat summary is insufficient.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V2`, also append:
  - `MAIN_BODY_GAPS:` with `- NONE` only when no unresolved main-body requirement remains
  - `QUALITY_RISKS:` with `- NONE` only when no material maintainability or heuristic-quality concern remains
- `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `QUALITY_RISKS` is exactly `- NONE`.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, also append:
  - `MAIN_BODY_GAPS:` with `- NONE` only when no unresolved main-body requirement remains
  - `QUALITY_RISKS:` with `- NONE` only when no material maintainability or heuristic-quality concern remains
  - `VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH`
  - `DIFF_ATTACK_SURFACES:` with at least one diff-derived failure surface
  - `INDEPENDENT_CHECKS_RUN:` with validator-owned checks not copied from coder evidence
  - `COUNTERFACTUAL_CHECKS:` with concrete code-path / symbol references, not just test names
  - `BOUNDARY_PROBES:` for interface / producer-consumer / storage / contract boundary checks
  - `NEGATIVE_PATH_CHECKS:` for invalid, missing, adversarial, or failure-path checks
  - `INDEPENDENT_FINDINGS:` with deliberate independent findings or baseline-confirmation notes
  - `RESIDUAL_UNCERTAINTY:` with explicit remaining uncertainty; `- NONE` is illegal for `VALIDATOR_RISK_TIER=HIGH`
- For `PACKET_FORMAT_VERSION >= 2026-04-01`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4` also appends:
  - `ANTI_VIBE_FINDINGS:` with `- NONE` only when the implementation is substantively grounded, not easy-surface or vibe-coded, and no shallow review concern remains inside signed scope
  - `SIGNED_SCOPE_DEBT:` with `- NONE` only when no signed-scope debt, cleanup IOU, or "fix later" residue was accepted
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, also append:
  - `PRIMITIVE_RETENTION_PROOF:` with concrete file:line or symbol evidence proving touched primitives remain present and callable after the change
  - `PRIMITIVE_RETENTION_GAPS:` with `- NONE` only when no primitive-retention gap remains inside signed scope
  - `SHARED_SURFACE_INTERACTION_CHECKS:` with concrete producer/consumer, registry, type, runtime, or contract interaction evidence across shared surfaces
  - `CURRENT_MAIN_INTERACTION_CHECKS:` with concrete current-`main` caller/consumer compatibility evidence against the packet diff
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, also append:
  - `DATA_CONTRACT_PROOF:` with concrete code, bound SurrealQL, typed `SCHEMAFULL` schema, authenticated record-user permission, SurrealKit rollout, or emitted-artifact evidence showing the packet was reviewed for SurrealDB-exclusive authority, LLM-first readability/parseability, and Loom-intertwined requirements
  - `DATA_CONTRACT_GAPS:` with `- NONE` only when no gap remains in those data-contract obligations inside signed scope
- `VALIDATOR_RISK_TIER` is validator-assigned and MUST NOT be lower than the packet `RISK_TIER`.
- `LEGAL_VERDICT=PASS` is legal only when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, and `COUNTERFACTUAL_CHECKS` are all present and non-empty.
- `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `QUALITY_RISKS` is exactly `- NONE`.
- For `PACKET_FORMAT_VERSION >= 2026-04-01`, `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are also exactly `- NONE`.
- For `PACKET_FORMAT_VERSION >= 2026-04-01`, `LEGAL_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `SPEC_ALIGNMENT_VERDICT=PASS` and `Verdict: PASS` are legal only when `PRIMITIVE_RETENTION_GAPS` is exactly `- NONE`.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS` all contain concrete code or symbol evidence.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `RISK_TIER=MEDIUM|HIGH` requires non-empty `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `MECHANICAL_TRACK_VERDICT=PASS` is legal only when `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `ENVIRONMENT_VERDICT`, `WORKFLOW_VALIDITY`, `SCOPE_VALIDITY`, `PROOF_COMPLETENESS`, `INTEGRATION_READINESS`, and `DOMAIN_GOAL_COMPLETION` are all in their PASS states.
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `SPEC_RETENTION_TRACK_VERDICT=PASS` is legal only when `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN`, `MAIN_BODY_GAPS`, and `PRIMITIVE_RETENTION_GAPS` are all exactly `- NONE`, and the report contains concrete `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, `CURRENT_MAIN_INTERACTION_CHECKS`, `SPEC_CLAUSE_MAP`, and `NEGATIVE_PROOF` evidence.
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `LEGAL_VERDICT=PASS` and `Verdict: PASS` are legal only when `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`.
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `LEGAL_VERDICT=PASS` is legal only when `DATA_CONTRACT_PROOF` is present and `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- If `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line verdict MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- `VALIDATOR_RISK_TIER=HIGH` requires at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- `VALIDATOR_RISK_TIER=MEDIUM|HIGH` requires at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- The lightest valid counterfactual step is still mandatory: one sentence per key changed code path in the form "if X were removed or altered, Y would break", where `X` names a concrete file, symbol, or code path.

Report fields must use bare `FIELD: VALUE` format (no markdown bullet prefix). Both formats are parsed correctly (per RGF-90), but bare format is preferred for consistency:

```text
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
Verdict: PASS
```

## Validation Gate Sequence (ONE REVIEW PAUSE; APPEND-FIRST)

The validation process MUST halt only at Gate 3 (final report presentation). All other gates are state recording/unlocks and must still be run in order.
(Legacy: `.GOV/reference/legacy/validator/VALIDATOR_GATES.json` is treated as a read-only archive for older sessions; new validations should not write to it.)

### Gate 1: WP APPEND (Records verdict; non-blocking)
1. Validator completes all checks and generates the full VALIDATION REPORT.
2. If verdict = PASS, before recording Gate 1 the Validator MUST update the WP closure state on the WP branch:
   - set work packet `**Status:** Done`
   - update `.GOV/roles_shared/records/TASK_BOARD.md` to `## Done` / `[MERGE_PENDING]` before merge, then `[VALIDATED]` only after main containment is verified
   - sync `.GOV/roles_shared/records/BUILD_ORDER.md` by hand
3. Validator appends the VALIDATION REPORT to the active official packet path (logical: `.GOV/work_packets/{WP_ID}/packet.md`; current physical: `.GOV/task_packets/{WP_ID}/packet.md`; legacy flat: `.GOV/task_packets/{WP_ID}.md`) (APPEND-ONLY per ).
4. Validator does **not** paste the full report to chat yet.

### Gate 2: COMMIT CLEARANCE (PASS only)
1. Only if verdict = PASS, Validator performs `git commit` on the WP branch and records the commit SHA.
   - PASS requirement: this commit MUST include the appended report plus the Task Board / packet / build-order closure updates and any required Base-WP activation-state fixes (`WP_TRACEABILITY_REGISTRY`, removal of stale STUB state) so the later merge + fast-forward exposes the validated WP state in every active worktree.
   - PASS requirement: after those closure updates and before merge, governance consistency is checked by reading the artifact (check script deleted 2026-09-23).

### Gate 3: FINAL REPORT PRESENTATION (Blocking; the only mechanical pause)
1. If verdict = FAIL or ABANDONED: run immediately after Gate 1, **before any remediation/discard begins**.
2. If verdict = PASS: run after Gate 2 and after the validation report append is committed (**right before merge to `main` / push of `main`**).
3. Validator **outputs the entire report to chat** using the Report Template.
4. **HALT.** Validator MUST NOT merge to `main` / push `main` (PASS) or authorize remediation/discard kickoff (FAIL/ABANDONED) until the user acknowledges.

### Gate 4: USER ACKNOWLEDGMENT (Unlock)
1. User explicitly acknowledges the report (e.g., "proceed", "approved", "continue").
2. If user requests changes or disputes findings -> return to validation, re-run checks, regenerate report.
3. PASS: Validator may merge the validated WP into `main`. Canonical integration push remains `main` only; backup pushes are allowed only to the matching backup branch for the current role or WP when preserving state.
4. FAIL: WP remains open for remediation (no merge/commit).

### Gate Commands
Removed 2026-09-23: the command surface was deleted with the governance harness.

**Violations:** Skipping Gate 1, committing without a PASS Gate 2, or merging to `main` / pushing `main` (PASS) / starting remediation or discard (FAIL/ABANDONED) without Gate 3+4 = PROTOCOL VIOLATION .

```
FLOW DIAGRAM:

  [Run all checks] --> [Generate Report Text]
                         |
                         v
                 GATE 1: APPEND TO WP (records verdict)
                         |
               +---------+----------+
               |                    |
             FAIL                 PASS
               |                    |
               v                    v
   GATE 3: SHOW REPORT IN CHAT   GATE 2: COMMIT CLEARANCE
               |                    |
               v                    v
             HALT              git commit (WP branch)
               |                    |
               v                    v
        GATE 4: ACKNOWLEDGE   GATE 3: SHOW REPORT IN CHAT
               |                    |
               v                    v
         remediation begins        HALT
                                    |
                                    v
                           GATE 4: ACKNOWLEDGE
                                    |
                                    v
                       merge to main / push main only
```

## Merge/Commit Authority (per Codex [CX-PROOF-002])
- After issuing PASS **and completing all validation gates**, the validator form that currently owns closure authority is responsible for the integration flow into `main`.
- Closure authority split:
  - `Classical Validator` owns merge/push authority for classical/manual-relay validation.
  - `Integration Validator` owns merge/push authority for orchestrator-managed validation unless the packet explicitly overrides it.
  - `WP Validator` never owns merge-to-`main` authority.
- Validator responsibilities after PASS:
  - merge the validated WP branch into `main`
  - commit any required closure-sync or conflict-resolution edits on `main`
  - ensure the canonical closed `[VALIDATED]` state lives on `main`
- Pre-merge governance gate (MANDATORY): before merging the WP branch into `main`, the Validator MUST confirm governance consistency on the closure branch (checked by reading the artifact; check script deleted 2026-09-23). Treat any activation-state drift (`WP_TRACEABILITY_REGISTRY`, Task Board STUB residue, stale build-order snapshot) as a merge-blocking failure, not post-merge cleanup.
- Coders must not merge their own work.
- Canonical push rule: only `main` is a canonical integration push target. Backup pushes to matching backup branches are allowed as safety copies, but they are not integration events.
- If a remote integration push is authorized, the Validator pushes `main` only after the merge is complete and `main` contains the final validated closure state.

## Post-Merge Cleanup (reduces branch confusion)
- Do NOT delete local WP branches or remote WP backup branches as routine cleanup.
- Any local or remote WP branch deletion requires explicit Operator approval naming the exact target(s).
- When deletion is approved (`approved` or `proceed`) for a presented exact action/target list, delete exactly those targets with plain `git branch -d` / `git push origin --delete`.

## Report Template
```
VALIDATION REPORT - {WP_ID}
Verdict: PASS | FAIL | NOT_PROVEN | OUTDATED_ONLY | ABANDONED | BLOCKED

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate on the committed handoff state; not tests): PASS | FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS | FAIL | NOT_RUN
- VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH
- GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED
- LEGAL_VERDICT: PASS | FAIL | PENDING
- SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED
- WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN
- SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN
- PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN
- INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN
- DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN
- MECHANICAL_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- SPEC_RETENTION_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH

Scope Inputs:
- Task Packet: resolved Work Packet path (`.GOV/work_packets/{WP_ID}/packet.md` logical; current physical `.GOV/task_packets/{WP_ID}/packet.md`; or legacy `.GOV/task_packets/{WP_ID}.md`) (status: {status})
- Spec: {spec version/anchors}

Files Checked:
- {list of every file inspected during validation}

CLAUSES_REVIEWED:
- {SPEC clause identifier/text fragment} -> {path:line evidence}
- When `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, use the exact clause text from `CLAUSE_CLOSURE_MATRIX` so the packet monitor and the report reconcile mechanically.

NOT_PROVEN:
- NONE
- {or list each unresolved clause/gap explicitly}

MAIN_BODY_GAPS:
- NONE
- {or list each unresolved main-body MUST/SHOULD gap explicitly}

QUALITY_RISKS:
- NONE
- {or list each maintainability / heuristic-quality risk explicitly}

DIFF_ATTACK_SURFACES:
- {diff-derived failure surface}

INDEPENDENT_CHECKS_RUN:
- {validator-owned check} => {observed result}

COUNTERFACTUAL_CHECKS:
- If `{path or symbol}` were removed or altered, {observable breakage / proof expectation} would break.

BOUNDARY_PROBES:
- {producer/consumer or boundary check the validator ran}

NEGATIVE_PATH_CHECKS:
- {invalid/missing/adversarial input or failure-path check}

INDEPENDENT_FINDINGS:
- {what the validator learned that was not copied from coder evidence}

RESIDUAL_UNCERTAINTY:
- {what still remains uncertain after review}

Findings:
- Requirement X: satisfied at {path:line}; evidence snippet...
- Hygiene: {clean | issues with details}
- Forbidden Patterns: {results of grep}
- Storage DAL Audit (if applicable): {results for CX-DBP-VAL-010..014}
- Architecture/RDD/LLM: {findings}
- Security/Red Team: {findings}

Tests:
- {command}: {pass/fail/not run + reason}
- Coverage note: {does disabling feature fail tests?}

Risks & Suggested Actions:
- {list any residual risk or missing coverage}
- {actionable steps for future work packets or immediate fixes}

Improvements & Future Proofing:
- {suggested improvements to the code or protocol observed during this audit}

Split-Verdict Rules:
- Use `SPEC_ALIGNMENT_VERDICT=PASS` only when every diff-scoped MUST/SHOULD clause claimed by DONE_MEANS + SPEC_ANCHOR is listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `NONE`.
- Use `HEURISTIC_REVIEW_VERDICT=PASS` only when `QUALITY_RISKS` is exactly `NONE`.
- Use `LEGAL_VERDICT=PASS` only when the report also records diff-derived attack surfaces, validator-owned independent checks, and concrete code-path counterfactuals.
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, declare both top-line tracks explicitly:
  - `MECHANICAL_TRACK_VERDICT` summarizes governance/tests/code review/environment/workflow completion.
  - `SPEC_RETENTION_TRACK_VERDICT` summarizes deep spec retention, primitive retention, shared-surface interaction, and current-`main` compatibility review.
- For that same packet family, `MECHANICAL_TRACK_VERDICT=PASS` is legal only when `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `ENVIRONMENT_VERDICT`, `WORKFLOW_VALIDITY`, `SCOPE_VALIDITY`, `PROOF_COMPLETENESS`, `INTEGRATION_READINESS`, and `DOMAIN_GOAL_COMPLETION` are all in their PASS states.
- For that same packet family, `SPEC_RETENTION_TRACK_VERDICT=PASS` is legal only when `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN`, `MAIN_BODY_GAPS`, and `PRIMITIVE_RETENTION_GAPS` are exactly `NONE`, and the report shows concrete `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, `CURRENT_MAIN_INTERACTION_CHECKS`, `SPEC_CLAUSE_MAP`, and `NEGATIVE_PROOF`.
- For that same packet family, `LEGAL_VERDICT=PASS` and top-level `Verdict: PASS` are legal only when both track verdicts are `PASS`.
- `VALIDATOR_RISK_TIER` is validator-assigned and must not downscope below the packet `RISK_TIER`.
- For `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- For `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, also reconcile the packet's live monitoring sections before PASS:
  - every `CLAUSE_CLOSURE_MATRIX` row must end `VALIDATOR_STATUS=CONFIRMED` (or `NOT_APPLICABLE`)
  - no row may remain `PENDING`
  - `SPEC_DEBT_STATUS` must be `OPEN_SPEC_DEBT=NO`, `BLOCKING_SPEC_DEBT=NO`, `DEBT_IDS=NONE`
- For `PACKET_FORMAT_VERSION >= 2026-03-16`, also inspect `SEMANTIC_PROOF_ASSETS` before PASS:
  - semantic tripwire tests must still target the landed contract
  - canonical contract examples must still match the emitted/consumed shape
  - each clause row must point to TESTS, EXAMPLES, or governed debt
- If tests pass but spec proof is incomplete, keep `TEST_VERDICT=PASS` and downgrade `SPEC_ALIGNMENT_VERDICT`.
- If the environment blocked full proof, record that in `ENVIRONMENT_VERDICT` instead of narrating an unconditional PASS.
 
Work Packet Update (APPEND-ONLY):
- MANDATORY APPEND: Every validation verdict (PASS/FAIL/ABANDONED) MUST be APPENDED to the end of the active official packet file (logical: `.GOV/work_packets/{WP_ID}/packet.md`; current physical: `.GOV/task_packets/{WP_ID}/packet.md`; legacy flat: `.GOV/task_packets/{WP_ID}.md`). OVERWRITING IS FORBIDDEN.
- CLOSURE REASONS: The append block MUST contain a "REASON FOR {VERDICT}" section explaining exactly why the WP was closed or failed, linking back to specific findings.
- STATUS + closure updates are PASS-gated: append the full Validation Report for PASS/FAIL/ABANDONED using the template below, but only after `verdict: PASS` may the Validator set work packet `**Status:** Done`, move TASK_BOARD to Done/Merge Pending, and sync BUILD_ORDER by hand. Promote to `Validated (PASS)` / `[VALIDATED]` only after main containment is real and recorded. **DO NOT OVERWRITE User Context or previous history [CX-010].**
- For non-PASS governed verdicts or `DISPOSITION=OUTDATED_ONLY|ABANDONED`, append the report but do not perform normal Done/Validated PASS closure updates on work packet/TASK_BOARD/BUILD_ORDER unless the governed lane explicitly records the non-PASS terminal closure path.
- TASK_BOARD update (merge-visible requirement): for PASS before merge, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` on the WP branch by editing the task board row by hand to `DONE_MERGE_PENDING`, and the closure commit MUST carry that update so merge truth is not overstated.
- TASK_BOARD update (post-merge requirement): after the approved closure commit is contained in local `main`, promote the entry by editing the task board row by hand to `DONE_VALIDATED`.
- TASK_BOARD update (on `main`): after merge, the canonical main-branch Task Board must already show the validated WP entry from that closure commit. Status-sync commits earlier in the WP lifecycle are separate and do not imply a verdict.
- Board consistency (on `main`): work packet `**Status:**` is source of truth; reconcile the Task Board to match packet reality before declaring PASS. Unresolved mismatch = FAIL pending correction.
- Activation consistency (merge-visible requirement): when validating an official packet, reconcile `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` and remove any stale `## Stub Backlog` entry for that Active Packet before merge; then update BUILD_ORDER by hand and check governance consistency by reading the artifacts so the official activation state is visible on `main` immediately after merge.
```

## Non-Negotiables
- Evidence over intuition; speculative language is prohibited .
- APPEND-ONLY WP HISTORY: Deleting or overwriting the status history in a Work Packet is a protocol violation. All verdicts must be appended.
- Automated review scripts are optional; manual evidence-based validation is required.
- If a check cannot be performed (env/tools unavailable), report as FAIL with reason - do not assume OK.
- No "pass with debt" for hard invariants, security, traceability, or spec alignment; either fix or obtain explicit user waiver per protocol.




## Phase bundle and leaf-surface rule [CX-913]

Retired 2026-09-24: topology ledger deleted with the harness.

## Spec-Realism Gate (mandatory enforcement before COMPLETED)

This role enforces the Spec-Realism Gate. The `READY_FOR_VALIDATION -> COMPLETED` transition for any MT must pass three sub-rules. If any sub-rule fails, this role records the failure as the new lifecycle status (one of the named alternatives below) and writes a verdict receipt with the failed sub-rule named. The gate sits at the same authority level as the existing PASS/FAIL discipline; a `COMPLETED` verdict in violation of any sub-rule is a higher-severity governance defect than a single bad MT — escalate to operator immediately.

**Sub-rule 1 — No deferred-live escape.** Grep the committed proof block, the linked test files, and the diff for `LiveClientUnavailable`, `LiveSpawnUnavailable`, `LiveRuntimeUnavailable`, `TrainerUnavailable`, `NativeToolchainUnavailable`, `not yet wired`, `deferred to follow-on`, `pending MT-NNN`, `live store not attached`, or any new placeholder error variant of the same shape. Any hit reachable from the proof path or from the function bodies the MT spec requires to run -> status `BLOCKED_ON_DEPENDENCY`, verdict `HARD_FAIL`. Name the missing dep in the verdict receipt.

**Sub-rule 2 — External-resource touch.** Read the MT contract's `owned_files` + `spec_anchors` + `implementation_notes`. For every external resource named — model artifact, SurrealDB table/record/field, HTTP endpoint, subprocess, file-format round-trip, OS-level surface, IPC channel actually routed to a running process — confirm at least one proof command touches the real resource. If the proof only touches mocks the implementer authored alongside the impl, status `NEEDS_EXTERNAL_RESOURCE`, verdict `HARD_FAIL`. Name the resource in the verdict receipt.

**Sub-rule 3 — Implementer did not self-certify.** Read `lifecycle.claimed_by` and the proposed `completed_by`. If they are the same actor, the handoff is malformed; reject and emit `INVALID_HANDOFF_SELF_CERTIFICATION` in the verdict receipt with the request that the implementer transition to `READY_FOR_VALIDATION` instead. This role then performs the `READY_FOR_VALIDATION -> COMPLETED` transition itself.

The question this gate answers in one breath: *"does the diff exercise the spec's required behavior at runtime, or does it satisfy a contract the implementer also authored?"* A passing answer is the first form. Anything in the second form is a sub-rule-1 or sub-rule-2 failure.

Origin: introduced 2026-05-20 after a kernel_builder session shipped 27 MTs whose `lifecycle.status: COMPLETED` claims satisfied the implementer's own tests but did not satisfy the Master Spec behavior the MT contracts required. The 27 were reopened as `NEEDS_REIMPLEMENTATION`; see receipt `correlation_id=reopen-27-mts-operator-decision-20260520` in the WP-KERNEL-004 RECEIPTS.jsonl. Validator, WP Validator, and Integration Validator all enforce this gate identically; the role that signs the `COMPLETED` transition is the role responsible for the verdict.
