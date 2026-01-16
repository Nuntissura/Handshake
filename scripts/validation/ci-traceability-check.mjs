#!/usr/bin/env node
/**
 * CI traceability check [CX-903]
 * Validates commit messages reference WP_IDs and that task packets exist.
 * Task Board + task packets are the primary micro-log; logger is optional (milestones/hard bugs only).
 */

import { execSync } from 'child_process';
import fs from 'fs';

import { resolveGovernanceReference } from './governance-reference.mjs';

let governanceRef = null;
try {
  governanceRef = resolveGovernanceReference();
} catch {
  governanceRef = null;
}

const bannerRef = governanceRef
  ? `Governance Reference: ${governanceRef.codexFilename}`
  : 'Governance Reference: UNRESOLVED (see docs/SPEC_CURRENT.md)';

console.log(`\ndY"? CI Traceability Check (${bannerRef})...\n`);

let errors = [];
let warnings = [];

// Get recent commits (last 10)
let commits;
try {
  const commitOutput = execSync('git log -10 --pretty=format:"%H|%s|%an|%ae"', {
    encoding: 'utf8',
  });
  commits = commitOutput
    .split('\n')
    .filter(Boolean)
    .map(line => {
      const [hash, subject, author, email] = line.split('|');
      return { hash, subject, author, email };
    });
} catch (err) {
  console.error('ƒ?O Could not retrieve git commits');
  console.error(err.message);
  process.exit(1);
}

console.log(`Found ${commits.length} recent commits to check\n`);

// Check 1: WP_ID references in commits
console.log('Check 1: WP_ID references in commits');
const wpIdPattern = /WP-[\w-]+/;
const commitsWithWpId = commits.filter(c => wpIdPattern.test(c.subject));
const commitsWithoutWpId = commits.filter(c => !wpIdPattern.test(c.subject));

console.log(`  ƒo. ${commitsWithWpId.length} commits reference WP_ID`);
if (commitsWithoutWpId.length > 0) {
  console.log(`  ƒsÿ‹,?  ${commitsWithoutWpId.length} commits without WP_ID:`);
  commitsWithoutWpId.slice(0, 3).forEach(c => {
    console.log(`    - ${c.hash.slice(0, 7)}: ${c.subject}`);
  });
  warnings.push(
    `${commitsWithoutWpId.length} commits without WP_ID reference`
  );
}

// Check 2: Task packets exist for referenced WP_IDs
console.log('\nCheck 2: Task packets exist for referenced WP_IDs');
const taskPacketDir = 'docs/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  errors.push('docs/task_packets/ directory does not exist [CX-213]');
  console.log('ƒ?O FAIL: No task_packets directory');
  console.log('  Run: mkdir -p docs/task_packets');
} else {
  const taskPackets = fs
    .readdirSync(taskPacketDir)
    .filter(f => f.endsWith('.md'));
  console.log(`  ƒo. docs/task_packets/ exists (${taskPackets.length} packets)`);

  const missingPackets = [];
  commitsWithWpId.forEach(commit => {
    const wpId = commit.subject.match(wpIdPattern)?.[0];
    if (wpId) {
      const hasPacket = taskPackets.some(p => p.includes(wpId));
      if (!hasPacket) {
        missingPackets.push({ commit, wpId });
      }
    }
  });

  if (missingPackets.length > 0) {
    console.log(
      `  ƒsÿ‹,?  ${missingPackets.length} WP_IDs in commits without task packet files:`
    );
    missingPackets.slice(0, 3).forEach(({ commit, wpId }) => {
      console.log(`    - ${commit.hash.slice(0, 7)}: ${wpId}`);
    });
    errors.push(
      `${missingPackets.length} commits reference WP_ID without matching task packet`
    );
  } else {
    console.log('  ƒo. All WP_IDs in commits have task packets');
  }
}

// Optional: Logger presence (info only)
console.log('\nCheck 3: Logger (optional, milestones/hard bugs)');
const loggerFiles = fs
  .readdirSync('.')
  .filter(f => f.startsWith('Handshake_logger_') && f.endsWith('.md'))
  .sort()
  .reverse();
if (loggerFiles.length === 0) {
  console.log('  ℹ️  Logger not present (optional)');
} else {
  console.log(`  ℹ️  Logger present: ${loggerFiles[0]} (milestones/hard bugs only)`);
}

// Check 4: Governance Reference exists (derived from docs/SPEC_CURRENT.md)
console.log('\nCheck 4: Governance Reference exists (from docs/SPEC_CURRENT.md)');
try {
  const ref = governanceRef || resolveGovernanceReference();
  if (!fs.existsSync(ref.codexPathAbs)) {
    errors.push(
      `Governance Reference file not found: ${ref.codexFilename} (resolved from docs/SPEC_CURRENT.md)`
    );
    console.log(`ƒ?O FAIL: Governance Reference missing: ${ref.codexFilename}`);
  } else {
    console.log(`  ƒo. ${ref.codexFilename} exists`);
  }
} catch (err) {
  errors.push(`Could not resolve Governance Reference from docs/SPEC_CURRENT.md: ${err.message}`);
  console.log('ƒ?O FAIL: Could not resolve Governance Reference from docs/SPEC_CURRENT.md');
}

// Check 5: Protocol files exist
console.log('\nCheck 5: Protocol files exist');
const protocolFiles = [
  'docs/ORCHESTRATOR_PROTOCOL.md',
  'docs/CODER_PROTOCOL.md',
];

protocolFiles.forEach(file => {
  if (!fs.existsSync(file)) {
    errors.push(`${file} not found [CX-900]`);
    console.log(`  ƒ?O FAIL: ${file} missing`);
  } else {
    console.log(`  ƒo. ${file} exists`);
  }
});

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0 && warnings.length === 0) {
  console.log('ƒo. CI traceability check PASSED\n');
  process.exit(0);
} else if (errors.length === 0 && warnings.length > 0) {
  console.log('ƒsÿ‹,?  CI traceability check PASSED with warnings\n');
  console.log('Warnings:');
  warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  console.log('\nWarnings do not block CI, but should be addressed.');
  process.exit(0);
} else {
  console.log('ƒ?O CI traceability check FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues to pass CI traceability check.');
  console.log('See: docs/SPEC_CURRENT.md');
  process.exit(1);
}
