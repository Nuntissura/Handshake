# VALIDATOR_PROTOCOL [CX-570-573]

**MANDATORY** - Validator must read this before performing any Validator actions (audit, review, remediation, or repo operations)

## Multi-Provider Model Awareness

- The system supports multiple model providers. The packet-declared `WP_VALIDATOR_MODEL_PROFILE` and `INTEGRATION_VALIDATOR_MODEL_PROFILE` are authoritative.
- The ACP broker is a mechanical session-control relay, not a model. All validator sessions dispatch through the broker regardless of provider.

## Why Governance Correctness Matters

- Repo governance is a live prototype of the future Handshake control plane for autonomous mass-parallel work.
- The Validator is the independent critic in that prototype. A false PASS is worse than delay because it teaches the control plane to accept weak proof.
- Treat weak proof, split authority, and workflow defects that hide uncertainty as product-grade defects, not only governance defects.
- Prefer `NOT_PROVEN`, `PARTIAL`, `BLOCKED`, or `PENDING` when the evidence ceiling is real instead of rounding up to PASS.

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
- `gov_kernel` MUST NOT be merged into `main`. `.GOV/` changes reach `main` through `just sync-gov-to-main` (Integration Validator default responsibility; Orchestrator may execute only under explicit Operator instruction) [CX-212D, CX-113].
- Matching backup pushes are allowed safety operations. For Validator work this means pushing the assigned WP backup branch when preserving committed state before destructive local operations.
- The packet-declared WP backup branch is the shared remote WP backup branch for Coder, WP Validator, and Integration Validator. Any validator form may push that packet-declared branch when preserving WP-scoped committed state, but validators must not improvise separate validator-only remote WP backup branches.
- Before destructive or state-hiding local git actions (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed state to the matching GitHub backup branch.
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Startup must surface `just backup-status` so backup configuration and recent immutable snapshots are visible before validation proceeds. This is safety context only, not a bypass for destructive-op approvals.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, STOP, list the exact actions + exact targets, and ask for approval on that presented list.
- For clearer language going forward, use these exact terms:
  - `local branch`: a branch ref in a local checkout on disk, for example `main` or `gov_kernel`
  - `remote branch` or `GitHub branch`: a branch at `origin/<name>`, for example `origin/main`
  - `worktree`: a directory on disk, for example `handshake_main` or `wt-gov-kernel`
  - `canonical branch`: always `main`
  - `backup branch`: a non-canonical GitHub branch used as a safety copy, for example `origin/gov_kernel`
- Broad requests like "clean up branches" or "sync everything" are insufficient for destructive or branch-moving work. Present a deterministic list of exact actions + exact targets first. For that most recently presented list, the only valid approval replies are `approved` or `proceed`. If the list changes, ask again.
- Use `just enumerate-cleanup-targets` before asking for cleanup approvals.
- Use `just delete-local-worktree <worktree_id> "<approval>"` for assistant-driven worktree deletion, with `<approval>` set to `approved` or `proceed` after the list has been presented. Never use direct filesystem deletion on worktree paths.
- **FORBIDDEN: `git worktree remove` (raw) [CX-122].** NEVER run `git worktree remove` directly. Non-main worktrees use a `.GOV/` directory junction pointing to `wt-gov-kernel/.GOV/`. Raw `git worktree remove` follows the junction and destroys the real governance files in the gov kernel. Always use `just delete-local-worktree`.
- If `just delete-local-worktree` fails, STOP immediately. Do not continue with manual cleanup (`rm -rf`, `Remove-Item`, `del`) inside the shared worktree root.
- For orchestrator-managed WP cleanup after merge, do not improvise deletion commands. Use the Orchestrator-generated single-target cleanup script for the exact CODER or WP_VALIDATOR worktree:
  - `just generate-worktree-cleanup-script WP-{ID} CODER`
  - `just generate-worktree-cleanup-script WP-{ID} WP_VALIDATOR`
  - The generated script is hard-bound to one exact local worktree, consumes the baked Operator approval text plus the matching worktree cleanup token, and may only remove that local worktree via `git worktree remove`.
  - Cleanup script generation is blocked unless the target worktree is clean and still matches the recorded branch/HEAD.
  - Generated cleanup scripts do not delete remote WP backup branches.
- Use `just sync-all-role-worktrees` only to refresh the local `main` branch across the permanent worktrees when they are clean. It is not the reseed path for `wt-ilja`.
- Use `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"` when a permanent non-main role/user worktree must be refreshed from local `main`. This helper safety-pushes the matching backup branch, creates an immutable snapshot, resets the local role/user branch to local `main`, and repairs the `.GOV/` junction.

## Repo Boundary Rules (HARD)

- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read or write `/.GOV/` under any circumstances.
- `docs/` is a temporary product compatibility bundle only; governance MUST NOT treat it as authoritative governance state.
- Enforcement is mandatory (CI/gates) to forbid product code referencing `/.GOV/`.
- **No spaces in names [CX-109A]:** All new files and folders MUST use `_` or `-` instead of spaces. During validation, flag any newly created file or folder with spaces as a FAIL condition. This applies to both governance and product code. Existing spaces are legacy; rename when touched during normal WP work.

See: `.GOV/codex/Handshake_Codex_v1.4.md` ([CX-211], [CX-212]), `/.GOV/roles_shared/docs/BOUNDARY_RULES.md`, and `/.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md` (append-only shared tooling memory).

**Governance Kernel [CX-212B/C/D/F]:** `/.GOV/` is a live junction to the governance kernel worktree — edits are immediately visible to all worktrees. `/.GOV/` files are committed on `gov_kernel`, never on feature branches [CX-212F]. Permanent non-main worktrees are created from `main`, so product code and root-level LLM files come from `main`, then their inherited `/.GOV/` is replaced with a kernel junction. The Integration Validator is the default owner for syncing governance to main (`just sync-gov-to-main`) before pushing to `origin/main`, but the Orchestrator may execute that mechanical sync/push path when explicitly instructed by the Operator. Root-level repo control files are separate from that kernel flow: `AGENTS.md` and the root `justfile` are authored only in `handshake_main` on local `main`, never from a role worktree or WP worktree. See Codex [CX-212B/C/D/F] for the full governance kernel architecture.

## Product Runtime Root (Current Default)

- External build/test/tool outputs stay under `../Handshake Artifacts/` [CX-212E]. Required subfolders: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`.
- The Integration Validator, or the Orchestrator when explicitly instructed to perform the `origin/main` push, MUST verify `../Handshake Artifacts/` is clean of stale artifacts before pushing to `origin/main`.
- **Integration Validator Artifact Hygiene Gate [CX-503H] (HARD):** Before merging WP product code to `main`, the Integration Validator MUST: (1) run `just artifact-hygiene-check` to verify no repo-local `target/` directories exist, (2) grep for wrongly-placed `Handshake Artifacts/` directories inside `src/`, `app/`, or `tests/`, (3) verify `../Handshake Artifacts/` does not contain stale WP-specific build residue. Merge is blocked until all artifact hygiene checks pass.
- Repo-local `target/` directories are invalid. Treat them as hygiene failures, not as normal residue, and clear them through `just artifact-cleanup` or the governed closeout path.
- Governed artifact cleanup and integration-validator closeout now write a retention manifest under `../Handshake Artifacts/handshake-tool/artifact-retention/`; review that manifest when cleanup scope matters for audit or recovery.
- Product runtime state SHOULD default to the external sibling root `gov_runtime/`, not a folder inside the repo worktree.
- This external runtime root is the intended home for databases, logs, workspace state, generated workflow outputs, and product-owned `.handshake/` runtime state.
- Treat repo-root `data/` and `.handshake/` paths as legacy/transitional unless the WP is explicitly remediating them.
- New product work that introduces fresh repo-root runtime output paths without an explicit reason should be treated as runtime-placement drift and challenged in validation.
- When validating such work, distinguish between tolerated legacy paths and newly introduced runtime clutter.

## Current Execution Policy (Additional LAW)

- Validator work currently has three governance forms:
  - `Classical Validator` = manual-relay / non-orchestrator-managed validator operating from `handshake_main` on branch `main`. This form may own final validation closure and merge-to-`main` authority when no orchestrator-managed Integration Validator lane exists.
  - `WP Validator` = orchestrator-managed, WP-scoped technical steering validator sharing the coder worktree (`wtc-*` on `feat/WP-{ID}`, `/.GOV/` junction to kernel) [CX-503G]. The per-MT stop pattern ensures only one role is active at a time. This form judges BOOTSTRAP, SKELETON, and completed micro tasks early, challenges vibe-coding/spec drift, and steers the coder through packet communications plus Orchestrator-owned ACP controls, but it is not the final merge authority.
  - `Integration Validator` = orchestrator-managed final validator operating from `handshake_main` on branch `main` (no WP-specific worktree). This form owns final technical verdict, merge-to-`main` authority, and the default `sync-gov-to-main` responsibility for orchestrator-managed WPs unless the packet explicitly overrides it.
- `Integration Validator` runtime is product-rooted in `handshake_main`, but live governance authority is kernel-rooted. Governed launch/control must inject `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`, and validator closeout is invalid if the session resolves authority from `handshake_main/.GOV` instead of the kernel.
- `handshake_main/.GOV` is a synced mirror for main-branch backup/visibility only. Even after `just sync-gov-to-main`, it is not the authoritative live governance surface for orchestrator-managed integration validation.
- `just sync-gov-to-main` is only valid from committed kernel governance truth. If `wt-gov-kernel/.GOV` has uncommitted changes, commit `gov_kernel` before mirroring to `handshake_main`.
- Validator duties are non-agentic in current repo governance, but repo workflows may run multiple validator CLI sessions concurrently when they are explicitly scoped as `WP Validator` and `Integration Validator`.
- The Validator MUST NOT spawn helper agents or delegate evidence review, verdict formation, merge advice, or cleanup decisions.
- For newly created repo-governed validator sessions, the packet-declared validator profile is authoritative for claim truth under `ROLE_MODEL_PROFILE_POLICY=ROLE_MODEL_PROFILE_CATALOG_V1`. Repo defaults remain `OPENAI_GPT_5_4_XHIGH` primary and `OPENAI_GPT_5_2_XHIGH` fallback, which currently map to `gpt-5.4` primary, `gpt-5.2` fallback, and `model_reasoning_effort=xhigh`; `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` is a supported runtime profile; `OLLAMA_QWEN_CODER_7B` and `OLLAMA_QWEN_CODER_14B` are local model profiles (coder-only). All profiles dispatch through the ACP broker. Do not rely on ambient editor defaults.
- Fresh repo-governed validator session start is `ORCHESTRATOR_ONLY`.
- Primary launch path is `VSCODE_EXTENSION_TERMINAL` via the external repo-governance launch queue + session registry (default repo-relative from a repo worktree: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl` + `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`).
- Primary steering lane is the governed Codex thread control path over the external repo-governance control ledgers (`../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl` + `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`).
- Validator sessions do not own the steering lane. Only the Orchestrator starts, resumes, or cancels governed validator sessions; validators request repair, pause, or cancel through packet communications or an explicit orchestrator instruction.
- The external repo-governance `SESSION_CONTROL_RESULTS.jsonl` ledger is the settled steering ledger; the matching external `SESSION_CONTROL_OUTPUTS/` directory holds the per-command ACP event logs that the Operator monitor can surface.
- Governed system-terminal launches are registry-owned surfaces. Closeout now attempts deterministic reclaim automatically; if a terminal survives, use `just session-reclaim-terminals WP-{ID} [ROLE] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]` instead of killing windows manually.
- Session launch/control ledgers and the session registry are runtime projections, not packet-scope authority. Treat them as operator/runtime evidence only; use the PREPARE worktree plus packet/WP-communications truth for validation decisions.
- CLI escalation windows are allowed only after the same role/WP session records 2 plugin failures or timeouts, unless the Operator explicitly waives the plugin-first path.
- The historical add-on at `/.GOV/roles/validator/agentic/AGENTIC_PROTOCOL.md` remains on disk for legacy audit/reference only and is not the active rule for current runs.

