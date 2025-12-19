#!/usr/bin/env node
/**
 * Pre-work validation [CX-580, CX-620]
 * Verifies task packet exists before work starts
 */

import fs from 'fs';
import path from 'path';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('❌ Usage: node pre-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\n🔍 Pre-work validation for ${WP_ID}...\n`);

let errors = [];

// Check 1: Task packet file exists
console.log('Check 1: Task packet file exists');
const taskPacketDir = 'docs/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
}

const taskPacketFiles = fs.readdirSync(taskPacketDir)
  .filter(f => f.includes(WP_ID) && f.endsWith('.md'));

if (taskPacketFiles.length === 0) {
  errors.push(`No task packet file found for ${WP_ID} in docs/task_packets/`);
  console.log('❌ FAIL: No task packet file');
} else {
  console.log(`✅ PASS: Found ${taskPacketFiles[0]}`);

  // Check 2: Packet has required fields
  console.log('\nCheck 2: Task packet structure');
  const packetPath = path.join(taskPacketDir, taskPacketFiles[0]);
  const packetContent = fs.readFileSync(packetPath, 'utf8');

  const requiredFields = [
    'TASK_ID',
    'RISK_TIER',
    'SCOPE',
    'TEST_PLAN',
    'DONE_MEANS',
    'BOOTSTRAP'
  ];

  const missingFields = requiredFields.filter(field =>
    !packetContent.includes(field)
  );

  if (missingFields.length > 0) {
    errors.push(`Task packet missing fields: ${missingFields.join(', ')}`);
    console.log(`❌ FAIL: Missing ${missingFields.join(', ')}`);
  } else {
    console.log('✅ PASS: All required fields present');
  }
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  console.log('✅ Pre-work validation PASSED');
  console.log('\nYou may proceed with implementation.');
  process.exit(0);
} else {
  console.log('❌ Pre-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  console.log('\nFix these issues before starting work.');
  console.log('See: .claude/ORCHESTRATOR_PROTOCOL.md or .claude/CODER_PROTOCOL.md');
  process.exit(1);
}
