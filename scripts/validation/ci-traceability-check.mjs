#!/usr/bin/env node
/**
 * CI traceability check [CX-903]
 * Validates commit messages reference WP_IDs and logger has entries
 * Run in CI to enforce workflow compliance
 */

import { execSync } from 'child_process';
import fs from 'fs';

console.log('\n🔍 CI Traceability Check (Codex v0.8)...\n');

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
  console.error('❌ Could not retrieve git commits');
  console.error(err.message);
  process.exit(1);
}

console.log(`Found ${commits.length} recent commits to check\n`);

// Check 1: WP_ID references in commits
console.log('Check 1: WP_ID references in commits');
const wpIdPattern = /WP-[\w-]+/;
const commitsWithWpId = commits.filter(c => wpIdPattern.test(c.subject));
const commitsWithoutWpId = commits.filter(c => !wpIdPattern.test(c.subject));

console.log(`  ✅ ${commitsWithWpId.length} commits reference WP_ID`);
if (commitsWithoutWpId.length > 0) {
  console.log(`  ⚠️  ${commitsWithoutWpId.length} commits without WP_ID:`);
  commitsWithoutWpId.slice(0, 3).forEach(c => {
    console.log(`    - ${c.hash.slice(0, 7)}: ${c.subject}`);
  });
  warnings.push(
    `${commitsWithoutWpId.length} commits without WP_ID reference`
  );
}

// Check 2: Logger file exists and has recent entries
console.log('\nCheck 2: Logger traceability');
const loggerFiles = fs
  .readdirSync('.')
  .filter(f => f.startsWith('Handshake_logger_') && f.endsWith('.md'))
  .sort()
  .reverse();

if (loggerFiles.length === 0) {
  errors.push('No logger file found in repository root');
  console.log('❌ FAIL: No logger file');
} else {
  const latestLogger = loggerFiles[0];
  console.log(`  Found logger: ${latestLogger}`);

  const loggerContent = fs.readFileSync(latestLogger, 'utf8');
  const entries = loggerContent.split('[RAW_ENTRY_ID]').slice(1); // Skip header

  console.log(`  ${entries.length} logger entries found`);

  // Check that commits with WP_ID have corresponding logger entries
  const missingLoggerEntries = [];
  commitsWithWpId.forEach(commit => {
    const wpIdMatch = commit.subject.match(wpIdPattern);
    if (wpIdMatch) {
      const wpId = wpIdMatch[0];
      const hasLoggerEntry = loggerContent.includes(wpId);
      if (!hasLoggerEntry) {
        missingLoggerEntries.push({ commit, wpId });
      }
    }
  });

  if (missingLoggerEntries.length > 0) {
    console.log(
      `  ⚠️  ${missingLoggerEntries.length} commits with WP_ID but no logger entry:`
    );
    missingLoggerEntries.slice(0, 3).forEach(({ commit, wpId }) => {
      console.log(`    - ${commit.hash.slice(0, 7)}: ${wpId}`);
    });
    warnings.push(
      `${missingLoggerEntries.length} WP_IDs in commits without logger entries`
    );
  } else {
    console.log('  ✅ All WP_IDs in commits have logger entries');
  }
}

// Check 3: Task packets directory exists
console.log('\nCheck 3: Task packets directory');
if (!fs.existsSync('docs/task_packets')) {
  errors.push('docs/task_packets/ directory does not exist [CX-213]');
  console.log('❌ FAIL: No task_packets directory');
  console.log('  Run: mkdir -p docs/task_packets');
} else {
  const taskPackets = fs
    .readdirSync('docs/task_packets')
    .filter(f => f.endsWith('.md'));
  console.log(`  ✅ docs/task_packets/ exists (${taskPackets.length} packets)`);

  // Check that task packets match logger WP_IDs
  if (loggerFiles.length > 0) {
    const loggerContent = fs.readFileSync(loggerFiles[0], 'utf8');
    const wpIdsInLogger = [...loggerContent.matchAll(/WP-[\w-]+/g)].map(
      m => m[0]
    );
    const uniqueWpIds = [...new Set(wpIdsInLogger)];

    const missingPackets = uniqueWpIds.filter(wpId => {
      return !taskPackets.some(p => p.includes(wpId));
    });

    if (missingPackets.length > 0) {
      console.log(
        `  ⚠️  ${missingPackets.length} WP_IDs in logger without task packets:`
      );
      missingPackets.slice(0, 3).forEach(wpId => {
        console.log(`    - ${wpId}`);
      });
      warnings.push(
        `${missingPackets.length} WP_IDs in logger without task packet files`
      );
    }
  }
}

// Check 4: Codex v0.8 exists
console.log('\nCheck 4: Codex v0.8 exists');
if (!fs.existsSync('Handshake Codex v0.8.md')) {
  errors.push('Handshake Codex v0.8.md not found in repository root');
  console.log('❌ FAIL: Codex v0.8 missing');
} else {
  console.log('  ✅ Handshake Codex v0.8.md exists');
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
    console.log(`  ❌ FAIL: ${file} missing`);
  } else {
    console.log(`  ✅ ${file} exists`);
  }
});

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0 && warnings.length === 0) {
  console.log('✅ CI traceability check PASSED\n');
  process.exit(0);
} else if (errors.length === 0 && warnings.length > 0) {
  console.log('⚠️  CI traceability check PASSED with warnings\n');
  console.log('Warnings:');
  warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  console.log('\nWarnings do not block CI, but should be addressed.');
  process.exit(0);
} else {
  console.log('❌ CI traceability check FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues to pass CI traceability check.');
  console.log('See: Handshake Codex v0.8.md');
  process.exit(1);
}