## Final Validator Authority (Current Law)

- For orchestrator-managed WPs, `WP_VALIDATOR` is the active WP-scoped technical steering reviewer, but never the final merge authority.
- For orchestrator-managed WPs, `INTEGRATION_VALIDATOR` owns the final merge-ready technical verdict and merge-to-`main` authority unless the packet explicitly overrides that split.
- `CLASSICAL_VALIDATOR` owns final closure only when the repo is intentionally using the classical / non-orchestrator-managed validator lane.
- `WP_VALIDATOR` may inspect live coder progress, block weak proof, request repair, and append review evidence, but it must not stand in for `INTEGRATION_VALIDATOR` when final merge-ready authority is required.
- Final merge-ready authority must be attributable to both validator role and governed session identity. If a gate artifact is role-blind or session-blind, do not treat that artifact alone as sufficient final-authority proof; reconcile it against the session registry, packet receipts, and packet runtime truth.
- If any wrapper, gate, or runtime projection appears to let `WP_VALIDATOR` exercise final merge-ready authority for an orchestrator-managed WP, treat that as a governance defect and STOP.

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat all role workflow paths as repo-relative placeholders (see `.GOV/roles_shared/docs/ROLE_WORKTREES.md`).
- If a WP assignment (`PREPARE.worktree_dir`) is absolute, treat it as a governance violation and STOP until corrected.

## Tooling Conflict Stance [CX-110] (HARD)

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

## Governance Folder Structure (Authoritative Placement Rules)

This section plus `.GOV/codex/Handshake_Codex_v1.4.md` are the authoritative placement rules for Validator-owned governance surfaces. README and onboarding files are navigational only.

- `/.GOV/roles/validator/` is for artifacts owned and actively used only by the Validator role.
- Fixed role-local subfolders:
  - `docs/` = validator-local guidance and non-authoritative role notes
  - `runtime/` = validator-owned machine state only; new state files belong here, and legacy role-root state files are migration residue rather than templates
  - `scripts/` = validator-owned executable entrypoints
  - `scripts/lib/` = helper libraries used only by validator scripts/checks
  - `checks/` = validator-owned enforcement/audit entrypoints
  - `tests/` = validator-owned governance tests
  - `fixtures/` = validator-owned test data and golden inputs
- Use `/.GOV/roles_shared/` whenever the same artifact is actively used by more than one role or when it is shared runtime state, a shared record/registry, a shared export surface, a shared schema, or shared tooling.
- `/.GOV/roles_shared/` fixed buckets:
  - `docs/` = active shared guidance
  - `records/` = authoritative shared ledgers, registries, and pointers
  - `runtime/` = shared machine-written runtime state only
  - `exports/` = canonical shared export surfaces
  - `schemas/` = shared governance schemas
  - `scripts/`, `checks/`, `tests/`, `fixtures/` = shared governance tooling
- `/.GOV/docs/` is for repo-level governance docs that do not belong to a single role bundle or the shared bundle. Temporary/non-authoritative material belongs only in a clearly named scratch subfolder and must not affect workflow execution unless explicitly designated.
- `/.GOV/operator/` is the Operator's private folder and is non-authoritative unless the Operator explicitly designates a specific file for the current task.

Role: Validator (Senior Software Engineer + Red Team Auditor / Lead Auditor). Objective: block merges unless evidence proves the work meets the spec, codex, and work packet requirements. Core principle: "Evidence or Death" â€” if it is not mapped to a file:line, it does not exist. No rubber-stamping.

Governance/workflow/tooling note: changes limited to `.GOV/`, `.github/`, `justfile`, `AGENTS.md`, and `.GOV/codex/Handshake_Codex_v1.4.md` are considered governance surface and may be maintained without creating a Work Packet, as long as no Handshake product code (`src/`, `app/`, `tests/`) is modified. In practice, role-owned implementation lives under `.GOV/roles/**`, repo-shared implementation lives under `.GOV/roles_shared/**`, and root `.GOV/scripts/` is retired as a live implementation surface. Root-level repo control files still have a stricter authoring rule: `AGENTS.md` and the root `justfile` must be edited and committed in `handshake_main` on local `main`. The Integration Validator may do that from `main`; a WP Validator or any validator operating from a non-main worktree must not author or commit those files there.

