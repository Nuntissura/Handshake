#!/usr/bin/env node
/**
 * Post-work validation [CX-571, CX-623]
 * Verifies work is complete and validated before commit.
 * Task packet + task board are the primary micro-log; logger is optional (milestones/hard bugs only).
 */

import fs from 'fs';
import { execSync } from 'child_process';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Г?O Usage: node post-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\ndY"? Post-work validation for ${WP_ID}...\n`);

let errors = [];
let warnings = [];

// Load task packet (if present)
const taskPacketDir = 'docs/task_packets';
let packetContent = '';
let packetPath = '';
if (fs.existsSync(taskPacketDir)) {
  const taskPacketFiles = fs.readdirSync(taskPacketDir)
    .filter(f => f.includes(WP_ID));
  if (taskPacketFiles.length > 0) {
    packetPath = `${taskPacketDir}/${taskPacketFiles[0]}`;
    packetContent = fs.readFileSync(packetPath, 'utf8');
  }
}

// Check 1: Validation documented in task packet
console.log('Check 1: Validation documented in task packet');
if (!packetContent) {
  errors.push(`No task packet found for ${WP_ID}`);
  console.log('Г?O FAIL: Task packet missing');
} else {
  const hasValidationSection = /VALIDATION/i.test(packetContent);
  const hasOutcome = /(PASS|FAIL|Result|Outcome)/i.test(packetContent);
  if (!hasValidationSection) {
    errors.push('Task packet missing VALIDATION section');
    console.log('Г?O FAIL: No VALIDATION section');
  } else if (!hasOutcome) {
    warnings.push('Validation section present but no outcomes recorded');
    console.log('Гs Л,?  WARN: Validation outcomes not recorded');
  } else {
    console.log('Гo. PASS: Validation recorded in task packet');
  }
}

// Check 2: Files actually changed
console.log('\nCheck 2: Files changed');
try {
  const gitStatus = execSync('git status --short').toString();
  if (gitStatus.trim().length === 0) {
    errors.push('No files changed (git status clean)');
    console.log('Г?O FAIL: No changes detected');
  } else {
    console.log('Гo. PASS: Changes detected');
  }
} catch (err) {
  warnings.push('Could not check git status');
}

// Check 3: Tests status (if applicable)
console.log('\nCheck 3: Tests status');
if (packetContent) {
  const testPlanHasTests = packetContent.includes('cargo test') || packetContent.includes('pnpm test');
  if (testPlanHasTests) {
    const hasValidationText = /VALIDATION/i.test(packetContent) && /(PASS|FAIL)/i.test(packetContent);
    if (!hasValidationText) {
      warnings.push('Tests in TEST_PLAN but not documented in task packet validation');
      console.log('Гs Л,?  WARN: Tests in TEST_PLAN but not documented');
    } else {
      console.log('Гo. PASS: Tests documented');
    }
  }
}

// Check 4: AI review (MEDIUM/HIGH only)
console.log('\nCheck 4: AI review (if required)');
if (packetContent) {
  const riskTier = packetContent.match(/RISK_TIER[:\\s]+(\\w+)/)?.[1];

  if (riskTier === 'MEDIUM' || riskTier === 'HIGH') {
    if (!fs.existsSync('ai_review.md')) {
      errors.push('AI review required but ai_review.md not found');
      console.log('Г?O FAIL: Missing ai_review.md');
    } else {
      const reviewContent = fs.readFileSync('ai_review.md', 'utf8');
      if (reviewContent.includes('Decision: BLOCK')) {
        errors.push('AI review decision is BLOCK - must resolve');
        console.log('Г?O FAIL: AI review BLOCKED');
      } else {
        console.log('Гo. PASS: AI review complete');
      }
    }
  } else {
    console.log('Г,1Л,?  INFO: AI review not required (LOW risk)');
  }
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Гs Л,?  Post-work validation PASSED with warnings\n');
    console.log('Warnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Гo. Post-work validation PASSED');
  }
  console.log('\nYou may proceed with commit.');
  process.exit(0);
} else {
  console.log('Г?O Post-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before committing.');
  console.log('See: docs/CODER_PROTOCOL.md');
  process.exit(1);
}
