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
} from '../../../roles_shared/checks/refinement-check.mjs';
import { ensureWpCommunications } from '../../../roles_shared/scripts/wp/ensure-wp-communications.mjs';
import {
  formatClauseProofPlanSection,
  formatCoderHandoffBriefSection,
  formatContractSurfacesSection,
  formatNotProvenAtRefinementTimeSection,
  formatSpecContextWindowsSection,
  formatValidatorHandoffBriefSection,
} from '../../../roles_shared/scripts/lib/refinement-brief-lib.mjs';
import {
  buildClauseClosureRows,
  deriveSharedSurfaceMonitoring,
  formatClauseClosureMatrixSection,
  formatSharedSurfaceMonitoringSection,
  formatSpecDebtStatusSection,
} from '../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs';
import {
  deriveSemanticProofAssets,
  formatSemanticProofAssetsSection,
} from '../../../roles_shared/scripts/lib/semantic-proof-lib.mjs';
import {
  communicationPathsForWp,
  EXECUTION_OWNER_VALUES,
  WORKFLOW_LANE_VALUES,
} from '../../../roles_shared/scripts/lib/wp-communications-lib.mjs';
import { preparedWorktreeSyncState } from '../../../roles_shared/scripts/lib/role-resume-utils.mjs';
import {
  buildRemoteBackupUrl,
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_SESSION_TOOL,
  CODEX_MODEL_ALIASES_ALLOWED,
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  EXECUTION_OWNER_RANGE_HELP,
  executionOwnerToPacketValue,
  MODEL_FAMILY_POLICY,
  PACKET_FORMAT_VERSION,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_REASONING_REQUIRED,
  ROLE_SESSION_RUNTIME,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
  SESSION_PLUGIN_REQUESTS_FILE,
  SESSION_REGISTRY_FILE,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_LAUNCH_POLICY,
} from '../../../roles_shared/scripts/session/session-policy.mjs';
import { GOV_ROOT_REPO_REL } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

const WP_ID = process.argv[2];
const allowOverwriteExisting = process.argv.includes('--overwrite-existing') || process.env.ALLOW_PACKET_OVERWRITE === '1';
const EXECUTION_OWNER_USAGE = `{${EXECUTION_OWNER_RANGE_HELP}}`;

if (!WP_ID || !WP_ID.startsWith('WP-')) {
  console.error(`ERROR: Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/create-task-packet.mjs WP-{phase}-{name}`);
  console.error(`Example: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/create-task-packet.mjs WP-1-Job-Cancel`);
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

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const match = text.match(re);
  return match ? match[1].trim() : '';
}

function detectCommunicationArtifactDrift(packetPath, packetText) {
  const communicationDir = parseSingleField(packetText, 'WP_COMMUNICATION_DIR');
  const threadFile = parseSingleField(packetText, 'WP_THREAD_FILE');
  const runtimeStatusFile = parseSingleField(packetText, 'WP_RUNTIME_STATUS_FILE');
  const receiptsFile = parseSingleField(packetText, 'WP_RECEIPTS_FILE');
  const declared = [communicationDir, threadFile, runtimeStatusFile, receiptsFile].filter(Boolean);
  if (declared.length === 0) return null;

  const missing = [];
  for (const target of [communicationDir, threadFile, runtimeStatusFile, receiptsFile].filter(Boolean)) {
    if (!fs.existsSync(target)) missing.push(target.replace(/\\/g, '/'));
  }
  if (declared.length !== 4 || missing.length > 0) {
    return {
      declaredCount: declared.length,
      missing,
      packetPath: packetPath.replace(/\\/g, '/'),
    };
  }
  return null;
}

// HARD GATE: Technical Refinement must exist and be signed before packet creation.
const refinementsDir = path.join(GOV_ROOT_REPO_REL, 'refinements');
if (!fs.existsSync(refinementsDir)) {
  fs.mkdirSync(refinementsDir, { recursive: true });
}

const refinementPath = defaultRefinementPath(WP_ID);
let userSignature = '';

if (!fs.existsSync(refinementPath)) {
  const refinementTemplatePath = path.join(GOV_ROOT_REPO_REL, 'templates', 'REFINEMENT_TEMPLATE.md');
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
      '# Present refinement to the user for review.',
      `just record-refinement ${WP_ID}`,
      '# After explicit user approval + one-time signature bundle:',
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
      '# After signature bundle:',
      `just orchestrator-prepare-and-packet ${WP_ID}`,
      `just pre-work ${WP_ID}`,
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
      '# After explicit user approval + one-time signature bundle:',
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
      `just create-task-packet ${WP_ID}`,
    ],
  });
  process.exit(1);
}

userSignature = refinementValidation.parsed.signature;

