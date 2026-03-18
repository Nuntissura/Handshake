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
} from '../../../roles_shared/checks/refinement-check.mjs';
import {
  formatClauseProofPlanSection,
  formatCoderHandoffBriefSection,
  formatContractSurfacesSection,
  formatNotProvenAtRefinementTimeSection,
  formatSpecContextWindowsSection,
  formatValidatorHandoffBriefSection,
} from '../../../roles_shared/scripts/lib/refinement-brief-lib.mjs';
import { validatePacketClosureMonitoring } from '../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs';
import {
  deriveSemanticProofAssets,
  formatSemanticProofAssetsSection,
  validateSemanticProofAssets,
} from '../../../roles_shared/scripts/lib/semantic-proof-lib.mjs';
import {
  CLI_ESCALATION_HOST_DEFAULT,
  CLI_SESSION_TOOL,
  CODEX_MODEL_ALIASES_ALLOWED,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  buildRemoteBackupUrl,
  packetUsesSharedRemoteWpBackup,
  packetUsesStructuredValidationReport,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  MODEL_FAMILY_POLICY,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_REASONING_REQUIRED,
  ROLE_SESSION_RUNTIME,
  SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS,
  SESSION_PLUGIN_BRIDGE_COMMAND,
  SESSION_PLUGIN_BRIDGE_ID,
  SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION,
  SESSION_START_AUTHORITY,
  SESSION_WAKE_CHANNEL_FALLBACK,
  SESSION_WAKE_CHANNEL_PRIMARY,
  SESSION_WATCH_POLICY,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
  SESSION_LAUNCH_POLICY,
  sessionPluginRequestsFileForPacketVersion,
  sessionRegistryFileForPacketVersion,
} from '../../../roles_shared/scripts/session/session-policy.mjs';
import { GOV_ROOT_REPO_REL } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

const WP_ID = process.argv[2];

if (!WP_ID) {
  console.error('Usage: node pre-work-check.mjs WP-{ID}');
  process.exit(1);
}

console.log(`\nPre-work validation for ${WP_ID}...\n`);

const errors = [];
const warnings = [];
const spec = JSON.parse(fs.readFileSync(path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'cor701-spec.json'), 'utf8'));

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = text.match(re);
  return m ? m[1].trim() : '';
}

function parseStatus(text) {
  const statusLine =
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1] ||
    '';
  return statusLine.trim();
}

function isIsoDate(s) {
  return /^\d{4}-\d{2}-\d{2}$/.test(String(s || '').trim());
}

function isVersionAtLeast(isoDate, minIsoDate) {
  if (!isIsoDate(isoDate) || !isIsoDate(minIsoDate)) return false;
  return isoDate >= minIsoDate;
}

function looksPlaceholder(value) {
  const v = String(value || '').trim();
  if (!v) return true;
  if (/^\{.+\}$/.test(v)) return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>/i.test(v)) return true;
  if (/^<unclaimed>/i.test(v)) return true;
  return false;
}