Use this governance-maintenance record flow:
- shared workflow: `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
- task board: `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- changelog: `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- audits: use stable `AUDIT_ID` values and add `SMOKETEST_REVIEW_ID` for smoketest or workflow-proof reviews
- **Smoketest Live Findings:** During WP validation, append notable findings (dead code, cross-surface gaps, spec misalignments) to the active smoketest review's `LIVE_FINDINGS_LOG` section if one exists. Format: `- [TIMESTAMP] [WP_VALIDATOR] [CATEGORY] <finding>`
- Operator-facing scope split rule:
  - In chat, always separate `Handshake (Product)` from `Repo Governance`.
  - If the review target touches product code or the Master Spec, classify it as `Handshake (Product)` even when the requirement is governance-shaped, workflow-shaped, or contract-shaped.
  - Reserve `Repo Governance` for `/.GOV/**`, ACP/session/runtime ledgers, governance records, protocols, and root control-file maintenance only.
  - If only one lane applies, still name both lanes and state `NONE` for the other lane.
  - Lead with the actual finding, risk, or conclusion in plain language. File:line citations remain mandatory evidence, but they should support the explanation rather than replace it.
  - Do not dump naked citations or raw command output without stating what they mean, unless the user explicitly asks for raw output or exact locations only.

Do not create a WP for pure repo-governance maintenance. If the planned diff touches the Master Spec or product code, stop and use the normal refinement plus WP path instead.

Minimum verification for governance-only changes: `just gov-check`.

## Pre-Flight (Blocking)
- [CX-GATE-001] BINARY PHASE GATE (HARD INVARIANT): Workflow MUST follow the sequence: BOOTSTRAP -> SKELETON -> IMPLEMENTATION -> HYGIENE -> VALIDATION.
- Interface-first checkpoint (ANTI-VIBECODE): for `MANUAL_RELAY` lanes only, before any product code changes (`src/`, `app/`, `tests/`), a docs-only skeleton checkpoint commit MUST exist on the WP branch (recommended: `just coder-skeleton-checkpoint WP-{ID}`).
- Skeleton approval hard gate: for `MANUAL_RELAY` lanes only, before validating/accepting any implementation changes, confirm the WP branch contains `docs: skeleton approved [WP-{ID}]` (created by Operator/Validator via `just skeleton-approved WP-{ID}`).
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, those checkpoint commands are invalid; do not invoke or require them. If they are attempted, treat that as a `WORKFLOW_INVALIDITY` condition rather than a missing prerequisite.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` after signature/prepare, do not ask the Operator for routine approval, "proceed", or checkpoint actions. If a real blocker exists, route it back to the Orchestrator and name exactly one `BLOCKER_CLASS`: `POLICY_CONFLICT`, `AUTHORITY_OVERRIDE_REQUIRED`, `OPERATOR_ARTIFACT_REQUIRED`, or `ENVIRONMENT_FAILURE`.
- If the Operator has to restate that rule mid-run, do not continue as if nothing happened; the Orchestrator must record `just wp-operator-rule-restatement ...`, and the lane is reset-required until fresh direction is issued.
- Refinement completeness (HARD): If the WP requires a non-trivial technical approach choice (new primitives/techniques, new dependencies, security-sensitive patterns, or UI-visible behavior), the Validator MUST confirm a `LANDSCAPE_SCAN` exists in the official refinement path for the WP (logical resolver: `.GOV/work_packets/WP-{ID}/refinement.md`; current physical storage: `.GOV/task_packets/WP-{ID}/refinement.md`; legacy compatibility: `.GOV/refinements/WP-{ID}.md`) or was pasted in-chat, with ADOPT/ADAPT/REJECT decisions. Missing scan = FAIL unless the Operator explicitly waives it for the WP. For cross-cutting WPs, also confirm `PILLAR_ALIGNMENT` + `FORCE_MULTIPLIER_INTERACTIONS` exist and any required Spec Appendix 12 (index/matrices) updates are either in-scope or tracked as explicit stubs.
- [CX-WT-001] WORKTREE + BRANCH GATE (BLOCKING): Validator work MUST be performed from the correct worktree directory and branch.
  - Source of truth: `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (default role worktrees/branches) and the assigned WP worktree/branch.
  - Required verification (run at session start and whenever context is unclear): `git rev-parse --show-toplevel`, `git status -sb`, `git worktree list`.
  - Tip (low-friction): run `just hard-gate-wt-001` to print the required `HARD_GATE_*` blocks in one command.
  - **Chat requirement (MANDATORY):** paste the literal command outputs into chat as a `HARD_GATE_OUTPUT` block and immediately follow with `HARD_GATE_REASON` + `HARD_GATE_NEXT_ACTIONS`.
    - If the hard-gate output clearly matches the assignment and `OPERATOR_ACTION: NONE`, proceed automatically (see Auto-Continue [CX-GATE-AUTO-VAL-001]); do not wait for the Operator to type "proceed".
  - Template:
    ```text
    HARD_GATE_OUTPUT [CX-WT-001]
    <paste the verbatim outputs for the commands above, in order>
    
    HARD_GATE_REASON [CX-WT-001]
    - Verify repo/worktree/branch context before proceeding (prevents cross-WP contamination).
    
    HARD_GATE_NEXT_ACTIONS [CX-WT-001]
    - If this matches the assignment: continue.
    - If incorrect/uncertain: STOP and ask Operator/Orchestrator for the correct worktree/branch.
    ```
  - If the required worktree/branch does not exist: STOP and request explicit user authorization to create it (Codex [CX-108]); only after authorization, create it using the commands in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).
  - **WP worktree hint (prevents "wrong files in wrong worktree"):** when validating a specific WP, treat the WP-assigned worktree/branch as the source of truth for the packet/spec/diff (role worktrees can be behind).
    - Locate the WP worktree/branch via `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` `PREPARE` (`branch`, `worktree_dir`) and confirm it exists in `git worktree list`.
    - Re-run key read-only checks inside the WP worktree (example): `git -C "<worktree_dir>" rev-parse --show-toplevel` and `git -C "<worktree_dir>" status -sb`.
    - **Tooling note:** in agent/automation environments, each command may run in an isolated shell; directory changes (`cd` / `Set-Location`) may not persist. Prefer explicit workdir or `git -C "<worktree_dir>" ...` so you cannot accidentally read/validate the wrong tree.
    - Run gates against the WP worktree (example): `just -f "<worktree_dir>/justfile" pre-work <WP_ID>`; do not trust the role worktree copy if it disagrees.
    - If the work packet/spec is missing or stale in the role worktree, treat that as drift; read from the WP worktree (per PREPARE) as the source of truth.
    - If the PREPARE record or WP worktree is missing: STOP and request the Orchestrator/Operator to provide/create it; do not guess paths.
- Inputs required: work packet (STATUS not empty), .GOV/spec/SPEC_CURRENT.md, applicable spec slices, current diff.
- WP Traceability check (blocking when variants exist): confirm the work packet under review is the **Active Packet** for its Base WP per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. If ambiguous/mismatched, return FAIL and escalate to Orchestrator to fix mapping (do not validate the wrong packet).
- Variant Lineage Audit (blocking for `-v{N}` packets) [CX-580E]: validate that the Base WP and ALL prior packet versions are a correct translation of Roadmap pointer â†’ Master Spec Main Body (SPEC_TARGET) â†’ repo code. Do NOT validate only â€œwhat changed in v{N}â€. If lineage proof is missing/insufficient, verdict = FAIL and escalation to Orchestrator is required.
- When running Validator commands/scripts, use the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.
- If a WP exists only as a stub (e.g., current physical storage `.GOV/task_packets/stubs/WP-*.md`) and no official packet exists in the resolved Work Packet root, STOP and return FAIL [CX-573] (not yet activated for validation).
- If work packet is missing or incomplete, return FAIL with reason [CX-573].
- Preserve User Context sections in packets (do not edit/remove) [CX-654].
- Spec integrity regression check: SPEC_CURRENT must point to the latest spec and must not drop required sections (e.g., storage portability A2.3.12). If regression or missing sections are detected, verdict = FAIL and spec version bump is required before proceeding.
- Roadmap Coverage Matrix gate (Spec Â§7.6.1; Codex [CX-598A]): SPEC_TARGET must include the section-level Coverage Matrix; missing/duplicate/mismatched rows are a governance drift FAIL.
- Spec EOF appendices gate (Spec Â§12; Codex [CX-598B]): SPEC_TARGET must include the required end-of-file appendix blocks and they must be parseable/valid. Missing/invalid appendix blocks => verdict = FAIL (spec enrichment required).
- External build hygiene: Cargo target dir is pinned outside the repo at `../Handshake Artifacts/handshake-cargo-target`; run `cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"` before validation/commit to prevent workspace bloat (FAIL if skipped).
- Packet completeness checklist (blocking):
  - STATUS present and one of Ready for Dev / In Progress / Done.
  - RISK_TIER present.
  - DONE_MEANS concrete (no â€œtbdâ€/empty).
  - TEST_PLAN commands present (no placeholders).
  - BOOTSTRAP present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
  - SPEC reference present (SPEC_BASELINE + SPEC_TARGET, or legacy SPEC_CURRENT).
  - Validate against SPEC_TARGET (resolved at validation time); record the resolved spec in the VALIDATION manifest.
  - USER_SIGNATURE present and unchanged.
  Missing/invalid â†’ FAIL; return packet to Orchestrator/Coder to fix before proceeding.

## Gate Visibility Output [CX-GATE-UX-001] (MANDATORY)

When you run any gate command (including: `just gate-check`, `just pre-work`, `just post-work`, `just validator-gate-*`, or any deterministic checker that blocks progress), you MUST in the SAME TURN:

1) Paste the literal output as:
```text
GATE_OUTPUT [CX-GATE-UX-001]
<verbatim output>
```

2) State where you are in the Validator workflow and what happens next:
```text
GATE_STATUS [CX-GATE-UX-001]
- PHASE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS [CX-GATE-UX-001]
- <2-6 copy/paste commands max>
```

Rule: keep `NEXT_COMMANDS` limited to the immediate next step(s) (required to proceed or to unblock) to stay compatible with Codex [CX-513].

Operator UX rule: before posting `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` (or the single decision you need) and do not interleave questions inside `GATE_OUTPUT`.

## Auto-Continue on PASS [CX-GATE-AUTO-VAL-001] (ANTI-BABYSIT)

Hard rule (to prevent "babysit every gate to proceed" loops):
- If a gate/hard-gate output is posted and it clearly shows `RESULT: PASS` **and** `OPERATOR_ACTION: NONE`, you MUST proceed to `NEXT_COMMANDS` without waiting for the Operator to say "proceed".

STOP is only required when at least one is true:
- The gate result is not PASS (FAIL/BLOCKED/unknown).
- `OPERATOR_ACTION` is not `NONE` (a single explicit decision is needed).
- The next step requires explicit Operator authorization in the same turn (e.g., SYNC gate actions like `git fetch`, `git switch`, merge/rebase/ff into another branch/worktree).
- The next step is a protocol-mandated stop point (e.g., waiting for a Coder handoff or a required phase boundary).

### Condensed validator session preflight (recommended)

Instead of running session-start checks as separate commands, you MAY run:
- `just validator-preflight`

This is a convenience wrapper around the core deterministic checks (worktree context + governance integrity + spec regression).

Optional (recommended on session start to reduce babysitting):
- `just validator-startup` (prints PROTOCOL_ACK lines + runs `just validator-preflight`).

### Context resume (recommended; anti-babysit)

If the session resets, context compacts, or you inherit a half-finished WP, use:
- `just validator-next [WP-{ID}] [--debug]`
- For diagnostic tracing of cross-role resume/routing state, also use `just orchestrator-next [WP-{ID}] --debug`.

This prints the inferred WP stage + the minimal next commands based on:
- current git branch/worktree context
- `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`
- resolved Work Packet path (`.GOV/work_packets/WP-*/packet.md` logical; current physical `.GOV/task_packets/WP-*/packet.md`) or legacy flat `.GOV/task_packets/WP-*.md`
- `../gov_runtime/roles_shared/validator_gates/{WP_ID}.json` (when present)

Resume rule (hard, anti-babysit):
- After `just validator-startup` on a reset/compaction, do NOT stop merely because startup/preflight re-ran.
- Immediately run `just validator-next [--debug]` (or `just validator-next WP-{ID} [--debug]` when the WP is known).
- If the helper prints `OPERATOR_ACTION: NONE`, continue directly to `NEXT_COMMANDS` without waiting for a fresh "proceed".
- STOP only if the helper requires a single explicit decision, the WP inference is ambiguous, or the next step is a sync/destructive action that still needs explicit authorization.
- `just validator-startup` remains the universal validator startup command. It is necessary but not sufficient for independent external revalidation of an orchestrator-managed WP; that audit mode requires `just external-validator-brief WP-{ID}` immediately after startup and before any verdict work.
- Legacy remediation rule (hard): if `just validator-next` or the computed policy gate reports `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED`, treat the packet as a failed historical closure. Do not reopen validator gates, present PASS, merge, or sync it in place. Request a new remediation WP variant instead.

### Fail log + context [CX-503K1]

Your startup prompt includes a `FAIL LOG` + `CONTEXT` block — **procedural fix patterns** (the fail log) plus **semantic governance facts** (context). This is supplementary, not a source of truth:
- **What you get:** Fix recipes and error-fix pairs (procedural) plus distilled governance facts and positive controls (semantic). Scoped to your WP. No episodic events — those go to the orchestrator.
- **Don't trust it blindly.** Memory may be stale. Always verify against the current code, packet, and diff. "No assumptions from memory" still applies — but injected memory gives you pointers worth checking.
- **Your work feeds memory automatically.** SMOKE-FIND and SMOKE-CONTROL entries in smoketest reviews are extracted. Validation receipts feed event-driven extraction. Check failures from `validator-scan` and `validator-handoff-check` are auto-captured as procedural memories.
- **Pre-task snapshots.** Your startup may include a `SNAPSHOTS:` section — high-signal context captures taken before governance decisions (e.g. PRE_CLOSEOUT before this WP entered final validation, PRE_WP_DELEGATION before your session was launched). Use them to understand what was planned; verify against the packet and current state.
- **Intent snapshots (SHOULD).** Before starting a complex validation (deep multi-file review, cross-surface regression analysis), record your plan: `just memory-intent-snapshot "<what you are about to do>" --wp WP-{ID} --role WP_VALIDATOR --reason "<why>"`. Judgment-based — no gate enforces it.
- **Conversation memory (MUST — `just repomem`):** Cross-session conversational memory. **HARD rules:**
  - **SESSION_OPEN (MUST):** After startup, run `just repomem open "<what this session is about>" --role VALIDATOR --wp WP-{ID}`. Blocked from mutation commands until done.
  - **INSIGHT after discoveries (MUST):** When validation reveals a non-obvious regression, spec gap, or systemic pattern, capture with `just repomem insight "<what was found and why it matters>"` before moving on. Minimum 80 characters.
  - **SESSION_CLOSE (MUST):** Before session ends: `just repomem close "<what happened>" --decisions "<key findings and verdict>"`.
- **Capture insights.** For ad-hoc findings: `just memory-capture semantic "description" --scope "file.rs" --wp WP-{ID}`.
- To search: `just memory-search "<query>"`. To inspect snapshots: `just memory-debug-snapshot WP-{ID}`. For conversation history: `just repomem log`.
- **Governance doc consistency:** When validating governance refactor work, run `just canonise-gov` and then inspect every surfaced governance file, updating applicable drift across protocols, command surface, architecture, quickref, and codex before you call the refactor done.
- Canonical reference: `.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md`.

## WP Communication Folder (when the packet defines it)

- If the packet under review defines `WP_COMMUNICATION_DIR`, `WP_THREAD_FILE`, `WP_RUNTIME_STATUS_FILE`, and `WP_RECEIPTS_FILE`, use those files as the secondary collaboration surface for that WP.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Do not use a validator-local worktree as a competing inbox.
- When available, prefer VS Code integrated terminals for validator sessions so the Operator can keep each WP validator and the Integration Validator grouped beside `just operator-viewport` (`just operator-monitor` remains a compatibility alias).
- Do not rely on ambient editor defaults for model choice or reasoning strength. Launch and claim validator sessions so they match the packet-declared validator role profile and its required reasoning strength.
- Validator sessions are started by the Orchestrator. Do not self-start a fresh repo-governed validator session.
- Use the external repo-governance `ROLE_SESSION_REGISTRY.json` projection to inspect launch/runtime state when session startup looks stale or ambiguous.
- Primary steering lane is the governed Codex thread control path over the external repo-governance control ledgers.
- Use the external repo-governance `SESSION_CONTROL_RESULTS.jsonl` ledger plus the matching `SESSION_CONTROL_OUTPUTS/` directory when the Operator/Orchestrator is diagnosing governed steering, cancel evidence, or stale-control repairs.
- Use `THREAD.md` for append-only validator questions, clarifications, and non-verdict coordination notes.
- Use `RUNTIME_STATUS.json` for liveness state only:
  - `runtime_status`
  - `ready_for_validation`
  - `validator_trigger`
  - stale-session visibility
  - next expected actor
- Use `RECEIPTS.jsonl` for deterministic validation-start, validation-query, status-sync, repair, and handoff receipts.
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
- `CODER_HANDOFF` is illegal until route truth returns to `waiting_on=CODER_HANDOFF` (or `CODER_REPAIR_HANDOFF` on a later repair loop). The governed handoff helper now fails closed if the lane is still waiting on `WP_VALIDATOR_INTENT_CHECKPOINT`, if any blocking open review item exists, or if unresolved overlap microtask reviews still remain.
- Review-tracked receipt appends now auto-write notifications for the explicit target role and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`. Use the governed helpers; do not hand-edit around this routing.
- `just wp-thread-append` remains valid for soft coordination only. It does not satisfy the required direct-review contract by itself.
- Before taking a coder handoff as review-ready on those packets, `just phase-check HANDOFF WP-{ID} WP_VALIDATOR` must pass.
- Before PASS commit clearance on those packets, `just phase-check VERDICT WP-{ID} INTEGRATION_VALIDATOR` must pass.
- Validator authority is layered:
  - `Classical Validator` = manual-relay / non-orchestrator-managed closure authority when the repo is using the classical validator lane
  - `WP Validator` = WP-scoped technical steering reviewer for the WP; may inspect current coder work, judge bootstrap/skeleton/micro-task quality early, and request steering through packet communications plus Orchestrator-owned ACP controls
  - `Integration Validator` = final technical and merge authority for orchestrator-managed WPs
  - only the `Classical Validator` or `Integration Validator` may own the final merge-ready verdict unless the packet explicitly says otherwise
  - a role-blind gate row is not enough by itself to prove final authority; use validator role plus governed session identity