// HARD GATE: if refinement indicates enrichment is needed, do not create a work packet.
try {
  const refinementContent = fs.readFileSync(refinementPath, 'utf8');
  const m = refinementContent.match(/^\s*-\s*ENRICHMENT_NEEDED\s*:\s*(YES|NO)\s*$/mi);
  const enrichmentNeeded = (m?.[1] || '').toUpperCase();
  const existingCapabilityAlignmentVerdict = String(refinementValidation?.parsed?.existingCapabilityAlignmentVerdict || '').toUpperCase();
  if (enrichmentNeeded === 'YES') {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'SIGNATURE',
      next: 'STOP',
      operatorAction: 'Spec update required (create new spec version + new WP variant)',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'Refinement declares ENRICHMENT_NEEDED=YES; do not create/lock a task packet until the spec update is completed.',
      gateOutputLines: [
        `BLOCKED: ${WP_ID} refinement declares ENRICHMENT_NEEDED=YES.`,
        'Do NOT create/lock a WP packet while a Main Body or appendix spec update is required.',
      ],
      nextCommands: [
        `# Run the spec update workflow (new spec version file + update ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md).`,
        '# If the refinement expanded appendices (primitive index, feature registry, UI guidance, interaction matrix), land those changes in the new spec version first.',
        '# Create a NEW WP variant anchored to the updated spec (new WP_ID; new one-time signature).',
      ],
    });
    process.exit(1);
  }
  if (existingCapabilityAlignmentVerdict === 'REUSE_EXISTING') {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'REFINEMENT',
      next: 'STOP',
      operatorAction: 'Reuse the existing stub/packet instead of creating a duplicate packet',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'The signed refinement concluded that an equivalent capability already exists and should be reused.',
      gateOutputLines: [
        `BLOCKED: ${WP_ID} refinement declares EXISTING_CAPABILITY_ALIGNMENT_VERDICT=REUSE_EXISTING.`,
        'Do NOT create a duplicate packet when the same capability already exists in governance + code reality.',
      ],
      nextCommands: [
        '# Reuse or route to the matched existing artifact recorded in the signed refinement.',
        '# If the current candidate is redundant, supersede or retire it instead of activating a duplicate WP.',
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
let signatureGate = null;
let prepareGate = null;
try {
  const gatesPath = path.join(GOV_ROOT_REPO_REL, 'roles', 'orchestrator', 'runtime', 'ORCHESTRATOR_GATES.json');
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
        `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
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
        `# If refinement was manually edited, revert USER_SIGNATURE to <pending> and re-run: just record-signature ${WP_ID} {newSignature} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}.`,
      ],
    });
    process.exit(1);
  }

  // HARD GATE: worktree + execution owner must be recorded AFTER signature and BEFORE packet creation.
  const lastPrepare = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'PREPARE');
  if (!lastPrepare) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'PREPARE',
      next: 'PREPARE',
      operatorAction: `Record workflow lane + execution owner for ${WP_ID} (MANUAL_RELAY|ORCHESTRATOR_MANAGED + ${EXECUTION_OWNER_RANGE_HELP})`,
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'WP worktree/branch + execution owner must be recorded AFTER signature and BEFORE packet creation.',
      gateOutputLines: [
        `BLOCKED: WP branch/worktree + execution owner not recorded for ${WP_ID}.`,
      ],
      nextCommands: [
        `just orchestrator-prepare-and-packet ${WP_ID}`,
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
          `just record-prepare ${WP_ID} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
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
        `just record-prepare ${WP_ID} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
        `just create-task-packet ${WP_ID}`,
      ],
    });
    process.exit(1);
  }
  signatureGate = lastSig;
  prepareGate = lastPrepare;
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
      `BLOCKED: Unable to verify signature in ${GOV_ROOT_REPO_REL}/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json.`,
    ],
    nextCommands: [
      `cat ${GOV_ROOT_REPO_REL}/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json`,
    ],
  });
  process.exit(1);
}

// Gate: signature must be present in SIGNATURE_AUDIT.md (protocol requirement).
try {
  const auditPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'records', 'SIGNATURE_AUDIT.md');
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
        `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
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
      `BLOCKED: Unable to verify signature in ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md.`,
    ],
    nextCommands: [
      `cat ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md`,
    ],
  });
  process.exit(1);
}

// Ensure WP folder exists (new folder structure: .GOV/task_packets/WP-{ID}/)
const taskPacketBaseDir = `${GOV_ROOT_REPO_REL}/task_packets`;
const wpDir = path.join(taskPacketBaseDir, WP_ID);
if (!fs.existsSync(wpDir)) {
  fs.mkdirSync(wpDir, { recursive: true });
  console.log(`Created WP directory: ${wpDir}/`);
}

const fileName = 'packet.md';
const filePath = path.join(wpDir, fileName);

// Also check legacy flat file location
const legacyFilePath = path.join(taskPacketBaseDir, `${WP_ID}.md`);
if (fs.existsSync(legacyFilePath)) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'STOP',
    operatorAction: `Legacy flat packet exists at ${legacyFilePath}. Migrate or remove before creating folder-based packet.`,
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Legacy flat packet file conflicts with new folder structure.',
    gateOutputLines: [`BLOCKED: ${legacyFilePath} exists.`],
    nextCommands: [`# Move: mkdir ${wpDir} && mv ${legacyFilePath} ${filePath}`],
  });
  process.exit(1);
}

// Check if file already exists
if (fs.existsSync(filePath)) {
  const packetText = fs.readFileSync(filePath, 'utf8');
  const communicationDrift = detectCommunicationArtifactDrift(filePath, packetText);
  if (communicationDrift) {
    printGateBlocks({
      wpId: WP_ID,
      stage: 'PACKET_CREATE',
      next: 'STATUS_SYNC',
      operatorAction: 'NONE',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'BLOCKED',
      why: 'The packet already exists, but its WP communication artifacts are incomplete. The packet is intentionally preserved on disk; repair the communication artifacts instead of recreating or deleting the packet.',
      gateOutputLines: [
        `BLOCKED: Repairable packet communication drift detected for ${communicationDrift.packetPath}.`,
        ...(communicationDrift.declaredCount !== 4
          ? [`- Packet declares ${communicationDrift.declaredCount} of 4 WP communication metadata fields; all 4 are required.`]
          : []),
        ...communicationDrift.missing.map((item) => `- Missing communication artifact: ${item}`),
        '- Packet was preserved intentionally. Auto-delete rollback is disabled for this workflow.',
      ],
      nextCommands: [
        `cat ${filePath.replace(/\\/g, '/')}`,
        `just ensure-wp-communications ${WP_ID}`,
        `just gov-check`,
      ],
    });
    process.exit(1);
  }

  if (allowOverwriteExisting) {
    console.warn(`[PACKET_CREATE] overwriting existing packet for ${WP_ID} at ${filePath.replace(/\\/g, '/')}`);
  } else {

    printGateBlocks({
      wpId: WP_ID,
      stage: 'PACKET_CREATE',
      next: 'STOP',
      operatorAction: 'NONE',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'FAIL',
      why: 'Task packet file already exists; generator will not overwrite it.',
      gateOutputLines: [
        `FAIL: Work packet already exists: ${filePath.replace(/\\/g, '/')}`,
      ],
      nextCommands: [
        `cat ${filePath.replace(/\\/g, '/')}`,
        '# If you need a revision, create a new packet ID: WP-...-v{N}.',
      ],
    });
    process.exit(1);
  }
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
const templatePath = path.join(GOV_ROOT_REPO_REL, 'templates', 'TASK_PACKET_TEMPLATE.md');
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
      `ls ${GOV_ROOT_REPO_REL}/templates`,
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
    const specCurrent = fs.readFileSync(path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'records', 'SPEC_CURRENT.md'), 'utf8');
    const m = specCurrent.match(/Handshake_Master_Spec_v[0-9.]+\.md/);
    if (m) specBaseline = m[0];
  } catch {
    // Leave placeholder if SPEC_CURRENT cannot be read or parsed.
  }

