#!/usr/bin/env node
/**
 * Pre-work validation [CX-580, CX-620]
 * - Verifies task packet exists before work starts
 * - Ensures deterministic manifest template (COR-701-style) is present so post-work can enforce gates
 */

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import {
  defaultRefinementPath,
  resolveSpecCurrent,
  validateRefinementFile,
} from './refinement-check.mjs';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node pre-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\nPre-work validation for ${WP_ID}...\n`);

const errors = [];
const warnings = [];
const spec = JSON.parse(fs.readFileSync(path.join('.GOV', 'scripts', 'validation', 'cor701-spec.json'), 'utf8'));

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = text.match(re);
  return m ? m[1].trim() : '';
}

function parseStatus(text) {
  const statusLine =
    (text.match(/^\\s*-\\s*\\*\\*Status:\\*\\*\\s*(.+)\\s*$/mi) || [])[1] ||
    (text.match(/^\\s*\\*\\*Status:\\*\\*\\s*(.+)\\s*$/mi) || [])[1] ||
    (text.match(/^\\s*Status:\\s*(.+)\\s*$/mi) || [])[1] ||
    '';
  return statusLine.trim();
}

function extractIndentedListAfterLabel(text, label, { stopLabels = [] } = {}) {
  const lines = text.split(/\r?\n/);
  const idx = lines.findIndex((l) => new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, 'i').test(l));
  if (idx === -1) return [];

  const stopRes = stopLabels.map((s) => new RegExp(`^\\s*-\\s*${s}\\s*:\\s*$`, 'i'));
  const items = [];

  for (let i = idx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (stopRes.some((re) => re.test(line))) break;
    if (/^##\s+\S/.test(line)) break;
    const m = line.match(/^\s{2,}-\s+(.+)\s*$/);
    if (m) items.push(m[1].trim());
  }
  return items;
}

function extractFencedBlockAfterHeading(text, heading) {
  const lines = text.split(/\r?\n/);
  const headingIdx = lines.findIndex((l) => new RegExp(`^#{2,6}\\s+${heading}\\b`, 'i').test(l));
  if (headingIdx === -1) return '';

  const sectionStart = headingIdx + 1;
  let sectionEnd = lines.length;
  for (let j = sectionStart; j < lines.length; j += 1) {
    if (/^#{1,6}\s+\S/.test(lines[j])) {
      sectionEnd = j;
      break;
    }
  }

  for (let i = sectionStart; i < sectionEnd; i += 1) {
    const fenceStart = lines[i].trim();
    const fenceRe = /^```([a-z0-9_-]+)?\s*$/i;
    const m = fenceStart.match(fenceRe);
    if (!m) continue;

    const bodyLines = [];
    for (let k = i + 1; k < sectionEnd; k += 1) {
      if (lines[k].trim() === '```') break;
      bodyLines.push(lines[k]);
    }
    return bodyLines.join('\n').trim();
  }

  return '';
}

function extractBulletListAfterHeading(text, heading) {
  const lines = text.split(/\r?\n/);
  const headingIdx = lines.findIndex((l) => new RegExp(`^#{2,6}\\s+${heading}\\b`, 'i').test(l));
  if (headingIdx === -1) return [];

  const sectionStart = headingIdx + 1;
  let sectionEnd = lines.length;
  for (let j = sectionStart; j < lines.length; j += 1) {
    if (/^#{1,6}\s+\S/.test(lines[j])) {
      sectionEnd = j;
      break;
    }
  }

  const items = [];
  for (let i = sectionStart; i < sectionEnd; i += 1) {
    const m = lines[i].match(/^\s*-\s+(.+)\s*$/);
    if (m) items.push(m[1].trim());
  }
  return items;
}

// Check 1: Task packet file exists
console.log('Check 1: Task packet file exists');
const taskPacketDir = '.GOV/task_packets';
if (!fs.existsSync(taskPacketDir)) {
  fs.mkdirSync(taskPacketDir, { recursive: true });
}

