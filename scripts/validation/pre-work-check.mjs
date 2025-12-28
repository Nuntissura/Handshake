#!/usr/bin/env node
/**
 * Pre-work validation [CX-580, CX-620]
 * - Verifies task packet exists before work starts
 * - Ensures deterministic manifest template (COR-701-style) is present so post-work can enforce gates
 */

import fs from 'fs';
import path from 'path';

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

if (taskPacketFiles.length === 0) {
  errors.push(`No task packet file found for ${WP_ID} in docs/task_packets/`);
  console.log('FAIL: No task packet file');
} else {
  const packetPath = path.join(taskPacketDir, taskPacketFiles[0]);
  packetContent = fs.readFileSync(packetPath, 'utf8');
  console.log(`PASS: Found ${taskPacketFiles[0]}`);

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

  const missingFields = requiredFields.filter((field) => !packetContent.includes(field));

  if (missingFields.length > 0) {
    errors.push(`Task packet missing fields: ${missingFields.join(', ')}`);
    console.log(`FAIL: Missing ${missingFields.join(', ')}`);
  } else {
    console.log('PASS: All required fields present');
  }

  // Check 3: Deterministic manifest template present
  console.log('\nCheck 3: Deterministic manifest template');
  if (!/##\s*validation/i.test(packetContent)) {
    errors.push('VALIDATION section missing (required for deterministic manifest)');
    console.log('FAIL: Missing VALIDATION section');
  } else {
    const lower = packetContent.toLowerCase();
    const fieldMissing = spec.requiredFields.filter((f) => !lower.includes(f.replace(/_/g, ' ')));
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
