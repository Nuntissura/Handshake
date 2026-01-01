#!/usr/bin/env node
/**
 * Task packet generator [CX-580-581]
 * Creates a task packet from template
 */

import fs from 'fs';
import path from 'path';
import {
  defaultRefinementPath,
  resolveSpecCurrent,
  validateRefinementFile,
} from './validation/refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID || !WP_ID.startsWith('WP-')) {
  console.error('❌ Usage: node create-task-packet.mjs WP-{phase}-{name}');
  console.error('Example: node create-task-packet.mjs WP-1-Job-Cancel');
  process.exit(1);
}

// HARD GATE: Technical Refinement must exist and be signed before packet creation.
const refinementsDir = path.join('docs', 'refinements');
if (!fs.existsSync(refinementsDir)) {
  fs.mkdirSync(refinementsDir, { recursive: true });
}

const refinementPath = defaultRefinementPath(WP_ID);
let userSignature = '';

if (!fs.existsSync(refinementPath)) {
  const refinementTemplatePath = path.join('docs', 'REFINEMENT_TEMPLATE.md');
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

  console.error('BLOCKED: Technical Refinement must be completed BEFORE task packet creation.');
  console.error(`Created refinement scaffold: ${refinementPath}`);
  console.error('Next steps:');
  console.error(`1) Fill ${refinementPath} (ASCII-only; token-in-window per SPEC_ANCHOR)`);
  console.error('2) Present refinement to the user (do NOT ask for signature in the same turn)');
  console.error(`3) Run: just record-refinement ${WP_ID}`);
  console.error(`4) After user review in a NEW turn, run: just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`);
  console.error(`5) Re-run: node scripts/create-task-packet.mjs ${WP_ID}`);
  process.exit(2);
}

const refinementValidation = validateRefinementFile(refinementPath, { expectedWpId: WP_ID, requireSignature: true });
if (!refinementValidation.ok) {
  console.error(`BLOCKED: Refinement is not approved/signed: ${refinementPath}`);
  refinementValidation.errors.forEach((e) => console.error(`- ${e}`));
  console.error('Next steps:');
  console.error(`- Ensure ${refinementPath} is complete.`);
  console.error(`- Run: just record-refinement ${WP_ID}`);
  console.error(`- After user review, run: just record-signature ${WP_ID} {usernameDDMMYYYYHHMM}`);
  process.exit(1);
}

userSignature = refinementValidation.parsed.signature;

// Gate: signature must be recorded in ORCHESTRATOR_GATES.json (prevents manual bypass).
try {
  const gatesPath = path.join('docs', 'ORCHESTRATOR_GATES.json');
  const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
  const logs = Array.isArray(gates.gate_logs) ? gates.gate_logs : [];
  const lastSig = [...logs].reverse().find((l) => l.wpId === WP_ID && l.type === 'SIGNATURE');
  if (!lastSig) {
    console.error(`BLOCKED: No signature record found for ${WP_ID} in ${gatesPath}.`);
    console.error(`Run: just record-signature ${WP_ID} ${userSignature}`);
    process.exit(1);
  }
  if (lastSig.signature !== userSignature) {
    console.error(`BLOCKED: Signature mismatch between refinement (${userSignature}) and gate log (${lastSig.signature}).`);
    process.exit(1);
  }
} catch {
  console.error('BLOCKED: Unable to verify signature in docs/ORCHESTRATOR_GATES.json.');
  process.exit(1);
}

// Gate: signature must be present in SIGNATURE_AUDIT.md (protocol requirement).
try {
  const auditPath = path.join('docs', 'SIGNATURE_AUDIT.md');
  const audit = fs.readFileSync(auditPath, 'utf8');
  if (!audit.includes(`| ${userSignature} |`)) {
    console.error(`BLOCKED: Signature not found in ${auditPath}.`);
    console.error(`Run: just record-signature ${WP_ID} ${userSignature} (this appends to the audit log).`);
    process.exit(1);
  }
} catch {
  console.error('BLOCKED: Unable to verify signature in docs/SIGNATURE_AUDIT.md.');
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

// Template content (canonical)
const templatePath = path.join('docs', 'TASK_PACKET_TEMPLATE.md');
if (!fs.existsSync(templatePath)) {
  console.error(`ƒ?O Missing template: ${templatePath}`);
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
    const specCurrent = fs.readFileSync(path.join('docs', 'SPEC_CURRENT.md'), 'utf8');
    const m = specCurrent.match(/Handshake_Master_Spec_v[0-9.]+\.md/);
    if (m) specBaseline = m[0];
  } catch {
    // Leave placeholder if SPEC_CURRENT cannot be read or parsed.
  }

const fill = (text, token, value) => text.split(token).join(value);

let template = templateBody;
template = fill(template, '{{WP_ID}}', WP_ID);
template = fill(template, '{{DATE_ISO}}', timestamp);
template = fill(template, '{{SPEC_BASELINE}}', specBaseline);
template = fill(template, '{{REQUESTOR}}', '{user or source}');
template = fill(template, '{{AGENT_ID}}', '{orchestrator agent ID}');
template = fill(template, '{{USER_SIGNATURE}}', userSignature);
template = fill(template, '{{SPEC_ANCHOR}}', '<fill>');

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
console.log('- SCOPE: What, Why, IN_SCOPE_PATHS, OUT_OF_SCOPE');
console.log('- RISK_TIER: Choose LOW/MEDIUM/HIGH');
console.log('- TEST_PLAN: List specific commands');
console.log('- DONE_MEANS: Define success criteria');
console.log('- BOOTSTRAP: Fill in FILES_TO_OPEN, SEARCH_TERMS, RISK_MAP');
console.log('- AUTHORITY: Fill SPEC_ANCHOR; keep SPEC_BASELINE as provenance');
