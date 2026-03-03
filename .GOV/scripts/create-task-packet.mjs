#!/usr/bin/env node
/**
 * Task packet generator [CX-580-581]
 * Creates a task packet from template
 */

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
  defaultRefinementPath,
  resolveSpecCurrent,
  validateRefinementFile,
} from './validation/refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID || !WP_ID.startsWith('WP-')) {
  console.error('ERROR: Usage: node create-task-packet.mjs WP-{phase}-{name}');
  console.error('Example: node create-task-packet.mjs WP-1-Job-Cancel');
  process.exit(1);
}

function printGateBlocks({ wpId, stage, next, operatorAction, gateRan, result, why, gateOutputLines, nextCommands }) {
  console.log('LIFECYCLE [CX-LIFE-001]');
  console.log(`- WP_ID: ${wpId}`);
  console.log(`- STAGE: ${stage}`);
  console.log(`- NEXT: ${next}`);
  console.log('');
  console.log(`OPERATOR_ACTION: ${operatorAction || 'NONE'}`);
  console.log('');
  console.log('GATE_OUTPUT [CX-GATE-UX-001]');
  for (const line of gateOutputLines || []) console.log(line);
  console.log('');
  console.log('GATE_STATUS [CX-GATE-UX-001]');
  console.log(`- PHASE: ${stage}`);
  console.log(`- GATE_RAN: ${gateRan}`);
  console.log(`- RESULT: ${result}`);
  console.log(`- WHY: ${why}`);
  console.log('');
  console.log('NEXT_COMMANDS [CX-GATE-UX-001]');
  for (const cmd of nextCommands || []) console.log(`- ${cmd}`);
}

// HARD GATE: Technical Refinement must exist and be signed before packet creation.
const refinementsDir = path.join('.GOV', 'refinements');
if (!fs.existsSync(refinementsDir)) {
  fs.mkdirSync(refinementsDir, { recursive: true });
}

const refinementPath = defaultRefinementPath(WP_ID);
let userSignature = '';

if (!fs.existsSync(refinementPath)) {
  const refinementTemplatePath = path.join('.GOV', 'templates', 'REFINEMENT_TEMPLATE.md');
  if (!fs.existsSync(refinementTemplatePath)) {
    console.error(`Missing refinement template: ${refinementTemplatePath}`);
    process.exit(1);
  }

  let resolved = null;
  try {
    resolved = resolveSpecCurrent();
  } catch {
    // Still create a scaffold deterministically; validation will fail until SPEC_CURRENT is resolvable.
  }

  const ts = new Date().toISOString();
  const raw = fs.readFileSync(refinementTemplatePath, 'utf8');
  const filled = raw
    .split('{{WP_ID}}').join(WP_ID)
    .split('{{DATE_ISO}}').join(ts)
    .split('{{SPEC_TARGET_RESOLVED}}').join(resolved ? resolved.specFileName : 'Handshake_Master_Spec_vXX.XX.md')
    .split('{{SPEC_TARGET_SHA1}}').join(resolved ? resolved.sha1 : '<fill>');

  fs.writeFileSync(refinementPath, filled, 'utf8');

  printGateBlocks({
    wpId: WP_ID,
    stage: 'REFINEMENT',
    next: 'REFINEMENT',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Refinement scaffold created; complete refinement + review + signature before packet creation.',
    gateOutputLines: [
      'BLOCKED: Technical Refinement must be completed BEFORE task packet creation.',
      `Created refinement scaffold: ${refinementPath.replace(/\\/g, '/')}`,
    ],
    nextCommands: [
      `cat ${refinementPath.replace(/\\/g, '/')}`,
      '# Fill the refinement (ASCII-only; token-in-window per SPEC_ANCHOR).',
      '# Present refinement to the user (do NOT ask for signature in the same turn).',
      `just record-refinement ${WP_ID}`,
      '# After explicit user review in a NEW turn:',
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`,
      '# After signature:',
      `just orchestrator-prepare-and-packet ${WP_ID} {Coder-A|Coder-B}`,
      '# If you only scaffolded refinement (no packet yet), re-run when unblocked:',
      `just create-task-packet ${WP_ID}`,
    ],
  });
  process.exit(2);
}

const refinementValidation = validateRefinementFile(refinementPath, { expectedWpId: WP_ID, requireSignature: true });
if (!refinementValidation.ok) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'SIGNATURE',
    next: 'SIGNATURE',
    operatorAction: `Collect explicit approval + one-time signature for ${WP_ID}`,
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Refinement is not approved/signed; task packet creation is forbidden until the refinement signature gate passes.',
    gateOutputLines: [
      `BLOCKED: Refinement is not approved/signed: ${refinementPath.replace(/\\/g, '/')}`,
      ...((refinementValidation.errors || []).map((e) => `- ${e}`)),
    ],
    nextCommands: [
      `cat ${refinementPath.replace(/\\/g, '/')}`,
      `just record-refinement ${WP_ID}`,
      '# After explicit user approval in a NEW turn:',
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`,
      `just create-task-packet ${WP_ID}`,
    ],
  });
  process.exit(1);
}