const fill = (text, token, value) => text.split(token).join(value);
const replaceSingleField = (text, label, value) =>
  text.replace(new RegExp(`^(\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*).*$`, 'mi'), `$1${value}`);
const escapeRegExp = (value) => String(value || '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const replaceSection = (text, heading, newSection) => {
  const lines = String(text || '').split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${escapeRegExp(heading)}\\b`, 'i');
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) {
    throw new Error(`Missing packet section heading: ${heading}`);
  }

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }

  const replacementLines = String(newSection || '').replace(/\r/g, '').trim().split('\n');
  return [
    ...lines.slice(0, startIndex),
    ...replacementLines,
    ...lines.slice(endIndex),
  ].join('\n');
};
const githubTreeBase = () => {
  try {
    const raw = execSync('git remote get-url origin', { encoding: 'utf8' }).trim();
    const normalized = raw.replace(/\.git$/, '');
    if (/^https?:\/\//i.test(normalized)) return normalized.replace(/\/+$/, '');
    const sshMatch = normalized.match(/^git@github\.com:(.+)$/i);
    if (sshMatch) return `https://github.com/${sshMatch[1].replace(/^\/+/, '')}`;
  } catch {
    // Fall through to pending placeholder.
  }
  return '<pending>';
};
const formatList = (items, { indent = '  - ', none = 'NONE' } = {}) => {
  const normalized = (items || []).map((item) => String(item || '').trim()).filter(Boolean);
  if (normalized.length === 0) return `${indent}${none}`;
  return normalized.map((item) => `${indent}${item}`).join('\n');
};
const formatSourceLog = (sources) => formatList(
  (sources || []).map((source) => {
    const kind = (source.kind || '').trim() || 'UNKNOWN';
    const title = (source.source || '').trim() || '<missing>';
    const date = (source.date || '').trim() || '<missing>';
    const retrievedAt = (source.retrievedAt || '').trim() || '<missing>';
    const url = (source.url || '').trim() || '<missing>';
    const why = (source.why || '').trim() || '<missing>';
    return `[${kind}] ${title} | ${date} | Retrieved: ${retrievedAt} | ${url} | Why: ${why}`;
  }),
);
const isIsoDate = (value) => /^\d{4}-\d{2}-\d{2}$/.test(String(value || '').trim());
const isVersionAtLeast = (isoDate, minIsoDate) => isIsoDate(isoDate) && isIsoDate(minIsoDate) && isoDate >= minIsoDate;
const deriveAddMarkerTarget = (specFileName) => {
  const match = String(specFileName || '').match(/v(\d{2}\.\d{3})/i);
  return match ? `[ADD v${match[1]}]` : '[ADD vXX.XXX]';
};

let template = templateBody;
template = fill(template, '{{WP_ID}}', WP_ID);
template = fill(template, '{{DATE_ISO}}', timestamp);
template = fill(template, '{{MERGE_BASE_SHA}}', mergeBaseSha);
template = fill(template, '{{SPEC_BASELINE}}', specBaseline);
template = fill(template, '{{REQUESTOR}}', 'Operator');
template = fill(template, '{{AGENT_ID}}', 'Orchestrator');
template = fill(template, '{{USER_SIGNATURE}}', userSignature);
template = fill(template, '{{SPEC_ANCHOR}}', '<fill>');

const refinementData = refinementValidation.parsed || {};
const isHydratedProfile = /^HYDRATED_RESEARCH_V1$/i.test(refinementData.refinementEnforcementProfile || '');
const hasRefinementHandoffSections = isVersionAtLeast(refinementData.refinementFormatVersion, '2026-03-15');
const specAddMarkerTarget = refinementData.packetHydration?.specAddMarkerTarget || deriveAddMarkerTarget(specBaseline);
template = replaceSingleField(template, 'REFINEMENT_ENFORCEMENT_PROFILE', refinementData.refinementEnforcementProfile || 'LEGACY_MANUAL');
template = replaceSingleField(template, 'PACKET_HYDRATION_PROFILE', isHydratedProfile ? 'HYDRATED_RESEARCH_V1' : 'LEGACY_MANUAL');
template = replaceSingleField(template, 'SPEC_ADD_MARKER_TARGET', specAddMarkerTarget);