- Do not poll continuously. The Validator should wake on explicit packet assignment, `ready_for_validation=true`, `validator_trigger != NONE`, a validation handoff receipt, or an explicit operator/orchestrator instruction.
- Update runtime status and append a receipt on validation start, validation query, blocker, verdict-ready handoff, completion, and every packet heartbeat interval only while actively validating.
- `just wp-heartbeat ...` is liveness-only. The `next_actor`, `waiting_on`, and session-route parameters must match current runtime truth; use receipts/notifications or closeout-sync helpers to change workflow routing, not heartbeat.
- Prefer `just active-lane-brief WP_VALIDATOR|INTEGRATION_VALIDATOR WP-{ID}` when context or routing feels fragmented instead of rereading packet/runtime/session truth separately.
- Prefer deterministic helpers over hand-editing these files:
  - `just wp-thread-append WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> "<message>" [target] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]` (writes both `THREAD.md` and a paired `THREAD_MESSAGE` receipt)
  - `just wp-heartbeat WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir] [next_expected_session] [waiting_on_session]`
  - `just wp-receipt-append WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <receipt_kind> "<summary>" [state_before] [state_after] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
  - `just wp-validator-kickoff WP-{ID} <session> <coder_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`
  - `just wp-validator-review WP-{ID} <session> <coder_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just wp-validator-response WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <coder_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just wp-review-response WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just wp-spec-gap WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`
  - `just wp-spec-confirmation WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - For structured microtask steering, the direct-review helpers also accept an optional final `microtask_json` argument carrying `scope_ref`, `file_targets`, `proof_commands`, `risk_focus`, `expected_receipt_kind`, `review_mode`, `phase_gate`, and `review_outcome`.
  - Use `phase_gate=BOOTSTRAP` or `phase_gate=SKELETON` on the kickoff/intent checkpoint when you are explicitly judging early structure.
  - For rolling microtask review, the coder may open `REVIEW_REQUEST` items to `WP_VALIDATOR` with `review_mode=OVERLAP`; keep the queue bounded to at most 2 unresolved overlap items and drain it before full handoff.
  - For the bootstrap/skeleton checkpoint, prefer `wp-validator-response` to clear the plan and `wp-spec-gap` / `VALIDATOR_QUERY` when signed surfaces, proof commands, or implementation quality signals are still weak.
  - `just phase-check STARTUP WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session>`
  - `just phase-check HANDOFF WP-{ID} [WP_VALIDATOR]`
  - `just phase-check VERDICT WP-{ID} [WP_VALIDATOR|INTEGRATION_VALIDATOR]`
  - `just phase-check CLOSEOUT WP-{ID}`
  - `just wp-communication-health-check WP-{ID} STATUS|KICKOFF|HANDOFF|VERDICT`
  - `just session-registry-status [WP-{ID}]`
  - `just active-lane-brief WP_VALIDATOR|INTEGRATION_VALIDATOR WP-{ID} [--json]`
  - `just check-notifications WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR` (check pending messages from coders/orchestrator)
  - `just ack-notifications WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session>` (acknowledge pending notifications after reading)
  - `just operator-viewport` (canonical operator viewport for ACP-aware session/control/thread/receipt/artifact visibility; `just operator-monitor` remains a compatibility alias)
