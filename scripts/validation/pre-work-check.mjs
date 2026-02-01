#!/usr/bin/env node
/**
 * Pre-work validation [CX-580, CX-620]
 * - Verifies task packet exists before work starts
 * - Ensures deterministic manifest template (COR-701-style) is present so post-work can enforce gates
 */

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
  defaultRefinementPath,
  validateRefinementFile,
} from './refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node pre-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\nPre-work validation for ${WP_ID}...\n`);

const errors = [];
const warnings = [];
const spec = JSON.parse(fs.readFileSync(path.join('scripts', 'validation', 'cor701-spec.json'), 'utf8'));

// Check 1: Task packet file exists
console.log('Check 1: Task packet file exists');
const taskPacketDir = 'docs/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
}

const taskPacketFiles = fs.readdirSync(taskPacketDir)
  .filter((f) => f.includes(WP_ID) && f.endsWith('.md'));

let packetContent = '';
let packetPath = '';

if (taskPacketFiles.length === 0) {
  errors.push(`No task packet file found for ${WP_ID} in docs/task_packets/`);
  console.log('FAIL: No task packet file');
} else {
  packetPath = path.join(taskPacketDir, taskPacketFiles[0]);
  packetContent = fs.readFileSync(packetPath, 'utf8');
  console.log(`PASS: Found ${taskPacketFiles[0]}`);

  // Check 1.5: Worktree + branch preflight (mechanical guard against wrong-worktree edits)
  console.log('\nCheck 1.5: Worktree + branch preflight [CX-WT-001]');
  const packetFormatVersion = (packetContent.match(/^\s*-\s*PACKET_FORMAT_VERSION\s*:\s*(.+)\s*$/mi) || [])[1]?.trim();
  const enforceWorktreeGate = !!packetFormatVersion;

  let currentBranch = '';
  let currentTop = '';
  try {
    currentBranch = execSync('git rev-parse --abbrev-ref HEAD', { encoding: 'utf8' }).trim();
    currentTop = execSync('git rev-parse --show-toplevel', { encoding: 'utf8' }).trim();
  } catch {
    warnings.push('Could not read current git branch/worktree (git rev-parse failed)');
  }

  let lastPrepare = null;
  try {
    const gatesPath = path.join('docs', 'ORCHESTRATOR_GATES.json');
    const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
    const logs = Array.isArray(gates?.gate_logs) ? gates.gate_logs : [];
    lastPrepare = [...logs].reverse().find((l) => l?.wpId === WP_ID && l?.type === 'PREPARE') || null;
  } catch {
    lastPrepare = null;
  }

  if (!lastPrepare) {
    const msg = `Missing PREPARE record in docs/ORCHESTRATOR_GATES.json for ${WP_ID} (expected: just record-prepare ${WP_ID} {Coder-A|Coder-B} ...)`;
    if (enforceWorktreeGate) {
      errors.push(msg);
      console.log('FAIL: ' + msg);
    } else {
      warnings.push(msg);
      console.log('WARN: ' + msg);
    }
  } else {
    const expectedBranch = (lastPrepare.branch || '').trim();
    const expectedWorktreeDir = (lastPrepare.worktree_dir || '').trim();

    if (expectedBranch && currentBranch && expectedBranch !== currentBranch) {
      errors.push(
        `Wrong branch for ${WP_ID}: expected ${expectedBranch} (from PREPARE), got ${currentBranch}`
      );
      console.log(`FAIL: Branch mismatch (expected ${expectedBranch}, got ${currentBranch})`);
    } else if (expectedBranch && currentBranch) {
      console.log(`PASS: Branch matches PREPARE (${currentBranch})`);
    }

    if (expectedWorktreeDir && currentTop) {
      const expectedAbs = path.isAbsolute(expectedWorktreeDir)
        ? path.resolve(expectedWorktreeDir)
        : path.resolve(currentTop, expectedWorktreeDir);
      const currentAbs = path.resolve(currentTop);
      if (expectedAbs.toLowerCase() !== currentAbs.toLowerCase()) {
        warnings.push(
          `Worktree_dir mismatch for ${WP_ID}: PREPARE says ${expectedWorktreeDir} (resolves to ${expectedAbs}), current is ${currentAbs}`
        );
      }
    }
  }

  // Check 2: Packet has required fields
  console.log('\nCheck 2: Task packet structure');
  const requiredFields = [
    'TASK_ID',
    'RISK_TIER',
    'SCOPE',
    'TEST_PLAN',
    'DONE_MEANS',
    'BOOTSTRAP',
  ];

  const lowerContent = packetContent.toLowerCase();
  const missingFields = requiredFields.filter((field) => !lowerContent.includes(field.toLowerCase()));

  if (missingFields.length > 0) {
    errors.push(`Task packet missing fields: ${missingFields.join(', ')}`);
    console.log(`FAIL: Missing ${missingFields.join(', ')}`);
  } else {
    console.log('PASS: All required fields present');
  }

  // Check 2.5: Spec provenance/target fields (non-blocking; backward compatible)
  const hasLegacySpec = /SPEC_CURRENT/i.test(packetContent);
  const hasSpecBaseline = /SPEC_BASELINE/i.test(packetContent);
  const hasSpecTarget = /SPEC_TARGET/i.test(packetContent);
  if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
    warnings.push('Spec reference missing: include SPEC_BASELINE (provenance) and SPEC_TARGET (closure target), or legacy SPEC_CURRENT.');
  }

  // Check 2.5B: MERGE_BASE_SHA (recommended for deterministic multi-commit post-work)
  const mergeBaseSha = (packetContent.match(/^\s*-\s*MERGE_BASE_SHA\s*:\s*([a-f0-9]{40})\s*$/mi) || [])[1]?.trim();
  if (!mergeBaseSha) {
    warnings.push('Packet missing MERGE_BASE_SHA; for multi-commit WPs prefer deterministic evidence: just post-work WP-{ID} --range <MERGE_BASE_SHA>..HEAD');
  }

  // Check 2.6: Canonical Status field (governance invariant)
  const statusLine = (packetContent.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || '';
  const statusNorm = statusLine.trim().toLowerCase();
  if (!statusLine) {
    errors.push('Missing canonical **Status:** field');
  }

  const isDoneLike = /\b(done|validated|complete)\b/i.test(statusLine);
  const requiresRefinementGate = !isDoneLike; // pre-work implies active work; enforce unless explicitly Done/Validated.

  // Check 2.7: Technical Refinement gate (unskippable for active packets)
  if (requiresRefinementGate) {
    console.log('\nCheck 2.7: Technical Refinement gate');

    const refinementFile = (packetContent.match(/^\s*-\s*REFINEMENT_FILE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || defaultRefinementPath(WP_ID);

    const refinementValidation = validateRefinementFile(refinementFile, { expectedWpId: WP_ID, requireSignature: true });
    if (!refinementValidation.ok) {
      errors.push(`Technical refinement gate failed (see ${refinementFile})`);
      refinementValidation.errors.forEach((e) => errors.push(`  - ${e}`));
    } else {
      console.log('PASS: Refinement file exists and is approved/signed');
    }

    const packetSig = (packetContent.match(/^\s*-\s*USER_SIGNATURE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*\*\*User Signature Locked:\*\*\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*User Signature Locked:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || '';

    if (!packetSig || /<pending>/i.test(packetSig)) {
      errors.push('USER_SIGNATURE missing or <pending> (active packets must be locked before work starts)');
    } else if (refinementValidation.ok && refinementValidation.parsed.signature && packetSig !== refinementValidation.parsed.signature) {
      errors.push(`USER_SIGNATURE mismatch: packet has ${packetSig}, refinement has ${refinementValidation.parsed.signature}`);
    }

    // Protocol requirement: signature must be present in SIGNATURE_AUDIT.md
    try {
      const auditPath = path.join('docs', 'SIGNATURE_AUDIT.md');
      const audit = fs.readFileSync(auditPath, 'utf8');
      if (packetSig && !audit.includes(`| ${packetSig} |`)) {
        errors.push(`USER_SIGNATURE not found in docs/SIGNATURE_AUDIT.md (${packetSig})`);
      }
    } catch {
      warnings.push('Could not verify signature against docs/SIGNATURE_AUDIT.md');
    }

    // Safety checkpoint gate: packet + refinement must be committed before development starts.
    // This prevents untracked/uncommitted WP artifacts from being lost during accidental clean/reset operations.
    console.log('\nCheck 2.8: WP checkpoint commit gate');
    try {
      execSync(`git cat-file -e HEAD:${packetPath.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Task packet is not committed yet (checkpoint required): ${packetPath.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} docs/SIGNATURE_AUDIT.md docs/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }

    try {
      execSync(`git cat-file -e HEAD:${refinementFile.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Refinement file is not committed yet (checkpoint required): ${refinementFile.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} docs/SIGNATURE_AUDIT.md docs/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }
  } else {
    console.log('\nCheck 2.7: Technical Refinement gate (skipped for Done/Validated packets)');
  }

  // Check 3: Deterministic manifest template present
  console.log('\nCheck 3: Deterministic manifest template');
  if (!/##\s*validation/i.test(packetContent)) {
    errors.push('VALIDATION section missing (required for deterministic manifest)');
    console.log('FAIL: Missing VALIDATION section');
  } else {
    const lower = packetContent.toLowerCase();
    const lowerNorm = lower.replace(/[-_]/g, ' ');
    const fieldMissing = spec.requiredFields.filter((f) => !lowerNorm.includes(f.replace(/_/g, ' ')));
    if (fieldMissing.length > 0) {
      errors.push(`Validation manifest missing fields: ${fieldMissing.join(', ')}`);
      console.log(`FAIL: Validation manifest missing fields: ${fieldMissing.join(', ')}`);
    } else {
      console.log('PASS: Manifest fields present');
    }

    if (!/gates passed/i.test(packetContent)) {
      errors.push('Validation manifest missing "Gates Passed" checklist');
      console.log('FAIL: Missing gates checklist');
    } else {
      const gateHits = spec.requiredGates.filter((g) => lower.includes(g));
      if (gateHits.length !== spec.requiredGates.length) {
        warnings.push('Validation manifest present but some gates are not listed (ensure template is fully copied)');
      } else {
        console.log('PASS: Gates checklist present');
      }
    }
  }
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Pre-work validation PASSED with warnings\n');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Pre-work validation PASSED');
  }
  console.log('\nYou may proceed with implementation.');
  process.exit(0);
} else {
  console.log('Pre-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before starting work.');
  console.log('See: docs/ORCHESTRATOR_PROTOCOL.md or docs/CODER_PROTOCOL.md');
  process.exit(1);
}