const taskPacketFiles = fs.readdirSync(taskPacketDir)
  .filter((f) => f.includes(WP_ID) && f.endsWith('.md'));

let packetContent = '';
let packetPath = '';
let lastPrepare = null;

if (taskPacketFiles.length === 0) {
  errors.push(`No task packet file found for ${WP_ID} in .GOV/task_packets/`);
  console.log('FAIL: No task packet file');
} else {
  packetPath = path.join(taskPacketDir, taskPacketFiles[0]);
  packetContent = fs.readFileSync(packetPath, 'utf8');
  console.log(`PASS: Found ${taskPacketFiles[0]}`);

  // Check 1.5: Worktree + branch preflight (mechanical guard against wrong-worktree edits)
  console.log('\nCheck 1.5: Worktree + branch preflight [CX-WT-001]');
  const packetFormatVersion = (packetContent.match(/^\s*-\s*PACKET_FORMAT_VERSION\s*:\s*(.+)\s*$/mi) || [])[1]?.trim();
  const enforceWorktreeGate = !!packetFormatVersion;

  let currentBranch = '';
  let currentTop = '';
  try {
    currentBranch = execSync('git rev-parse --abbrev-ref HEAD', { encoding: 'utf8' }).trim();
    currentTop = execSync('git rev-parse --show-toplevel', { encoding: 'utf8' }).trim();
  } catch {
    warnings.push('Could not read current git branch/worktree (git rev-parse failed)');
  }

  try {
    const gatesPath = path.join('.GOV', 'roles', 'orchestrator', 'ORCHESTRATOR_GATES.json');
    const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
    const logs = Array.isArray(gates?.gate_logs) ? gates.gate_logs : [];
    lastPrepare = [...logs].reverse().find((l) => l?.wpId === WP_ID && l?.type === 'PREPARE') || null;
  } catch {
    lastPrepare = null;
  }

  if (!lastPrepare) {
    const msg = `Missing PREPARE record in .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json for ${WP_ID} (expected: just record-prepare ${WP_ID} {Coder-A|Coder-B} ...)`;
    if (enforceWorktreeGate) {
      errors.push(msg);
      console.log('FAIL: ' + msg);
    } else {
      warnings.push(msg);
      console.log('WARN: ' + msg);
    }
  } else {
    const expectedBranch = (lastPrepare.branch || '').trim();
    const expectedWorktreeDir = (lastPrepare.worktree_dir || '').trim();

    if (expectedBranch && currentBranch && expectedBranch !== currentBranch) {
      errors.push(
        `Wrong branch for ${WP_ID}: expected ${expectedBranch} (from PREPARE), got ${currentBranch}`
      );
      console.log(`FAIL: Branch mismatch (expected ${expectedBranch}, got ${currentBranch})`);
    } else if (expectedBranch && currentBranch) {
      console.log(`PASS: Branch matches PREPARE (${currentBranch})`);
    }

    if (expectedWorktreeDir && currentTop) {
      const expectedAbs = path.isAbsolute(expectedWorktreeDir)
        ? path.resolve(expectedWorktreeDir)
        : path.resolve(currentTop, expectedWorktreeDir);
      const currentAbs = path.resolve(currentTop);
      if (expectedAbs.toLowerCase() !== currentAbs.toLowerCase()) {
        const msg = `Worktree_dir mismatch for ${WP_ID}: PREPARE says ${expectedWorktreeDir} (resolves to ${expectedAbs}), current is ${currentAbs}`;
        if (enforceWorktreeGate) {
          errors.push(msg);
          console.log('FAIL: ' + msg);
        } else {
          warnings.push(msg);
          console.log('WARN: ' + msg);
        }
      } else {
        console.log(`PASS: Worktree_dir matches PREPARE (${expectedWorktreeDir})`);
      }
    }
  }

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

  const lowerContent = packetContent.toLowerCase();
  const missingFields = requiredFields.filter((field) => !lowerContent.includes(field.toLowerCase()));

  if (missingFields.length > 0) {
    errors.push(`Task packet missing fields: ${missingFields.join(', ')}`);
    console.log(`FAIL: Missing ${missingFields.join(', ')}`);
  } else {
    console.log('PASS: All required fields present');
  }

  // Check 2.5: Spec provenance/target fields (non-blocking; backward compatible)
  const hasLegacySpec = /SPEC_CURRENT/i.test(packetContent);
  const hasSpecBaseline = /SPEC_BASELINE/i.test(packetContent);
  const hasSpecTarget = /SPEC_TARGET/i.test(packetContent);
  if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
    warnings.push('Spec reference missing: include SPEC_BASELINE (provenance) and SPEC_TARGET (closure target), or legacy SPEC_CURRENT.');
  }

  // Check 2.5B: MERGE_BASE_SHA (recommended for deterministic multi-commit post-work)
  const mergeBaseSha = (packetContent.match(/^\s*-\s*MERGE_BASE_SHA\s*:\s*([a-f0-9]{40})\s*$/mi) || [])[1]?.trim();
  if (!mergeBaseSha) {
    warnings.push('Packet missing MERGE_BASE_SHA; for multi-commit WPs prefer deterministic evidence: just post-work WP-{ID} --range <MERGE_BASE_SHA>..HEAD');
  }

  // Check 2.6: Canonical Status field (governance invariant)
  const statusLine = (packetContent.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (packetContent.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || '';
  const statusNorm = statusLine.trim().toLowerCase();
  if (!statusLine) {
    errors.push('Missing canonical **Status:** field');
  }

  const isDoneLike = /\b(done|validated|complete)\b/i.test(statusLine);
  const requiresRefinementGate = !isDoneLike; // pre-work implies active work; enforce unless explicitly Done/Validated.

  // Check 2.6A: Agentic mode flag (required for active packets in modern format)
  const agenticModeRaw = (packetContent.match(/^\s*-\s*AGENTIC_MODE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
  const agenticMode = agenticModeRaw.toUpperCase();
  const isModernPacket = !!packetFormatVersion;

  if (isModernPacket && requiresRefinementGate) {
    if (!/^(YES|NO)$/.test(agenticMode)) {
      errors.push('AGENTIC_MODE missing or invalid (expected YES|NO) for active packets');
    }

    if (agenticMode === 'YES') {
      const orchModel = (packetContent.match(/^\s*-\s*ORCHESTRATOR_MODEL\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
      const orchStart = (packetContent.match(/^\s*-\s*ORCHESTRATION_STARTED_AT_UTC\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';

      const isPlaceholder = (v) => {
        const s = (v || '').trim();
        if (!s) return true;
        if (/^<pending>$/i.test(s)) return true;
        if (/^<fill/i.test(s)) return true;
        if (/^<unclaimed>$/i.test(s)) return true;
        return false;
      };

      if (isPlaceholder(orchModel)) {
        errors.push('ORCHESTRATOR_MODEL is required when AGENTIC_MODE=YES');
      }
      if (isPlaceholder(orchStart)) {
        errors.push('ORCHESTRATION_STARTED_AT_UTC is required when AGENTIC_MODE=YES');
      } else {
        const rfc3339Utc = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;
        if (!rfc3339Utc.test(orchStart)) {
          errors.push(`ORCHESTRATION_STARTED_AT_UTC must be RFC3339 UTC (got: ${orchStart})`);
        }
      }
    }
  } else if (isModernPacket && !agenticModeRaw) {
    warnings.push('AGENTIC_MODE missing (expected YES|NO) for modern packets');
  }

  // Check 2.6B: Sub-agent delegation decision (optional, Operator-gated)
  const hasSubAgentHeading = /##\s*SUB_AGENT_DELEGATION\b/i.test(packetContent);
  const subAgentDelegationRaw = parseSingleField(packetContent, 'SUB_AGENT_DELEGATION');
  const operatorApprovalRaw = parseSingleField(packetContent, 'OPERATOR_APPROVAL_EVIDENCE');
  const subAgentAssumptionRaw = parseSingleField(packetContent, 'SUB_AGENT_REASONING_ASSUMPTION');
  const hasSubAgentFields = !!(subAgentDelegationRaw || operatorApprovalRaw || subAgentAssumptionRaw);

  if (isModernPacket && requiresRefinementGate && (hasSubAgentHeading || hasSubAgentFields)) {
    const looksTemplate = (v) => /\|/.test(v || '') || /<pending>/i.test(v || '') || /<fill/i.test(v || '') || /<unclaimed>/i.test(v || '');

    if (!subAgentDelegationRaw) {
      errors.push('SUB_AGENT_DELEGATION section present but SUB_AGENT_DELEGATION field is missing');
    } else if (looksTemplate(subAgentDelegationRaw)) {
      errors.push('SUB_AGENT_DELEGATION contains template placeholders (set to DISALLOWED or ALLOWED)');
    } else if (!/^(DISALLOWED|ALLOWED)\b/i.test(subAgentDelegationRaw.trim())) {
      errors.push(`SUB_AGENT_DELEGATION invalid (expected DISALLOWED|ALLOWED; got: ${subAgentDelegationRaw})`);
    }

    const isAllowed = /^ALLOWED\b/i.test((subAgentDelegationRaw || '').trim());

    if (subAgentAssumptionRaw) {
      if (!/\bLOW\b/i.test(subAgentAssumptionRaw)) {
        errors.push('SUB_AGENT_REASONING_ASSUMPTION must be LOW (HARD)');
      }
    } else if (hasSubAgentHeading) {
      warnings.push('SUB_AGENT_REASONING_ASSUMPTION missing (expected LOW (HARD))');
    }

    if (isAllowed) {
      if (!operatorApprovalRaw || looksTemplate(operatorApprovalRaw) || /^N\/?A\b/i.test(operatorApprovalRaw.trim())) {
        errors.push('OPERATOR_APPROVAL_EVIDENCE is required when SUB_AGENT_DELEGATION=ALLOWED (paste exact Operator approval line from chat)');
      }
      if (!subAgentAssumptionRaw || !/\bLOW\b/i.test(subAgentAssumptionRaw)) {
        errors.push('SUB_AGENT_REASONING_ASSUMPTION must be LOW (HARD) when SUB_AGENT_DELEGATION=ALLOWED');
      }
    } else if (operatorApprovalRaw && !/^N\/?A\b/i.test(operatorApprovalRaw.trim())) {
      warnings.push('OPERATOR_APPROVAL_EVIDENCE should be N/A when SUB_AGENT_DELEGATION=DISALLOWED');
    }
  }

  // Check 2.7: Technical Refinement gate (unskippable for active packets)
  if (requiresRefinementGate) {
    console.log('\nCheck 2.7: Technical Refinement gate');

    const refinementFile = (packetContent.match(/^\s*-\s*REFINEMENT_FILE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || defaultRefinementPath(WP_ID);

    const refinementValidation = validateRefinementFile(refinementFile, { expectedWpId: WP_ID, requireSignature: true });
    if (!refinementValidation.ok) {
      errors.push(`Technical refinement gate failed (see ${refinementFile})`);
      refinementValidation.errors.forEach((e) => errors.push(`  - ${e}`));
    } else {
      console.log('PASS: Refinement file exists and is approved/signed');
    }

    const packetSig = (packetContent.match(/^\s*-\s*USER_SIGNATURE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*\*\*User Signature Locked:\*\*\s*(.+)\s*$/mi) || [])[1]?.trim()
      || (packetContent.match(/^\s*User Signature Locked:\s*(.+)\s*$/mi) || [])[1]?.trim()
      || '';

    if (!packetSig || /<pending>/i.test(packetSig)) {
      errors.push('USER_SIGNATURE missing or <pending> (active packets must be locked before work starts)');
    } else if (refinementValidation.ok && refinementValidation.parsed.signature && packetSig !== refinementValidation.parsed.signature) {
      errors.push(`USER_SIGNATURE mismatch: packet has ${packetSig}, refinement has ${refinementValidation.parsed.signature}`);
    }

    // Protocol requirement: signature must be present in SIGNATURE_AUDIT.md
    try {
      const auditPath = path.join('.GOV', 'roles_shared', 'SIGNATURE_AUDIT.md');
      const audit = fs.readFileSync(auditPath, 'utf8');
      if (packetSig && !audit.includes(`| ${packetSig} |`)) {
        errors.push(`USER_SIGNATURE not found in .GOV/roles_shared/SIGNATURE_AUDIT.md (${packetSig})`);
      }
    } catch {
      warnings.push('Could not verify signature against .GOV/roles_shared/SIGNATURE_AUDIT.md');
    }

    // Safety checkpoint gate: packet + refinement must be committed before development starts.
    // This prevents untracked/uncommitted WP artifacts from being lost during accidental clean/reset operations.
    console.log('\nCheck 2.8: WP checkpoint commit gate');
    try {
      execSync(`git cat-file -e HEAD:${packetPath.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Task packet is not committed yet (checkpoint required): ${packetPath.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} .GOV/roles_shared/SIGNATURE_AUDIT.md .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }

    try {
      execSync(`git cat-file -e HEAD:${refinementFile.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Refinement file is not committed yet (checkpoint required): ${refinementFile.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} .GOV/roles_shared/SIGNATURE_AUDIT.md .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }
  } else {
    console.log('\nCheck 2.7: Technical Refinement gate (skipped for Done/Validated packets)');
  }

  // Check 3: Deterministic manifest template present
  console.log('\nCheck 3: Deterministic manifest template');
  if (!/##\s*validation/i.test(packetContent)) {
    errors.push('VALIDATION section missing (required for deterministic manifest)');
    console.log('FAIL: Missing VALIDATION section');
  } else {
    const lower = packetContent.toLowerCase();
    const lowerNorm = lower.replace(/[-_]/g, ' ');
    const fieldMissing = spec.requiredFields.filter((f) => !lowerNorm.includes(f.replace(/_/g, ' ')));
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

  const status = parseStatus(packetContent) || '<missing>';
  const statusNorm = status.toLowerCase();
  const startAllowed = /ready\s*for\s*dev|in\s*progress/.test(statusNorm) && !/blocked|stub|done|validated/.test(statusNorm);

  console.log('');
  if (startAllowed) {
    console.log('You may proceed with implementation.');
  } else {
    console.log(`NOTE: Task packet Status is "${status}". Do NOT start implementation unless Status is Ready for Dev or In Progress.`);
  }

  // Automatic Coder handoff template (printed when packet is actually startable).
  if (startAllowed) {
    let resolved = null;
    try {
      resolved = resolveSpecCurrent();
    } catch {
      resolved = null;
    }

    const riskTier = parseSingleField(packetContent, 'RISK_TIER') || '<missing>';
    const baseWpId = parseSingleField(packetContent, 'BASE_WP_ID') || '<missing>';
    const mergeBaseSha = parseSingleField(packetContent, 'MERGE_BASE_SHA') || '<missing>';
    const refinementFile = defaultRefinementPath(WP_ID).replace(/\\/g, '/');

    const inScope = extractIndentedListAfterLabel(packetContent, 'IN_SCOPE_PATHS', { stopLabels: ['OUT_OF_SCOPE'] });
    const outOfScope = extractIndentedListAfterLabel(packetContent, 'OUT_OF_SCOPE', { stopLabels: [] });
    const doneMeans = extractBulletListAfterHeading(packetContent, 'DONE_MEANS');
    const testPlan = extractFencedBlockAfterHeading(packetContent, 'TEST_PLAN');

    const subAgentDelegation = parseSingleField(packetContent, 'SUB_AGENT_DELEGATION') || 'DISALLOWED (field missing)';
    const operatorApproval = parseSingleField(packetContent, 'OPERATOR_APPROVAL_EVIDENCE') || 'N/A';
    const subAgentAssumption = parseSingleField(packetContent, 'SUB_AGENT_REASONING_ASSUMPTION') || 'LOW (default)';

    const expectedBranch = (lastPrepare?.branch || '').trim();
    const expectedWorktreeDir = (lastPrepare?.worktree_dir || '').trim();
    const coderId = (lastPrepare?.coder_id || '').trim() || '<unknown>';

    console.log('\nCODER_HANDOFF [CX-HANDOFF-001]');
    console.log(`- WP_ID: ${WP_ID}`);
    console.log(`- BASE_WP_ID: ${baseWpId}`);
    console.log(`- Status: ${status}`);
    console.log(`- RISK_TIER: ${riskTier}`);
    console.log(`- MERGE_BASE_SHA: ${mergeBaseSha}`);
    if (resolved?.specFileName) console.log(`- SPEC_CURRENT_RESOLVED: ${resolved.specFileName}`);
    console.log(`- Task packet: ${packetPath.replace(/\\/g, '/')}`);
    console.log(`- Refinement: ${refinementFile}`);
    console.log(`- Worktree_dir (repo-relative): ${expectedWorktreeDir || '<missing>'}`);
    console.log(`- Branch: ${expectedBranch || '<missing>'}`);
    console.log(`- Coder: ${coderId}`);
    console.log(`- Sub-agent delegation: ${subAgentDelegation}`);
    console.log(`- Sub-agent reasoning assumption: ${subAgentAssumption}`);
    console.log(`- Operator approval evidence: ${operatorApproval}`);

    if (/^ALLOWED\b/i.test(subAgentDelegation.trim())) {
      console.log('\nSUB_AGENT_RULES (HARD) [CX-HANDOFF-001]');
      console.log('- Sub-agents are LOW reasoning (draft-only). Verify everything against SPEC_CURRENT + DONE_MEANS.');
      console.log('- Sub-agents MUST NOT edit `.GOV/**` (including task packets/refinements or `## VALIDATION_REPORTS`).');
      console.log('- Sub-agents MUST NOT run gates/commits/branch ops as official evidence.');
      console.log('- Follow: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.');
    }

    console.log('\nCODER_START_COMMANDS [CX-HANDOFF-001]');
    console.log('```bash');
    console.log(`# Verify you are in the correct WP worktree/branch (paste outputs to chat):`);
    console.log('just hard-gate-wt-001');
    console.log('');
    console.log(`# Re-validate WP gates in your environment:`);
    console.log(`just pre-work ${WP_ID}`);
    console.log('```');

    if (inScope.length > 0) {
      console.log('\nIN_SCOPE_PATHS [CX-HANDOFF-001]');
      inScope.forEach((p) => console.log(`- ${p}`));
    }

    if (outOfScope.length > 0) {
      console.log('\nOUT_OF_SCOPE [CX-HANDOFF-001]');
      outOfScope.forEach((p) => console.log(`- ${p}`));
    }

    if (doneMeans.length > 0) {
      console.log('\nDONE_MEANS [CX-HANDOFF-001]');
      doneMeans.forEach((d) => console.log(`- ${d}`));
    }

    if (testPlan) {
      console.log('\nTEST_PLAN (verbatim) [CX-HANDOFF-001]');
      console.log('```bash');
      console.log(testPlan);
      console.log('```');
    }
  }
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
  console.log('See: .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md or .GOV/roles/coder/CODER_PROTOCOL.md');
  process.exit(1);
}
