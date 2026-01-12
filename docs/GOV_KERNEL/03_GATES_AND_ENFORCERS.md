# 03) Gates and Enforcers (Kernel)

This kernel assumes governance is enforced by **mechanical gates** (scripts/hooks/CI) rather than by convention.

Design principle:
- Artifacts define authority.
- Gates make authority executable by rejecting drift and “momentum failures”.
- State transitions are recorded in append-only or monotonic state files.

Implementation note:
- A project MAY implement gates using any tooling (Node, Python, Rust, shell), but the **semantics** below are normative if the project claims kernel conformance.

## 1. Single command surface (recommended)

Kernel recommendation: expose all governance commands via a single command surface (example: `justfile`).

Rationale:
- Small-context agents can follow deterministic commands without rediscovering ad-hoc scripts.
- CI can reuse the same command surface for parity.

Minimum recommended commands (names may be standardized across projects):
- `record-refinement <WP_ID>`
- `record-signature <WP_ID> <signature>`
- `record-prepare <WP_ID> <coder_id> [branch] [worktree_dir]`
- `create-task-packet <WP_ID>`
- `gate-check <WP_ID>`
- `pre-work <WP_ID>`
- `post-work <WP_ID>`
- `validator-gate-*` (validator state machine)

## 2. Orchestrator gates (REFINEMENT → SIGNATURE → PREPARE)

Kernel objective: prevent creating an “executable packet” until a WP is demonstrably anchored to the spec and explicitly approved.

### 2.1 Gate state file (normative)
The Orchestrator gate tool MUST persist a state file (example path: `docs/ORCHESTRATOR_GATES.json`) with an append-only log, at minimum:
- `wpId`
- `type` in `{REFINEMENT, SIGNATURE, PREPARE}`
- `timestamp` (ISO-8601)
- additional fields per gate (below)

### 2.2 REFINEMENT gate (recording)
Inputs:
- `WP_ID`
- refinement file path (optional)

Required checks:
- WP_ID has canonical form (project-defined, but stable and parseable; commonly `WP-...`).
- Refinement file passes `refinement-check` structural validation with `requireSignature=false`.
- If the Master Spec pointer is resolvable, record:
  - resolved spec file name
  - resolved spec SHA1

Required writes:
- Append a `REFINEMENT` entry to Orchestrator gate logs.

Required behavior:
- Must output a “gate locked” warning: signatures MUST NOT be requested/recorded in the same turn as refinement recording.

### 2.3 SIGNATURE gate (one-time signature consumption)
Inputs:
- `WP_ID`
- `signature` token (project-defined; must be unambiguous and reproducible)

Required checks:
- A REFINEMENT gate entry exists for this WP.
- **Anti-momentum**: signature must not be recorded too soon after refinement (time-based minimum interval or equivalent).
- Refinement passes structural validation with `requireSignature=false`.
- Refinement declares `ENRICHMENT_NEEDED=NO` (if enrichment is required, signing is forbidden).
- Refinement contains deterministic `USER_APPROVAL_EVIDENCE` matching a required literal string (example pattern: `APPROVE REFINEMENT <WP_ID>`).
- Refinement is not already signed.
- Signature is one-time use (must not already appear anywhere in repo history/surface as defined by project policy).

Required writes:
- Update refinement file:
  - `USER_REVIEW_STATUS: APPROVED`
  - `USER_SIGNATURE: <signature>`
- Append a signature record to an append-only audit file (example: `docs/SIGNATURE_AUDIT.md`).
- Append a `SIGNATURE` entry in `docs/ORCHESTRATOR_GATES.json` including:
  - signature
  - refinement path

Required behavior:
- Must instruct the operator/agent that packet creation is still blocked until PREPARE is recorded.

### 2.4 PREPARE gate (assignment + branch/worktree readiness)
Purpose:
- Prevent “coding without a home”: packet creation must be blocked until the WP branch/worktree exists and a coder is assigned.

Required checks:
- A SIGNATURE entry exists for the WP.
- A WP branch exists locally (name derived from WP_ID or explicitly provided).
- A git worktree exists for that branch (required when concurrency rules demand it).