function extractIndentedListAfterLabel(text, label, { stopLabels = [] } = {}) {
  const lines = text.split(/\r?\n/);
  const idx = lines.findIndex((l) => new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, 'i').test(l));
  if (idx === -1) return [];

  const stopRes = stopLabels.map((s) => new RegExp(`^\\s*-\\s*${s}\\s*:\\s*$`, 'i'));
  const topLevelLabelRe = /^-\s*(?:\*\*)?[A-Z][A-Z0-9_ ()/.-]*(?:\*\*)?\s*:\s*$/;
  const items = [];

  for (let i = idx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (stopRes.some((re) => re.test(line))) break;
    if (/^##\s+\S/.test(line)) break;
    if (topLevelLabelRe.test(line)) break;
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

function extractSectionBlock(text, heading) {
  const lines = text.split(/\r?\n/);
  const headingIdx = lines.findIndex((line) => new RegExp(`^#{2,6}\\s+${heading}\\b`, 'i').test(line));
  if (headingIdx === -1) return '';

  const headingMatch = lines[headingIdx].match(/^(#{2,6})\s+/);
  const headingLevel = headingMatch ? headingMatch[1].length : 2;
  const sectionLines = [lines[headingIdx]];
  let inFence = false;
  for (let i = headingIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    const trimmed = line.trim();
    if (/^```/.test(trimmed)) {
      inFence = !inFence;
      sectionLines.push(line);
      continue;
    }
    const nextHeadingMatch = !inFence ? line.match(/^(#{1,6})\s+\S/) : null;
    if (nextHeadingMatch && nextHeadingMatch[1].length <= headingLevel) break;
    sectionLines.push(line);
  }
  return sectionLines.join('\n').trim();
}

function extractFencedBlockAfterLabel(text, label) {
  const lines = text.split(/\r?\n/);
  const idx = lines.findIndex((l) => new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, 'i').test(l));
  if (idx === -1) return '';

  let i = idx + 1;
  while (i < lines.length && lines[i].trim() === '') i += 1;
  if (i >= lines.length) return '';
  if (!/^```/.test(lines[i].trim())) return '';

  const body = [];
  for (let j = i + 1; j < lines.length; j += 1) {
    if (lines[j].trim() === '```') break;
    body.push(lines[j]);
  }
  return body.join('\n').trim();
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
  const topLevelLabelRe = /^-\s*(?:\*\*)?[A-Z][A-Z0-9_ ()/.-]*(?:\*\*)?\s*:\s*$/;
  for (let i = sectionStart; i < sectionEnd; i += 1) {
    if (topLevelLabelRe.test(lines[i])) break;
    const m = lines[i].match(/^\s*-\s+(.+)\s*$/);
    if (m) items.push(m[1].trim());
  }
  return items;
}

function normalizeList(items) {
  const normalized = (items || []).map((item) => String(item || '').trim()).filter(Boolean);
  return normalized.every((item) => item.toUpperCase() === 'NONE') ? [] : normalized;
}

function normalizeBlock(text) {
  return String(text || '').replace(/\r/g, '').trim();
}

function sameList(a, b) {
  const aa = normalizeList(a);
  const bb = normalizeList(b);
  return aa.length === bb.length && aa.every((item, idx) => item === bb[idx]);
}

function escapeRegex(s) {
  return (s ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function hasCommitByExactSubject(subject) {
  const subjectRe = `^${escapeRegex(subject)}$`;
  try {
    const sha = execSync(`git log -n 1 --format=%H --grep="${subjectRe}"`, { encoding: 'utf8' }).trim();
    return Boolean(sha);
  } catch {
    return false;
  }
}

// Check 1: Task packet file exists
console.log('Check 1: Task packet file exists');
const taskPacketDir = `${GOV_ROOT_REPO_REL}/task_packets`;
const packetFilename = `${WP_ID}.md`;

let packetContent = '';
let packetPath = '';
let lastPrepare = null;

if (!fs.existsSync(taskPacketDir)) {
  errors.push(`Task packet directory not found: ${taskPacketDir}`);
  console.log(`FAIL: Missing directory ${taskPacketDir}`);
} else {
  packetPath = path.join(taskPacketDir, packetFilename);
  if (!fs.existsSync(packetPath)) {
    errors.push(`No exact task packet file found for ${WP_ID}: expected ${taskPacketDir}/${packetFilename}`);
    console.log(`FAIL: Missing ${packetFilename}`);
  } else {
    packetContent = fs.readFileSync(packetPath, 'utf8');
    console.log(`PASS: Found ${packetFilename}`);
  }

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
    const gatesPath = path.join(GOV_ROOT_REPO_REL, 'roles', 'orchestrator', 'runtime', 'ORCHESTRATOR_GATES.json');
    const gates = JSON.parse(fs.readFileSync(gatesPath, 'utf8'));
    const logs = Array.isArray(gates?.gate_logs) ? gates.gate_logs : [];
    lastPrepare = [...logs].reverse().find((l) => l?.wpId === WP_ID && l?.type === 'PREPARE') || null;
  } catch {
    lastPrepare = null;
  }

  if (!lastPrepare) {
    const msg = `Missing PREPARE record in ${GOV_ROOT_REPO_REL}/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json for ${WP_ID} (expected: just record-prepare ${WP_ID} {Coder-A..Coder-Z} ...)`;
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
  const mergeBaseShaRaw = parseSingleField(packetContent, 'MERGE_BASE_SHA');
  const mergeBaseSha = (mergeBaseShaRaw.match(/[a-f0-9]{40}/i) || [])[0]?.trim() || '';
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
  const isModernPacket = !!packetFormatVersion;
  const usesStructuredValidationReport = packetUsesStructuredValidationReport(packetFormatVersion);
  const clauseClosureMonitorProfile = parseSingleField(packetContent, 'CLAUSE_CLOSURE_MONITOR_PROFILE');
  const usesClauseClosureMonitor = /^CLAUSE_MONITOR_V1$/i.test(clauseClosureMonitorProfile || '');
  const semanticProofProfile = parseSingleField(packetContent, 'SEMANTIC_PROOF_PROFILE');
  const usesSemanticProofProfile = /^DIFF_SCOPED_SEMANTIC_V1$/i.test(semanticProofProfile || '');

  if (isModernPacket && usesStructuredValidationReport && requiresRefinementGate) {
    console.log('\nCheck 2.6AA: Bootstrap claim checkpoint');
    const coderModel = parseSingleField(packetContent, 'CODER_MODEL');
    const coderStrength = parseSingleField(packetContent, 'CODER_REASONING_STRENGTH');
    const hasBootstrapClaim = hasCommitByExactSubject(`docs: bootstrap claim [${WP_ID}]`);
    const hasSkeletonCheckpoint = hasCommitByExactSubject(`docs: skeleton checkpoint [${WP_ID}]`);
    const bootstrapIsRequired =
      /\bin progress\b/i.test(statusLine)
      || hasSkeletonCheckpoint
      || !looksPlaceholder(coderModel)
      || !looksPlaceholder(coderStrength);

    if (bootstrapIsRequired && !hasBootstrapClaim) {
      errors.push(
        `Missing docs-only bootstrap claim commit for ${WP_ID}. Expected commit subject: docs: bootstrap claim [${WP_ID}]`
      );
      console.log(`FAIL: Missing bootstrap claim commit docs: bootstrap claim [${WP_ID}]`);
    } else if (bootstrapIsRequired) {
      console.log(`PASS: Bootstrap claim commit present for ${WP_ID}`);
    } else {
      console.log('PASS: Bootstrap claim not required yet (packet still pre-claim / Ready for Dev).');
    }
  }

  // Check 2.6A: Agentic mode flag (required for active packets in modern format)
  const agenticModeRaw = (packetContent.match(/^\s*-\s*AGENTIC_MODE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
  const agenticMode = agenticModeRaw.toUpperCase();
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

  // Check 2.6BC: Session policy for new packet format (enforced for packet format >= 2026-03-12)
  if (isModernPacket && isVersionAtLeast(packetFormatVersion, '2026-03-12')) {
    console.log('\nCheck 2.6BC: Session policy');

    const remoteBackupBranch = parseSingleField(packetContent, 'REMOTE_BACKUP_BRANCH');
    const remoteBackupUrl = parseSingleField(packetContent, 'REMOTE_BACKUP_URL');
    const expectedFields = [
      ['SESSION_START_AUTHORITY', SESSION_START_AUTHORITY],
      ['SESSION_HOST_PREFERENCE', SESSION_HOST_PREFERENCE],
      ['SESSION_HOST_FALLBACK', SESSION_HOST_FALLBACK],
      ['SESSION_LAUNCH_POLICY', SESSION_LAUNCH_POLICY],
      ['ROLE_SESSION_RUNTIME', ROLE_SESSION_RUNTIME],
      ['CLI_SESSION_TOOL', CLI_SESSION_TOOL],
      ['SESSION_PLUGIN_BRIDGE_ID', SESSION_PLUGIN_BRIDGE_ID],
      ['SESSION_PLUGIN_BRIDGE_COMMAND', SESSION_PLUGIN_BRIDGE_COMMAND],
      ['SESSION_PLUGIN_REQUESTS_FILE', sessionPluginRequestsFileForPacketVersion(packetFormatVersion)],
      ['SESSION_REGISTRY_FILE', sessionRegistryFileForPacketVersion(packetFormatVersion)],
      ['SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION', String(SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION)],
      ['SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS', String(SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS)],
      ['SESSION_WATCH_POLICY', SESSION_WATCH_POLICY],
      ['SESSION_WAKE_CHANNEL_PRIMARY', SESSION_WAKE_CHANNEL_PRIMARY],
      ['SESSION_WAKE_CHANNEL_FALLBACK', SESSION_WAKE_CHANNEL_FALLBACK],
      ['CLI_ESCALATION_HOST_DEFAULT', CLI_ESCALATION_HOST_DEFAULT],
      ['MODEL_FAMILY_POLICY', MODEL_FAMILY_POLICY],
      ['CODEX_MODEL_ALIASES_ALLOWED', CODEX_MODEL_ALIASES_ALLOWED],
      ['ROLE_SESSION_PRIMARY_MODEL', ROLE_SESSION_PRIMARY_MODEL],
      ['ROLE_SESSION_FALLBACK_MODEL', ROLE_SESSION_FALLBACK_MODEL],
      ['ROLE_SESSION_REASONING_REQUIRED', ROLE_SESSION_REASONING_REQUIRED],
      ['ROLE_SESSION_REASONING_CONFIG_KEY', ROLE_SESSION_REASONING_CONFIG_KEY],
      ['ROLE_SESSION_REASONING_CONFIG_VALUE', ROLE_SESSION_REASONING_CONFIG_VALUE],
      ['CODER_STARTUP_COMMAND', 'just coder-startup'],
      ['CODER_RESUME_COMMAND', `just coder-next ${WP_ID}`],
      ['WP_VALIDATOR_LOCAL_BRANCH', defaultWpValidatorBranch(WP_ID)],
      ['WP_VALIDATOR_LOCAL_WORKTREE_DIR', defaultWpValidatorWorktreeDir(WP_ID)],
      ['WP_VALIDATOR_STARTUP_COMMAND', 'just validator-startup'],
      ['WP_VALIDATOR_RESUME_COMMAND', `just validator-next ${WP_ID}`],
      ['INTEGRATION_VALIDATOR_LOCAL_BRANCH', defaultIntegrationValidatorBranch(WP_ID)],
      ['INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR', defaultIntegrationValidatorWorktreeDir(WP_ID)],
      ['INTEGRATION_VALIDATOR_STARTUP_COMMAND', 'just validator-startup'],
      ['INTEGRATION_VALIDATOR_RESUME_COMMAND', `just validator-next ${WP_ID}`],
    ];

    if (!remoteBackupBranch) {
      errors.push('REMOTE_BACKUP_BRANCH missing/invalid for packets with PACKET_FORMAT_VERSION >= 2026-03-12');
    } else if (packetUsesSharedRemoteWpBackup(packetFormatVersion)) {
      expectedFields.push(['WP_VALIDATOR_REMOTE_BACKUP_BRANCH', remoteBackupBranch]);
      expectedFields.push(['INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH', remoteBackupBranch]);
    } else {
      expectedFields.push(['WP_VALIDATOR_REMOTE_BACKUP_BRANCH', defaultWpValidatorBranch(WP_ID)]);
      expectedFields.push(['INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH', defaultIntegrationValidatorBranch(WP_ID)]);
    }

    for (const [label, expected] of expectedFields) {
      const actual = parseSingleField(packetContent, label);
      if (actual !== expected) {
        errors.push(`${label} missing/invalid for packets with PACKET_FORMAT_VERSION >= 2026-03-12 (expected ${expected}; got: ${actual || '<missing>'})`);
      }
    }

    const validatorBackupUrl = parseSingleField(packetContent, 'WP_VALIDATOR_REMOTE_BACKUP_URL');
    if (!remoteBackupUrl) {
      errors.push('REMOTE_BACKUP_URL missing/invalid for packets with PACKET_FORMAT_VERSION >= 2026-03-12');
    } else if (packetUsesSharedRemoteWpBackup(packetFormatVersion)) {
      if (validatorBackupUrl !== remoteBackupUrl) {
        errors.push(`WP_VALIDATOR_REMOTE_BACKUP_URL must mirror REMOTE_BACKUP_URL (${remoteBackupUrl}; got: ${validatorBackupUrl || '<missing>'})`);
      }
    } else {
      const legacyWpValidatorRemoteBackupUrl = buildRemoteBackupUrl(remoteBackupUrl.replace(/\/tree\/.*$/, ''), defaultWpValidatorBranch(WP_ID));
      if (validatorBackupUrl !== legacyWpValidatorRemoteBackupUrl) {
        errors.push(`WP_VALIDATOR_REMOTE_BACKUP_URL must remain ${legacyWpValidatorRemoteBackupUrl} for packets with PACKET_FORMAT_VERSION < 2026-03-16 (got: ${validatorBackupUrl || '<missing>'})`);
      }
    }

    const integrationBackupUrl = parseSingleField(packetContent, 'INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL');
    if (!remoteBackupUrl) {
      // Keep a single missing-global-field error above; no extra URL rule needed.
    } else if (packetUsesSharedRemoteWpBackup(packetFormatVersion)) {
      if (integrationBackupUrl !== remoteBackupUrl) {
        errors.push(`INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL must mirror REMOTE_BACKUP_URL (${remoteBackupUrl}; got: ${integrationBackupUrl || '<missing>'})`);
      }
    } else {
      const legacyIntegrationRemoteBackupUrl = buildRemoteBackupUrl(remoteBackupUrl.replace(/\/tree\/.*$/, ''), defaultIntegrationValidatorBranch(WP_ID));
      if (integrationBackupUrl !== legacyIntegrationRemoteBackupUrl) {
        errors.push(`INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL must remain ${legacyIntegrationRemoteBackupUrl} for packets with PACKET_FORMAT_VERSION < 2026-03-16 (got: ${integrationBackupUrl || '<missing>'})`);
      }
    }
  }

  // Check 2.6C: UI/UX rubric + stub tracking (enforced for packet format >= 2026-03-06)
  if (isModernPacket && requiresRefinementGate && isVersionAtLeast(packetFormatVersion, '2026-03-06')) {
    console.log('\nCheck 2.6C: UI/UX rubric + stub tracking');

    const uiApplicable = parseSingleField(packetContent, 'UI_UX_APPLICABLE');
    if (looksPlaceholder(uiApplicable) || !/^(YES|NO)$/i.test(uiApplicable || '')) {
      errors.push('UI_UX_APPLICABLE missing/invalid (expected YES|NO) for packets with PACKET_FORMAT_VERSION >= 2026-03-06');
    }

    const uiVerdict = parseSingleField(packetContent, 'UI_UX_VERDICT');
    if (looksPlaceholder(uiVerdict) || !/^(OK|NEEDS_STUBS|UNKNOWN)$/i.test(uiVerdict || '')) {
      errors.push('UI_UX_VERDICT missing/invalid (expected OK|NEEDS_STUBS|UNKNOWN) for packets with PACKET_FORMAT_VERSION >= 2026-03-06');
    }

    const stubWpIdsRaw = parseSingleField(packetContent, 'STUB_WP_IDS');
    if (looksPlaceholder(stubWpIdsRaw)) {
      errors.push('STUB_WP_IDS must be set (comma-separated WP-... IDs or NONE) for packets with PACKET_FORMAT_VERSION >= 2026-03-06');
    } else if (stubWpIdsRaw && !/^NONE$/i.test(stubWpIdsRaw)) {
      const ids = stubWpIdsRaw.split(',').map((s) => s.trim()).filter(Boolean);
      if (ids.length === 0) {
        errors.push('STUB_WP_IDS must be NONE or a comma-separated list of WP-... IDs');
      } else {
        for (const id of ids) {
          if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(id)) {
            errors.push(`STUB_WP_IDS contains invalid WP id: ${id}`);
            continue;
          }
          const stubPath = path.join(GOV_ROOT_REPO_REL, 'task_packets', 'stubs', `${id}.md`);
          if (!fs.existsSync(stubPath)) {
            errors.push(`Stub referenced in STUB_WP_IDS does not exist: ${stubPath.replace(/\\/g, '/')}`);
          }
        }
      }
    }

    if (/^YES$/i.test(uiApplicable || '')) {
      if (!/##\s*UI_UX_SPEC\b/i.test(packetContent)) {
        errors.push('UI_UX_APPLICABLE=YES requires ## UI_UX_SPEC in the task packet');
      }

      const surfaces = extractIndentedListAfterLabel(packetContent, 'UI_SURFACES');
      if (surfaces.length === 0 || surfaces.every((s) => /<fill/i.test(s))) {
        errors.push('UI_UX_APPLICABLE=YES requires UI_SURFACES to list at least one concrete surface');
      }

      const controls = extractIndentedListAfterLabel(packetContent, 'UI_CONTROLS (buttons/dropdowns/inputs)');
      if (controls.length === 0) {
        errors.push('UI_UX_APPLICABLE=YES requires UI_CONTROLS to list at least one concrete control');
      } else {
        const anyTooltip = controls.some((s) => /\bTooltip:\b/i.test(s) && !/Tooltip:\s*<fill/i.test(s));
        if (!anyTooltip) {
          errors.push('UI_UX_APPLICABLE=YES requires UI_CONTROLS entries to include concrete Tooltip: text');
        }
      }
    }
  }

  if (isModernPacket && isVersionAtLeast(packetFormatVersion, '2026-03-15') && requiresRefinementGate && !usesClauseClosureMonitor) {
    errors.push('CLAUSE_CLOSURE_MONITOR_PROFILE missing/invalid for active packets with PACKET_FORMAT_VERSION >= 2026-03-15 (expected CLAUSE_MONITOR_V1)');
  }

  if (usesClauseClosureMonitor) {
    console.log('\nCheck 2.6D: Clause closure monitoring');
    const closureMonitorValidation = validatePacketClosureMonitoring(packetContent, {
      requireRows: true,
    });
    for (const error of closureMonitorValidation.errors) {
      errors.push(error);
    }
  }

  if (isModernPacket && isVersionAtLeast(packetFormatVersion, '2026-03-16') && requiresRefinementGate && !usesSemanticProofProfile) {
    errors.push('SEMANTIC_PROOF_PROFILE missing/invalid for active packets with PACKET_FORMAT_VERSION >= 2026-03-16 (expected DIFF_SCOPED_SEMANTIC_V1)');
  }

  if (usesSemanticProofProfile) {
    console.log('\nCheck 2.6E: Semantic proof assets');
    const semanticProofValidation = validateSemanticProofAssets(packetContent);
    for (const error of semanticProofValidation.errors) {
      errors.push(error);
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
      const auditPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'records', 'SIGNATURE_AUDIT.md');
      const audit = fs.readFileSync(auditPath, 'utf8');
      if (packetSig && !audit.includes(`| ${packetSig} |`)) {
        errors.push(`USER_SIGNATURE not found in ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md (${packetSig})`);
      }
    } catch {
      warnings.push(`Could not verify signature against ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md`);
    }

    const refinementProfile = parseSingleField(packetContent, 'REFINEMENT_ENFORCEMENT_PROFILE');
    const packetHydrationProfile = parseSingleField(packetContent, 'PACKET_HYDRATION_PROFILE');
    const isHydratedPacket = /^HYDRATED_RESEARCH_V1$/i.test(packetHydrationProfile || '');

    if (isHydratedPacket) {
      console.log('Check 2.7A: Packet/refinement hydration drift');

      if (!/^HYDRATED_RESEARCH_V1$/i.test(refinementProfile || '')) {
        errors.push('PACKET_HYDRATION_PROFILE=HYDRATED_RESEARCH_V1 requires REFINEMENT_ENFORCEMENT_PROFILE=HYDRATED_RESEARCH_V1 in the task packet');
      }
      if (!/^HYDRATED_RESEARCH_V1$/i.test(refinementValidation?.parsed?.refinementEnforcementProfile || '')) {
        errors.push('PACKET_HYDRATION_PROFILE=HYDRATED_RESEARCH_V1 requires the signed refinement to use REFINEMENT_ENFORCEMENT_PROFILE=HYDRATED_RESEARCH_V1');
      }

      const refinementData = refinementValidation?.parsed || {};
      const hasRefinementHandoffSections = isVersionAtLeast(refinementData.refinementFormatVersion, '2026-03-15');
      const packetResearchRequired = parseSingleField(packetContent, 'RESEARCH_CURRENCY_REQUIRED');
      const packetResearchVerdict = parseSingleField(packetContent, 'RESEARCH_CURRENCY_VERDICT');
      const packetResearchDepthVerdict = parseSingleField(packetContent, 'RESEARCH_DEPTH_VERDICT');
      const packetGitHubProjectScoutingVerdict = parseSingleField(packetContent, 'GITHUB_PROJECT_SCOUTING_VERDICT');
      const packetResearchSources = extractIndentedListAfterLabel(packetContent, 'SOURCE_LOG');
      const packetResearchSynthesis = extractIndentedListAfterLabel(packetContent, 'RESEARCH_SYNTHESIS');
      const packetGitHubProjectDecisions = extractIndentedListAfterLabel(packetContent, 'GITHUB_PROJECT_DECISIONS');
      const packetHasMatrixResearchSection = /##\s+MATRIX_RESEARCH_RUBRIC\b/i.test(packetContent);
      const packetMatrixResearchRequired = parseSingleField(packetContent, 'MATRIX_RESEARCH_REQUIRED');
      const packetMatrixResearchVerdict = parseSingleField(packetContent, 'MATRIX_RESEARCH_VERDICT');
      const packetMatrixSourceScanDecisions = extractIndentedListAfterLabel(packetContent, 'SOURCE_SCAN_DECISIONS');
      const packetMatrixGrowthCandidates = extractIndentedListAfterLabel(packetContent, 'MATRIX_GROWTH_CANDIDATES');
      const packetMatrixEngineeringTricks = extractIndentedListAfterLabel(packetContent, 'ENGINEERING_TRICKS_CARRIED_OVER');
      const packetPrimitivesTouched = extractIndentedListAfterLabel(packetContent, 'PRIMITIVES_TOUCHED');
      const packetPrimitivesExposed = extractIndentedListAfterLabel(packetContent, 'PRIMITIVES_EXPOSED');
      const packetPrimitivesCreated = extractIndentedListAfterLabel(packetContent, 'PRIMITIVES_CREATED');
      const packetMechanicalEnginesTouched = extractIndentedListAfterLabel(packetContent, 'MECHANICAL_ENGINES_TOUCHED');
      const packetFeatureRegistryAction = parseSingleField(packetContent, 'FEATURE_REGISTRY_ACTION');
      const packetUiGuidanceAction = parseSingleField(packetContent, 'UI_GUIDANCE_ACTION');
      const packetInteractionMatrixAction = parseSingleField(packetContent, 'INTERACTION_MATRIX_ACTION');
      const packetAppendixVerdict = parseSingleField(packetContent, 'APPENDIX_MAINTENANCE_VERDICT');
      const packetPillarVerdict = parseSingleField(packetContent, 'PILLAR_ALIGNMENT_VERDICT');
      const packetPillarsTouched = extractIndentedListAfterLabel(packetContent, 'PILLARS_TOUCHED');
      const packetPillarsRequiringStubs = extractIndentedListAfterLabel(packetContent, 'PILLARS_REQUIRING_STUBS');
      const packetPrimitiveMatrixVerdict = parseSingleField(packetContent, 'PRIMITIVE_MATRIX_VERDICT');
      const packetForceMultiplierVerdict = parseSingleField(packetContent, 'FORCE_MULTIPLIER_VERDICT');
      const packetForceMultiplierResolutions = extractIndentedListAfterLabel(packetContent, 'FORCE_MULTIPLIER_RESOLUTIONS');
      const packetPillarDecompositionVerdict = parseSingleField(packetContent, 'PILLAR_DECOMPOSITION_VERDICT');
      const packetDecompositionRows = extractIndentedListAfterLabel(packetContent, 'DECOMPOSITION_ROWS');
      const packetExecutionRuntimeAlignmentVerdict = parseSingleField(packetContent, 'EXECUTION_RUNTIME_ALIGNMENT_VERDICT');
      const packetAlignmentRows = extractIndentedListAfterLabel(packetContent, 'ALIGNMENT_ROWS');
      const packetExistingCapabilityAlignmentVerdict = parseSingleField(packetContent, 'EXISTING_CAPABILITY_ALIGNMENT_VERDICT');
      const packetMatchedArtifactResolutions = extractIndentedListAfterLabel(packetContent, 'MATCHED_ARTIFACT_RESOLUTIONS');
      const packetCodeRealitySummary = extractIndentedListAfterLabel(packetContent, 'CODE_REALITY_SUMMARY');
      const packetHasGuiAdviceSection = /##\s+GUI_IMPLEMENTATION_ADVICE\b/i.test(packetContent);
      const packetGuiAdviceRequired = parseSingleField(packetContent, 'GUI_ADVICE_REQUIRED');
      const packetGuiAdviceVerdict = parseSingleField(packetContent, 'GUI_IMPLEMENTATION_ADVICE_VERDICT');
      const packetGuiReferenceDecisions = extractIndentedListAfterLabel(packetContent, 'GUI_REFERENCE_DECISIONS');
      const packetHandshakeGuiAdvice = extractIndentedListAfterLabel(packetContent, 'HANDSHAKE_GUI_ADVICE');
      const packetHiddenGuiRequirements = extractIndentedListAfterLabel(packetContent, 'HIDDEN_GUI_REQUIREMENTS');
      const packetGuiEngineeringTricks = extractIndentedListAfterLabel(packetContent, 'GUI_ENGINEERING_TRICKS_TO_CARRY');
      const packetScopeWhat = parseSingleField(packetContent, 'What');
      const packetScopeWhy = parseSingleField(packetContent, 'Why');
      const packetRequestor = parseSingleField(packetContent, 'REQUESTOR');
      const packetAgentId = parseSingleField(packetContent, 'AGENT_ID');
      const packetRiskTier2 = parseSingleField(packetContent, 'RISK_TIER');
      const packetSpecAddMarkerTarget = parseSingleField(packetContent, 'SPEC_ADD_MARKER_TARGET');
      const packetBuildOrderDomain = parseSingleField(packetContent, 'BUILD_ORDER_DOMAIN');
      const packetBuildOrderTechBlocker = parseSingleField(packetContent, 'BUILD_ORDER_TECH_BLOCKER');
      const packetBuildOrderValueTier = parseSingleField(packetContent, 'BUILD_ORDER_VALUE_TIER');
      const packetBuildOrderDependsOn = parseSingleField(packetContent, 'BUILD_ORDER_DEPENDS_ON');
      const packetBuildOrderBlocks = parseSingleField(packetContent, 'BUILD_ORDER_BLOCKS');
      const packetInScope = extractIndentedListAfterLabel(packetContent, 'IN_SCOPE_PATHS', { stopLabels: ['OUT_OF_SCOPE'] });
      const packetOutOfScope = extractIndentedListAfterLabel(packetContent, 'OUT_OF_SCOPE');
      const packetTestPlan = extractFencedBlockAfterHeading(packetContent, 'TEST_PLAN');
      const packetDoneMeans = extractBulletListAfterHeading(packetContent, 'DONE_MEANS');
      const packetSpecAnchor =
        parseSingleField(packetContent, 'SPEC_ANCHOR_PRIMARY')
        || parseSingleField(packetContent, 'SPEC_ANCHOR');
      const packetFilesToOpen = extractIndentedListAfterLabel(packetContent, 'FILES_TO_OPEN', { stopLabels: ['SEARCH_TERMS'] });
      const packetSearchTerms = extractIndentedListAfterLabel(packetContent, 'SEARCH_TERMS', { stopLabels: ['RUN_COMMANDS'] });
      const packetRunCommands = extractFencedBlockAfterLabel(packetContent, 'RUN_COMMANDS');
      const packetRiskMap = extractIndentedListAfterLabel(packetContent, 'RISK_MAP');
      const packetUiSurfaces = extractIndentedListAfterLabel(packetContent, 'UI_SURFACES');
      const packetUiControls = extractIndentedListAfterLabel(packetContent, 'UI_CONTROLS (buttons/dropdowns/inputs)');
      const packetUiStates = extractIndentedListAfterLabel(packetContent, 'UI_STATES (empty/loading/error)');
      const packetUiMicrocopy = extractIndentedListAfterLabel(packetContent, 'UI_MICROCOPY_NOTES (labels, helper text, hover explainers)');
      const packetUiAccessibility = extractIndentedListAfterLabel(packetContent, 'UI_ACCESSIBILITY_NOTES');

      const expectedSourceLog = (refinementData.researchSources || []).map((source) => {
        const kind = (source.kind || '').trim() || 'UNKNOWN';
        const title = (source.source || '').trim() || '<missing>';
        const date = (source.date || '').trim() || '<missing>';
        const retrievedAt = (source.retrievedAt || '').trim() || '<missing>';
        const url = (source.url || '').trim() || '<missing>';
        const why = (source.why || '').trim() || '<missing>';
        return `[${kind}] ${title} | ${date} | Retrieved: ${retrievedAt} | ${url} | Why: ${why}`;
      });

      if ((packetResearchRequired || '').toUpperCase() !== (refinementData.researchCurrencyRequired || '').toUpperCase()) {
        errors.push('RESEARCH_CURRENCY_REQUIRED in the packet drifted from the signed refinement');
      }
      if ((packetResearchVerdict || '').toUpperCase() !== (refinementData.researchCurrencyVerdict || '').toUpperCase()) {
        errors.push('RESEARCH_CURRENCY_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetResearchDepthVerdict || '').toUpperCase() !== (refinementData.researchDepthVerdict || '').toUpperCase()) {
        errors.push('RESEARCH_DEPTH_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetGitHubProjectScoutingVerdict || '').toUpperCase() !== (refinementData.githubProjectScoutingVerdict || '').toUpperCase()) {
        errors.push('GITHUB_PROJECT_SCOUTING_VERDICT in the packet drifted from the signed refinement');
      }
      if (!sameList(packetResearchSources, expectedSourceLog)) {
        errors.push('RESEARCH_SIGNAL SOURCE_LOG in the packet drifted from the signed refinement');
      }
      if (!sameList(packetResearchSynthesis, refinementData.researchSynthesis || [])) {
        errors.push('RESEARCH_SIGNAL RESEARCH_SYNTHESIS in the packet drifted from the signed refinement');
      }
      if (!sameList(packetGitHubProjectDecisions, refinementData.githubProjectDecisions || [])) {
        errors.push('GITHUB_PROJECT_DECISIONS in the packet drifted from the signed refinement');
      }
      if (packetHasMatrixResearchSection || refinementData.matrixResearchRequired || refinementData.matrixResearchVerdict) {
        if ((packetMatrixResearchRequired || '').toUpperCase() !== (refinementData.matrixResearchRequired || '').toUpperCase()) {
          errors.push('MATRIX_RESEARCH_REQUIRED in the packet drifted from the signed refinement');
        }
        if ((packetMatrixResearchVerdict || '').toUpperCase() !== (refinementData.matrixResearchVerdict || '').toUpperCase()) {
          errors.push('MATRIX_RESEARCH_VERDICT in the packet drifted from the signed refinement');
        }
        if (!sameList(packetMatrixSourceScanDecisions, refinementData.matrixResearchSourceDecisions || [])) {
          errors.push('SOURCE_SCAN_DECISIONS in the packet drifted from the signed refinement');
        }
        if (!sameList(packetMatrixGrowthCandidates, refinementData.matrixGrowthCandidates || [])) {
          errors.push('MATRIX_GROWTH_CANDIDATES in the packet drifted from the signed refinement');
        }
        if (!sameList(packetMatrixEngineeringTricks, refinementData.matrixResearchEngineeringTricks || [])) {
          errors.push('ENGINEERING_TRICKS_CARRIED_OVER in the packet drifted from the signed refinement');
        }
      }
      if (!sameList(packetPrimitivesTouched, refinementData.primitivesTouched || [])) {
        errors.push('PRIMITIVES_TOUCHED in the packet drifted from the signed refinement');
      }
      if (!sameList(packetPrimitivesExposed, refinementData.primitivesExposed || [])) {
        errors.push('PRIMITIVES_EXPOSED in the packet drifted from the signed refinement');
      }
      if (!sameList(packetPrimitivesCreated, refinementData.primitivesCreated || [])) {
        errors.push('PRIMITIVES_CREATED in the packet drifted from the signed refinement');
      }
      if (!sameList(packetMechanicalEnginesTouched, refinementData.mechanicalEnginesTouched || [])) {
        errors.push('MECHANICAL_ENGINES_TOUCHED in the packet drifted from the signed refinement');
      }
      if ((packetFeatureRegistryAction || '').toUpperCase() !== (refinementData.featureRegistryAction || '').toUpperCase()) {
        errors.push('FEATURE_REGISTRY_ACTION in the packet drifted from the signed refinement');
      }
      if ((packetUiGuidanceAction || '').toUpperCase() !== (refinementData.uiGuidanceAction || '').toUpperCase()) {
        errors.push('UI_GUIDANCE_ACTION in the packet drifted from the signed refinement');
      }
      if ((packetInteractionMatrixAction || '').toUpperCase() !== (refinementData.interactionMatrixAction || '').toUpperCase()) {
        errors.push('INTERACTION_MATRIX_ACTION in the packet drifted from the signed refinement');
      }
      if ((packetAppendixVerdict || '').toUpperCase() !== (refinementData.appendixMaintenanceVerdict || '').toUpperCase()) {
        errors.push('APPENDIX_MAINTENANCE_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetPillarVerdict || '').toUpperCase() !== (refinementData.pillarAlignmentVerdict || '').toUpperCase()) {
        errors.push('PILLAR_ALIGNMENT_VERDICT in the packet drifted from the signed refinement');
      }
      if (!sameList(packetPillarsTouched, refinementData.pillarsTouched || [])) {
        errors.push('PILLARS_TOUCHED in the packet drifted from the signed refinement');
      }
      if (!sameList(packetPillarsRequiringStubs, refinementData.pillarsRequiringStubs || [])) {
        errors.push('PILLARS_REQUIRING_STUBS in the packet drifted from the signed refinement');
      }
      if ((packetPrimitiveMatrixVerdict || '').toUpperCase() !== (refinementData.primitiveMatrixVerdict || '').toUpperCase()) {
        errors.push('PRIMITIVE_MATRIX_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetForceMultiplierVerdict || '').toUpperCase() !== (refinementData.forceMultiplierVerdict || '').toUpperCase()) {
        errors.push('FORCE_MULTIPLIER_VERDICT in the packet drifted from the signed refinement');
      }
      if (!sameList(packetForceMultiplierResolutions, refinementData.forceMultiplierResolutions || [])) {
        errors.push('FORCE_MULTIPLIER_RESOLUTIONS in the packet drifted from the signed refinement');
      }
      if ((packetPillarDecompositionVerdict || '').toUpperCase() !== (refinementData.pillarDecompositionVerdict || '').toUpperCase()) {
        errors.push('PILLAR_DECOMPOSITION_VERDICT in the packet drifted from the signed refinement');
      }
      if (!sameList(packetDecompositionRows, refinementData.pillarDecompositionRows || [])) {
        errors.push('DECOMPOSITION_ROWS in the packet drifted from the signed refinement');
      }
      if ((packetExecutionRuntimeAlignmentVerdict || '').toUpperCase() !== (refinementData.executionRuntimeAlignmentVerdict || '').toUpperCase()) {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT_VERDICT in the packet drifted from the signed refinement');
      }
      if (!sameList(packetAlignmentRows, refinementData.executionRuntimeAlignmentRows || [])) {
        errors.push('ALIGNMENT_ROWS in the packet drifted from the signed refinement');
      }
      if ((packetExistingCapabilityAlignmentVerdict || '').toUpperCase() !== (refinementData.existingCapabilityAlignmentVerdict || '').toUpperCase()) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetExistingCapabilityAlignmentVerdict || '').toUpperCase() === 'REUSE_EXISTING') {
        errors.push('PACKET_HYDRATION_PROFILE=HYDRATED_RESEARCH_V1 cannot proceed when EXISTING_CAPABILITY_ALIGNMENT_VERDICT=REUSE_EXISTING; reuse the matched artifact instead of starting duplicate work');
      }
      if (!sameList(packetMatchedArtifactResolutions, refinementData.matchedArtifactResolutions || [])) {
        errors.push('MATCHED_ARTIFACT_RESOLUTIONS in the packet drifted from the signed refinement');
      }
      if (!sameList(packetCodeRealitySummary, refinementData.codeRealitySummary || [])) {
        errors.push('CODE_REALITY_SUMMARY in the packet drifted from the signed refinement');
      }
      if (packetHasGuiAdviceSection || refinementData.guiAdviceRequired || refinementData.guiImplementationAdviceVerdict) {
        if ((packetGuiAdviceRequired || '').toUpperCase() !== (refinementData.guiAdviceRequired || '').toUpperCase()) {
          errors.push('GUI_ADVICE_REQUIRED in the packet drifted from the signed refinement');
        }
        if ((packetGuiAdviceVerdict || '').toUpperCase() !== (refinementData.guiImplementationAdviceVerdict || '').toUpperCase()) {
          errors.push('GUI_IMPLEMENTATION_ADVICE_VERDICT in the packet drifted from the signed refinement');
        }
        if (!sameList(packetGuiReferenceDecisions, refinementData.guiReferenceDecisions || [])) {
          errors.push('GUI_REFERENCE_DECISIONS in the packet drifted from the signed refinement');
        }
        if (!sameList(packetHandshakeGuiAdvice, refinementData.handshakeGuiAdvice || [])) {
          errors.push('HANDSHAKE_GUI_ADVICE in the packet drifted from the signed refinement');
        }
        if (!sameList(packetHiddenGuiRequirements, refinementData.hiddenGuiRequirements || [])) {
          errors.push('HIDDEN_GUI_REQUIREMENTS in the packet drifted from the signed refinement');
        }
        if (!sameList(packetGuiEngineeringTricks, refinementData.guiEngineeringTricks || [])) {
          errors.push('GUI_ENGINEERING_TRICKS_TO_CARRY in the packet drifted from the signed refinement');
        }
      }

      const packetUiApplicable = parseSingleField(packetContent, 'UI_UX_APPLICABLE');
      const packetUiVerdict2 = parseSingleField(packetContent, 'UI_UX_VERDICT');
      const packetStubWpIds2 = parseSingleField(packetContent, 'STUB_WP_IDS');
      if ((packetUiApplicable || '').toUpperCase() !== (refinementData.uiApplicable || '').toUpperCase()) {
        errors.push('UI_UX_APPLICABLE in the packet drifted from the signed refinement');
      }
      if ((packetUiVerdict2 || '').toUpperCase() !== (refinementData.uiVerdict || '').toUpperCase()) {
        errors.push('UI_UX_VERDICT in the packet drifted from the signed refinement');
      }
      if ((packetStubWpIds2 || '').trim() !== (refinementData.stubWpIdsRaw || '').trim()) {
        errors.push('STUB_WP_IDS in the packet drifted from the signed refinement');
      }

      const hydration = refinementData.packetHydration || {};
      if ((packetRequestor || '').trim() !== (hydration.requestor || '').trim()) errors.push('REQUESTOR in the packet drifted from the signed refinement');
      if ((packetAgentId || '').trim() !== (hydration.agentId || '').trim()) errors.push('AGENT_ID in the packet drifted from the signed refinement');
      if ((packetRiskTier2 || '').toUpperCase() !== (hydration.riskTier || '').toUpperCase()) errors.push('RISK_TIER in the packet drifted from the signed refinement');
      if ((packetSpecAddMarkerTarget || '').trim() !== (hydration.specAddMarkerTarget || '').trim()) errors.push('SPEC_ADD_MARKER_TARGET in the packet drifted from the signed refinement');
      if ((packetBuildOrderDomain || '').toUpperCase() !== (hydration.buildOrderDomain || '').toUpperCase()) errors.push('BUILD_ORDER_DOMAIN in the packet drifted from the signed refinement');
      if ((packetBuildOrderTechBlocker || '').toUpperCase() !== (hydration.buildOrderTechBlocker || '').toUpperCase()) errors.push('BUILD_ORDER_TECH_BLOCKER in the packet drifted from the signed refinement');
      if ((packetBuildOrderValueTier || '').toUpperCase() !== (hydration.buildOrderValueTier || '').toUpperCase()) errors.push('BUILD_ORDER_VALUE_TIER in the packet drifted from the signed refinement');
      if ((packetBuildOrderDependsOn || '').trim() !== (hydration.buildOrderDependsOnRaw || '').trim()) errors.push('BUILD_ORDER_DEPENDS_ON in the packet drifted from the signed refinement');
      if ((packetBuildOrderBlocks || '').trim() !== (hydration.buildOrderBlocksRaw || '').trim()) errors.push('BUILD_ORDER_BLOCKS in the packet drifted from the signed refinement');
      if ((packetScopeWhat || '').trim() !== (hydration.what || '').trim()) errors.push('SCOPE What in the packet drifted from the signed refinement');
      if ((packetScopeWhy || '').trim() !== (hydration.why || '').trim()) errors.push('SCOPE Why in the packet drifted from the signed refinement');
      if (!sameList(packetInScope, hydration.inScopePaths || [])) errors.push('IN_SCOPE_PATHS in the packet drifted from the signed refinement');
      if (!sameList(packetOutOfScope, hydration.outOfScope || [])) errors.push('OUT_OF_SCOPE in the packet drifted from the signed refinement');
      if (normalizeBlock(packetTestPlan) !== normalizeBlock(hydration.testPlan || '')) errors.push('TEST_PLAN in the packet drifted from the signed refinement');
      if (!sameList(packetDoneMeans, hydration.doneMeans || [])) errors.push('DONE_MEANS in the packet drifted from the signed refinement');
      if (!sameList(packetPrimitivesExposed, hydration.primitivesExposed || [])) errors.push('QUALITY_GATE PRIMITIVES_EXPOSED in the packet drifted from the signed refinement');
      if (!sameList(packetPrimitivesCreated, hydration.primitivesCreated || [])) errors.push('QUALITY_GATE PRIMITIVES_CREATED in the packet drifted from the signed refinement');
      if ((packetSpecAnchor || '').trim() !== (hydration.specAnchorPrimary || '').trim()) errors.push('SPEC_ANCHOR_PRIMARY in the packet drifted from the signed refinement');
      if (!sameList(packetFilesToOpen, hydration.filesToOpen || [])) errors.push('BOOTSTRAP FILES_TO_OPEN in the packet drifted from the signed refinement');
      if (!sameList(packetSearchTerms, hydration.searchTerms || [])) errors.push('BOOTSTRAP SEARCH_TERMS in the packet drifted from the signed refinement');
      if (normalizeBlock(packetRunCommands) !== normalizeBlock(hydration.runCommands || '')) errors.push('BOOTSTRAP RUN_COMMANDS in the packet drifted from the signed refinement');
      if (!sameList(packetRiskMap, hydration.riskMap || [])) errors.push('BOOTSTRAP RISK_MAP in the packet drifted from the signed refinement');

      const packetSpecContextWindows = extractSectionBlock(packetContent, 'SPEC_CONTEXT_WINDOWS');
      const expectedSpecContextWindows = formatSpecContextWindowsSection(refinementData.specAnchors);
      if (normalizeBlock(packetSpecContextWindows) !== normalizeBlock(expectedSpecContextWindows)) {
        errors.push('SPEC_CONTEXT_WINDOWS in the packet drifted from the signed refinement');
      }

      const packetSemanticProofAssets = extractSectionBlock(packetContent, 'SEMANTIC_PROOF_ASSETS');
      const expectedSemanticProofAssets = formatSemanticProofAssetsSection(deriveSemanticProofAssets({
        semanticTripwireTests: refinementData.semanticTripwireTests,
        canonicalContractExamples: refinementData.canonicalContractExamples,
        testPlan: hydration.testPlan,
        doneMeans: hydration.doneMeans,
        specAnchors: refinementData.specAnchors,
      }));
      if (normalizeBlock(packetSemanticProofAssets) !== normalizeBlock(expectedSemanticProofAssets)) {
        errors.push('SEMANTIC_PROOF_ASSETS in the packet drifted from the signed refinement');
      }

      if (hasRefinementHandoffSections) {
        const packetClauseProofPlan = extractSectionBlock(packetContent, 'CLAUSE_PROOF_PLAN');
        const packetContractSurfaces = extractSectionBlock(packetContent, 'CONTRACT_SURFACES');
        const packetCoderHandoffBrief = extractSectionBlock(packetContent, 'CODER_HANDOFF_BRIEF');
        const packetValidatorHandoffBrief = extractSectionBlock(packetContent, 'VALIDATOR_HANDOFF_BRIEF');
        const packetNotProvenAtRefinementTime = extractSectionBlock(packetContent, 'NOT_PROVEN_AT_REFINEMENT_TIME');

        const expectedClauseProofPlan = formatClauseProofPlanSection(refinementData.clauseProofPlan);
        const expectedContractSurfaces = formatContractSurfacesSection(refinementData.contractSurfaces);
        const expectedCoderHandoffBrief = formatCoderHandoffBriefSection({
          implementationOrder: refinementData.coderImplementationOrder,
          hotFiles: refinementData.coderHotFiles,
          tripwireTests: refinementData.coderTripwireTests,
          carryForwardWarnings: refinementData.coderCarryForwardWarnings,
        });
        const expectedValidatorHandoffBrief = formatValidatorHandoffBriefSection({
          clausesToInspect: refinementData.validatorClausesToInspect,
          filesToRead: refinementData.validatorFilesToRead,
          commandsToRun: refinementData.validatorCommandsToRun,
          postMergeSpotchecks: refinementData.validatorPostMergeSpotchecks,
        });
        const expectedNotProvenAtRefinementTime = formatNotProvenAtRefinementTimeSection(refinementData.refinementNotProven);

        if (normalizeBlock(packetClauseProofPlan) !== normalizeBlock(expectedClauseProofPlan)) {
          errors.push('CLAUSE_PROOF_PLAN in the packet drifted from the signed refinement');
        }
        if (normalizeBlock(packetContractSurfaces) !== normalizeBlock(expectedContractSurfaces)) {
          errors.push('CONTRACT_SURFACES in the packet drifted from the signed refinement');
        }
        if (normalizeBlock(packetCoderHandoffBrief) !== normalizeBlock(expectedCoderHandoffBrief)) {
          errors.push('CODER_HANDOFF_BRIEF in the packet drifted from the signed refinement');
        }
        if (normalizeBlock(packetValidatorHandoffBrief) !== normalizeBlock(expectedValidatorHandoffBrief)) {
          errors.push('VALIDATOR_HANDOFF_BRIEF in the packet drifted from the signed refinement');
        }
        if (normalizeBlock(packetNotProvenAtRefinementTime) !== normalizeBlock(expectedNotProvenAtRefinementTime)) {
          errors.push('NOT_PROVEN_AT_REFINEMENT_TIME in the packet drifted from the signed refinement');
        }
      }

      if (/^YES$/i.test(refinementData.uiApplicable || '')) {
        if (!sameList(packetUiSurfaces, refinementData.uiSpec?.surfaces || [])) errors.push('UI_SURFACES in the packet drifted from the signed refinement');
        if (!sameList(packetUiControls, refinementData.uiSpec?.controls || [])) errors.push('UI_CONTROLS in the packet drifted from the signed refinement');
        if (!sameList(packetUiStates, refinementData.uiSpec?.states || [])) errors.push('UI_STATES in the packet drifted from the signed refinement');
        if (!sameList(packetUiMicrocopy, refinementData.uiSpec?.microcopy || [])) errors.push('UI_MICROCOPY_NOTES in the packet drifted from the signed refinement');
        if (!sameList(packetUiAccessibility, refinementData.uiSpec?.accessibility || [])) errors.push('UI_ACCESSIBILITY_NOTES in the packet drifted from the signed refinement');
      }

      const forbiddenPlaceholders = [
        '{user or source}',
        '{orchestrator agent ID}',
        'path/to/file',
        'measurable criterion 1',
        'measurable criterion 2',
        '# ...task-specific commands...',
        '<fill>',
      ];
      for (const token of forbiddenPlaceholders) {
        if (packetContent.includes(token)) {
          errors.push(`HYDRATED_RESEARCH_V1 packets must not contain placeholder token: ${token}`);
        }
      }
    }

    // Safety checkpoint gate: packet + refinement must be committed before development starts.
    // This prevents untracked/uncommitted WP artifacts from being lost during accidental clean/reset operations.
    console.log('\nCheck 2.8: WP checkpoint commit gate');
    try {
      execSync(`git cat-file -e HEAD:${packetPath.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Task packet is not committed yet (checkpoint required): ${packetPath.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md ${GOV_ROOT_REPO_REL}/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
    }

    try {
      execSync(`git cat-file -e HEAD:${refinementFile.replace(/\\/g, '/')}`, { stdio: 'ignore' });
    } catch {
      errors.push(`Refinement file is not committed yet (checkpoint required): ${refinementFile.replace(/\\/g, '/')}`);
      errors.push(`Commit it on the WP branch before handoff (example): git add ${packetPath.replace(/\\/g, '/')} ${refinementFile.replace(/\\/g, '/')} ${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md ${GOV_ROOT_REPO_REL}/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json && git commit -m "docs: checkpoint packet+refinement [${WP_ID}]"`);
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
  const statusNorm = status.toLowerCase().replace(/[-_]/g, ' ');
  const startAllowed = /ready\s*for\s*dev|in\s*progress/.test(statusNorm) && !/blocked|stub|done|validated/.test(statusNorm);

  console.log('');
  if (startAllowed) {
    console.log('You may proceed with the workflow (SKELETON -> approval -> implementation).');
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
      console.log(`- Sub-agents MUST NOT edit \`${GOV_ROOT_REPO_REL}/**\` (including task packets/refinements or \`## VALIDATION_REPORTS\`).`);
      console.log('- Sub-agents MUST NOT run gates/commits/branch ops as official evidence.');
      console.log(`- Follow: \`/${GOV_ROOT_REPO_REL}/roles/coder/agentic/AGENTIC_PROTOCOL.md\` Section 6.`);
    }

    console.log('\nCODER_START_COMMANDS [CX-HANDOFF-001]');
    console.log('```bash');
    console.log(`# Re-validate WP gates in your environment (also verifies branch/worktree vs PREPARE):`);
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
  console.log(`See: ${GOV_ROOT_REPO_REL}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md or ${GOV_ROOT_REPO_REL}/roles/coder/CODER_PROTOCOL.md`);
  process.exit(1);
}