const workflowLane = (prepareGate?.workflow_lane || signatureGate?.workflow_lane || '').trim();
const executionLane = (prepareGate?.execution_lane || signatureGate?.execution_lane || prepareGate?.coder_id || '').trim();
const baseWpId = WP_ID.replace(/-v\d+$/, '');
const localBranch = (prepareGate?.branch || defaultCoderBranch(WP_ID)).trim() || defaultCoderBranch(WP_ID);
const localWorktreeDir = (prepareGate?.worktree_dir || defaultCoderWorktreeDir(WP_ID)).trim() || defaultCoderWorktreeDir(WP_ID);
const remoteBackupBranch = localBranch;
const originTreeBase = githubTreeBase();
const remoteBackupUrl = buildRemoteBackupUrl(originTreeBase, remoteBackupBranch);
const wpValidatorBranch = defaultWpValidatorBranch(WP_ID);
const wpValidatorWorktreeDir = defaultWpValidatorWorktreeDir(WP_ID);
// Validators keep distinct local branches/worktrees, but all WP-scoped roles share one remote WP backup branch.
const wpValidatorRemoteBackupBranch = remoteBackupBranch;
const wpValidatorRemoteBackupUrl = remoteBackupUrl;
const integrationValidatorBranch = defaultIntegrationValidatorBranch(WP_ID);
const integrationValidatorWorktreeDir = defaultIntegrationValidatorWorktreeDir(WP_ID);
const integrationValidatorRemoteBackupBranch = remoteBackupBranch;
const integrationValidatorRemoteBackupUrl = remoteBackupUrl;
const packetWpCommunicationPaths = communicationPathsForWp(WP_ID);
template = replaceSingleField(template, 'BASE_WP_ID', baseWpId);
template = replaceSingleField(template, 'LOCAL_BRANCH', localBranch);
template = replaceSingleField(template, 'LOCAL_WORKTREE_DIR', localWorktreeDir);
template = replaceSingleField(template, 'REMOTE_BACKUP_BRANCH', remoteBackupBranch);
template = replaceSingleField(template, 'REMOTE_BACKUP_URL', remoteBackupUrl);
template = replaceSingleField(template, 'SESSION_START_AUTHORITY', SESSION_START_AUTHORITY);
template = replaceSingleField(template, 'SESSION_HOST_PREFERENCE', SESSION_HOST_PREFERENCE);
template = replaceSingleField(template, 'SESSION_HOST_FALLBACK', SESSION_HOST_FALLBACK);
template = replaceSingleField(template, 'SESSION_LAUNCH_POLICY', SESSION_LAUNCH_POLICY);
template = replaceSingleField(template, 'ROLE_SESSION_RUNTIME', ROLE_SESSION_RUNTIME);
template = replaceSingleField(template, 'CLI_SESSION_TOOL', CLI_SESSION_TOOL);
template = replaceSingleField(template, 'SESSION_PLUGIN_BRIDGE_ID', SESSION_PLUGIN_BRIDGE_ID);
template = replaceSingleField(template, 'SESSION_PLUGIN_BRIDGE_COMMAND', SESSION_PLUGIN_BRIDGE_COMMAND);
template = replaceSingleField(template, 'SESSION_PLUGIN_REQUESTS_FILE', SESSION_PLUGIN_REQUESTS_FILE);
template = replaceSingleField(template, 'SESSION_REGISTRY_FILE', SESSION_REGISTRY_FILE);
template = replaceSingleField(template, 'SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION', String(SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION));
template = replaceSingleField(template, 'SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS', String(SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS));
template = replaceSingleField(template, 'SESSION_WATCH_POLICY', SESSION_WATCH_POLICY);
template = replaceSingleField(template, 'SESSION_WAKE_CHANNEL_PRIMARY', SESSION_WAKE_CHANNEL_PRIMARY);
template = replaceSingleField(template, 'SESSION_WAKE_CHANNEL_FALLBACK', SESSION_WAKE_CHANNEL_FALLBACK);
template = replaceSingleField(template, 'CLI_ESCALATION_HOST_DEFAULT', CLI_ESCALATION_HOST_DEFAULT);
template = replaceSingleField(template, 'MODEL_FAMILY_POLICY', MODEL_FAMILY_POLICY);
template = replaceSingleField(template, 'CODEX_MODEL_ALIASES_ALLOWED', CODEX_MODEL_ALIASES_ALLOWED);
template = replaceSingleField(template, 'ROLE_SESSION_PRIMARY_MODEL', ROLE_SESSION_PRIMARY_MODEL);
template = replaceSingleField(template, 'ROLE_SESSION_FALLBACK_MODEL', ROLE_SESSION_FALLBACK_MODEL);
template = replaceSingleField(template, 'ROLE_SESSION_REASONING_REQUIRED', ROLE_SESSION_REASONING_REQUIRED);
template = replaceSingleField(template, 'ROLE_SESSION_REASONING_CONFIG_KEY', ROLE_SESSION_REASONING_CONFIG_KEY);
template = replaceSingleField(template, 'ROLE_SESSION_REASONING_CONFIG_VALUE', ROLE_SESSION_REASONING_CONFIG_VALUE);
template = replaceSingleField(template, 'WP_VALIDATOR_LOCAL_BRANCH', wpValidatorBranch);
template = replaceSingleField(template, 'WP_VALIDATOR_LOCAL_WORKTREE_DIR', wpValidatorWorktreeDir);
template = replaceSingleField(template, 'WP_VALIDATOR_REMOTE_BACKUP_BRANCH', wpValidatorRemoteBackupBranch);
template = replaceSingleField(template, 'WP_VALIDATOR_REMOTE_BACKUP_URL', wpValidatorRemoteBackupUrl);
template = replaceSingleField(template, 'INTEGRATION_VALIDATOR_LOCAL_BRANCH', integrationValidatorBranch);
template = replaceSingleField(template, 'INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR', integrationValidatorWorktreeDir);
template = replaceSingleField(template, 'INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH', integrationValidatorRemoteBackupBranch);
template = replaceSingleField(template, 'INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL', integrationValidatorRemoteBackupUrl);
template = replaceSingleField(template, 'WP_COMMUNICATION_DIR', packetWpCommunicationPaths.dir);
template = replaceSingleField(template, 'WP_THREAD_FILE', packetWpCommunicationPaths.threadFile);
template = replaceSingleField(template, 'WP_RUNTIME_STATUS_FILE', packetWpCommunicationPaths.runtimeStatusFile);
template = replaceSingleField(template, 'WP_RECEIPTS_FILE', packetWpCommunicationPaths.receiptsFile);
template = replaceSingleField(template, 'PACKET_FORMAT_VERSION', PACKET_FORMAT_VERSION);
const normalizedWorkflowLane = workflowLane.toUpperCase();
const normalizedExecutionOwner = executionOwnerToPacketValue(executionLane) || executionLane.toUpperCase().replace('-', '_');
const legacyOrchestratorAgentic = /^ORCHESTRATOR-AGENTIC$/i.test(executionLane.replace(/[\s_]+/g, '-'));

if (legacyOrchestratorAgentic) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'STOP',
    operatorAction: 'Migrate the signature bundle to the current workflow tuple',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Legacy Orchestrator-Agentic execution is not a valid packet-creation path in current repo governance.',
    gateOutputLines: [
      `BLOCKED: ${WP_ID} still resolves to legacy Orchestrator-Agentic execution.`,
      'The Orchestrator remains non-agentic and cannot be the execution owner for a new packet.',
    ],
    nextCommands: [
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
      `just record-prepare ${WP_ID} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE} [branch] [worktree_dir]`,
      `just create-task-packet ${WP_ID}`,
    ],
  });
  process.exit(1);
}

