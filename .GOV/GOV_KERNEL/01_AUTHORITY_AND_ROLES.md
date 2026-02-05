# 01) Authority and Roles (Kernel)

This kernel assumes a â€œlaw stackâ€ where **precedence is explicit** and **roles are mechanically separated** so small-context agents can operate safely.

## 1. Authority stack (precedence is not implicit)

Each project MUST define a precedence order and keep it stable. A canonical order (highest â†’ lowest):

1. **Platform/system constraints**
   - Non-negotiable runtime constraints from the execution environment (tooling limits, sandboxing, secrets, etc.).
2. **Project Codex** (`<PROJECT> Codex vX.Y.md`, repo root)
   - Behavioral constitution for agents and humans interacting with the repo.
   - Must include hard bans (destructive cleanup, unsafe sync) and a conflict/override protocol.
3. **Master Spec** (`<PROJECT>_Master_Spec_vNN.NNN.md`, repo root) + pointer (`.GOV/roles_shared/SPEC_CURRENT.md`)
   - The authoritative product/architecture specification.
   - `.GOV/roles_shared/SPEC_CURRENT.md` MUST be the single pointer to the current spec file.
4. **Role Protocols** (`.GOV/roles/*/*_PROTOCOL.md`)
   - Defines what each role may and may not do.
   - Must include a refinement/signature/packetization process if mechanical gates are used.
5. **Repo-local guardrails** (`AGENTS.md`, repo root)
   - Tight, local instructions for agent execution (branch/worktree rules, safety gates, repo hygiene).
6. **Work authorization artifacts** (`.GOV/task_packets/*.md`, `.GOV/refinements/*.md`, `.GOV/roles_shared/TASK_BOARD.md`)
   - Make â€œwhat is allowedâ€ explicit and auditable.
7. **Gate tooling** (`.GOV/scripts/validation/*`, `.GOV/scripts/hooks/*`, `.github/workflows/*`, `justfile`)
   - Mechanical enforcement; tools MUST not silently change the law stack.

Kernel rule: when two sources conflict, the **higher** source wins. Overrides MUST be explicit and logged (see `.GOV/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`).

## 2. Roles (mechanical separation of duties)

This kernel uses roles as safety boundaries. A role is not a â€œpersonaâ€; it is a **capability envelope**.

### 2.1 OPERATOR (human authority)
Purpose:
- Sets priorities and selects what work is activated.
- Grants explicit approvals for sync/destructive operations.
- Provides signatures for refinement activation and scope overrides.

Non-delegable responsibilities:
- Any exception to hard bans.
- Any explicit â€œsync gateâ€ actions (fetch/merge/rebase/switch) if forbidden by the Codex/Protocol.

### 2.2 ORCHESTRATOR (workflow + spec-to-work translation)
Purpose:
- Translates the Master Spec into executable work authorization artifacts (refinements + task packets).
- Maintains the Task Board and traceability registries.
- Runs Orchestrator gates that record approvals/signatures and prevent momentum failures.

Hard boundary (kernel default):
- Orchestrator MUST NOT implement product code. It only authors/maintains governance artifacts and runs read-only inspection.

### 2.3 CODER (implementation)
Purpose:
- Implements exactly what an activated task packet authorizes, within explicit in-scope paths.
- Produces deterministic evidence (manifests) suitable for Validator review.

Hard boundary:
- Coder MUST NOT change scope, redefine requirements, or â€œfix adjacent thingsâ€ unless a task packet contains a waiver/authorization.

### 2.4 VALIDATOR (audit + acceptance gate)
Purpose:
- Performs evidence-based verification against task packet requirements.
- Verifies tests/builds and traces requirements to file:line evidence.
- Controls the final â€œPASS â†’ commit/merge eligibilityâ€ state (via validator gates).

Hard boundary:
- Validator MUST NOT implement feature code while acting as Validator (to preserve independence).

### 2.5 Optional roles (supported patterns)
These roles MAY exist if explicitly defined in protocols:
- **Tooling agent**: runs diagnostics, builds bundles, or triages CI failures.
- **Debugger**: incident/runbook execution (must not change scope).
- **Red Hat / Red Team mode**: adversarial review framing; typically a Validator sub-mode.

## 3. Branching, worktrees, and concurrency (portable rule set)

Kernel objective: avoid cross-contamination of context and changes when multiple roles or WPs run concurrently.

Mandatory rules:
- One work packet (WP) â†’ one feature branch (e.g., `feat/WP-<ID>`).
- Concurrency across active WPs MUST use `git worktree` (separate working directories).
- A single working tree MUST NOT be shared across concurrent WPs.

Recommended rules:
- One role â†’ one default worktree (e.g., `wt-orchestrator`, `wt-validator`) plus per-WP worktrees as needed.
- Task packets SHOULD specify the expected branch/worktree name so small-context models can validate they are â€œin the right placeâ€.

## 4. Safety: destructive operations and sync gates

To keep the governance system reversible and auditable:

Destructive operations MUST be explicitly authorized in the same turn (examples):
- `git clean -fd`, `git clean -xdf`
- `git reset --hard`
- deleting non-temporary files via `rm`, `del`, `Remove-Item`

If cleanup/reset is authorized:
1. Make it reversible first: `git stash push -u -m "SAFETY: before <operation>"`
2. Preview deletions: `git clean -nd`
3. Proceed only with explicit Operator confirmation.

Sync gate (project-policy-dependent, but kernel-ready):
- If the Codex/Protocol forbids sync actions by default, an agent MUST request explicit authorization before:
  - `git fetch origin` (network)
  - `git switch ...`
  - `git merge` / `git rebase` / fast-forward pulls

## 5. Session â€œenvironment hard gateâ€ (recommended)

For deterministic safety, a role protocol SHOULD require the agent to capture the repo state before work:
- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Rationale: prevents work being performed in the wrong worktree/branch, which is a primary failure mode when models hand off mid-task.



