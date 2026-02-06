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

const usage = () => [
  'Usage: node post-work-check.mjs WP-{ID} [options]',
  '',
  'Options:',
  '  --rev <git-rev>         Validate a single commit (rev^..rev)',
  '  --range <a>..<b>        Validate an explicit range (a..b)',
  '  -h, --help              Show this help',
].join('\n');

const args = process.argv.slice(2);
if (args.includes('-h') || args.includes('--help')) {
  console.log(usage());
  process.exit(0);
}

const WP_ID = args[0];
if (!WP_ID) {
  console.error(usage());
  process.exit(1);
}

const cliArgs = args.slice(1);

const SPEC_PATH = path.join('.GOV', 'scripts', 'validation', 'cor701-spec.json');
const spec = JSON.parse(fs.readFileSync(SPEC_PATH, 'utf8'));

console.log(`\nPost-work validation for ${WP_ID} (deterministic manifest + gates)...\n`);

const errors = [];
const warnings = [];

const gitTrim = (command) => execSync(command, { encoding: 'utf8' }).trim();
const gitBuffer = (command) => execSync(command);

const resolveMergeBase = () => {
  try {
    const base = gitTrim('git merge-base main HEAD');
    return base || null;
  } catch {
    return null;
  }
};

const readFileIfExists = (p) => {
  try {
    return fs.readFileSync(p, 'utf8');
  } catch {
    return '';
  }
};

const sha1Hex = (bufOrString) => crypto.createHash('sha1').update(bufOrString).digest('hex');

const normalizeLf = (text) => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
const normalizeCrlf = (text) => normalizeLf(text).replace(/\n/g, '\r\n');

const isLikelyText = (buf) => !buf.includes(0);

const sha1VariantsForText = (text) => {
  const lf = normalizeLf(text);
  return {
    lf: sha1Hex(lf),
    crlf: sha1Hex(normalizeCrlf(lf)),
  };
};

const sha1VariantsForGitBlob = (buf) => {
  const lf = sha1Hex(buf);
  if (!isLikelyText(buf)) {
    return { lf, crlf: lf };
  }

  const txt = buf.toString('utf8');
  const { crlf } = sha1VariantsForText(txt);
  return { lf, crlf };
};

const sha1VariantsForWorktreeFile = (p) => {
  const buf = fs.readFileSync(p);
  const raw = sha1Hex(buf);
  if (!isLikelyText(buf)) {
    return { raw, lf: raw, crlf: raw };
  }

  const txt = buf.toString('utf8');
  const { lf, crlf } = sha1VariantsForText(txt);
  return { raw, lf, crlf };
};

// Use LF-normalized hash for worktree reads to avoid CRLF-based false negatives on Windows.
const computeSha1 = (p) => sha1VariantsForWorktreeFile(p).lf;

const MERGE_BASE = resolveMergeBase();

const resolveFirstParent = (rev) => {
  try {
    const parents = gitTrim(`git show -s --pretty=%P ${rev}`);
    const list = parents.split(/\s+/).map((p) => p.trim()).filter(Boolean);
    return list.length > 0 ? list[0] : null;
  } catch {
    return null;
  }
};

const parseRangeArg = (raw) => {
  const trimmed = (raw ?? '').trim();
  if (!trimmed.includes('..')) return null;
  const parts = trimmed.split('..');
  if (parts.length !== 2) return null;
  const [base, head] = parts.map((p) => p.trim()).filter(Boolean);
  if (!base || !head) return null;
  return { base, head };
};