if (!WORKFLOW_LANE_VALUES.includes(normalizedWorkflowLane) || !EXECUTION_OWNER_VALUES.includes(normalizedExecutionOwner)) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'PREPARE',
    operatorAction: 'Complete or repair the workflow tuple before packet creation',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Official packet creation now requires an explicit workflow lane and coder execution owner.',
    gateOutputLines: [
      `BLOCKED: invalid workflow tuple for ${WP_ID}.`,
      `- workflow_lane: ${workflowLane || '<missing>'}`,
      `- execution_owner: ${executionLane || '<missing>'}`,
    ],
    nextCommands: [
      `just record-signature ${WP_ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
      `just record-prepare ${WP_ID} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE} [branch] [worktree_dir]`,
      `just create-task-packet ${WP_ID}`,
    ],
  });
  process.exit(1);
}

template = replaceSingleField(template, 'WORKFLOW_LANE', normalizedWorkflowLane);
template = replaceSingleField(template, 'EXECUTION_OWNER', normalizedExecutionOwner);
template = replaceSingleField(template, 'AGENTIC_MODE', 'NO');
template = replaceSingleField(template, 'ORCHESTRATOR_MODEL', 'N/A');
template = replaceSingleField(template, 'ORCHESTRATION_STARTED_AT_UTC', 'N/A');
template = replaceSingleField(template, 'CODER_MODEL', executionLane || '<unclaimed>');
template = replaceSingleField(template, 'CODER_REASONING_STRENGTH', '<unclaimed>');
template = replaceSingleField(template, 'SUB_AGENT_DELEGATION', 'DISALLOWED');
template = replaceSingleField(template, 'OPERATOR_APPROVAL_EVIDENCE', 'N/A');

let clauseClosureRows = [];
if (isHydratedProfile) {
  const hydration = refinementData.packetHydration || {};
  template = replaceSingleField(template, 'REQUESTOR', hydration.requestor || 'Operator');
  template = replaceSingleField(template, 'AGENT_ID', hydration.agentId || 'Orchestrator');
  template = replaceSingleField(template, 'RISK_TIER', hydration.riskTier || '<fill>');
  template = replaceSingleField(template, 'BUILD_ORDER_DOMAIN', hydration.buildOrderDomain || '<fill>');
  template = replaceSingleField(template, 'BUILD_ORDER_TECH_BLOCKER', hydration.buildOrderTechBlocker || '<fill>');
  template = replaceSingleField(template, 'BUILD_ORDER_VALUE_TIER', hydration.buildOrderValueTier || '<fill>');
  template = replaceSingleField(template, 'BUILD_ORDER_DEPENDS_ON', hydration.buildOrderDependsOnRaw || 'NONE');
  template = replaceSingleField(template, 'BUILD_ORDER_BLOCKS', hydration.buildOrderBlocksRaw || 'NONE');
  template = replaceSingleField(template, 'UI_UX_APPLICABLE', refinementData.uiApplicable || 'NO');
  template = replaceSingleField(template, 'UI_UX_VERDICT', refinementData.uiVerdict || 'OK');
  template = replaceSingleField(template, 'STUB_WP_IDS', refinementData.stubWpIdsRaw || 'NONE');
  const primarySpecAnchor =
    (hydration.specAnchorPrimary || '').trim()
    || String(refinementData.specAnchors?.[0]?.specAnchor || '').trim()
    || 'See SPEC_CONTEXT_WINDOWS ANCHOR 1';
  template = replaceSection(template, 'AUTHORITY', `
