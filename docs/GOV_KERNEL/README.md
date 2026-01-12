# Governance Kernel Spec (Project-Agnostic)

This directory defines a **project-agnostic, mechanically gated governance system** intended for:
- Multi-role separation of duties (Operator / Orchestrator / Coder / Validator).
- Deterministic execution with auditability (“evidence-first”).
- Reliable handoff between **small-context local models** and **large-context cloud models**.

This is a **kernel**: it specifies the *minimum standardized artifacts, file formats, gate semantics, and interlocks* that make the workflow portable across projects.

Non-goals:
- This does not define your product architecture or feature requirements.
- This does not replace your project’s “law stack” (Codex + Master Spec + role protocols). It defines how those documents must be structured and mechanically enforced.

## Files (normative)
- `docs/GOV_KERNEL/01_AUTHORITY_AND_ROLES.md`: authority stack, role boundaries, branch/worktree rules.
- `docs/GOV_KERNEL/02_ARTIFACTS_AND_CONTRACTS.md`: canonical governance artifacts (files/dirs), required headings/fields, and failure modes when missing.
- `docs/GOV_KERNEL/03_GATES_AND_ENFORCERS.md`: gate semantics and state machines for Orchestrator/Coder/Validator enforcement scripts.
- `docs/GOV_KERNEL/04_SMALL_CONTEXT_HANDOFF.md`: how to packetize work so any model can continue deterministically.
- `docs/GOV_KERNEL/05_CI_HOOKS_AND_CONFIG.md`: CI parity, hooks, and determinism config surface.
- `docs/GOV_KERNEL/06_VERSIONING_AND_DRIFT_CONTROL.md`: versioning rules and drift prevention across docs/tools.

## Files (non-normative)
- `docs/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`: **example instantiation** mapping a concrete repo (Handshake) to this kernel, including a full inventory of governing files and scripts.

## Conformance model
A project “implements this kernel” if:
1. The canonical artifacts exist with the required structure and determinism constraints.
2. The gate scripts (or equivalent tooling) enforce the same semantics (it can be different code, but must enforce the same contract).
3. A fresh agent can start from the entrypoints and reliably reproduce: *what is the current spec*, *what work is authorized*, *what is in scope*, *what evidence exists*, and *what gates remain*.
