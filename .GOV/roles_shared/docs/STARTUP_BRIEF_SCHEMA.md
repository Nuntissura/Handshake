# Startup Brief Schema

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- PURPOSE: compact operational guidance compiled from repeated memory evidence

## Authority Boundary

Startup briefs are not protocol law. They are Memory-Manager-curated operational memory used at role startup to prevent repeated procedural mistakes, wrong topology assumptions, and avoidable tool failures.

When a startup brief conflicts with `AGENTS.md`, `.GOV/codex/Handshake_Codex_v1.4.md`, a role protocol, a signed packet, or live runtime truth, the higher authority wins and the brief must be repaired.

## Proposal And Authority Workflow

Memory Manager orders memory so repeated failures, operator corrections, stale assumptions, and high-access governance lessons become actionable instead of staying as passive recall entries.

Allowed Memory Manager outcomes:

- verified startup brief card updates for narrow anti-repeat behavior
- deterministic tooling-repair proposals when a failure should be prevented by a command/check
- governance refactor candidates when the fix requires protocol, task-board, Codex, lifecycle, or command-surface changes

Authority stays with the active workflow coordinator:

- `ORCHESTRATOR` reviews Memory Manager proposals and makes governance changes for `ORCHESTRATOR_MANAGED`
- `CLASSIC_ORCHESTRATOR` reviews Memory Manager proposals and makes governance changes for `MANUAL_RELAY`
- Memory Manager does not approve, validate, or implement protocol/task-board/Codex changes directly

## Required Sections

Every role startup brief must contain:

- `# <ROLE> Startup Brief`
- `## Status`
- `## Use`
- `## Action Cards`

Every action card must use this shape:

```markdown
### RAM-<ROLE>-<ACTION>-<NNN>

- ACTION: <UPPERCASE_ACTION>
- TRIGGER: <when this card applies>
- FAILURE_PATTERN: <repeated mistake or wrong assumption>
- DO: <specific action to take>
- DO_NOT: <specific action to avoid>
- VERIFY: <observable proof>
- SOURCE: <memory ids, changelog ids, or brief source>
```

## Shared Brief

`.GOV/roles_shared/docs/SHARED_STARTUP_BRIEF.md` uses the same action-card shape and may include cross-role cards for pathing, PowerShell, topology, toolcalling, and governance memory usage.

## Maintenance Rules

- Memory Manager may update startup brief files and proposal files only.
- Memory Manager must not edit role protocols, Codex law, task boards, packets, or product code.
- Repeated procedural memories should become action cards when the same role/action failure appears more than once or when the Operator explicitly reports recurring friction and the fix is operational rather than legal/protocol-level.
- Mechanical Memory Manager may report candidate cards; intelligent Memory Manager verifies and writes cards.
- Broader governance changes must be emitted as proposals for Orchestrator or Classic Orchestrator review, not silently applied by Memory Manager.