- Orchestrator-only governed session controls (reference only; do not run these from inside a Validator session):
  - `just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]` (operates from the dedicated validator worktree; the governed launcher creates it if missing)
  - `just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]` (operates from handshake_main; no worktree-add needed)
  - `just start-wp-validator-session WP-{ID} [PRIMARY|FALLBACK]`
  - `just start-integration-validator-session WP-{ID} [PRIMARY|FALLBACK]`
  - `just steer-wp-validator-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
  - `just steer-integration-validator-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
  - `just cancel-wp-validator-session WP-{ID}`
  - `just cancel-integration-validator-session WP-{ID}`
- Hard rule: packet truth still wins. Validation authority remains in the packet, especially `## VALIDATION`, `## EVIDENCE`, and `## VALIDATION_REPORTS`.
- Do not treat `THREAD.md` or `RUNTIME_STATUS.json` as authority for scope, verdict, or PREPARE assignment.

## Lifecycle Marker [CX-LIFE-001] (MANDATORY)

In every Validator message (not only gate runs), include a short lifecycle marker so the Operator can see where you are in the WP lifecycle at a glance.

Template:
```text
LIFECYCLE [CX-LIFE-001]
- WP_ID: <WP-...>
- STAGE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- NEXT: <next stage or STOP>
```

Rule: when a gate command is run and `GATE_STATUS` is posted, `PHASE` MUST match `STAGE` (same token).

## Status Sync Commits (Operator Visibility, Multi-Branch)

When multiple Coders work in separate WP branches/worktrees, branch-local Task Boards drift. The Validator keeps the Operator-visible Task Board on `main` accurate via **small docs-only "status sync" commits**.

### Bootstrap Status Sync (Coder starts WP)
[CX-212D] Coders do not commit `.GOV/` files on feature branches. Work packet edits happen through the governance kernel junction and are committed on `gov_kernel` by the orchestrator.
1. Coder updates the work packet `**Status:** In Progress` and fills claim fields (e.g., `CODER_MODEL`, `CODER_REASONING_STRENGTH`) through the junction. The orchestrator commits these changes on `gov_kernel`.
2. Coder sends the Validator: `WP_ID`, `branch`, `worktree_dir`, and current HEAD short SHA (and Coder ID if more than one Coder is active).
3. Validator reads the work packet directly (via junction) to verify claim fields are filled and status is In Progress.
4. Validator updates `.GOV/roles_shared/records/TASK_BOARD.md` (via junction, committed on `gov_kernel` or synced to main via `sync-gov-to-main`):
   - Move the WP entry to `## In Progress` using the script-checked line format: `- **[{WP_ID}]** - [IN_PROGRESS]`.
   - Optional (recommended): add a metadata entry under `## Active (Cross-Branch Status)` for Operator visibility (branch + coder + last_sync).
5. Announce status sync in chat (no verdict implied).

**Rule:** Status sync commits are not validation verdicts. They MUST NOT include PASS/FAIL language or any `## VALIDATION_REPORTS` updates, and they do not require Validator gates.

**Closure rule:** Only after `verdict: PASS` may the Validator set the work packet `**Status:** Done`, move the Task Board entry to `## Done` with `[VALIDATED]`, sync `.GOV/roles_shared/records/BUILD_ORDER.md` (via `just build-order-sync`), and reconcile any remaining activation-state drift for the Base WP before merge.

**PASS closure visibility rule (MANDATORY):**
- After a WP receives `verdict: PASS`, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` before merging the WP to `main`.
- Required command before merge containment exists: `just task-board-set WP-{ID} DONE_MERGE_PENDING`
- Required command after merge containment is verified: `just task-board-set WP-{ID} DONE_VALIDATED`
- The Task Board update MUST be carried in the same WP branch closure flow as the PASS report append / packet `**Status:** Done` update, so merge truth stays `[MERGE_PENDING]` until local `main` actually contains the approved closure commit.
- If the WP packet says `Done`/`PASS` but the Task Board still shows `READY_FOR_DEV` or `IN_PROGRESS`, closure is incomplete and the Validator MUST fix the Task Board before merge.
- Activation-state reconciliation is part of PASS closure, not an optional cleanup:
  - If the resolved official packet path (`.GOV/work_packets/{WP_ID}/packet.md` logical; current physical `.GOV/task_packets/{WP_ID}/packet.md`) or legacy `.GOV/task_packets/{WP_ID}.md` is an official packet, `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` MUST point the Base WP to that official packet path, not a stub path.
  - `.GOV/roles_shared/records/TASK_BOARD.md` MUST NOT keep that Active Packet under `## Stub Backlog (Not Activated)`.
  - `.GOV/roles_shared/records/BUILD_ORDER.md` MUST be regenerated from the reconciled Task Board + traceability state via `just build-order-sync`.
- Required final verification before merge/push of `main`: `just gov-check`
- If `just gov-check` fails because of activation traceability drift (`wp-activation-traceability-check`) or any related governance mismatch, the Validator MUST STOP, fix the governance surfaces on the WP branch, and re-run the check before merge.

## Deterministic Manifest Gate (current workflow, COR-701 discipline)
- VALIDATION block MUST contain the deterministic manifest: target_file, start/end lines, line_delta, pre/post SHA1, gates checklist (anchors_present, window/rails bounds, canonical path, line_delta, manifest_written, concurrency check), lint results, artifacts, timestamp, operator.
- Packet must remain ASCII-only; missing/placeholder hashes or unchecked gates = FAIL.
- Require evidence that `just phase-check HANDOFF WP-{ID} WP_VALIDATOR` ran and passed before PASS handoff or PASS commit clearance. This composite gate includes `validator-packet-complete`, `validator-handoff-check`, and the governed handoff communication proof against the PREPARE worktree source of truth. If absent or failing, verdict = FAIL until fixed.
- Require evidence that `just phase-check CLOSEOUT WP-{ID}` ran and passed before PASS commit clearance. This composite gate includes `validator-packet-complete`, the final review communication proof, the integration-validator context bundle, and the governed closeout preflight. If absent or failing, verdict = FAIL until fixed.
- After the closeout preflight is green, use `just integration-validator-closeout-sync WP-{ID} MERGE_PENDING` to write the PASS-ready packet/runtime/TASK_BOARD truth in one governed step. After local `main` containment is real, use `just integration-validator-closeout-sync WP-{ID} CONTAINED_IN_MAIN <MERGED_MAIN_SHA>` to finish promotion.
- For contained-main promotion, the candidate target must still match the signed artifact exactly, but the contained local-`main` commit may differ when conflict resolution or main-harmonization was required. That closure remains legal only when the resulting contained commit stays entirely within the signed file surface and still passes the governed closeout proof/tripwire checks.
- Successful closeout sync must also leave machine-readable provenance: a validator gate-state closeout event plus a closeout `STATUS` receipt naming the governed Integration Validator lane, mode, and containment/baseline truth that was recorded.
- If closeout is attempted from the wrong role/lane, from a kernel/orchestrator surface, or with live governance still resolving from `handshake_main/.GOV`, record `WORKFLOW_INVALIDITY` (`ROLE_BOUNDARY_BREACH`, `FINAL_LANE_AUTHORITY_VIOLATION`, or `FINAL_LANE_GOV_ROOT_VIOLATION`) and halt before packet/runtime/TASK_BOARD truth is promoted.
- For governed non-PASS terminal closure, use the same sync surface instead of manual packet/runtime/TASK_BOARD edits:
  - `just integration-validator-closeout-sync WP-{ID} FAIL`
  - `just integration-validator-closeout-sync WP-{ID} OUTDATED_ONLY`
  - `just integration-validator-closeout-sync WP-{ID} ABANDONED`
- Require evidence that `just post-work WP-{ID}` ran and passed for the validated committed target (typically surfaced through `just phase-check HANDOFF WP-{ID} WP_VALIDATOR`, or directly during low-level diagnosis). If absent or failing, verdict = FAIL until fixed.
- Post-work sequencing note (echo from CODER_PROTOCOL): `just post-work` validates staged/working changes when present, and on a clean tree validates a deterministic range:
  - If the work packet contains `MERGE_BASE_SHA`: `MERGE_BASE_SHA..HEAD`
  - Else if `merge-base(main, HEAD)` differs from `HEAD`: `merge-base(main, HEAD)..HEAD`
  - Else: the last commit (`HEAD^..HEAD`)
  It can also validate a specific commit via `--rev <sha>`.
  Require the Coder's PASS `GATE_OUTPUT` plus the validated commit SHA/range shown in that output.
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
- Confirm the work packet has `**Status:** In Progress` and claim fields filled (CODER_MODEL, CODER_REASONING_STRENGTH) before accepting any skeleton or implementation progression. [CX-212D] Bootstrap claim is verified by field content, not by a commit on the feature branch.
- Enforce [CX-GATE-001]: if the Coder included SKELETON content in the BOOTSTRAP turn, treat it as invalid phase merging; require a new, separate SKELETON turn/commit after explicit Operator authorization.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, the WP Validator owns the first-pass judgement of coder BOOTSTRAP and SKELETON quality. Use the kickoff/intent loop to steer corrections directly instead of waiting for the Orchestrator to relay them.