Required writes:
- Append a `PREPARE` entry including:
  - `coder_id`
  - `branch`
  - `worktree_dir`

## 3. Packet creation gate (`create-task-packet`)

Kernel objective: a task packet is an “executable contract”. Creating it must be impossible to do “early”.

Required checks before writing `docs/task_packets/<WP_ID>.md`:
- A refinement file exists; if missing, tooling SHOULD create a scaffold and then HARD-BLOCK (exit non-zero) until refinement is completed and reviewed.
- Refinement is approved/signed and signature is present.
- Refinement declares `ENRICHMENT_NEEDED=NO`.
- The signature exists in:
  - the refinement file,
  - the Orchestrator gate state log,
  - the signature audit log.
- A PREPARE record exists after the SIGNATURE record.

Required behavior:
- Create the packet from the canonical template.
- Populate provenance fields (e.g., `SPEC_BASELINE`) deterministically from the resolved spec pointer where possible.

## 4. Coder phase gate (`gate-check`)

Kernel objective: enforce “interface-first” sequencing and prevent merged phases.

Required checks (conceptual):
- BOOTSTRAP must exist before SKELETON.
- A literal `SKELETON APPROVED` marker must exist (outside code fences) before implementation evidence is accepted.
- Gate parsing must ignore fenced code blocks to avoid false positives.

Failure modes prevented:
- “Implemented while still designing” (hard to audit).
- “Turn merging” where a model writes design + implementation without a review checkpoint.

## 5. Pre-work gate (`pre-work`)

Kernel objective: block implementation until the work contract is complete, signed, and checkpointed.

Required checks:
- Activated task packet exists for WP_ID.
- Packet includes required structural fields (scope + test plan + done means + bootstrap).
- If the packet is not explicitly Done/Validated:
  - Refinement exists and is signed.
  - Packet USER_SIGNATURE matches refinement signature.
  - Signature exists in signature audit log.
- **Checkpoint commit gate**: packet and refinement must exist in `HEAD` (prevents loss of untracked artifacts).
- Packet contains a deterministic validation manifest template (COR-701-style fields) to enable post-work validation.

## 6. Post-work gate (`post-work`)

Kernel objective: make changes auditable by forcing a per-file manifest and verifying it against the git diff.

Minimum required semantics:
- For every changed non-doc file, there must be a manifest block in the packet validation section that includes:
  - target file path
  - start/end line window for intended changes
  - expected line delta
  - deterministic Pre-SHA1 and Post-SHA1
- Gate tooling must verify, at minimum:
  - the file exists and is openable
  - the diff is contained within declared windows (unless waivered)
  - the reported line delta matches git numstat delta
  - the pre/post hashes match the declared states (HEAD/INDEX policy is project-defined but must be consistent)

## 7. Validator gates (REPORT_PRESENTED → USER_ACKNOWLEDGED → WP_APPENDED → COMMITTED)

Kernel objective: make validation evidence visible to the Operator before allowing a commit/merge step.

Required state machine:
1. `present-report <WP_ID> <PASS|FAIL>`
2. `acknowledge <WP_ID>` (Operator acknowledges report was seen)
3. `append <WP_ID>` (validator appends report to packet)
4. `commit <WP_ID>` (PASS only; unlocks commit)

Required properties:
- Gate state stored in a deterministic JSON state file (example: `docs/VALIDATOR_GATES.json`).
- Anti-momentum interval between gate transitions.
- FAIL verdict must permanently block the commit gate for that WP_ID (must create new WP variant to re-pass).

## 8. Auxiliary governance checks (kernel-recommended)

These checks are not always required for kernel conformance, but they harden portability:
- **Task board format check**: enforces strict, machine-parseable WP state lines.
- **Task packet claim check**: when Status is `In Progress`, require Coder claim fields (model + reasoning strength) to be non-placeholder.
- **Worktree concurrency check**: detect multiple active WPs in one worktree (project-defined heuristic).
- **Spec-current check**: ensures `docs/SPEC_CURRENT.md` points to the newest spec version by version parsing policy.
- **Codex check**: detects forbidden patterns (architecture violations, unsafe APIs, debug prints) and codex drift across docs.