const parseCli = (argv) => {
  const parsed = { rev: null, range: null };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--rev') {
      const rev = argv[i + 1];
      if (!rev) {
        console.error('Error: --rev requires a value.\n');
        console.error(usage());
        process.exit(1);
      }
      parsed.rev = rev;
      i += 1;
      continue;
    }
    if (arg === '--range') {
      const raw = argv[i + 1];
      if (!raw) {
        console.error('Error: --range requires a value.\n');
        console.error(usage());
        process.exit(1);
      }
      const range = parseRangeArg(raw);
      if (!range) {
        console.error('Error: --range must be in the form <base>..<head>.\n');
        console.error(usage());
        process.exit(1);
      }
      parsed.range = range;
      i += 1;
      continue;
    }
    console.error(`Error: Unknown argument: ${arg}\n`);
    console.error(usage());
    process.exit(1);
  }
  if (parsed.rev && parsed.range) {
    console.error('Error: Use only one of --rev or --range.\n');
    console.error(usage());
    process.exit(1);
  }
  return parsed;
};

const cli = parseCli(cliArgs);

const loadGitVersion = (rev, targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    // Suppress git stderr for expected "missing preimage" lookups (new files at base/HEAD).
    return execSync(`git show ${rev}:${gitPath}`, { stdio: ['ignore', 'pipe', 'pipe'] });
  } catch {
    return null;
  }
};

const loadHeadVersion = (targetPath) => {
  return loadGitVersion('HEAD', targetPath);
};

const loadIndexVersion = (targetPath) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    // Suppress git stderr for expected "missing index preimage" lookups.
    return execSync(`git show :${gitPath}`, { stdio: ['ignore', 'pipe', 'pipe'] });
  } catch {
    return null;
  }
};