userSignature = refinementValidation.parsed.signature;

// HARD GATE: if refinement indicates enrichment is needed, do not create a task packet.
try {
  const refinementContent = fs.readFileSync(refinementPath, 'utf8');
  const m = refinementContent.match(/^\s*-\s*ENRICHMENT_NEEDED\s*:\s*(YES|NO)\s*$/mi);
  const enrichmentNeeded = (m?.[1] || '').toUpperCase();
  if (enrichmentNeeded === 'YES') {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'SIGNATURE',
      next: 'STOP',
      operatorAction: 'Spec enrichment required (create new spec version + new WP variant)',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'Refinement declares ENRICHMENT_NEEDED=YES; do not create/lock a task packet until enrichment is completed.',
      gateOutputLines: [
        `BLOCKED: ${WP_ID} refinement declares ENRICHMENT_NEEDED=YES.`,
        'Do NOT create/lock a WP packet while enrichment is required.',
      ],
      nextCommands: [
        '# Run the spec enrichment workflow (new spec version file + update .GOV/roles_shared/SPEC_CURRENT.md).',
        '# Create a NEW WP variant anchored to the updated spec (new WP_ID; new one-time signature).',
      ],
    });
    process.exit(1);
  }
} catch {
  // If refinement cannot be read, earlier validation would have failed; keep defensive behavior deterministic.
  printGateBlocks({
    wpId: WP_ID,
    stage: 'REFINEMENT',
    next: 'REFINEMENT',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Unable to read refinement file; cannot proceed deterministically.',
    gateOutputLines: [
      `BLOCKED: Unable to read refinement file: ${refinementPath.replace(/\\/g, '/')}`,
    ],
    nextCommands: [
      `cat ${refinementPath.replace(/\\/g, '/')}`,
    ],
  });
  process.exit(1);
}