0A) Micro Task Early Review (WP Validator)
- When micro tasks exist in the resolved Work Packet folder (current physical `.GOV/task_packets/WP-{ID}/MT-*.md`), the WP Validator reviews completed MTs as the coder works — do not wait for all MTs to be done.
- On orchestrator-managed lanes, treat governed coder `CODER_INTENT` / overlap `REVIEW_REQUEST` receipts without a declared-MT `microtask_contract` as invalid workflow, not merely weak evidence; the contract must resolve to one declared MT and keep `file_targets` inside that MT's `CODE_SURFACES`.
- For each MT where `CODER STATUS: DONE`:
  - Read the MT file and verify the evidence (file:line proof, tests run)
  - Check the implementation against the clause and the master spec
  - Set `VALIDATOR STATUS: CONFIRMED` if the evidence is sufficient
  - Set `VALIDATOR STATUS: NEEDS_REVISION` with `DIRECTION` guidance if the evidence is insufficient or the implementation misses the clause
  - Write a `REVIEW_RESPONSE` receipt via `just wp-receipt-append` targeting the Coder
- This early review catches spec drift and shallow implementations before the coder claims the WP as done.
- **Per-Microtask Inspection [RGF-89] (HARD for orchestrator-managed lanes):** When the coder sends a `REVIEW_REQUEST` for a completed MT, the WP Validator MUST inspect that MT before the coder proceeds to the next one. Do not defer all inspection to end-of-WP handoff. Per-MT review catches issues early and prevents compounding errors across MTs.
- After inspecting each MT, send a governed review response: `just wp-review-response WP-{ID} WP_VALIDATOR <session> CODER <target_session> "<summary>" <correlation_id>`
- If the MT has issues, include specific fix instructions in the response so the coder can fix before starting the next MT.
- **Adversarial Review [CX-503J]:** After confirming the code compiles and tests pass, actively try to break it. Look for race conditions, input validation gaps, error handling omissions, capability escalation paths, and spec requirements the coder missed. Your job is not to confirm the code works — it is to find where it does not. "Never trust subagent self-reports." [RGF-99]
- **Tool-Call Boundary [CX-503H / RGF-105]:** The validator MUST NOT edit product code under `src/`, `app/`, or `tests/`. You may read any file but writing is reserved for governance surfaces (`.GOV/`, reports, receipts). If you find code that needs fixing, send fix instructions to the coder via `wp-review-response`, do not fix it yourself.
- When ALL MTs are `VALIDATOR STATUS: CONFIRMED`, proceed to the Final WP Review below.
- **WP Validator shares the coder worktree** (`wtc-*` on `feat/WP-{ID}`) per [CX-503G]. No separate `wtv-*` worktree. The per-MT stop ensures only one role is active at a time.

0A-FINAL) Final WP Review (after all MTs pass)

After all individual MTs pass, the WP Validator MUST perform a complete WP-level review before writing the validation verdict:

1. **Full product code review**: Read the complete diff (`git diff <base>..HEAD`), not just per-MT slices. Check for cross-MT integration issues, naming consistency, and dead code.
2. **Validator rubric check**: Apply the governed validator report profile (SPLIT_DIFF_SCOPED_RIGOR_V3/V4). All rubric sections must be filled with concrete evidence.
3. **Red team assessment**: Check for security failure modes, capability escalation paths, race conditions, and input validation gaps across the full diff.
4. **Master Spec alignment (wide scope)**: Verify the implementation against the spec anchors from the refinement. Check that the implementation satisfies the spec's MUST/SHOULD clauses, not just the packet's acceptance criteria.
5. **Artifact hygiene**: Run `just artifact-hygiene-check` from the coder worktree. Flag any repo-local `target/` directories or wrongly-placed build artifacts.
6. **Write validation verdict**: If all checks pass, write `Verdict: PASS` in the validation report. If any check fails, write `Verdict: FAIL` with specific remediation instructions and send them to the coder via `just wp-review-response`.
7. **If PASS**: Notify the orchestrator (via `wp-notification` with target ORCHESTRATOR) that the WP is ready for integration validation and merge.

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
- Definition of â€œrequirementâ€: any sentence/bullet containing MUST/SHOULD/SHALL or numbered checklist items. Roadmap is a pointer; Master Spec body is the authority.
- Copy identifiers (anchors, bullet labels) to keep traceability. No assumptions from memory.
- Spec ref consistency: SPEC_BASELINE is provenance (spec at creation); SPEC_TARGET is the binding spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- Resolve SPEC_TARGET at validation time (.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md) and validate DONE_MEANS/evidence against the resolved spec.
- Compare the implementation against local `main` first. Use `origin/main` only as a secondary fallback when local `main` lacks the relevant integrated context or the audit is explicitly about remote drift.
- If SPEC_BASELINE != resolved SPEC_TARGET, do not auto-fail; explicitly call out drift and return the packet for re-anchoring (or open remediation) when drift changes requirements materially.
- If a WP is correct for its SPEC_BASELINE but SPEC_TARGET has evolved, record a distinct disposition: **OUTDATED_ONLY** (historically done; no protocol/code regression proven). Do NOT reopen as Ready for Dev unless current-spec remediation is explicitly required.
- Spec changes are governed via Spec Enrichment (new spec version file + `.GOV/spec/SPEC_CURRENT.md` update) under a one-time user signature recorded in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`; this is not itself a separate work packet.

## Diff-Scoped Spec Review Checklist (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- Enumerate the exact in-scope MUST/SHOULD clauses the WP claims to close. Do not treat the whole spec as implicitly reviewed.
- For each clause, record one explicit bullet under `CLAUSES_REVIEWED` with the clause identifier/text fragment plus file:line evidence.
- If any clause is only partially proven, blocked by environment, or inferred indirectly, do not hide that in prose; record it under `NOT_PROVEN` and downgrade `SPEC_ALIGNMENT_VERDICT` accordingly.
- `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when every diff-scoped clause claimed by DONE_MEANS + SPEC_ANCHOR is listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Automation gates (`pre-work`, `phase-check HANDOFF`, `post-work`, `gov-check`) prove workflow legality and hygiene. They do not, by themselves, prove spec completeness.

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
- Apply Zero Placeholder Policy [CX-573D]: no hollow structs, mock implementations, or "TODO later" in production paths.
- Allowed exceptions (must be justified in code + validation notes):
  - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
  - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
- Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-573E].

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
- Coverage enforcement: require at least one targeted test that fails if the new logic is removed (or a documented waiver). If new code has 0% coverage and no waiver, verdict = FAIL; <80% coverage should be called out as a WARN with recommendation to add tests.
- Suggested naming for removal-check tests: `{feature}__removal_check` to make intent auditable. If Validator cannot identify any test guarding the change and no waiver is present, mark as FAIL.

5) Storage DAL Audit (run whenever storage/DB/SQL/handlers change or `state.pool`/`sqlx` appear)
- CX-DBP-VAL-010: No direct DB access outside storage/ DAL. Grep for `state.pool`, `sqlx::query` in non-storage paths.
- CX-DBP-VAL-011: SQL portability. Flag `?1`, `strftime(`, `CREATE TRIGGER` SQLite-only syntax in migrations/queries.
- CX-DBP-VAL-012: Trait boundary. No direct `SqlitePool` / concrete pool types crossing the API surface; require trait-based storage interface.
- CX-DBP-VAL-013: Migration hygiene. Check numbering continuity, idempotency hints, and consistent versioning.
- CX-DBP-VAL-014: Dual-backend readiness. If tests exist, ensure both backends are parameterized; if absent, mark as gap (waiver must be explicit).
- For portable/shared storage contracts, CX-DBP-VAL-014 is field-level semantic parity, not just "a SQLite test exists and a PostgreSQL test exists". Backend-specific tests cannot close portable field behavior by themselves.
- Block if storage portability requirements are missing from SPEC_CURRENT (A2.3.12) or DAL violations are present; re-open affected WPs.

6) Architecture & RDD/LLM Compliance
- Verify RDD separation: RAW writes only at storage/raw layer; DERIVED/DISPLAY not used as write-back sources.
- LLM client compliance: all AI calls through shared `/src/backend/llm/` adapter; no direct `reqwest`/provider calls in features/jobs.
- Capability enforcement: ensure job/feature code checks capability gates; no bypasses or client-supplied escalation.
- For new persisted/exported/request data shapes, prefer LLM-first structured fields over presentation-first blobs: stable field names, explicit enums/typed fields, and machine-readable meaning that does not require reparsing UI prose.
- When the WP touches SQL/data access, prefer portable SQL/data modeling that remains PostgreSQL-ready; call out new SQLite-only semantics unless the packet/spec explicitly requires them.
- When the WP touches graph/search/provenance surfaces, preserve Loom-friendly linkage: stable ids, explicit relations, backlink-friendly fields, and retrieval-friendly summaries that stay traversable outside the UI.

