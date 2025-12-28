#!/usr/bin/env node
/**
 * Post-work validation (deterministic manifest + gates)
 * - Enforces manifest schema and gate coverage inherited from COR-701 (anchors/rails/window/sha1/line_delta/concurrency)
 * - Keeps existing surface: `node post-work-check.mjs WP-{ID}` (also used by `just post-work {wp}`)
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execSync } from 'child_process';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node post-work-check.mjs WP-{ID}');
  process.exit(1);
}

const SPEC_PATH = path.join('scripts', 'validation', 'cor701-spec.json');
const spec = JSON.parse(fs.readFileSync(SPEC_PATH, 'utf8'));

console.log(`\nPost-work validation for ${WP_ID} (deterministic manifest + gates)...\n`);

const errors = [];
const warnings = [];

const readFileIfExists = (p) => {
  try {
    return fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
};

const computeSha1 = (p) => {
  const buf = fs.readFileSync(p);
  return crypto.createHash('sha1').update(buf).digest('hex');
};

const loadHeadVersion = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const data = execSync(`git show HEAD:${gitPath}`, { encoding: 'utf8' });
    return data;
  } catch {
    return null;
  }
};

const getNumstatDelta = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const out = execSync(`git diff --numstat HEAD -- "${gitPath}"`, { encoding: 'utf8' }).trim();
    if (!out) return null;
    const [addsStr, delsStr] = out.split('\t');
    const adds = parseInt(addsStr, 10);
    const dels = parseInt(delsStr, 10);
    if (Number.isNaN(adds) || Number.isNaN(dels)) return null;
    return adds - dels;
  } catch {
    return null;
  }
};

const parseDiffHunks = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diff = execSync(`git diff --unified=0 HEAD -- "${gitPath}"`, { encoding: 'utf8' });
    const hunks = [];
    const hunkHeader = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/;
    diff.split('\n').forEach((line) => {
      const m = line.match(hunkHeader);
      if (m) {
        const [_, oStart, oLen, nStart, nLen] = m;
        hunks.push({
          oldStart: parseInt(oStart, 10),
          oldLen: oLen ? parseInt(oLen, 10) : 1,
          newStart: parseInt(nStart, 10),
          newLen: nLen ? parseInt(nLen, 10) : 1,
        });
      }
    });
    return hunks;
  } catch {
    return [];
  }
};

const taskPacketDir = 'docs/task_packets';
let packetContent = '';
let packetPath = '';
if (fs.existsSync(taskPacketDir)) {
  const taskPacketFiles = fs.readdirSync(taskPacketDir)
    .filter((f) => f.includes(WP_ID));
  if (taskPacketFiles.length > 0) {
    packetPath = `${taskPacketDir}/${taskPacketFiles[0]}`;
    packetContent = readFileIfExists(packetPath);
  }
}

const parseValidationManifest = (content) => {
  if (!content) return null;
  const lines = content.split('\n');
  const startIdx = lines.findIndex((line) => /^##\s*validation/i.test(line));
  if (startIdx === -1) return null;
  const manifestLines = [];
  for (let i = startIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^##\s+/.test(line)) break;
    manifestLines.push(line);
  }

  const getField = (label) => {
    const re = new RegExp(`-\\s*${label}\\s*:\\s*(.+)`, 'i');
    const hit = manifestLines.find((l) => re.test(l));
    if (!hit) return '';
    return hit.match(re)[1].trim().replace(/^`|`$/g, '');
  };

  const gatesBlockStart = manifestLines.findIndex((l) => /Gates Passed/i.test(l));
  const gateSet = new Set();
  if (gatesBlockStart !== -1) {
    for (let i = gatesBlockStart + 1; i < manifestLines.length; i += 1) {
      const line = manifestLines[i].trim();
      if (!line.startsWith('-')) break;
      const m = line.match(/- \[(x|X)\]\s*([a-z0-9_]+)/i);
      if (m) gateSet.add(m[2].toLowerCase());
    }
  }

  return {
    target_file: getField('Target File'),
    start: getField('Start'),
    end: getField('End'),
    pre_sha1: getField('Pre-SHA1'),
    post_sha1: getField('Post-SHA1'),
    line_delta: getField('Line Delta'),
    gates_passed: gateSet,
    rawLines: manifestLines.join('\n'),
  };
};

// Check 1: manifest present and ASCII only
console.log('Check 1: Validation manifest present');
if (!packetContent) {
  errors.push('No task packet found for this WP_ID');
} else if (!/VALIDATION/i.test(packetContent)) {
  errors.push('Task packet missing VALIDATION section');
} else if (/[^\x00-\x7F]/.test(packetContent)) {
  errors.push('Task packet contains non-ASCII characters (manifest must be ASCII)');
}

const manifest = parseValidationManifest(packetContent);
if (!manifest) {
  errors.push('VALIDATION section found but manifest fields not parsed');
}

// Check 2: manifest schema
if (manifest) {
  console.log('\nCheck 2: Manifest fields');
  spec.requiredFields.forEach((field) => {
    const value = manifest[field];
    if (!value || (typeof value === 'string' && value.trim() === '')) {
      errors.push(`Manifest missing required field: ${field}`);
    }
  });

  const shaRegex = /^[a-f0-9]{40}$/i;
  if (manifest.pre_sha1 && !shaRegex.test(manifest.pre_sha1)) {
    errors.push('pre_sha1 must be a 40-char hex SHA1');
  }
  if (manifest.post_sha1 && !shaRegex.test(manifest.post_sha1)) {
    errors.push('post_sha1 must be a 40-char hex SHA1');
  }

  const startNum = parseInt(manifest.start, 10);
  const endNum = parseInt(manifest.end, 10);
  if (Number.isNaN(startNum) || Number.isNaN(endNum) || startNum < 1 || endNum < startNum) {
    errors.push('Start/End must be integers with start >=1 and end >= start');
  }

  const deltaNum = parseInt(manifest.line_delta, 10);
  if (manifest.line_delta === '' || Number.isNaN(deltaNum)) {
    errors.push('line_delta must be an integer (adds - dels)');
  }

  spec.requiredGates.forEach((gate) => {
    if (!manifest.gates_passed.has(gate)) {
      errors.push(`Gate missing or unchecked: ${gate} (${spec.gateErrorCodes[gate]})`);
    }
  });
}

// Check 3: file integrity vs manifest
if (manifest && manifest.target_file) {
  console.log('\nCheck 3: File integrity');
  const targetPath = path.normalize(manifest.target_file.replace(/^`|`$/g, ''));
  if (!fs.existsSync(targetPath)) {
    errors.push(`Target file does not exist: ${targetPath} (${spec.gateErrorCodes.filename_canonical_and_openable})`);
  } else {
    const postSha = computeSha1(targetPath);
    if (manifest.post_sha1 && manifest.post_sha1 !== postSha) {
      errors.push(`post_sha1 mismatch for ${targetPath} (${spec.gateErrorCodes.post_sha1_captured})`);
    }

    const headContent = loadHeadVersion(targetPath);
    if (headContent === null) {
      warnings.push('Could not load HEAD version for concurrency check (new file or not tracked)');
    } else {
      const headSha = crypto.createHash('sha1').update(headContent).digest('hex');
      if (manifest.pre_sha1 && headSha !== manifest.pre_sha1) {
        errors.push(`pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
      }
    }

    const hunks = parseDiffHunks(targetPath);
    const windowStart = parseInt(manifest.start, 10);
    const windowEnd = parseInt(manifest.end, 10);
    hunks.forEach((h) => {
      const oldEnd = h.oldStart + Math.max(h.oldLen - 1, 0);
      const newEnd = h.newStart + Math.max(h.newLen - 1, 0);
      const oldOutside = h.oldLen > 0 && (h.oldStart < windowStart || oldEnd > windowEnd);
      const newOutside = h.newLen > 0 && (h.newStart < windowStart || newEnd > windowEnd);
      if (oldOutside || newOutside) {
        errors.push(`Diff touches lines outside declared window [${windowStart}, ${windowEnd}] (${spec.gateErrorCodes.rails_untouched_outside_window})`);
      }
    });

    const numstatDelta = getNumstatDelta(targetPath);
    const deltaNum = parseInt(manifest.line_delta, 10);
    if (numstatDelta !== null && !Number.isNaN(deltaNum) && numstatDelta !== deltaNum) {
      errors.push(`line_delta (${deltaNum}) does not match git diff delta (${numstatDelta}) (${spec.gateErrorCodes.line_delta_equals_expected})`);
    }
  }
}

// Check 4: git status sanity
console.log('\nCheck 4: Git status');
try {
  const gitStatus = execSync('git status --short', { encoding: 'utf8' }).trim();
  if (!gitStatus) {
    errors.push('No files changed (git status clean)');
  }
} catch {
  warnings.push('Could not read git status');
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Post-work validation PASSED with warnings\n');
    console.log('Warnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Post-work validation PASSED');
  }
  console.log('\nYou may proceed with commit.');
  process.exit(0);
} else {
  console.log('Post-work validation FAILED\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before committing (gates enforce determinism).');
  console.log('See: docs/CODER_PROTOCOL.md');
  process.exit(1);
}
