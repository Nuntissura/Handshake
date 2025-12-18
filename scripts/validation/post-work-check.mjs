#!/usr/bin/env node
/**
 * Post-work validation [CX-571, CX-623]
 * Verifies work is complete and validated before commit
 */

import fs from 'fs';
import { execSync } from 'child_process';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('❌ Usage: node post-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\n🔍 Post-work validation for ${WP_ID}...\n`);

let errors = [];
let warnings = [];

// Check 1: Validation documented
console.log('Check 1: Validation documented');
const loggerFiles = fs.readdirSync('.')
  .filter(f => f.startsWith('Handshake_logger_') && f.endsWith('.md'))
  .sort()
  .reverse();

if (loggerFiles.length === 0) {
  errors.push('No logger file found');
} else {
  const loggerContent = fs.readFileSync(loggerFiles[0], 'utf8');
  const entries = loggerContent.split('[RAW_ENTRY_ID]');
  const relevantEntry = entries.find(e => e.includes(WP_ID));

  if (!relevantEntry) {
    errors.push(`No logger entry found for ${WP_ID}`);
    console.log('❌ FAIL: No logger entry');
  } else if (!relevantEntry.includes('RESULT')) {
    errors.push('Logger entry missing RESULT field');
    console.log('❌ FAIL: Missing RESULT');
  } else if (relevantEntry.includes('RESULT]\nNone')) {
    warnings.push('Work not marked complete in logger');
    console.log('⚠️  WARN: RESULT still "None"');
  } else {
    console.log('✅ PASS: Logger entry complete');
  }
}

// Check 2: Files actually changed
console.log('\nCheck 2: Files changed');
try {
  const gitStatus = execSync('git status --short').toString();
  if (gitStatus.trim().length === 0) {
    errors.push('No files changed (git status clean)');
    console.log('❌ FAIL: No changes detected');
  } else {
    console.log('✅ PASS: Changes detected');
  }
} catch (err) {
  warnings.push('Could not check git status');
}

// Check 3: Tests status (if applicable)
console.log('\nCheck 3: Tests status');
const taskPacketDir = 'docs/task_packets';
if (fs.existsSync(taskPacketDir)) {
  const taskPacketFiles = fs.readdirSync(taskPacketDir)
    .filter(f => f.includes(WP_ID));

  if (taskPacketFiles.length > 0) {
    const packetContent = fs.readFileSync(
      `${taskPacketDir}/${taskPacketFiles[0]}`,
      'utf8'
    );

    if (packetContent.includes('cargo test') || packetContent.includes('pnpm test')) {
      // Check if tests were mentioned in logger
      if (loggerFiles.length > 0) {
        const loggerContent = fs.readFileSync(loggerFiles[0], 'utf8');
        if (!loggerContent.includes('test') && !loggerContent.includes('VALIDATION')) {
          warnings.push('Tests in TEST_PLAN but not documented in logger');
          console.log('⚠️  WARN: Tests not documented');
        } else {
          console.log('✅ PASS: Validation documented');
        }
      }
    }
  }
}

// Check 4: AI review (MEDIUM/HIGH only)
console.log('\nCheck 4: AI review (if required)');
if (fs.existsSync(taskPacketDir)) {
  const taskPacketFiles = fs.readdirSync(taskPacketDir)
    .filter(f => f.includes(WP_ID));

  if (taskPacketFiles.length > 0) {
    const packetContent = fs.readFileSync(
      `${taskPacketDir}/${taskPacketFiles[0]}`,
      'utf8'
    );
    const riskTier = packetContent.match(/RISK_TIER[:\s]+(\w+)/)?.[1];

    if (riskTier === 'MEDIUM' || riskTier === 'HIGH') {
      if (!fs.existsSync('ai_review.md')) {
        errors.push('AI review required but ai_review.md not found');
        console.log('❌ FAIL: Missing ai_review.md');
      } else {
        const reviewContent = fs.readFileSync('ai_review.md', 'utf8');
        if (reviewContent.includes('Decision: BLOCK')) {
          errors.push('AI review decision is BLOCK - must resolve');
          console.log('❌ FAIL: AI review BLOCKED');
        } else {
          console.log('✅ PASS: AI review complete');
        }
      }
    } else {
      console.log('ℹ️  INFO: AI review not required (LOW risk)');
    }
  }
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('⚠️  Post-work validation PASSED with warnings\n');
    console.log('Warnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('✅ Post-work validation PASSED');
  }
  console.log('\nYou may proceed with commit.');
  process.exit(0);
} else {
  console.log('❌ Post-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before committing.');
  console.log('See: .claude/CODER_PROTOCOL.md');
  process.exit(1);
}
