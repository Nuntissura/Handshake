# Validator Workflow File-Touch Map (Repo Governance)

This document maps what the Validator workflow reads/writes so the Operator can:
- sanity-check scope boundaries (governance vs product),
- and spot drive/host-specific path leaks early.

All paths in this map are repo-relative and must remain drive-agnostic.

## Always (Session Start / Context Check)

Read-only (evidence/context):
- `.GOV/codex/Handshake_Codex_v1.4.md`
- `AGENTS.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/spec/SPEC_CURRENT.md`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- resolved Work Packet path for the target WP (logical `.GOV/work_packets/WP-*/packet.md`; current physical `.GOV/task_packets/WP-*/packet.md`; legacy flat `.GOV/task_packets/WP-*.md`)

Git metadata (read-only, via `git ...`):
- `.git/*` (including worktree metadata under `.git/worktrees/*` when using worktrees)

## Command Map (Validator)

Removed 2026-09-23: the command surface was deleted with the governance harness.