## AUTHORITY
- SPEC_BASELINE: ${specBaseline} (recorded_at: ${timestamp})
- SPEC_TARGET: ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: ${specAddMarkerTarget}
- SPEC_ANCHOR_PRIMARY: ${primarySpecAnchor}
- Codex: ${GOV_ROOT_REPO_REL}/codex/Handshake_Codex_v1.4.md
- Task Board: ${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md
- WP Traceability: ${GOV_ROOT_REPO_REL}/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
`);

  clauseClosureRows = buildClauseClosureRows({
    clauseProofPlan: refinementData.clauseProofPlan,
    specAnchors: refinementData.specAnchors,
    doneMeans: hydration.doneMeans,
    inScopePaths: hydration.inScopePaths,
    testPlan: hydration.testPlan,
    canonicalContractExamples: refinementData.canonicalContractExamples,
  });
  const sharedSurfaceMonitoring = deriveSharedSurfaceMonitoring({
    hotFiles: refinementData.coderHotFiles,
    tripwireTests: refinementData.coderTripwireTests,
    inScopePaths: hydration.inScopePaths,
  });
  const semanticProofAssets = deriveSemanticProofAssets({
    semanticTripwireTests: refinementData.semanticTripwireTests,
    canonicalContractExamples: refinementData.canonicalContractExamples,
    testPlan: hydration.testPlan,
    doneMeans: hydration.doneMeans,
    specAnchors: refinementData.specAnchors,
  });

  template = replaceSingleField(template, 'CLAUSE_CLOSURE_MONITOR_PROFILE', 'CLAUSE_MONITOR_V1');
  template = replaceSingleField(template, 'SEMANTIC_PROOF_PROFILE', 'DIFF_SCOPED_SEMANTIC_V1');
  template = replaceSection(template, 'CLAUSE_CLOSURE_MATRIX', formatClauseClosureMatrixSection(clauseClosureRows));
  template = replaceSection(template, 'SPEC_DEBT_STATUS', formatSpecDebtStatusSection());
  template = replaceSection(template, 'SHARED_SURFACE_MONITORING', formatSharedSurfaceMonitoringSection(sharedSurfaceMonitoring));
  template = replaceSection(template, 'SEMANTIC_PROOF_ASSETS', formatSemanticProofAssetsSection(semanticProofAssets));

  template = replaceSection(template, 'SPEC_CONTEXT_WINDOWS', formatSpecContextWindowsSection(refinementData.specAnchors));

  if (hasRefinementHandoffSections) {
    template = replaceSection(template, 'CLAUSE_PROOF_PLAN', formatClauseProofPlanSection(refinementData.clauseProofPlan));
    template = replaceSection(template, 'CONTRACT_SURFACES', formatContractSurfacesSection(refinementData.contractSurfaces));
    template = replaceSection(template, 'CODER_HANDOFF_BRIEF', formatCoderHandoffBriefSection({
      implementationOrder: refinementData.coderImplementationOrder,
      hotFiles: refinementData.coderHotFiles,
      tripwireTests: refinementData.coderTripwireTests,
      carryForwardWarnings: refinementData.coderCarryForwardWarnings,
    }));
    template = replaceSection(template, 'VALIDATOR_HANDOFF_BRIEF', formatValidatorHandoffBriefSection({
      clausesToInspect: refinementData.validatorClausesToInspect,
      filesToRead: refinementData.validatorFilesToRead,
      commandsToRun: refinementData.validatorCommandsToRun,
      postMergeSpotchecks: refinementData.validatorPostMergeSpotchecks,
    }));
    template = replaceSection(template, 'NOT_PROVEN_AT_REFINEMENT_TIME', formatNotProvenAtRefinementTimeSection(refinementData.refinementNotProven));
  } else {
    template = replaceSection(template, 'CLAUSE_PROOF_PLAN', `
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Use the signed refinement directly for clause proof planning.
`);
    template = replaceSection(template, 'CONTRACT_SURFACES', `
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Reconstruct contract-surface checks from the signed refinement when needed.
`);
    template = replaceSection(template, 'CODER_HANDOFF_BRIEF', `
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Read the signed refinement directly for execution guidance.
`);
    template = replaceSection(template, 'VALIDATOR_HANDOFF_BRIEF', `
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Read the signed refinement directly for inspection guidance.
`);
    template = replaceSection(template, 'NOT_PROVEN_AT_REFINEMENT_TIME', `
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Uncertainty tracking remains in the signed refinement only.
`);
  }

  template = replaceSection(template, 'RESEARCH_SIGNAL', `
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: ${refinementData.researchCurrencyRequired || 'NO'}
- RESEARCH_CURRENCY_VERDICT: ${refinementData.researchCurrencyVerdict || 'NOT_APPLICABLE'}
- RESEARCH_DEPTH_VERDICT: ${refinementData.researchDepthVerdict || 'NOT_APPLICABLE'}
- GITHUB_PROJECT_SCOUTING_VERDICT: ${refinementData.githubProjectScoutingVerdict || 'NOT_APPLICABLE'}
- SOURCE_LOG:
${formatSourceLog(refinementData.researchSources)}
- RESEARCH_SYNTHESIS:
${formatList(refinementData.researchSynthesis)}
- GITHUB_PROJECT_DECISIONS:
${formatList(refinementData.githubProjectDecisions)}
`);

  template = replaceSection(template, 'MATRIX_RESEARCH_RUBRIC', `
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: ${refinementData.matrixResearchRequired || 'NO'}
- MATRIX_RESEARCH_VERDICT: ${refinementData.matrixResearchVerdict || 'NOT_APPLICABLE'}
- SOURCE_SCAN_DECISIONS:
${formatList(refinementData.matrixResearchSourceDecisions)}
- MATRIX_GROWTH_CANDIDATES:
${formatList(refinementData.matrixGrowthCandidates)}
- ENGINEERING_TRICKS_CARRIED_OVER:
${formatList(refinementData.matrixResearchEngineeringTricks)}
`);

  template = replaceSection(template, 'PRIMITIVES_AND_MATRIX', `
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
${formatList(refinementData.primitivesTouched)}
- PRIMITIVES_EXPOSED:
${formatList(refinementData.primitivesExposed)}
- PRIMITIVES_CREATED:
${formatList(refinementData.primitivesCreated)}
- MECHANICAL_ENGINES_TOUCHED:
${formatList(refinementData.mechanicalEnginesTouched)}
- PRIMITIVE_INDEX_ACTION: ${refinementData.primitiveIndexAction || 'NO_CHANGE'}
- FEATURE_REGISTRY_ACTION: ${refinementData.featureRegistryAction || 'NO_CHANGE'}
- UI_GUIDANCE_ACTION: ${refinementData.uiGuidanceAction || 'NOT_APPLICABLE'}
- INTERACTION_MATRIX_ACTION: ${refinementData.interactionMatrixAction || 'NO_CHANGE'}
- APPENDIX_MAINTENANCE_VERDICT: ${refinementData.appendixMaintenanceVerdict || 'OK'}
- PILLAR_ALIGNMENT_VERDICT: ${refinementData.pillarAlignmentVerdict || 'OK'}
- PILLARS_TOUCHED:
${formatList(refinementData.pillarsTouched)}
- PILLARS_REQUIRING_STUBS:
${formatList(refinementData.pillarsRequiringStubs)}
- PRIMITIVE_MATRIX_VERDICT: ${refinementData.primitiveMatrixVerdict || 'NONE_FOUND'}
- FORCE_MULTIPLIER_VERDICT: ${refinementData.forceMultiplierVerdict || 'OK'}
- FORCE_MULTIPLIER_RESOLUTIONS:
${formatList(refinementData.forceMultiplierResolutions)}
- STUB_WP_IDS: ${refinementData.stubWpIdsRaw || 'NONE'}
`);

  template = replaceSection(template, 'PILLAR_DECOMPOSITION', `
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: ${refinementData.pillarDecompositionVerdict || 'OK'}
- DECOMPOSITION_ROWS:
${formatList(refinementData.pillarDecompositionRows)}
`);

  template = replaceSection(template, 'EXECUTION_RUNTIME_ALIGNMENT', `
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: ${refinementData.executionRuntimeAlignmentVerdict || 'OK'}
- ALIGNMENT_ROWS:
${formatList(refinementData.executionRuntimeAlignmentRows)}
`);

  template = replaceSection(template, 'EXISTING_CAPABILITY_ALIGNMENT', `
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: ${refinementData.existingCapabilityAlignmentVerdict || 'OK'}
- MATCHED_ARTIFACT_RESOLUTIONS:
${formatList(refinementData.matchedArtifactResolutions)}
- CODE_REALITY_SUMMARY:
${formatList(refinementData.codeRealitySummary)}
`);

  template = replaceSection(template, 'GUI_IMPLEMENTATION_ADVICE', `
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: ${refinementData.guiAdviceRequired || 'NO'}
- GUI_IMPLEMENTATION_ADVICE_VERDICT: ${refinementData.guiImplementationAdviceVerdict || 'NOT_APPLICABLE'}
- GUI_REFERENCE_DECISIONS:
${formatList(refinementData.guiReferenceDecisions)}
- HANDSHAKE_GUI_ADVICE:
${formatList(refinementData.handshakeGuiAdvice)}
- HIDDEN_GUI_REQUIREMENTS:
${formatList(refinementData.hiddenGuiRequirements)}
- GUI_ENGINEERING_TRICKS_TO_CARRY:
${formatList(refinementData.guiEngineeringTricks)}
`);

  template = replaceSection(template, 'SCOPE', `
## SCOPE
- What: ${hydration.what || '<fill>'}
- Why: ${hydration.why || '<fill>'}
- IN_SCOPE_PATHS:
${formatList(hydration.inScopePaths)}
- OUT_OF_SCOPE:
${formatList(hydration.outOfScope)}
`);

  template = replaceSection(template, 'QUALITY_GATE', `
## QUALITY_GATE
### TEST_PLAN
\`\`\`bash
${(hydration.testPlan || '').trim()}
\`\`\`

### DONE_MEANS
${formatList(hydration.doneMeans, { indent: '- ' })}

- PRIMITIVES_EXPOSED:
${formatList(hydration.primitivesExposed)}
- PRIMITIVES_CREATED:
${formatList(hydration.primitivesCreated)}

### ROLLBACK_HINT
\`\`\`bash
git revert <commit-sha>
\`\`\`
`);

  template = replaceSection(template, 'BOOTSTRAP', `
## BOOTSTRAP
- FILES_TO_OPEN:
${formatList(hydration.filesToOpen)}
- SEARCH_TERMS:
${formatList(hydration.searchTerms)}
- RUN_COMMANDS:
  \`\`\`bash
${(hydration.runCommands || '').trim()}
  \`\`\`
- RISK_MAP:
${formatList(hydration.riskMap)}
`);

  template = replaceSection(template, 'END_TO_END_CLOSURE_PLAN', `
## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - NONE
- REQUIRED_PROVENANCE_FIELDS:
  - NONE
- VERIFICATION_PLAN:
  - Record end-to-end trust/provenance requirements only if this WP introduces a cross-boundary apply path.
- ERROR_TAXONOMY_PLAN:
  - N/A for initial coder handoff.
- UI_GUARDRAILS:
  - N/A for initial coder handoff.
- VALIDATOR_ASSERTIONS:
  - Validate the packet-scoped spec anchors, in-scope files, and deterministic evidence recorded during implementation.
`);

  template = replaceSection(template, 'VALIDATION', `
## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-\`${GOV_ROOT_REPO_REL}/\` files, repeat the manifest block once per changed file (multiple \`**Target File**\` entries are supported).
- SHA1 hint: stage your changes and run \`just cor701-sha <changed file>\` to get deterministic \`Pre-SHA1\` / \`Post-SHA1\` values.
- **Target File**: \`N/A (fill after implementation)\`
- **Start**: N/A
- **End**: N/A
- **Line Delta**: N/A
- **Pre-SHA1**: \`N/A\`
- **Post-SHA1**: \`N/A\`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: ${GOV_ROOT_REPO_REL}/spec/SPEC_CURRENT.md -> ${specBaseline}
- **Notes**:
`);

  template = replaceSection(template, 'EVIDENCE_MAPPING', `
## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: \`N/A (fill during implementation)\`
`);

  if (/^YES$/i.test(refinementData.uiApplicable || '')) {
    template = replaceSection(template, 'UI_UX_SPEC', `
## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For \`PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1\`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
${formatList(refinementData.uiSpec?.surfaces)}
- UI_CONTROLS (buttons/dropdowns/inputs):
${formatList(refinementData.uiSpec?.controls)}
- UI_STATES (empty/loading/error):
${formatList(refinementData.uiSpec?.states)}
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
${formatList(refinementData.uiSpec?.microcopy)}
- UI_ACCESSIBILITY_NOTES:
${formatList(refinementData.uiSpec?.accessibility)}
`);
  } else {
    template = replaceSection(template, 'UI_UX_SPEC', `
## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
`);
  }
}