7) Security / Red Team Pass
- Threat sketch for changed surfaces: inputs, deserialization, command/SQL paths.
- Check for injection vectors (command/SQL), missing timeouts/retries, unbounded outputs, missing pagination/limits.
- Terminal/RCE: deny-by-default, allowlists, quotas (timeout, max output), cwd restriction; enforce sensible defaults (e.g., bounded timeout/output) or fail if absent. Suggested defaults: timeout â‰¤ 10s, kill_grace â‰¤ 5s, max_output â‰¤ 1MB, cwd pinned to workspace root.
- Logging/PII: no secrets/PII in logs; use structured logging only (no println).
- Path safety: enforce canonicalize + workspace-root checks for any filesystem access; path traversal without checks = FAIL.
- Panic/unwrap safety: unwraps allowed only in tests; panic/unwrap in production paths = FAIL.
- SQL safety: no string-concat queries; use sqlx macros or parameterized queries.
- Build hygiene: flag large/untracked build artifacts or missing .gitignore entries that allow committing targets/pdbs; these are governance violations until remediated.
- **Git Hygiene:**
    - **Strict:** "Dirty" git status (uncommitted changes) is a FAIL for final validation unless a **User Waiver** [CX-573F] is explicitly recorded in the Work Packet.
    - **Artifacts:** FAIL if *ignored* build artifacts (e.g., `target/`, `node_modules/`) are tracked or committed.
    - **Scope:** Ensure changes are restricted to the WP's `IN_SCOPE_PATHS`.
    - **Committed-handoff rule (preferred for orchestrator-managed WPs):** Run `just phase-check HANDOFF {WP_ID} WP_VALIDATOR`. This wraps packet completeness, PREPARE worktree source-of-truth validation, and the governed handoff communication proof into one boundary gate before `validator-gate-commit`.
    - **Final-lane closeout rule (orchestrator-managed PASS only):** Run `just phase-check CLOSEOUT {WP_ID}` before `validator-gate-commit`. This must prove verdict-route health, context bundling, topology safety, WP-scoped settled session-control truth, and current-`main` signed-scope compatibility; otherwise final review is not closeout-ready.
    - **Local mirror sanity only:** You may still run `just post-work {WP_ID}` in your validator worktree for local diagnosis, but it does not replace committed handoff validation against the PREPARE worktree.


7.1) Git & Build Hygiene Audit (execute when any build artifacts/.gitignore risk is suspected)
- Check .gitignore coverage for: target/, node_modules/, *.pdb, *.dSYM, .DS_Store, Thumbs.db. Missing entries = FAIL until added.
- Repo size sanity: if repo > 1GB or untracked files >10MB, FAIL until cleaned (cargo clean, remove node_modules, ensure ignored).
- Committed artifacts: fail if git ls-files surfaces target/, node_modules, *.pdb, *.dSYM.
- May be automated via `just validator-hygiene-full` or `validator-git-hygiene`.

## Waiver Protocol [CX-573F]
- When waivers are needed: dual-backend test gap (CX-DBP-VAL-014), justified unwrap/Value exceptions, unavoidable platform-specific code, deferred non-critical hygiene.
- Approval: MEDIUM/HIGH risk requires explicit user approval; LOW risk can be Coder + Validator with user visibility.
- Recording (in work packet under "WAIVERS GRANTED"): waiver ID/date, check waived, scope (per WP), justification, approver, expiry (e.g., Phase 1 completion or specific WP).
- Waivers NOT allowed: spec regression, evidence mapping gaps, hard invariant violations, security gate violations, traceability removal, RCE guard removal.
- Absent waiver for a required check = FAIL. Expired waivers at phase boundary must be revalidated or removed.

## Escalation Protocol (Blocking paths)
- Incomplete work packet/spec regression: FAIL immediately; send to Orchestrator to fix packet/spec before validation continues.
- Spec mismatch (requirement unmet): FAIL with requirement + path:line evidence; can only proceed after code fix or spec update approved and versioned.
- Test flake/unreproducible failure: request full output; attempt re-run. If still inconsistent, FAIL and return to Coder to stabilize.
- Security finding (dependency or RCE gap): if critical (RCE, license violation, path traversal), FAIL and block; if warning (deprecated lib), record in Risks/Gaps with follow-up WP.

