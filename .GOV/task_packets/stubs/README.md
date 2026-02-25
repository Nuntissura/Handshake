# Task Packet Stubs (Backlog)

This folder contains **Work Packet stubs**: lightweight placeholders to track Phase items without triggering the full Work Packet lifecycle (Technical Refinement, USER_SIGNATURE lock, and deterministic gates).

Stubs are legitimate backlog items, but they are **not** executable task packets:
- They are not consumed by `just pre-work` / `just post-work`.
- They may contain placeholders and draft scope.
- They MUST be activated into an official packet in `.GOV/task_packets/` before any coding starts.
- For current-spec Phase 1 roadmap additions (`[ADD v<current>]`), stubs MUST include `ROADMAP_ADD_COVERAGE` metadata with exact spec line numbers (`SPEC=...; PHASE=7.6.3; LINES=...`) so `just gov-check` can verify full coverage.

Activation procedure (summary):
1) Technical Refinement Block in chat (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`)
2) USER_SIGNATURE approval
3) Create `.GOV/refinements/WP-*.md`
4) Create official task packet via `just create-task-packet WP-*`
5) Move Task Board entry out of STUB