// Write the file
fs.writeFileSync(filePath, template, 'utf8');

// Copy refinement into WP folder (co-located)
const refinementSrc = defaultRefinementPath(WP_ID);
if (fs.existsSync(refinementSrc)) {
  const refinementDst = path.join(wpDir, 'refinement.md');
  fs.copyFileSync(refinementSrc, refinementDst);
  console.log(`Copied refinement to: ${refinementDst}`);
}

// Generate micro task files from CLAUSE_CLOSURE_MATRIX rows
if (clauseClosureRows && clauseClosureRows.length > 0) {
  const mtTemplatePath = `${GOV_ROOT_REPO_REL}/templates/MICRO_TASK_TEMPLATE.md`;
  const mtTemplate = fs.existsSync(mtTemplatePath) ? fs.readFileSync(mtTemplatePath, 'utf8') : null;

  // Parse pipe-delimited clause closure rows into fields
  const parsePipeField = (row, field) => {
    const re = new RegExp(`${field}:\\s*(.+?)(?:\\s*\\|\\s*|$)`, 'i');
    const m = String(row).match(re);
    return m ? m[1].trim() : '<fill>';
  };

  // Also pull RISK_IF_MISSED from the refinement's CLAUSE_PROOF_PLAN if available
  const clauseProofPlan = refinementData?.clauseProofPlan || [];
  const riskByClause = {};
  for (const item of clauseProofPlan) {
    const clauseMatch = String(item).match(/CLAUSE:\s*(.+?)(?:\s*\|\s*|$)/i);
    const riskMatch = String(item).match(/RISK_IF_MISSED:\s*(.+?)(?:\s*\|\s*|$)/i);
    if (clauseMatch && riskMatch) {
      riskByClause[clauseMatch[1].trim()] = riskMatch[1].trim();
    }
  }

  clauseClosureRows.forEach((row, idx) => {
    const mtId = `MT-${String(idx + 1).padStart(3, '0')}`;
    const mtPath = path.join(wpDir, `${mtId}.md`);
    const clause = parsePipeField(row, 'CLAUSE');
    const codeSurfaces = parsePipeField(row, 'CODE_SURFACES');
    const tests = parsePipeField(row, 'TESTS');
    const riskIfMissed = riskByClause[clause] || '<fill>';

    if (mtTemplate) {
      let mt = mtTemplate;
      mt = mt.replace(/\{\{MT_ID\}\}/g, mtId);
      mt = mt.replace(/\{\{WP_ID\}\}/g, WP_ID);
      mt = mt.replace(/\{\{CLAUSE_TEXT\}\}/g, clause);
      mt = mt.replace(/\{\{CODE_SURFACES\}\}/g, codeSurfaces);
      mt = mt.replace(/\{\{EXPECTED_TESTS\}\}/g, tests);
      mt = mt.replace(/\{\{DEPENDS_ON\}\}/g, idx === 0 ? 'NONE' : `MT-${String(idx).padStart(3, '0')}`);
      mt = mt.replace(/\{\{RISK_IF_MISSED\}\}/g, riskIfMissed);
      fs.writeFileSync(mtPath, mt, 'utf8');
    } else {
      const mt = `# ${mtId}: ${clause}\n\n## METADATA\n- WP_ID: ${WP_ID}\n- MT_ID: ${mtId}\n- CLAUSE: ${clause}\n- CODE_SURFACES: ${codeSurfaces}\n- EXPECTED_TESTS: ${tests}\n- DEPENDS_ON: ${idx === 0 ? 'NONE' : `MT-${String(idx).padStart(3, '0')}`}\n- RISK_IF_MISSED: ${riskIfMissed}\n\n## CODER\n- STATUS: PENDING\n\n## VALIDATOR\n- STATUS: PENDING\n`;
      fs.writeFileSync(mtPath, mt, 'utf8');
    }
    console.log(`Created micro task: ${mtPath}`);
  });
  console.log(`Generated ${clauseClosureRows.length} micro task files in ${wpDir}/`);
}