const getNumstatDelta = (targetPath, { staged, baseRev, headRev }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    if (baseRev && headRev) {
      const out = gitTrim(`git diff --numstat ${baseRev} ${headRev} -- "${gitPath}"`);
      if (!out) return null;
      const [addsStr, delsStr] = out.split('\t');
      const adds = parseInt(addsStr, 10);
      const dels = parseInt(delsStr, 10);
      if (Number.isNaN(adds) || Number.isNaN(dels)) return null;
      return adds - dels;
    }

    const diffArgs = staged ? '--cached' : '';
    const out = gitTrim(`git diff ${diffArgs} --numstat HEAD -- "${gitPath}"`);
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

const parseDiffHunks = (targetPath, { staged, baseRev, headRev }) => {
  try {
    const gitPath = targetPath.replace(/\\/g, '/');
    const diffArgs = staged ? '--cached' : '';
    const diff = baseRev && headRev
      ? gitTrim(`git diff --unified=0 ${baseRev} ${headRev} -- "${gitPath}"`)
      : gitTrim(`git diff ${diffArgs} --unified=0 HEAD -- "${gitPath}"`);
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

const taskPacketDir = '.GOV/task_packets';
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

const parseMergeBaseSha = (content) => {
  if (!content) return null;
  const m = content.match(/^\s*-\s*MERGE_BASE_SHA\s*:\s*([a-f0-9]{40})\s*$/mi);
  return m ? m[1].toLowerCase() : null;
};

const PACKET_MERGE_BASE_SHA = parseMergeBaseSha(packetContent);

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
  if (p.startsWith('.GOV/')) return false;
  return true;
};

const getStagedFiles = () => {
  try {
    // --diff-filter=d excludes deleted files (they cannot have manifest entries since
    // the file doesn't exist on disk for SHA1 verification and End>=Start>=1 fails)
    const out = gitTrim('git diff --name-only --cached --diff-filter=d');
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const getWorkingFiles = () => {
  try {
    // --diff-filter=d excludes deleted files (same rationale as above)
    const out = gitTrim('git diff --name-only HEAD --diff-filter=d');
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

const isModernPacket = /^\s*-\s*PACKET_FORMAT_VERSION\s*:/mi.test(packetContent);
const extractSection = (content, heading) => {
  if (!content) return null;
  const lines = content.split('\n');
  const headingRe = new RegExp(`^##\\s+${heading}\\s*$`, 'i');
  const startIdx = lines.findIndex((l) => headingRe.test(l.trimEnd()));
  if (startIdx === -1) return null;
  const section = [];
  for (let i = startIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.startsWith('## ')) break;
    section.push(line);
  }
  return section.join('\n');
};

// Check 0: Canonical evidence must live in the packet for modern packets.
// This is intentionally mechanical to keep validation reproducible.
if (isModernPacket) {
  const evidenceMapping = extractSection(packetContent, 'EVIDENCE_MAPPING');
  if (!evidenceMapping) {
    errors.push('Missing ## EVIDENCE_MAPPING section (required for modern packets)');
  } else {
    const hasFileLine = /(?:src[\\/]|app[\\/]|\.GOV[\\/])[^`\s]*:\d+\b/i.test(evidenceMapping);
    if (!hasFileLine) {
      errors.push('EVIDENCE_MAPPING has no file:line evidence (add REQUIREMENT -> EVIDENCE: path:line entries)');
    }
  }

  const evidence = extractSection(packetContent, 'EVIDENCE');
  if (!evidence) {
    errors.push('Missing ## EVIDENCE section (required for modern packets)');
  } else {
    const evidenceLines = evidence.split('\n');
    const hasCommand = evidenceLines.some((l) => /COMMAND\s*:/i.test(l) && !/<paste>/i.test(l));
    const hasExitCode = evidenceLines.some((l) => /EXIT_CODE\s*:\s*\d+/i.test(l));
    if (!(hasCommand && hasExitCode)) {
      errors.push('EVIDENCE must include at least one COMMAND + EXIT_CODE entry for modern packets');
    }
  }
}

const inScopePaths = parseInScopePaths(packetContent);
const stagedFiles = getStagedFiles();
const workingFiles = getWorkingFiles();

const getRangeFiles = (baseRev, headRev) => {
  try {
    const out = gitTrim(`git diff --name-only --diff-filter=d ${baseRev} ${headRev}`);
    return out ? out.split('\n').filter(Boolean) : [];
  } catch {
    return [];
  }
};

const resolveEvaluation = () => {
  if (cli.range) {
    return { mode: 'range', baseRev: cli.range.base, headRev: cli.range.head, reason: 'explicit --range' };
  }
  if (cli.rev) {
    const base = resolveFirstParent(cli.rev);
    if (!base) {
      errors.push(`Cannot determine parent commit for --rev ${cli.rev}; provide an explicit --range instead.`);
      return { mode: 'range', baseRev: null, headRev: null, reason: 'invalid --rev' };
    }
    return { mode: 'range', baseRev: base, headRev: cli.rev, reason: 'single-commit --rev (rev^..rev)' };
  }
  if (stagedFiles.length > 0) return { mode: 'staged', baseRev: null, headRev: null, reason: 'staged changes present' };
  if (workingFiles.length > 0) return { mode: 'worktree', baseRev: null, headRev: null, reason: 'working tree changes present' };

  const head = 'HEAD';
  if (PACKET_MERGE_BASE_SHA) {
    return { mode: 'range', baseRev: PACKET_MERGE_BASE_SHA, headRev: head, reason: 'clean tree; validate packet MERGE_BASE_SHA..HEAD' };
  }
  if (MERGE_BASE) {
    try {
      const headSha = gitTrim('git rev-parse HEAD');
      if (MERGE_BASE !== headSha) {
        return { mode: 'range', baseRev: MERGE_BASE, headRev: head, reason: 'clean tree; validate merge-base(main, HEAD)..HEAD' };
      }
    } catch {
      // ignore
    }
  }
  const parent = resolveFirstParent(head);
  if (parent) {
    return { mode: 'range', baseRev: parent, headRev: head, reason: 'clean tree; validate last commit (HEAD^..HEAD)' };
  }
  if (MERGE_BASE) {
    return { mode: 'range', baseRev: MERGE_BASE, headRev: head, reason: 'clean tree; fallback to merge-base(main, HEAD)..HEAD' };
  }
  errors.push('No staged/working changes and unable to resolve a git range (no parent commit and no merge-base).');
  return { mode: 'range', baseRev: null, headRev: null, reason: 'no range available' };
};

const evaluation = resolveEvaluation();
const useStaged = evaluation.mode === 'staged';
const useRange = evaluation.mode === 'range' && evaluation.baseRev && evaluation.headRev;
const rangeFiles = useRange ? getRangeFiles(evaluation.baseRev, evaluation.headRev) : [];
const changedFiles = useStaged ? stagedFiles : (evaluation.mode === 'worktree' ? workingFiles : rangeFiles);

const resolveRev = (rev) => {
  try {
    return gitTrim(`git rev-parse ${rev}`);
  } catch {
    return rev;
  }
};

console.log(`\nDiff selection: ${evaluation.mode} (${evaluation.reason})`);
if (useRange) {
  const resolvedBase = resolveRev(evaluation.baseRev);
  const resolvedHead = resolveRev(evaluation.headRev);
  console.log(`Git range: ${resolvedBase}..${resolvedHead}`);
}

if (useStaged && workingFiles.length > stagedFiles.length) {
  // Avoid warning noise for validator-only governance state.
  const stagedSet = new Set(stagedFiles.map((p) => p.replace(/\\/g, '/')));
  const allowlistedUnstaged = new Set([
    '.GOV/roles_shared/TASK_BOARD.md',
    '.GOV/roles_shared/SIGNATURE_AUDIT.md',
    '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
    '.GOV/roles/validator/VALIDATOR_GATES.json',
    packetPath.replace(/\\/g, '/'),
    `.GOV/refinements/${WP_ID}.md`,
  ].filter(Boolean));

  const isAllowlistedUnstaged = (p) =>
    allowlistedUnstaged.has(p) || p.startsWith('.GOV/validator_gates/');

  const hasRelevantUnstaged = workingFiles
    .map((p) => p.replace(/\\/g, '/'))
    .filter((p) => !stagedSet.has(p))
    .some((p) => !isAllowlistedUnstaged(p));

  if (hasRelevantUnstaged) {
    warnings.push('Working tree has unstaged changes; post-work validation uses STAGED changes only.');
  }
}

// Check 2: manifest schema (per target file)
if (manifests) {
  console.log('\nCheck 2: Manifest fields');
  const shaRegex = /^[a-f0-9]{40}$/i;
  // Validate scope (best-effort): changed files must be subset of IN_SCOPE_PATHS (plus allowed governance files),
  // unless a waiver is present. This only applies to the evaluated diff set (staged preferred).
  const allowlisted = new Set([
    '.GOV/roles_shared/TASK_BOARD.md',
    '.GOV/roles_shared/SIGNATURE_AUDIT.md',
    '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
    '.GOV/roles/validator/VALIDATOR_GATES.json',
    packetPath.replace(/\\/g, '/'),
    `.GOV/refinements/${WP_ID}.md`,
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

    // pre/post SHA checks (staged/worktree/range-aware)
    const preRev = useRange ? evaluation.baseRev : 'HEAD';
    const preContent = loadGitVersion(preRev, targetPath);
    if (preContent !== null) {
      const pre = sha1VariantsForGitBlob(preContent);
      if (manifest.pre_sha1 && manifest.pre_sha1 !== pre.lf) {
        if (manifest.pre_sha1 === pre.crlf) {
          warnings.push(`${label}: pre_sha1 matches CRLF-normalized ${preRev} for ${targetPath}; prefer LF blob SHA1=${pre.lf}`);
        } else if (!useRange && MERGE_BASE) {
          // Back-compat behavior for staged/worktree mode: accept merge-base preimages as a warning.
          const baseContent = loadGitVersion(MERGE_BASE, targetPath);
          const base = baseContent ? sha1VariantsForGitBlob(baseContent) : null;
          const matchesBase = base && (manifest.pre_sha1 === base.lf || manifest.pre_sha1 === base.crlf);
          if (matchesBase) {
            warnings.push(`${label}: pre_sha1 matches merge-base(${MERGE_BASE}) for ${targetPath} (common after WP commits); prefer LF blob SHA1=${base.lf}`);
          } else if (hasGitWaiver) {
            warnings.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage}) - WAIVER APPLIED`);
            warnings.push(`${label}: expected pre_sha1 (HEAD LF blob) = ${pre.lf}`);
          } else {
            errors.push(`${label}: pre_sha1 does not match HEAD for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
            errors.push(`${label}: expected pre_sha1 (HEAD LF blob) = ${pre.lf}`);
            if (base) errors.push(`${label}: expected pre_sha1 (merge-base LF blob) = ${base.lf}`);
          }
        } else if (hasGitWaiver) {
          warnings.push(`${label}: pre_sha1 does not match ${preRev} for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage}) - WAIVER APPLIED`);
          warnings.push(`${label}: expected pre_sha1 (LF blob) = ${pre.lf}`);
        } else {
          errors.push(`${label}: pre_sha1 does not match ${preRev} for ${targetPath} (${spec.gateErrorCodes.current_file_matches_preimage})`);
          errors.push(`${label}: expected pre_sha1 (LF blob) = ${pre.lf}`);
        }
      }
    } else if (useRange) {
      warnings.push(`${label}: Could not load ${preRev} version (new file or not tracked at ${preRev}): ${targetPath}`);
    } else {
      warnings.push(`${label}: Could not load HEAD version (new file or not tracked): ${targetPath}`);
    }

    const postContent = useStaged
      ? loadIndexVersion(targetPath)
      : (useRange ? loadGitVersion(evaluation.headRev, targetPath) : null);
    const post = postContent === null
      ? sha1VariantsForWorktreeFile(targetPath)
      : sha1VariantsForGitBlob(postContent);
    const expectedPost = postContent === null ? post.lf : post.lf;
    if (manifest.post_sha1 && manifest.post_sha1 !== expectedPost) {
      const acceptable = new Set([post.crlf, post.raw].filter(Boolean));
      if (acceptable.has(manifest.post_sha1)) {
        warnings.push(`${label}: post_sha1 matches non-canonical EOL variant for ${targetPath}; prefer LF blob SHA1=${expectedPost}`);
      } else {
        errors.push(`${label}: post_sha1 mismatch for ${targetPath} (${spec.gateErrorCodes.post_sha1_captured})`);
        errors.push(`${label}: expected post_sha1 (LF) = ${expectedPost}`);
      }
    }

    const hunks = parseDiffHunks(targetPath, { staged: useStaged, baseRev: useRange ? evaluation.baseRev : null, headRev: useRange ? evaluation.headRev : null });
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

    const numstatDelta = getNumstatDelta(targetPath, { staged: useStaged, baseRev: useRange ? evaluation.baseRev : null, headRev: useRange ? evaluation.headRev : null });
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
  if (changedFiles.length === 0) {
    if (useRange) {
      errors.push(`No files changed in range ${evaluation.baseRev}..${evaluation.headRev}`);
    } else {
      errors.push('No files changed (git status clean)');
    }
  }
} catch {
  warnings.push('Could not read git status');
}

// Results
console.log('\n' + '='.repeat(50));
if (errors.length === 0) {
  if (warnings.length > 0) {
    console.log('Post-work validation PASSED (deterministic manifest gate; not tests) with warnings\n');
    console.log('Warnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  } else {
    console.log('Post-work validation PASSED (deterministic manifest gate; not tests)');
  }
  console.log('\nYou may proceed with commit.');
  process.exit(0);
} else {
  console.log('Post-work validation FAILED (deterministic manifest gate; not tests)\n');
  console.log('Errors:');
  errors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
  if (warnings.length > 0) {
    console.log('\nWarnings:');
    warnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
  }
  console.log('\nFix these issues before committing (gates enforce determinism).');
  console.log('See: .GOV/roles/coder/CODER_PROTOCOL.md');
  process.exit(1);
}