// Gate: signature must be recorded in ORCHESTRATOR_GATES.json (prevents manual bypass).
try {
  const gatesPath = path.join('.GOV', 'roles', 'orchestrator', 'ORCHESTRATOR_GATES.json');
  const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
  const logs = Array.isArray(gates.gate_logs) ? gates.gate_logs : [];
  const lastSig = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'SIGNATURE');
  if (!lastSig) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'SIGNATURE',
      next: 'SIGNATURE',
      operatorAction: `Record signature via Orchestrator gate for ${WP_ID}`,
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'No signature record found in ORCHESTRATOR_GATES.json; packet creation cannot be bypassed by manual edits.',
      gateOutputLines: [
        `BLOCKED: No signature record found for ${WP_ID} in ${gatesPath.replace(/\\/g, '/')}.`,
        'Note: If USER_SIGNATURE was manually edited into the refinement file, revert it back to <pending> first.',
      ],
      nextCommands: [
        `cat ${refinementPath.replace(/\\/g, '/')}`,
        `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`,
        `just create-task-packet ${WP_ID}`,
      ],
    });
    process.exit(1);
  }
  if (lastSig.signature !== userSignature) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'SIGNATURE',
      next: 'SIGNATURE',
      operatorAction: 'Resolve signature mismatch (refinement vs gate log)',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'Refinement USER_SIGNATURE does not match the recorded SIGNATURE gate; packet creation is forbidden.',
      gateOutputLines: [
        `BLOCKED: Signature mismatch between refinement (${userSignature}) and gate log (${lastSig.signature}).`,
      ],
      nextCommands: [
        `cat ${refinementPath.replace(/\\/g, '/')}`,
        `# If refinement was manually edited, revert USER_SIGNATURE to <pending> and re-run: just record-signature ${WP_ID} {newSignature}.`,
      ],
    });
    process.exit(1);
  }

  // HARD GATE: worktree + coder assignment must be recorded AFTER signature and BEFORE packet creation.
  const lastPrepare = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'PREPARE');
  if (!lastPrepare) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'PREPARE',
      next: 'PREPARE',
      operatorAction: `Choose coder assignment (Coder-A|Coder-B) for ${WP_ID}`,
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'WP worktree/branch + coder assignment must be recorded AFTER signature and BEFORE packet creation.',
      gateOutputLines: [
        `BLOCKED: WP branch/worktree + coder assignment not recorded for ${WP_ID}.`,
      ],
      nextCommands: [
        `just orchestrator-prepare-and-packet ${WP_ID} {Coder-A|Coder-B}`,
        `# (or run separately) just worktree-add ${WP_ID}`,
        `# (or run separately) just record-prepare ${WP_ID} {Coder-A|Coder-B}`,
      ],
    });
    process.exit(1);
  }
  try {
    const sigTs = Date.parse(lastSig.timestamp);
    const prepTs = Date.parse(lastPrepare.timestamp);
    if (!Number.isNaN(sigTs) && !Number.isNaN(prepTs) && prepTs <= sigTs) {
      printGateBlocks({
        wpId: WP_ID,
        stage: 'PREPARE',
        next: 'PREPARE',
        operatorAction: `Re-run PREPARE for ${WP_ID}`,
        gateRan: `just create-task-packet ${WP_ID}`,
        result: 'BLOCKED',
        why: 'PREPARE record must occur after SIGNATURE; ordering check failed.',
        gateOutputLines: [
          `BLOCKED: PREPARE record must occur after SIGNATURE for ${WP_ID}.`,
          `- signature_ts=${lastSig.timestamp}`,
          `- prepare_ts=${lastPrepare.timestamp}`,
        ],
        nextCommands: [
          `just record-prepare ${WP_ID} {Coder-A|Coder-B}`,
          `just create-task-packet ${WP_ID}`,
        ],
      });
      process.exit(1);
    }
  } catch {
    // If timestamps are unparsable, treat as blocked to preserve determinism.
    printGateBlocks({
      wpId: WP_ID,
      stage: 'PREPARE',
      next: 'PREPARE',
      operatorAction: `Re-run PREPARE for ${WP_ID}`,
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'Unable to verify PREPARE ordering deterministically; re-record PREPARE.',
      gateOutputLines: [
        `BLOCKED: Unable to verify PREPARE ordering for ${WP_ID}.`,
      ],
      nextCommands: [
        `just record-prepare ${WP_ID} {Coder-A|Coder-B}`,
        `just create-task-packet ${WP_ID}`,
      ],
    });
    process.exit(1);
  }
} catch {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'SIGNATURE',
    next: 'SIGNATURE',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Unable to verify signature in ORCHESTRATOR_GATES.json; cannot proceed deterministically.',
    gateOutputLines: [
      'BLOCKED: Unable to verify signature in .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json.',
    ],
    nextCommands: [
      'cat .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
    ],
  });
  process.exit(1);
}

// Gate: signature must be present in SIGNATURE_AUDIT.md (protocol requirement).
try {
  const auditPath = path.join('.GOV', 'roles_shared', 'SIGNATURE_AUDIT.md');
  const audit = fs.readFileSync(auditPath, 'utf8');
  if (!audit.includes(`| ${userSignature} |`)) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'SIGNATURE',
      next: 'SIGNATURE',
      operatorAction: 'NONE',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'Signature not present in SIGNATURE_AUDIT.md; signature gate is incomplete.',
      gateOutputLines: [
        `BLOCKED: Signature not found in ${auditPath.replace(/\\/g, '/')}.`,
      ],
      nextCommands: [
        `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`,
        `just create-task-packet ${WP_ID}`,
      ],
    });
    process.exit(1);
  }
} catch {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'SIGNATURE',
    next: 'SIGNATURE',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Unable to verify signature in SIGNATURE_AUDIT.md; cannot proceed deterministically.',
    gateOutputLines: [
      'BLOCKED: Unable to verify signature in .GOV/roles_shared/SIGNATURE_AUDIT.md.',
    ],
    nextCommands: [
      'cat .GOV/roles_shared/SIGNATURE_AUDIT.md',
    ],
  });
  process.exit(1);
}