let wpCommunicationPaths = null;
try {
  wpCommunicationPaths = ensureWpCommunications({
    wpId: WP_ID,
    baseWpId,
    workflowLane: (template.match(/^\s*-\s*(?:\*\*)?WORKFLOW_LANE(?:\*\*)?\s*:\s*(.+)\s*$/mi) || [])[1] || '<missing>',
    executionOwner: (template.match(/^\s*-\s*(?:\*\*)?EXECUTION_OWNER(?:\*\*)?\s*:\s*(.+)\s*$/mi) || [])[1] || '<missing>',
    localBranch,
    localWorktreeDir,
    agenticMode: (template.match(/^\s*-\s*(?:\*\*)?AGENTIC_MODE(?:\*\*)?\s*:\s*(.+)\s*$/mi) || [])[1] || 'NO',
    packetStatus: 'Ready for Dev',
    initializedAt: timestamp,
  });
} catch (error) {
  printGateBlocks({
    wpId: WP_ID,
    stage: 'PACKET_CREATE',
    next: 'STATUS_SYNC',
    operatorAction: 'NONE',
    gateRan: `just create-task-packet ${WP_ID}`,
    result: 'BLOCKED',
    why: 'Task packet was created, but WP communication artifacts could not be bootstrapped deterministically. The packet is intentionally preserved on disk; repair the communication artifacts instead of deleting and recreating the packet.',
    gateOutputLines: [
      `BLOCKED: WP communication folder bootstrap failed for ${WP_ID}.`,
      `- ${(error && error.message) ? error.message : String(error)}`,
      '- Packet was preserved intentionally. Auto-delete rollback is disabled for this workflow.',
    ],
    nextCommands: [
      `cat ${filePath.replace(/\\/g, '/')}`,
      `just ensure-wp-communications ${WP_ID}`,
      `just gov-check`,
    ],
  });
  process.exit(1);
}

{
  const isRevision = baseWpId !== WP_ID;
  const syncState = preparedWorktreeSyncState(WP_ID, prepareGate, process.cwd());

  const nextCommands = [
    `cat ${filePath.replace(/\\/g, '/')}`,
    `just task-board-set ${WP_ID} READY_FOR_DEV`,
  ];
  if (!isHydratedProfile) {
    nextCommands.splice(1, 0, '# Fill placeholders: UI_UX_APPLICABLE, UI_UX_VERDICT, STUB_WP_IDS, SCOPE, RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, SPEC_ANCHOR.');
  }
  if (isRevision) {
    nextCommands.push(`just wp-traceability-set ${baseWpId} ${WP_ID}`);
  }
  if (!syncState.ok) {
    nextCommands.push(`# Validator: fast-forward ${syncState.expectedBranch || 'the assigned WP branch'} and ${syncState.worktreeAbs || 'the assigned WP worktree'} until they contain the official packet, current SPEC_CURRENT snapshot, current TASK_BOARD/traceability state, and current PREPARE record.`);
    nextCommands.push(`# Then in the assigned WP worktree: just pre-work ${WP_ID}`);
    nextCommands.push(`just orchestrator-next ${WP_ID}`);

    printGateBlocks({
      wpId: WP_ID,
      stage: 'PACKET_CREATE',
      next: 'STATUS_SYNC',
      operatorAction: 'NONE',
      gateRan: `just create-task-packet ${WP_ID}`,
      result: 'PASS',
      why: 'Task packet was created, but coder handoff is blocked until the assigned WP worktree contains the current packet/spec/governance state.',
      gateOutputLines: [
        `OK: Task packet created: ${filePath.replace(/\\/g, '/')}`,
        `OK: WP communication folder ready: ${wpCommunicationPaths.dir}`,
        ...syncState.issues.map((issue) => `SYNC_REQUIRED: ${issue}`),
      ],
      nextCommands,
    });
  } else {
    nextCommands.push(`just pre-work ${WP_ID}`);
    if (/^CODER_[A-Z]$/i.test(normalizedExecutionOwner)) {
      nextCommands.push(`just launch-coder-session ${WP_ID}`);
      nextCommands.push(`just launch-wp-validator-session ${WP_ID}`);
      nextCommands.push(`just session-registry-status ${WP_ID}`);
      nextCommands.push(`# Integration Validator stays downstream of WP validation PASS; launch later with: just launch-integration-validator-session ${WP_ID}`);
      nextCommands.push(`# Then provide a relayable implementation brief in chat for ${executionLane}; orchestrator implementation agents stay blocked in this lane.`);
    }
    nextCommands.push('# Use the assigned worktree/branch from ORCHESTRATOR_GATES.json PREPARE for the chosen workflow lane + execution owner.');

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
        `OK: WP communication folder ready: ${wpCommunicationPaths.dir}`,
      ],
      nextCommands,
    });
  }
}