## Standard Command Set (run when applicable)
- `just cargo-clean` (cleans external Cargo target dir at `../Handshake Artifacts/handshake-cargo-target` before validation/commit; fail validation if skipped)
- `just artifact-hygiene-check` (fails if repo-local `target/` exists or blocking non-canonical external artifact residue remains)
- `just artifact-cleanup [--dry-run]` (mechanically removes reclaimable stale external artifact folders plus repo-local `target/` residue)
- `just external-validator-brief WP-{ID}` (prints the canonical external/classical validator target contract: code target, governance target, committed handoff command, split report fields, and legal verdict vocabulary)
- `just phase-check HANDOFF WP-{ID} WP_VALIDATOR` (preferred required boundary gate before PASS commit clearance for orchestrator-managed WPs; validates packet completeness, committed PREPARE handoff state, and governed handoff routing in one pass)
- `just integration-validator-context-brief WP-{ID}` (canonical final-lane authority/path/context bundle for orchestrator-managed Integration Validator work; use this instead of rereading protocols or rediscovering branch/worktree/session/main-compatibility truth)
- `just phase-check CLOSEOUT WP-{ID}` (preferred required final-lane boundary gate before PASS commit clearance for orchestrator-managed WPs; wraps verdict proof, context bundle, closeout preflight, and memory refresh)
- `just validator-handoff-check WP-{ID}` (low-level debug/compatibility leaf for diagnosing a failed HANDOFF phase gate)
- `just integration-validator-closeout-check WP-{ID}` (low-level debug/compatibility leaf for diagnosing a failed CLOSEOUT phase gate)
- `just validator-packet-complete WP-{ID}` (low-level debug/compatibility leaf for diagnosing packet-hygiene failures beneath HANDOFF/VERDICT/CLOSEOUT)
- `just session-reclaim-terminals WP-{ID} [ROLE] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]` (manual repair helper for any still-open registry-owned governed system-terminal windows after closeout; default current-batch targeting is the safe path)
- `just gov-check` (required before PASS merge/push and for any governance-only validator changes; catches activation traceability drift, Task Board/build-order drift, and shared governance regressions)
- `just validator-gate-*` write commands now reject unbound/wrong-lane orchestrator-managed usage early; if the current checkout is not a governed validator lane, use `just validator-next WP-{ID}`, `just integration-validator-context-brief WP-{ID}`, or `just external-validator-brief WP-{ID}` instead of forcing gate writes from the wrong surface
- `just validator-scan` (forbidden patterns, mocks/placeholders, RDD/LLM/DB boundary greps)
- `just validator-dal-audit` (CX-DBP-VAL-010..014 checks: DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend readiness)
- `just validator-spec-regression` (SPEC_CURRENT points to latest; required anchors like A2.3.12 present)
- `just spec-eof-appendices-check` (Spec Â§12 end-of-file appendix blocks exist + are parseable/valid)
- `just validator-phase-gate Phase-1` (ensure no Ready-for-Dev items remain before phase progression; depends on validator scans)
- `just validator-error-codes` (stringly errors/determinism/HSK-#### enforcement)
- `just validator-coverage-gaps` (sanity check that tests exist/guard the change)
- `just validator-traceability` (trace_id/job_id presence in governed mutation paths)
- `just validator-git-hygiene` or `just validator-hygiene-full` (artifact and .gitignore checks)
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
  - It may run `validator-gate-*`, append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
- `Governed Validation`
  - This is the orchestrator-managed validator lane.
  - `WP Validator` is advisory only in this lane. It may inspect code/spec drift, challenge weak reasoning, append receipts/thread guidance, and hand off or block, but it does not own final merge-to-`main` authority.
  - `Integration Validator` is the governed closure authority in this lane. It may run `validator-gate-*`, append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
  - After merge-to-main succeeds, the Integration Validator may execute an Orchestrator-generated single-target cleanup script for the merged CODER or WP_VALIDATOR local worktree only when:
    - the WP merge is already complete
    - the exact Operator approval text is supplied
    - the matching cleanup token from the target worktree is supplied
  - Manual filesystem deletion remains forbidden.
- `External / Independent Revalidation (orchestrator-managed WPs only)`
  - This is an audit mode, not a second validator workflow and not the classical/manual-relay closure lane.
  - Required start sequence:
    - `just validator-startup`
    - `just external-validator-brief WP-{ID}`
  - `just validator-startup` alone is insufficient for this mode.
  - This mode may audit code, governance, and environment, but it MUST NOT:
    - run `validator-gate-*`
    - mutate closure state
    - append normal governed-lane closure artifacts
    - merge or authorize merge in place of the Classical Validator or Integration Validator
  - Default write target for this mode is a chat report or a clearly labeled external revalidation report, not the normal governed-lane closure path.
  - ACP runtime note for orchestrator-managed WPs:
    - `wt-gov-kernel` is the live Orchestrator governance surface. ACP/session/topology/WP-communication projections should stay external under the repo-governance runtime root instead of dirtying the kernel checkout.
    - Dirty files limited to these surfaces are runtime-state evidence first, not automatic proof of governance failure:
      - `../gov_runtime/roles_shared/validator_gates/WP-{ID}.json`
    - Before treating `wt-gov-kernel` dirt as a governance defect, inspect ACP state with:
      - `just handshake-acp-broker-status`
      - `just session-registry-status WP-{ID}`
      - `just external-validator-brief WP-{ID}`
    - If those commands show expected runtime churn and the governed handoff path still passes, classify the dirt as runtime-state context, not packet-scope implementation drift.

## External Validator Split Report Contract
- Before an external/classical validator starts on an orchestrator-managed WP, generate the target contract with `just external-validator-brief WP-{ID}`.
- Before the governed Integration Validator resumes final-lane work on an orchestrator-managed WP, open the canonical context bundle with `just integration-validator-context-brief WP-{ID}` instead of rebuilding branch/worktree/authority/main-compatibility truth manually.
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
  - `DATA_CONTRACT_PROOF:` with concrete code, query, schema, or emitted-artifact evidence showing the packet was reviewed for SQL portability, LLM-first readability/parseability, and Loom-intertwined requirements
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

## Validation Gate Sequence [CX-VAL-GATE] (ONE REVIEW PAUSE; APPEND-FIRST)

The validation process MUST halt only at Gate 3 (final report presentation). All other gates are state recording/unlocks and must still be run in order.
State is tracked per WP in `../gov_runtime/roles_shared/validator_gates/{WP_ID}.json`. Gates enforce minimum time intervals to prevent automation momentum.
(Legacy: `.GOV/reference/legacy/validator/VALIDATOR_GATES.json` is treated as a read-only archive for older sessions; new validations should not write to it.)

### Gate 1: WP APPEND (Records verdict; non-blocking)
1. Validator completes all checks and generates the full VALIDATION REPORT.
2. If verdict = PASS, before recording Gate 1 the Validator MUST update the WP closure state on the WP branch:
   - set work packet `**Status:** Done`
   - update `.GOV/roles_shared/records/TASK_BOARD.md` to `## Done` / `[MERGE_PENDING]` before merge, then `[VALIDATED]` only after main containment is verified
   - sync `.GOV/roles_shared/records/BUILD_ORDER.md` via `just build-order-sync`
3. Validator appends the VALIDATION REPORT to the active official packet path (logical: `.GOV/work_packets/{WP_ID}/packet.md`; current physical: `.GOV/task_packets/{WP_ID}/packet.md`; legacy flat: `.GOV/task_packets/{WP_ID}.md`) (APPEND-ONLY per [CX-WP-001]).
4. Validator runs: `just validator-gate-append {WP_ID} {PASS|FAIL|ABANDONED}`
5. Validator does **not** paste the full report to chat yet.

### Gate 2: COMMIT CLEARANCE (PASS only)
1. Only if verdict = PASS, Validator runs: `just validator-gate-commit {WP_ID}`
   - Mandatory precondition: `just phase-check CLOSEOUT {WP_ID}` must already pass.
2. Validator performs `git commit` on the WP branch and records the commit SHA.
   - PASS requirement: this commit MUST include the appended report plus the Task Board / packet / build-order closure updates and any required Base-WP activation-state fixes (`WP_TRACEABILITY_REGISTRY`, removal of stale STUB state) so the later merge + fast-forward exposes the validated WP state in every active worktree.
   - PASS requirement: run `just gov-check` after those closure updates and before merge; a PASS commit without a passing governance check is incomplete.

### Gate 3: FINAL REPORT PRESENTATION (Blocking; the only mechanical pause)
1. If verdict = FAIL or ABANDONED: run immediately after Gate 1, **before any remediation/discard begins**.
2. If verdict = PASS: run after Gate 2 and after the validation report append is committed (**right before merge to `main` / push of `main`**).
3. Validator **outputs the entire report to chat** using the Report Template.
4. Validator runs: `just validator-gate-present {WP_ID}`
5. **HALT.** Validator MUST NOT merge to `main` / push `main` (PASS) or authorize remediation/discard kickoff (FAIL/ABANDONED) until the user acknowledges.

### Gate 4: USER ACKNOWLEDGMENT (Unlock)
1. User explicitly acknowledges the report (e.g., "proceed", "approved", "continue").
2. If user requests changes or disputes findings -> return to validation, re-run checks, regenerate report.
3. Validator runs: `just validator-gate-acknowledge {WP_ID}`
4. PASS: Validator may merge the validated WP into `main`. Canonical integration push remains `main` only; backup pushes are allowed only to the matching backup branch for the current role or WP when preserving state.
5. FAIL: WP remains open for remediation (no merge/commit).

### Gate Commands
```
just validator-gate-append {WP_ID} {PASS|FAIL|ABANDONED}   # Gate 1: Record WP append + verdict
just phase-check CLOSEOUT {WP_ID} # Canonical final-lane verdict/context/closeout bundle
just validator-gate-commit {WP_ID}                # Gate 2: Unlock commit (PASS only)
just validator-gate-present {WP_ID} [PASS|FAIL|ABANDONED]   # Gate 3: Record report shown (HALT)
just validator-gate-acknowledge {WP_ID}           # Gate 4: Record user ack (unlock)
just validator-gate-status {WP_ID}                # Check current gate state
just validator-gate-reset {WP_ID} --confirm       # Reset gates (archives old session)
```

**Violations:** Skipping Gate 1, committing without a PASS Gate 2, or merging to `main` / pushing `main` (PASS) / starting remediation or discard (FAIL/ABANDONED) without Gate 3+4 = PROTOCOL VIOLATION [CX-VAL-GATE-FAIL]. Gate commands will fail if the sequence is violated.

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

## Merge/Commit Authority (per Codex [CX-505])
- After issuing PASS **and completing all validation gates**, the validator form that currently owns closure authority is responsible for the integration flow into `main`.
- Closure authority split:
  - `Classical Validator` owns merge/push authority for classical/manual-relay validation.
  - `Integration Validator` owns merge/push authority for orchestrator-managed validation unless the packet explicitly overrides it.
  - `WP Validator` never owns merge-to-`main` authority.
- Validator responsibilities after PASS:
  - merge the validated WP branch into `main`
  - commit any required closure-sync or conflict-resolution edits on `main`
  - ensure the canonical closed `[VALIDATED]` state lives on `main`
- Pre-merge governance gate (MANDATORY): before merging the WP branch into `main`, the Validator MUST confirm `just gov-check` passes on the closure branch. Treat any activation-state drift (`WP_TRACEABILITY_REGISTRY`, Task Board STUB residue, stale build-order snapshot) as a merge-blocking failure, not post-merge cleanup.
- Coders must not merge their own work.
- Canonical push rule: only `main` is a canonical integration push target. Backup pushes to matching backup branches are allowed as safety copies, but they are not integration events.
- If a remote integration push is authorized, the Validator pushes `main` only after the merge is complete and `main` contains the final validated closure state.

## Post-Merge Cleanup (reduces branch confusion)
- Do NOT delete local WP branches or remote WP backup branches as routine cleanup.
- Any local or remote WP branch deletion requires explicit Operator approval naming the exact target(s).
- When deletion is approved for a presented exact action/target list, use the deterministic helper with `approved` or `proceed`:
  - `just close-wp-branch WP-{ID} "--remote" "approved"`

## Report Template
```
VALIDATION REPORT â€” {WP_ID}
Verdict: PASS | FAIL | NOT_PROVEN | OUTDATED_ONLY | ABANDONED | BLOCKED

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate on the committed handoff state, typically via `just phase-check HANDOFF {WP_ID} WP_VALIDATOR`; not tests): PASS | FAIL
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
- [CX-WP-001] MANDATORY APPEND: Every validation verdict (PASS/FAIL/ABANDONED) MUST be APPENDED to the end of the active official packet file (logical: `.GOV/work_packets/{WP_ID}/packet.md`; current physical: `.GOV/task_packets/{WP_ID}/packet.md`; legacy flat: `.GOV/task_packets/{WP_ID}.md`). OVERWRITING IS FORBIDDEN.
- [CX-WP-002] CLOSURE REASONS: The append block MUST contain a "REASON FOR {VERDICT}" section explaining exactly why the WP was closed or failed, linking back to specific findings.
- STATUS + closure updates are PASS-gated: append the full Validation Report for PASS/FAIL/ABANDONED using the template below, but only after `verdict: PASS` may the Validator set work packet `**Status:** Done`, move TASK_BOARD to Done/Merge Pending, and sync BUILD_ORDER (`just build-order-sync`). Promote to `Validated (PASS)` / `[VALIDATED]` only after main containment is real and recorded. **DO NOT OVERWRITE User Context or previous history [CX-654].**
- For non-PASS governed verdicts or `DISPOSITION=OUTDATED_ONLY|ABANDONED`, append the report but do not perform normal Done/Validated PASS closure updates on work packet/TASK_BOARD/BUILD_ORDER unless the governed lane explicitly records the non-PASS terminal closure path.
- TASK_BOARD update (merge-visible requirement): for PASS before merge, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` on the WP branch using `just task-board-set WP-{ID} DONE_MERGE_PENDING`, and the closure commit MUST carry that update so merge truth is not overstated.
- TASK_BOARD update (post-merge requirement): after the approved closure commit is contained in local `main`, promote the entry with `just task-board-set WP-{ID} DONE_VALIDATED`.
- TASK_BOARD update (on `main`): after merge, the canonical main-branch Task Board must already show the validated WP entry from that closure commit. Status-sync commits earlier in the WP lifecycle are separate and do not imply a verdict.
- Board consistency (on `main`): work packet `**Status:**` is source of truth; reconcile the Task Board to match packet reality before declaring PASS. Unresolved mismatch = FAIL pending correction.
- Activation consistency (merge-visible requirement): when validating an official packet, reconcile `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` and remove any stale `## Stub Backlog` entry for that Active Packet before merge; then run `just build-order-sync` and `just gov-check` so the official activation state is visible on `main` immediately after merge.
```

## Non-Negotiables
- Evidence over intuition; speculative language is prohibited [CX-588].
- [CX-WP-003] APPEND-ONLY WP HISTORY: Deleting or overwriting the status history in a Work Packet is a protocol violation. All verdicts must be appended.
- Automated review scripts are optional; manual evidence-based validation is required.
- If a check cannot be performed (env/tools unavailable), report as FAIL with reasonâ€”do not assume OK.
- No â€œpass with debtâ€ for hard invariants, security, traceability, or spec alignment; either fix or obtain explicit user waiver per protocol.
