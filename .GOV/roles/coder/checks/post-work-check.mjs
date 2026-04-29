#!/usr/bin/env node
/**
 * Post-work validation (deterministic manifest + gates)
 * - Enforces manifest schema and gate coverage inherited from COR-701 (anchors/rails/window/sha1/line_delta/concurrency)
 * - Deterministic handoff manifest checker used by the shared HANDOFF phase gate
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execSync } from 'child_process';
import { GOV_ROOT_REPO_REL, resolveRefinementPath, resolveWorkPacketPath } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';
import {
  collectBudgetCountedFiles,
  classifyWpChangedPath,
  deriveWpScopeContract,
  formatBoundedItemList,
  isGovernanceOnlyPath,
  isTransientProofArtifactPath,
  normalizeRepoPath,
  parsePacketSingleField,
  parsePacketScopeDiscipline,
  scopeDisciplineRequiresEnforcement,
} from '../../../roles_shared/scripts/lib/scope-surface-lib.mjs';
import { validatePacketClosureMonitoring } from '../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs';
import {
  activeBaselineCompileWaiversForWp,
  evaluateWaiverCoverage,
} from '../../../roles_shared/scripts/lib/baseline-waiver-ledger-lib.mjs';
import { resolveCommittedCoderHandoffRange } from '../../../roles_shared/scripts/lib/role-resume-utils.mjs';
import { resolveGitBaselineMergeBase } from '../scripts/lib/coder-governance-lib.mjs';

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

const SPEC_PATH = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'cor701-spec.json');
const spec = JSON.parse(fs.readFileSync(SPEC_PATH, 'utf8'));

console.log(`\nPost-work validation for ${WP_ID} (deterministic manifest + gates)...\n`);

const errors = [];
const warnings = [];

const gitTrim = (command) => execSync(command, { encoding: 'utf8' }).trim();
const gitTrimQuiet = (command) => execSync(command, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
const gitBuffer = (command) => execSync(command);

const resolveMergeBase = () => {
  return resolveGitBaselineMergeBase(gitTrimQuiet);
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

const MERGE_BASE_INFO = resolveMergeBase();
const MERGE_BASE = MERGE_BASE_INFO.base;
const MERGE_BASE_REF = MERGE_BASE_INFO.ref;

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

const GOV_DISPLAY_ROOT = '.GOV';
const toDisplayGovPath = (value) => {
  const normalized = path.normalize(String(value || '')).replace(/\\/g, '/');
  const govRootNormalized = path.normalize(GOV_ROOT_REPO_REL).replace(/\\/g, '/');
  if (normalized === govRootNormalized) return GOV_DISPLAY_ROOT;
  if (normalized.startsWith(`${govRootNormalized}/`)) {
    return `${GOV_DISPLAY_ROOT}${normalized.slice(govRootNormalized.length)}`;
  }
  return normalized;
};

const resolvedPacket = resolveWorkPacketPath(WP_ID);
const packetPathActual = resolvedPacket?.packetPath || `${GOV_ROOT_REPO_REL}/task_packets/${WP_ID}.md`;
const packetPath = toDisplayGovPath(packetPathActual);
const packetContent = readFileIfExists(packetPathActual);
const packetFormatVersion = parsePacketSingleField(packetContent, 'PACKET_FORMAT_VERSION');
const usesAntiVibeRigor = packetFormatVersion >= '2026-04-01';
const workflowLane = parsePacketSingleField(packetContent, 'WORKFLOW_LANE').toUpperCase();
const zeroDeltaProofAllowed = parsePacketSingleField(packetContent, 'ZERO_DELTA_PROOF_ALLOWED').toUpperCase() === 'YES';
const usesSkeletonCheckpointGate = workflowLane !== 'ORCHESTRATOR_MANAGED';
const scopeContract = deriveWpScopeContract({ wpId: WP_ID, packetContent });
const scopeDiscipline = parsePacketScopeDiscipline(packetContent);
const enforceScopeDiscipline = scopeDisciplineRequiresEnforcement(parsePacketSingleField(packetContent, 'PACKET_FORMAT_VERSION'));

const PACKET_COMMITTED_HANDOFF_RANGE = resolveCommittedCoderHandoffRange(packetContent, WP_ID);

const requiresManifest = (filePath) => {
  const p = filePath.replace(/\\/g, '/');
  if (p.startsWith(`${GOV_DISPLAY_ROOT}/`)) return false;
  if (p.startsWith(`${GOV_ROOT_REPO_REL}/`)) return false;
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

const getUntrackedFiles = () => {
  try {
    const out = gitTrim('git ls-files --others --exclude-standard');
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

const uniquePaths = (paths) => Array.from(new Set((paths || []).map((value) => normalizeRepoPath(value)).filter(Boolean)));

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
  errors.push('No work packet found for this WP_ID');
} else if (!/VALIDATION/i.test(packetContent)) {
  errors.push('work packet missing VALIDATION section');
} else if (/[^\x00-\x7F]/.test(packetContent)) {
  errors.push('work packet contains non-ASCII characters (manifest must be ASCII)');
}

const hasGitWaiver = parseWaivers(packetContent);
if (hasGitWaiver) {
  console.log('NOTE: Git hygiene waiver detected [CX-573F]. Strict git checks relaxed.');
}
const baselineCompileWaivers = activeBaselineCompileWaiversForWp(WP_ID);
if (baselineCompileWaivers.length > 0) {
  console.log(`NOTE: ${baselineCompileWaivers.length} active baseline compile waiver(s) detected. Scope relaxation is path-limited to waiver allowed_edit_paths.`);
}

const pathFromScopeViolation = (value) => {
  const text = String(value || '').trim();
  const idx = text.indexOf(':');
  return normalizeRepoPath(idx >= 0 ? text.slice(idx + 1).trim() : text);
};

const waiverCoverageForScopeViolations = (violations) => evaluateWaiverCoverage({
  paths: (violations || []).map((entry) => pathFromScopeViolation(entry)).filter(Boolean),
  waivers: baselineCompileWaivers,
});

const formatWaiverIds = (coverage) => Array.from(new Set(
  (coverage?.covered || []).flatMap((entry) => entry.waiver_ids || []),
)).join(',') || 'none';

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

const escapePacketRegex = (value) => String(value || '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const hasConcreteStatusField = (section, label) => {
  const re = new RegExp(`^\\s*-\\s*${escapePacketRegex(label)}\\s*:\\s*(.+)\\s*$`, 'mi');
  const match = String(section || '').match(re);
  if (!match) return false;
  const value = String(match[1] || '').trim();
  if (!value) return false;
  if (/^<pending>$/i.test(value)) return false;
  if (/^n\/?a$/i.test(value)) return false;
  return true;
};

// Check 0: Canonical evidence must live in the packet for modern packets.
// This is intentionally mechanical to keep validation reproducible.
if (isModernPacket) {
  const clauseClosureMonitorProfile = parsePacketSingleField(packetContent, 'CLAUSE_CLOSURE_MONITOR_PROFILE');
  if (/^CLAUSE_MONITOR_V1$/i.test(clauseClosureMonitorProfile)) {
    const closureMonitorValidation = validatePacketClosureMonitoring(packetContent, {
      requireRows: true,
    });
    if (closureMonitorValidation.errors.length > 0) {
      errors.push(`CLAUSE_CLOSURE_MATRIX invalid for handoff: ${closureMonitorValidation.errors.join('; ')}`);
    }
  }

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
    const hasExitCode = evidenceLines.some((l) => /EXIT_CODE\s*:\s*`?\d+`?/i.test(l));
    if (!(hasCommand && hasExitCode)) {
      errors.push('EVIDENCE must include at least one COMMAND + EXIT_CODE entry for modern packets');
    }
  }

  const handoffRigorProfile = parsePacketSingleField(packetContent, 'CODER_HANDOFF_RIGOR_PROFILE');
  if (/^MAIN_BODY_SELF_CRITIQUE_V1$/i.test(handoffRigorProfile)) {
    const statusHandoff = extractSection(packetContent, 'STATUS_HANDOFF');
    if (!statusHandoff) {
      errors.push('Missing ## STATUS_HANDOFF section (required for MAIN_BODY_SELF_CRITIQUE_V1)');
    } else {
      const requiredStatusFields = [
        'Current WP_STATUS',
        'What changed in this update',
        'Main-body clauses self-audited',
        'Known gaps / weak spots',
        'Heuristic risks / maintainability concerns',
        'Validator focus request',
        'Next step / handoff hint',
      ];
      for (const label of requiredStatusFields) {
        if (!hasConcreteStatusField(statusHandoff, label)) {
          errors.push(`STATUS_HANDOFF missing concrete field: ${label}`);
        }
      }
    }
  } else if (/^RUBRIC_SELF_AUDIT_V2$/i.test(handoffRigorProfile)) {
    const statusHandoff = extractSection(packetContent, 'STATUS_HANDOFF');
    if (!statusHandoff) {
      errors.push('Missing ## STATUS_HANDOFF section (required for RUBRIC_SELF_AUDIT_V2)');
    } else {
      const requiredStatusFields = [
        'Current WP_STATUS',
        'What changed in this update',
        'Requirements / clauses self-audited',
        'Checks actually run',
        'Known gaps / weak spots',
        'Heuristic risks / maintainability concerns',
        'Validator focus request',
        'Rubric contract understanding proof',
        'Rubric scope discipline proof',
        'Rubric baseline comparison',
        'Rubric end-to-end proof',
        'Rubric architecture fit self-review',
        'Rubric heuristic quality self-review',
        'Rubric anti-gaming / counterfactual check',
        'Next step / handoff hint',
      ];
      if (usesAntiVibeRigor) {
        requiredStatusFields.splice(requiredStatusFields.length - 1, 0,
          'Rubric anti-vibe / substance self-check',
          'Signed-scope debt ledger',
          'Data contract self-check',
        );
      }
      for (const label of requiredStatusFields) {
        if (!hasConcreteStatusField(statusHandoff, label)) {
          errors.push(`STATUS_HANDOFF missing concrete field: ${label}`);
        }
      }
    }
  }
}

const stagedFiles = getStagedFiles();
const untrackedFiles = getUntrackedFiles();
const workingFiles = uniquePaths([...getWorkingFiles(), ...untrackedFiles]);

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

  if (PACKET_COMMITTED_HANDOFF_RANGE) {
    const { baseRev, headRev, source } = PACKET_COMMITTED_HANDOFF_RANGE;
    const reason = source === 'PACKET_EXPLICIT_HANDOFF_RANGE'
      ? 'clean tree; validate packet explicit committed coder handoff range'
      : 'clean tree; validate packet MERGE_BASE_SHA..HEAD';
    return { mode: 'range', baseRev, headRev, reason };
  }
  const head = 'HEAD';
  if (MERGE_BASE) {
    try {
      const headSha = gitTrim('git rev-parse HEAD');
      if (MERGE_BASE !== headSha) {
        return { mode: 'range', baseRev: MERGE_BASE, headRev: head, reason: `clean tree; validate merge-base(${MERGE_BASE_REF}, HEAD)..HEAD` };
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
    return { mode: 'range', baseRev: MERGE_BASE, headRev: head, reason: `clean tree; fallback to merge-base(${MERGE_BASE_REF}, HEAD)..HEAD` };
  }
  errors.push('No staged/working changes and unable to resolve a git range (no parent commit and no merge-base).');
  return { mode: 'range', baseRev: null, headRev: null, reason: 'no range available' };
};

const evaluation = resolveEvaluation();
const useStaged = evaluation.mode === 'staged';
const useRange = evaluation.mode === 'range' && evaluation.baseRev && evaluation.headRev;
const rangeFiles = useRange ? getRangeFiles(evaluation.baseRev, evaluation.headRev) : [];
const changedFiles = uniquePaths(useStaged ? stagedFiles : (evaluation.mode === 'worktree' ? workingFiles : rangeFiles));
const branchLocalChangedFiles = uniquePaths([...stagedFiles, ...workingFiles]);

// Phase gate: product code changes require a docs-only skeleton checkpoint commit.
// This is intentionally mechanical to prevent "vibecoding" ahead of interface checkpointing [CX-GATE-001].
const escapeRegex = (s) => (s ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const hasSkeletonCheckpointCommit = (wpId) => {
  const wp = (wpId ?? '').trim();
  if (!wp) return false;
  const subjectRe = `^docs: skeleton checkpoint \\[${escapeRegex(wp)}\\]$`;
  try {
    const sha = gitTrim(`git log -n 1 --format=%H --grep="${subjectRe}"`);
    return Boolean((sha || '').trim());
  } catch {
    return false;
  }
};

const hasSkeletonApprovedCommit = (wpId) => {
  const wp = (wpId ?? '').trim();
  if (!wp) return false;
  const subjectRe = `^docs: skeleton approved \\[${escapeRegex(wp)}\\]$`;
  try {
    const sha = gitTrim(`git log -n 1 --format=%H --grep="${subjectRe}"`);
    return Boolean((sha || '').trim());
  } catch {
    return false;
  }
};

const hasSkeletonCheckpoint = hasSkeletonCheckpointCommit(WP_ID);
const hasSkeletonApproved = hasSkeletonApprovedCommit(WP_ID);
const productChanged = branchLocalChangedFiles
  .filter((p) => p.startsWith('src/') || p.startsWith('app/') || p.startsWith('tests/'));

if (usesSkeletonCheckpointGate && productChanged.length > 0 && !hasSkeletonCheckpoint) {
  errors.push('Phase gate violation [CX-GATE-001]: Product code changes detected without a docs-only skeleton checkpoint commit on this WP branch.');
  errors.push(`Expected commit subject: docs: skeleton checkpoint [${WP_ID}] (create via: just coder-skeleton-checkpoint ${WP_ID})`);
  errors.push(`Changed product paths (subset): ${productChanged.slice(0, 10).join(', ')}`);
  if (productChanged.length > 10) {
    errors.push(`Changed product paths: (+${productChanged.length - 10} more)`);
  }
}

if (usesSkeletonCheckpointGate && productChanged.length > 0 && !hasSkeletonApproved) {
  errors.push('Phase gate violation [CX-GATE-001]: Product code changes detected without a skeleton approval commit (Operator/Validator-only unblock).');
  errors.push(`Expected commit subject: docs: skeleton approved [${WP_ID}] (create via: just skeleton-approved ${WP_ID})`);
}

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
    `${GOV_DISPLAY_ROOT}/roles_shared/records/TASK_BOARD.md`,
    `${GOV_DISPLAY_ROOT}/roles_shared/records/SIGNATURE_AUDIT.md`,
    `ORCHESTRATOR_GATES.json`,
    packetPath,
    toDisplayGovPath(resolveRefinementPath(WP_ID) || `${GOV_ROOT_REPO_REL}/refinements/${WP_ID}.md`),
  ].filter(Boolean));

  const isAllowlistedUnstaged = (p) =>
    p.startsWith(`${GOV_DISPLAY_ROOT}/`)
    || p.startsWith(`${GOV_ROOT_REPO_REL}/`)
    || allowlistedUnstaged.has(p)
    ;

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
  const evaluatedScopeViolations = [];
  const branchLocalScopeViolations = [];
  const governanceNoiseWarnings = [];
  const transientArtifactWarnings = [];

  for (const changedFile of changedFiles) {
    const classification = classifyWpChangedPath(changedFile, scopeContract);
    const normalizedPath = normalizeRepoPath(changedFile) || changedFile;
    if (isTransientProofArtifactPath(normalizedPath)) {
      transientArtifactWarnings.push(normalizedPath);
      continue;
    }
    if (isGovernanceOnlyPath(normalizedPath)) {
      governanceNoiseWarnings.push(normalizedPath);
      continue;
    }
    if (!classification.allowed) {
      evaluatedScopeViolations.push(`${classification.kind}: ${classification.path}`);
    }
  }

  for (const changedFile of branchLocalChangedFiles) {
    const classification = classifyWpChangedPath(changedFile, scopeContract);
    const normalizedPath = normalizeRepoPath(changedFile) || changedFile;
    if (isTransientProofArtifactPath(normalizedPath)) {
      transientArtifactWarnings.push(normalizedPath);
      continue;
    }
    if (isGovernanceOnlyPath(normalizedPath)) {
      governanceNoiseWarnings.push(normalizedPath);
      continue;
    }
    if (!classification.allowed) {
      branchLocalScopeViolations.push(`${classification.kind}: ${classification.path}`);
    }
  }

  const uniqueEvaluatedViolations = Array.from(new Set(evaluatedScopeViolations));
  const uniqueBranchLocalViolations = Array.from(new Set(branchLocalScopeViolations));
  const uniqueGovernanceNoiseWarnings = Array.from(new Set(governanceNoiseWarnings));
  const uniqueTransientArtifactWarnings = Array.from(new Set(transientArtifactWarnings));

  const evaluatedWaiverCoverage = waiverCoverageForScopeViolations(uniqueEvaluatedViolations);
  const branchLocalWaiverCoverage = waiverCoverageForScopeViolations(uniqueBranchLocalViolations);

  if (uniqueEvaluatedViolations.length > 0 && !hasGitWaiver && !evaluatedWaiverCoverage.ok) {
    errors.push(`Out-of-scope files in the evaluated diff: ${formatBoundedItemList(uniqueEvaluatedViolations, { noun: 'entry' })}`);
  } else if (uniqueEvaluatedViolations.length > 0 && hasGitWaiver) {
    warnings.push(`Out-of-scope files in the evaluated diff but waiver present [CX-573F]: ${formatBoundedItemList(uniqueEvaluatedViolations, { noun: 'entry' })}`);
  } else if (uniqueEvaluatedViolations.length > 0 && evaluatedWaiverCoverage.ok) {
    warnings.push(`Out-of-scope files in the evaluated diff covered by active baseline compile waiver(s) ${formatWaiverIds(evaluatedWaiverCoverage)}: ${formatBoundedItemList(uniqueEvaluatedViolations, { noun: 'entry' })}`);
  }

  if (uniqueBranchLocalViolations.length > 0 && !hasGitWaiver && !branchLocalWaiverCoverage.ok) {
    errors.push(`Branch-local scope drift detected outside the evaluated diff: ${formatBoundedItemList(uniqueBranchLocalViolations, { noun: 'entry' })}`);
  } else if (uniqueBranchLocalViolations.length > 0 && hasGitWaiver) {
    warnings.push(`Branch-local scope drift detected but waiver present [CX-573F]: ${formatBoundedItemList(uniqueBranchLocalViolations, { noun: 'entry' })}`);
  } else if (uniqueBranchLocalViolations.length > 0 && branchLocalWaiverCoverage.ok) {
    warnings.push(`Branch-local scope drift covered by active baseline compile waiver(s) ${formatWaiverIds(branchLocalWaiverCoverage)}: ${formatBoundedItemList(uniqueBranchLocalViolations, { noun: 'entry' })}`);
  }

  if (uniqueGovernanceNoiseWarnings.length > 0) {
    warnings.push(`Governance-only drift visible in this worktree (not counted as WP evidence): ${formatBoundedItemList(uniqueGovernanceNoiseWarnings, { noun: 'path' })}`);
  }

  if (uniqueTransientArtifactWarnings.length > 0) {
    warnings.push(`Transient proof artifacts visible in this worktree (not counted as WP evidence): ${formatBoundedItemList(uniqueTransientArtifactWarnings, { noun: 'path' })}`);
  }

  if (enforceScopeDiscipline) {
    if (!scopeDiscipline.touchedFileBudgetValid) {
      errors.push('TOUCHED_FILE_BUDGET must be an integer >= 1 for PACKET_FORMAT_VERSION >= 2026-03-23');
    } else {
      const evaluatedBudgetFiles = collectBudgetCountedFiles(changedFiles, scopeContract);
      const branchLocalBudgetFiles = collectBudgetCountedFiles(branchLocalChangedFiles, scopeContract);
      const evaluatedBudgetWaiverCoverage = evaluateWaiverCoverage({ paths: evaluatedBudgetFiles, waivers: baselineCompileWaivers });
      const branchLocalBudgetWaiverCoverage = evaluateWaiverCoverage({ paths: branchLocalBudgetFiles, waivers: baselineCompileWaivers });
      if (evaluatedBudgetFiles.length > scopeDiscipline.touchedFileBudget && !hasGitWaiver && !evaluatedBudgetWaiverCoverage.ok) {
        errors.push(`Touched file budget exceeded in evaluated diff: ${evaluatedBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(evaluatedBudgetFiles, { noun: 'path' })})`);
      } else if (evaluatedBudgetFiles.length > scopeDiscipline.touchedFileBudget && hasGitWaiver) {
        warnings.push(`Touched file budget exceeded in evaluated diff but waiver present [CX-573F]: ${evaluatedBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(evaluatedBudgetFiles, { noun: 'path' })})`);
      } else if (evaluatedBudgetFiles.length > scopeDiscipline.touchedFileBudget && evaluatedBudgetWaiverCoverage.ok) {
        warnings.push(`Touched file budget exceeded in evaluated diff but covered by active baseline compile waiver(s) ${formatWaiverIds(evaluatedBudgetWaiverCoverage)}: ${evaluatedBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(evaluatedBudgetFiles, { noun: 'path' })})`);
      } else {
        console.log(`PASS: touched file budget respected in evaluated diff (${evaluatedBudgetFiles.length}/${scopeDiscipline.touchedFileBudget})`);
      }

      if (branchLocalBudgetFiles.length > scopeDiscipline.touchedFileBudget && !hasGitWaiver && !branchLocalBudgetWaiverCoverage.ok) {
        errors.push(`Touched file budget exceeded by branch-local scope drift: ${branchLocalBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(branchLocalBudgetFiles, { noun: 'path' })})`);
      } else if (branchLocalBudgetFiles.length > scopeDiscipline.touchedFileBudget && hasGitWaiver) {
        warnings.push(`Touched file budget exceeded by branch-local scope drift but waiver present [CX-573F]: ${branchLocalBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(branchLocalBudgetFiles, { noun: 'path' })})`);
      } else if (branchLocalBudgetFiles.length > scopeDiscipline.touchedFileBudget && branchLocalBudgetWaiverCoverage.ok) {
        warnings.push(`Touched file budget exceeded by branch-local scope drift but covered by active baseline compile waiver(s) ${formatWaiverIds(branchLocalBudgetWaiverCoverage)}: ${branchLocalBudgetFiles.length} > ${scopeDiscipline.touchedFileBudget} (${formatBoundedItemList(branchLocalBudgetFiles, { noun: 'path' })})`);
      }
    }
    if (!scopeDiscipline.broadToolAllowlistValid) {
      const allowlistDetail = scopeDiscipline.invalidBroadToolTokens.includes('NONE_WITH_OTHERS')
        ? 'NONE cannot be combined with other broad tool allowlist tokens'
        : `invalid token(s): ${scopeDiscipline.invalidBroadToolTokens.join(', ')}`;
      errors.push(`BROAD_TOOL_ALLOWLIST invalid: ${allowlistDetail}`);
    } else {
      console.log(`INFO: broad tool allowlist = ${(scopeDiscipline.broadToolAllowlist.length > 0 ? scopeDiscipline.broadToolAllowlist.join(', ') : 'NONE')}`);
    }
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
      if (zeroDeltaProofAllowed) {
        warnings.push(
          `No files changed in range ${evaluation.baseRev}..${evaluation.headRev}; ZERO_DELTA_PROOF_ALLOWED=YES so the empty diff is accepted for this proof-only/status-sync packet.`,
        );
      } else {
        errors.push(`No files changed in range ${evaluation.baseRev}..${evaluation.headRev}`);
      }
    } else {
      if (zeroDeltaProofAllowed) {
        warnings.push('No files changed (git status clean); ZERO_DELTA_PROOF_ALLOWED=YES so the empty diff is accepted for this proof-only/status-sync packet.');
      } else {
        errors.push('No files changed (git status clean)');
      }
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
  console.log(`See: ${GOV_ROOT_REPO_REL}/roles/coder/CODER_PROTOCOL.md`);
  process.exit(1);
}
