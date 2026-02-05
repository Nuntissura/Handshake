# Task Packet Stub: WP-1-Governance-Kernel-Conformance-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-Governance-Kernel-Conformance-v1
- BASE_WP_ID: WP-1-Governance-Kernel-Conformance
- Created: 2026-01-12
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (currently Handshake_Master_Spec_v02.107.md)
- SPEC_ANCHOR_CANDIDATE: Handshake_Master_Spec_v02.107.md 7.5.4

## Why this stub exists
Handshake relies on a rigid, mechanically gated workflow so multi-role work stays auditable and so small-context local models can hand off deterministically.

This stub tracks the work required to bring repo tooling and governance surfaces into explicit conformance with the Governance Kernel introduced in Handshake_Master_Spec_v02.107.md 7.5.4 and documented in `.GOV/GOV_KERNEL/*`.

## Scope sketch (draft)
- In scope:
  - Remediate governance reference drift across CI/hooks/docs (Codex/spec/protocol filename/version references).
  - Ensure CI parity with the canonical local command surface (`just` targets) for governance checks.
  - Ensure template and shim paths are canonical and do not break deterministic gate tooling.
  - Add/adjust automated drift checks where needed to keep the kernel enforceable over time.
- Out of scope:
  - Product feature implementation (no `src/`, `app/`, `tests/` changes unless explicitly required by governance enforcement and approved).

## Known gaps / triggers (evidence pointers)
- Governance drift hazards are summarized in `.GOV/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md`.
- CI/hook messaging and some docs reference obsolete Codex versions (example string: "Codex v0.8").

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` Part 2.5.2).
2. USER_SIGNATURE.
3. Create `.GOV/refinements/WP-1-Governance-Kernel-Conformance-v1.md`.
4. Create official task packet via `just create-task-packet WP-1-Governance-Kernel-Conformance-v1`.
5. Update `.GOV/roles_shared/TASK_BOARD.md` to move `WP-1-Governance-Kernel-Conformance-v1` out of STUB when activated.


