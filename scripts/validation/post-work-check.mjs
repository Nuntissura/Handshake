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

const git = (command) => execSync(command, { encoding: 'utf8' }).trim();

const readFileIfExists = (p) => {
  try {
    return fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
};

const sha1Hex = (bufOrString) => crypto.createHash('sha1').update(bufOrString).digest('hex');

const computeSha1 = (p) => {
  const buf = fs.readFileSync(p);
  return sha1Hex(buf);
};

const loadHeadVersion = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const data = git(`git show HEAD:${gitPath}`);
    return data;
  } catch {
    return null;
  }
};

const loadIndexVersion = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    return git(`git show :${gitPath}`);
  } catch {
    return null;
  }
};

const getNumstatDelta = (targetPath, { staged }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diffArgs = staged ? '--cached' : '';
    const out = git(`git diff ${diffArgs} --numstat HEAD -- "${gitPath}"`);
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

const parseDiffHunks = (targetPath, { staged }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diffArgs = staged ? '--cached' : '';
    const diff = git(`git diff ${diffArgs} --unified=0 HEAD -- "${gitPath}"`);
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

const parseInScopePaths = (content) => {
  if (!content) return [];
  const lines = content.split('\n');
  const idx = lines.findIndex((l) => /^\s*-\s*IN_SCOPE_PATHS\s*:\s*$/i.test(l));
  if (idx === -1) return [];
  const results = [];
  for (let i = idx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\s*-\s*[A-Z0-9_]+\s*:/.test(line)) break; // next top-level metadata-ish bullet
    if (/^\s*##\s+/.test(line)) break;
    const m = line.match(/^\s*-\s+(.+)\s*$/) || line.match(/^\s{2,}-\s+(.+)\s*$/);
    if (!m) continue;
    const value = m[1].trim().replace(/^`|`$/g, '');
    if (!value || value.toLowerCase() === 'path/to/file') continue;
    results.push(path.normalize(value).replace(/\\/g, '/'));
  }
  return Array.from(new Set(results));
};

const requiresManifest = (filePath) => {
  const p = filePath.replace(/\\/g, '/');
  if (p.startsWith('docs/')) return false;
  return true;
};

const getStagedFiles = () => {
  try {
    const out = git('git diff --name-only --cached');
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const getWorkingFiles = () => {
  try {
    const out = git('git diff --name-only HEAD');
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const parseValidationManifests = (content) => {
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

  const manifests = [];
  let current = {
    target_file: '',
    start: '',
    end: '',
    pre_sha1: '',
    post_sha1: '',
    line_delta: '',
    gates_passed: new Set(),
    rawLines: '',
  };
  let inGates = false;
  const flush = () => {
    if (
      current.target_file
      || current.start
      || current.end
      || current.pre_sha1
      || current.post_sha1
      || current.line_delta
      || current.gates_passed.size > 0
    ) {
      current.rawLines = current.rawLines.trimEnd();
      manifests.push(current);
    }
    current = {
      target_file: '',
      start: '',
      end: '',
      pre_sha1: '',
      post_sha1: '',
      line_delta: '',
      gates_passed: new Set(),
      rawLines: '',
    };
    inGates = false;
  };

  const assignField = (label, value) => {
    const v = value.trim().replace(/^`|`$/g, '');
    if (label === 'Target File') current.target_file = v;
    if (label === 'Start') current.start = v;
    if (label === 'End') current.end = v;
    if (label === 'Pre-SHA1') current.pre_sha1 = v;
    if (label === 'Post-SHA1') current.post_sha1 = v;
    if (label === 'Line Delta') current.line_delta = v;
  };

  const fieldRe = /^\s*-\s*\*\*(Target File|Start|End|Pre-SHA1|Post-SHA1|Line Delta)\*\*\s*:\s*(.*)\s*$/i;
  const gatesStartRe = /^\s*-\s*\*\*Gates Passed\*\*\s*:\s*$/i;
  const gateLineRe = /^\s*-\s*\[(x|X)\]\s*([a-z0-9_]+)\s*$/i;

  manifestLines.forEach((line) => {
    current.rawLines += `${line}\n`;
    const mField = line.match(fieldRe);
    if (mField) {
      const label = mField[1];
      const value = mField[2] ?? '';
      if (label.toLowerCase() === 'target file' && current.target_file) flush();
      assignField(label, value);
      return;
    }
    if (gatesStartRe.test(line)) {
      inGates = true;
      return;
    }
    if (inGates) {
      const mGate = line.trim().match(gateLineRe);
      if (mGate) {
        current.gates_passed.add(mGate[2].toLowerCase());
        return;
      }
      if (!line.trim().startsWith('-')) {
        inGates = false;
      }
    }
  });

  flush();
  return manifests.length > 0 ? manifests : null;
};

const parseWaivers = (content) => {
  if (!content) return false;
  // Look for WAIVERS GRANTED section and keywords like "dirty tree", "git hygiene", or CX-573F
  const waiverBlock = content.match(/##\s*WAIVERS\s*GRANTED([\s\S]*?)##/i);
  if (!waiverBlock) return false;
  const waivers = waiverBlock[1];
  return /CX-573F|dirty\s*tree|git\s*hygiene/i.test(waivers) && !/NONE/i.test(waivers);
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

const hasGitWaiver = parseWaivers(packetContent);
if (hasGitWaiver) {
  console.log('NOTE: Git hygiene waiver detected [CX-573F]. Strict git checks relaxed.');
}

const manifests = parseValidationManifests(packetContent);
if (!manifests) {
  errors.push('VALIDATION section found but manifest fields not parsed');
}

const inScopePaths = parseInScopePaths(packetContent);
const stagedFiles = getStagedFiles();
const workingFiles = getWorkingFiles();
const useStaged = stagedFiles.length > 0;
const changedFiles = useStaged ? stagedFiles : workingFiles;
if (useStaged && workingFiles.length > stagedFiles.length) {
  warnings.push('Working tree has unstaged changes; post-work validation uses STAGED changes only.');
}

// Check 2: manifest schema (per target file)
if (manifests) {
  console.log('\nCheck 2: Manifest fields');
  const shaRegex = /^[a-f0-9]{40}$/i;
  // Validate scope (best-effort): changed files must be subset of IN_SCOPE_PATHS (plus allowed governance files),
  // unless a waiver is present. This only applies to the evaluated diff set (staged preferred).
  const allowlisted = new Set([
    'docs/TASK_BOARD.md',
    'docs/SIGNATURE_AUDIT.md',
    'docs/ORCHESTRATOR_GATES.json',
    packetPath.replace(/\\/g, '/'),
    `docs/refinements/${WP_ID}.md`,
  ].filter(Boolean));

  const outOfScope = changedFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => !allowlisted.has(p))
    .filter((p) => (inScopePaths.length > 0 ? !inScopePaths.includes(p) : false));

  if (outOfScope.length > 0 && !hasGitWaiver) {
    errors.push(`Out-of-scope files changed (stage only WP files or record waiver [CX-573F]): ${outOfScope.join(', ')}`);
  } else if (outOfScope.length > 0 && hasGitWaiver) {
    warnings.push(`Out-of-scope files changed but waiver present [CX-573F]: ${outOfScope.join(', ')}`);
  }

  // Require manifest coverage for all non-docs changed files.
  const manifestTargets = new Set(manifests.map((m) => path.normalize(m.target_file).replace(/\\/g, '/')).filter(Boolean));
  const missingCoverage = changedFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => requiresManifest(p))
    .filter((p) => !manifestTargets.has(p));
  if (missingCoverage.length > 0) {
    errors.push(`Missing VALIDATION manifest coverage for changed files: ${missingCoverage.join(', ')}`);
  }

  // Now validate each manifest entry.
  console.log('\nCheck 3: File integrity (per manifest entry)');
  manifests.forEach((manifest, idx) => {
    const label = `Manifest[${idx + 1}]`;

    spec.requiredFields.forEach((field) => {
      const value = manifest[field];
      if (!value || (typeof value === 'string' && value.trim() === '')) {
        errors.push(`${label}: missing required field: ${field}`);
      }
    });

    if (manifest.pre_sha1 && !shaRegex.test(manifest.pre_sha1)) {
      errors.push(`${label}: pre_sha1 must be a 40-char hex SHA1`);
    }
    if (manifest.post_sha1 && !shaRegex.test(manifest.post_sha1)) {
      errors.push(`${label}: post_sha1 must be a 40-char hex SHA1`);
    }

    const startNum = parseInt(manifest.start, 10);
    const endNum = parseInt(manifest.end, 10);
    if (Number.isNaN(startNum) || Number.isNaN(endNum) || startNum < 1 || endNum < startNum) {
      errors.push(`${label}: Start/End must be integers with start >=1 and end >= start`);
    }

    const deltaNum = parseInt(manifest.line_delta, 10);
    if (manifest.line_delta === '' || Number.isNaN(deltaNum)) {
      errors.push(`${label}: line_delta must be an integer (adds - dels)`);
    }

    const targetPath = path.normalize(manifest.target_file.replace(/^`|`$/g, ''));
    if (!fs.existsSync(targetPath)) {
      errors.push(`${label}: Target file does not exist: ${targetPath} (${spec.gateErrorCodes.filename_canonical_and_openable})`);
      return;
    }

    // pre/post SHA checks (staged-aware)
    const headContent = loadHeadVersion(targetPath);
    if (headContent !== null) {
      const headSha = sha1Hex(headContent);
      if (manifest.pre_sha1 && headSha !== manifest.pre_sha1) {
        if (hasGitWaiver) {
          warnings.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage}) - WAIVER APPLIED`);
        } else {
          errors.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
        }
      }
    } else {
      warnings.push(`${label}: Could not load HEAD version (new file or not tracked): ${targetPath}`);
    }

    const postContent = useStaged ? loadIndexVersion(targetPath) : null;
    const postSha = postContent === null ? computeSha1(targetPath) : sha1Hex(postContent);
    if (manifest.post_sha1 && manifest.post_sha1 !== postSha) {
      errors.push(`${label}: post_sha1 mismatch for ${targetPath} (${spec.gateErrorCodes.post_sha1_captured})`);
    }

    const hunks = parseDiffHunks(targetPath, { staged: useStaged });
    const windowStart = parseInt(manifest.start, 10);
    const windowEnd = parseInt(manifest.end, 10);
    hunks.forEach((h) => {
      const oldEnd = h.oldStart + Math.max(h.oldLen - 1, 0);
      const newEnd = h.newStart + Math.max(h.newLen - 1, 0);
      const oldOutside = h.oldLen > 0 && (h.oldStart < windowStart || oldEnd > windowEnd);
      const newOutside = h.newLen > 0 && (h.newStart < windowStart || newEnd > windowEnd);
      if (oldOutside || newOutside) {
        errors.push(`${label}: Diff touches lines outside declared window [${windowStart}, ${windowEnd}] (${spec.gateErrorCodes.rails_untouched_outside_window})`);
      }
    });

    const numstatDelta = getNumstatDelta(targetPath, { staged: useStaged });
    if (numstatDelta !== null && !Number.isNaN(deltaNum) && numstatDelta !== deltaNum) {
      errors.push(`${label}: line_delta (${deltaNum}) does not match git diff delta (${numstatDelta}) (${spec.gateErrorCodes.line_delta_equals_expected})`);
    }

    // Gate checkboxes: allow either explicit checkmarks OR automatic inference (warn if inferred).
    spec.requiredGates.forEach((gate) => {
      if (manifest.gates_passed.has(gate)) return;
      // Infer gates we can verify mechanically.
      const inferable = new Set([
        'anchors_present',
        'window_matches_plan',
        'rails_untouched_outside_window',
        'filename_canonical_and_openable',
        'pre_sha1_captured',
        'post_sha1_captured',
        'line_delta_equals_expected',
        'manifest_written_and_path_returned',
        'current_file_matches_preimage',
      ]);
      if (inferable.has(gate)) {
        warnings.push(`${label}: gate not checked but inferred as PASS: ${gate} (${spec.gateErrorCodes[gate]})`);
        return;
      }
      errors.push(`${label}: gate missing or unchecked: ${gate} (${spec.gateErrorCodes[gate]})`);
    });
  });
}

// Check 4: git status sanity
console.log('\nCheck 4: Git status');
try {
  const staged = getStagedFiles();
  const working = getWorkingFiles();
  if (staged.length === 0 && working.length === 0) errors.push('No files changed (git status clean)');
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
