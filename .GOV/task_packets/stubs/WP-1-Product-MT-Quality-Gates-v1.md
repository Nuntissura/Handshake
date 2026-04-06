# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-MT-Quality-Gates-v1

## STUB_METADATA
- WP_ID: WP-1-Product-MT-Quality-Gates-v1
- BASE_WP_ID: WP-1-Product-MT-Quality-Gates
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Compile-Validation-Gate, WP-1-Product-MT-Task-Board
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Per-MT quality gates in the product execution pipeline
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec MT lifecycle and quality assurance
  - Handshake_Master_Spec session execution pipeline
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-106, Claude Agent Teams TaskCompleted hook pattern)

## INTENT (DRAFT)
- What: Per-MT quality gates in the product execution pipeline. When an MT is marked complete, the product runtime runs configurable quality checks (compilation, test filter, artifact hygiene, lint) before advancing the MT status. Failed gates block the MT and notify the coder with specific failure details. Based on Claude Agent Teams TaskCompleted hook pattern.
- Why: Quality issues caught after MT completion require expensive re-validation cycles. Running configurable quality checks at the MT completion boundary catches common defects (compilation failures, test regressions, lint violations, stale artifacts) before the work product reaches the validator, reducing wasted review cycles and token spend. The TaskCompleted hook pattern provides a clean extension point for pluggable quality checks.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Quality gate framework: pluggable check interface at the MT completion boundary.
  - Built-in gates: compilation check (delegates to Compile Validation Gate), test filter, artifact hygiene, lint.
  - Configurable gate set per project and per complexity tier.
  - Gate execution ordering: fast checks first, expensive checks gated behind earlier passes.
  - Structured failure output per gate: gate name, pass/fail, error details, suggested fix.
  - MT status blocked on gate failure; coder notified with structured failure details.
  - Flight Recorder events for gate-pass, gate-fail, and gate-skip per MT.
  - Gate configuration in project-level config (not hardcoded).
- OUT_OF_SCOPE:
  - The compilation check implementation itself (delegates to WP-1-Product-Compile-Validation-Gate).
  - Full test suite execution (test filter runs targeted tests, not full suite).
  - Custom gate plugin API for third-party gates (v1 supports built-in gates only).
  - Gate result caching across MTs.

## ACCEPTANCE_CRITERIA (DRAFT)
- When an MT is marked complete, the product runtime executes the configured quality gates before advancing MT status.
- Failed quality gates block the MT from advancing to review status.
- The coder session receives structured failure details per failed gate (gate name, error details, suggested fix).
- Quality gates execute in configured order with fast checks first; later gates are skipped if earlier gates fail.
- The compilation gate delegates to the Compile Validation Gate infrastructure.
- Test filter gate runs tests matching the MT's declared scope and reports failures.
- Artifact hygiene gate checks for stale build artifacts, uncommitted generated files, and other hygiene issues.
- Lint gate runs the project's configured linter and reports violations.
- Gate configuration is project-level and per-complexity-tier without code changes.
- Flight Recorder emits gate-pass, gate-fail, and gate-skip events per MT with timing and details.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Product-Compile-Validation-Gate for the compilation check implementation.
- Depends on WP-1-Product-MT-Task-Board for the MT status lifecycle and completion boundary.
- Requires project-level configuration for gate sets and ordering.
- No spec blockers identified.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Gate execution time adds latency to every MT completion; needs fast-check-first ordering and configurable timeouts.
- Risk: Overly strict gates may block valid MTs on stylistic issues; gate severity levels (blocking vs. advisory) may be needed.
- Risk: Test filter scope matching may be imprecise, running too many or too few tests.
- Unknown: Whether gates should run synchronously (blocking) or asynchronously (coder continues while gates run).
- Unknown: Optimal default gate set that balances quality improvement against latency cost.

## DISCOVERY_ORIGIN
- Source: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION
- RGF Reference: RGF-106
- Pattern: Claude Agent Teams TaskCompleted hook pattern with pluggable quality checks at the task completion boundary.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-MT-Quality-Gates-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-MT-Quality-Gates-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
