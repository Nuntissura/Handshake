# Task Packets Directory

This directory contains task packets for AI-autonomous workflow enforcement.

## Purpose

Task packets are structured work items that define:
- **Scope**: What to change and what NOT to change
- **Quality Gate**: Risk tier, test plan, done criteria
- **Bootstrap**: Files to open, search terms, commands to run
- **Traceability**: Links to specs, WP_IDs, and status/validation recorded in the task packet + `docs/TASK_BOARD.md`

## Naming Convention

Task packet files follow the pattern:
```
WP-{phase}-{name}.md
```

Examples:
- `WP-1-Job-Cancel.md`
- `WP-2-Workflow-Status.md`
- `WP-Debug-Auth-Error.md`

## Creating Task Packets

**Orchestrators** create task packets using:
```bash
just create-task-packet WP-{phase}-{name}
```

This generates a task packet from template with all required fields.

**Required fields:**
- TASK_ID, DATE, REQUESTOR, AGENT_ID, ROLE
- SCOPE (What, Why, IN_SCOPE_PATHS, OUT_OF_SCOPE)
- RISK_TIER (LOW/MEDIUM/HIGH)
- TEST_PLAN (commands to run)
- DONE_MEANS (success criteria)
- ROLLBACK_HINT (how to undo)
- BOOTSTRAP (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP)
- AUTHORITY (SPEC_CURRENT, Codex, Task Board; logger only if requested for milestone/hard bug)

## Validation

Before starting work:
```bash
just pre-work WP-{ID}
```

Before committing:
```bash
just post-work WP-{ID}
```

Full workflow validation:
```bash
just validate-workflow WP-{ID}
```

## See Also

- [Handshake Codex v1.4](../../Handshake%20Codex%20v1.4.md) - Governance rules
- [ORCHESTRATOR_PROTOCOL.md](../ORCHESTRATOR_PROTOCOL.md) - Orchestrator checklist
- [CODER_PROTOCOL.md](../CODER_PROTOCOL.md) - Coder checklist
- [QUALITY_GATE.md](../QUALITY_GATE.md) - Gate 0 and Gate 1 requirements