// Ensure directory exists
const taskPacketDir = '.GOV/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
  console.log(`Created directory: ${taskPacketDir}/`);
}

const fileName = `${WP_ID}.md`;
const filePath = path.join(taskPacketDir, fileName);

// Check if file already exists
if (fs.existsSync(filePath)) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'STOP',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'FAIL',
    why: 'Task packet file already exists; generator will not overwrite it.',
    gateOutputLines: [
      `FAIL: Task packet already exists: ${filePath.replace(/\\/g, '/')}`,
    ],
    nextCommands: [
      `cat ${filePath.replace(/\\/g, '/')}`,
      '# If you need a revision, create a new packet ID: WP-...-v{N}.',
    ],
  });
  process.exit(1);
}

// Get current timestamp
const timestamp = new Date().toISOString();

let mergeBaseSha = '<fill>';
try {
  const out = execSync('git merge-base main HEAD', { encoding: 'utf8' }).trim();
  if (/^[a-f0-9]{40}$/i.test(out)) mergeBaseSha = out.toLowerCase();
} catch {
  // Leave as <fill> if merge-base cannot be resolved (e.g., unusual repo state).
}

// Template content (canonical)
const templatePath = path.join('.GOV', 'templates', 'TASK_PACKET_TEMPLATE.md');
if (!fs.existsSync(templatePath)) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'STOP',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'FAIL',
    why: 'Task packet template is missing; cannot generate packet deterministically.',
    gateOutputLines: [
      `FAIL: Missing template: ${templatePath.replace(/\\/g, '/')}`,
    ],
    nextCommands: [
      'ls .GOV/templates',
    ],
  });
  process.exit(1);
}

const rawTemplate = fs.readFileSync(templatePath, 'utf8');
const templateLines = rawTemplate.split('\n');
const templateStartIdx = templateLines.findIndex((line) => line.startsWith('# Task Packet:'));
const templateBody = templateStartIdx === -1
  ? rawTemplate
  : templateLines.slice(templateStartIdx).join('\n');

  let specBaseline = 'Handshake_Master_Spec_vXX.XX.md';
  try {
    const specCurrent = fs.readFileSync(path.join('.GOV', 'roles_shared', 'SPEC_CURRENT.md'), 'utf8');
    const m = specCurrent.match(/Handshake_Master_Spec_v[0-9.]+\.md/);
    if (m) specBaseline = m[0];
  } catch {
    // Leave placeholder if SPEC_CURRENT cannot be read or parsed.
  }

const fill = (text, token, value) => text.split(token).join(value);

let template = templateBody;
template = fill(template, '{{WP_ID}}', WP_ID);
template = fill(template, '{{DATE_ISO}}', timestamp);
template = fill(template, '{{MERGE_BASE_SHA}}', mergeBaseSha);
template = fill(template, '{{SPEC_BASELINE}}', specBaseline);
template = fill(template, '{{REQUESTOR}}', '{user or source}');
template = fill(template, '{{AGENT_ID}}', '{orchestrator agent ID}');
template = fill(template, '{{USER_SIGNATURE}}', userSignature);
template = fill(template, '{{SPEC_ANCHOR}}', '<fill>');

// Write the file
fs.writeFileSync(filePath, template, 'utf8');

{
  const baseWpId = WP_ID.replace(/-v\d+$/, '');
  const isRevision = baseWpId !== WP_ID;

  const nextCommands = [
    `cat ${filePath.replace(/\\/g, '/')}`,
    '# Fill placeholders: REQUESTOR, AGENT_ID, SCOPE, RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, SPEC_ANCHOR.',
    `just task-board-set ${WP_ID} READY_FOR_DEV`,
  ];
  if (isRevision) {
    nextCommands.push(`just wp-traceability-set ${baseWpId} ${WP_ID}`);
  }
  nextCommands.push(`just pre-work ${WP_ID}`);
  nextCommands.push('# Delegate to Coder with packet path + assigned worktree/branch from ORCHESTRATOR_GATES.json PREPARE.');

  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'PRE_WORK',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'PASS',
    why: 'Task packet created from template.',
    gateOutputLines: [
      `OK: Task packet created: ${filePath.replace(/\\/g, '/')}`,
    ],
    nextCommands,
  });
}
