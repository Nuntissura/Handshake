# Work Packet Stub: WP-1-Product-Governance-Snapshot-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Governance-Snapshot-v1
- BASE_WP_ID: WP-1-Product-Governance-Snapshot
- CREATED_AT: 2026-02-05
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: §7.5.4.8–§7.5.4.9 (Governance Pack: Instantiation + Template Volume)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md:7.5.4.8 (Governance Pack: Project-Specific Instantiation)
  - Handshake_Master_Spec_v02.123.md:7.5.4.9 (Governance Pack: Template Volume)
  - Handshake_Master_Spec_v02.123.md:Role Mailbox export requirements (export manifest + root constraints)

## INTENT (DRAFT)
- NOTE: The WP ID includes the word "Snapshot" for historical naming reasons. This WP is about a **compatibility `docs/` bundle** (short-term) and a migration to **product-embedded governance resources** (end state). Do not rename the file.
- What: Maintain a short-term **compatibility governance bundle** under `docs/` (matching current runtime/tests), and implement an end-state where governance defaults/templates are shipped **inside the product** (not in repo folders).
- Why: Repo governance in `.GOV/` is evolving rapidly and must remain decoupled from runtime. Product behavior must be deterministic and portable without depending on `.GOV/` or repo-local `docs/` at runtime.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Identify the minimal runtime-critical governance surface currently hardcoded by runtime/tests (e.g., `docs/SPEC_CURRENT.md`, `docs/TASK_BOARD.md`, `docs/OSS_REGISTER.md`, `docs/ROLE_MAILBOX/**`, plus any other required files).
  - Define `docs/` as a **compatibility bundle only** (short-term): it exists to keep the current product working while we migrate.
  - Implement product-embedded governance resources (templates/defaults) so Handshake can bootstrap/validate governance without reading repo folders.
  - Replace runtime/test dependencies on `docs/**` with product-embedded resources + product-owned state/config (portable, deterministic).
  - Add a hard guardrail: runtime MUST NOT touch `.GOV/` (no reads, no writes), enforced by CI/gates.
- OUT_OF_SCOPE:
  - Rewriting governance workflows in `.GOV/` (this WP is product/runtime decoupling work, not governance authoring refactors).
  - Changing WP/task-packet lifecycle semantics.

## ACCEPTANCE_CRITERIA (DRAFT)
- Short-term (compatibility):
  - A fresh clone/build of Handshake can run current tests/flows that still expect `docs/**` to exist.
- End-state (required for completion):
  - Handshake runtime does not require repo `docs/` to exist for governance-critical behavior.
  - Handshake runtime does not read/write `.GOV/` (hard enforcement).
  - Governance defaults/templates used by the product are shipped inside the product (embedded/bundled resources) and versioned with the code.
  - Any governance *state* is stored in product-owned storage (DB/workspace data dir), not in repo folders.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Identify every runtime/test hardcoded governance path (grep + confirm intent).
- Agree on the end-state resource model for embedded/bundled templates (Rust include/embed vs packaged resources).
- Decide the product-owned on-disk state location (must not be `.GOV/`; avoid `docs/`).

## RISKS / UNKNOWNs (DRAFT)
- Confusion between “governance workspace” (`.GOV/`) and “compatibility bundle” (`docs/`) without hard guardrails.
- Existing code paths may assume repo-root relative paths; must be migrated to embedded resources/configured data dirs.
- Cross-platform path/case differences and packaging differences (Windows dev vs Linux CI).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Governance-Snapshot-v1.md` (approved/signed). (Keep filename; do not rename.)
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Governance-Snapshot-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
