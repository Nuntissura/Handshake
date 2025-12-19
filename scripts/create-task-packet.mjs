#!/usr/bin/env node
/**
 * Task packet generator [CX-580-581]
 * Creates a task packet from template
 */

import fs from 'fs';
import path from 'path';

const WP_ID = process.argv[2];

if (!WP_ID || !WP_ID.startsWith('WP-')) {
  console.error('❌ Usage: node create-task-packet.mjs WP-{phase}-{name}');
  console.error('Example: node create-task-packet.mjs WP-1-Job-Cancel');
  process.exit(1);
}

// Ensure directory exists
const taskPacketDir = 'docs/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
  console.log(`Created directory: ${taskPacketDir}/`);
}

const fileName = `${WP_ID}.md`;
const filePath = path.join(taskPacketDir, fileName);

// Check if file already exists
if (fs.existsSync(filePath)) {
  console.error(`❌ Task packet already exists: ${filePath}`);
  console.error('Edit the existing file or use a different WP_ID.');
  process.exit(1);
}

// Get current timestamp
const timestamp = new Date().toISOString();

// Template content
const template = `# Task Packet: ${WP_ID}

## Metadata
- TASK_ID: ${WP_ID}
- STATUS: Backlog
- DATE: ${timestamp}
- REQUESTOR: {user or source}
- AGENT_ID: {orchestrator agent ID}
- ROLE: Orchestrator

## Scope
- **What**: {1-2 sentence description of what needs to be done}
- **Why**: {Business/technical rationale for this work}
- **IN_SCOPE_PATHS**:
  * {specific file or directory that will be modified}
  * {another specific path}
- **OUT_OF_SCOPE**:
  * {what should NOT be changed in this task}
  * {work deferred to future tasks}

## Quality Gate
- **RISK_TIER**: {LOW | MEDIUM | HIGH}
  - LOW: Docs-only or comments; no behavior change
  - MEDIUM: Code change within one module; no schema/IPC changes
  - HIGH: Cross-module, IPC, migrations, auth/security, dependency updates
- **TEST_PLAN**:
  \`\`\`bash
  # Commands coder MUST run before claiming done:
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  pnpm -C app run lint
  cargo clippy --all-targets --all-features
  just ai-review  # Required for MEDIUM/HIGH
  \`\`\`
- **DONE_MEANS**:
  * {Specific measurable criterion 1}
  * {Specific measurable criterion 2}
  * All tests pass
  * Validation commands complete successfully
- **ROLLBACK_HINT**:
  \`\`\`bash
  git revert <commit-sha>
  # OR: Specific undo steps if needed
  \`\`\`

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * docs/ARCHITECTURE.md
  * {5-10 implementation-specific files the coder should inspect}
- **SEARCH_TERMS**:
  * "{key symbol or function name}"
  * "{error message if debugging}"
  * "{feature name}"
  * "{5-20 exact strings to grep}"
- **RUN_COMMANDS**:
  \`\`\`bash
  just dev  # Start development environment
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  \`\`\`
- **RISK_MAP**:
  * "{potential failure mode 1}" -> "{affected subsystem}"
  * "{potential failure mode 2}" -> "{affected subsystem}"
  * "{3-8 identified risks}" -> "{where they impact}"

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: (optional; milestones/hard bugs only)
- **ADRs**: {if relevant to this work}

## Notes
- **Assumptions**: {Mark any assumptions as "ASSUMPTION: ..."}
- **Open Questions**: {Questions that need resolution before or during work}
- **Dependencies**: {Other work this depends on, if any}

## Validation
- Command:
- Result:
- Notes:

## Status / Handoff
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:
`;

// Write the file
fs.writeFileSync(filePath, template, 'utf8');

console.log(`✅ Task packet created: ${filePath}`);
console.log('');
console.log('Next steps:');
console.log('1. Edit the file and fill in all {placeholder} values');
console.log('2. Update docs/TASK_BOARD.md to "Ready for Dev"');
console.log('3. Verify completeness: just pre-work ' + WP_ID);
console.log('4. Delegate to coder with packet path');
console.log('');
console.log('Template fields to complete:');
console.log('- Metadata: REQUESTOR, AGENT_ID');
console.log('- Scope: What, Why, IN_SCOPE_PATHS, OUT_OF_SCOPE');
console.log('- RISK_TIER: Choose LOW/MEDIUM/HIGH');
console.log('- TEST_PLAN: List specific commands');
console.log('- DONE_MEANS: Define success criteria');
console.log('- BOOTSTRAP: Fill in FILES_TO_OPEN, SEARCH_TERMS, RISK_MAP');
console.log('- Authority: Update Task Board / optional logger reference');
