import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

const SPEC_CURRENT_PATH = path.join('.GOV', 'roles_shared', 'SPEC_CURRENT.md');
const TASK_BOARD_PATH = path.join('.GOV', 'roles_shared', 'TASK_BOARD.md');

export function resolveSpecCurrent() {
  if (!fs.existsSync(SPEC_CURRENT_PATH)) {
    throw new Error(`Missing ${SPEC_CURRENT_PATH}`);
  }
  const specCurrent = fs.readFileSync(SPEC_CURRENT_PATH, 'utf8');
  const m = specCurrent.match(/Handshake_Master_Spec_v[0-9._]+\.md/);
  if (!m) {
    throw new Error(`Could not resolve spec filename from ${SPEC_CURRENT_PATH}`);
  }
  const specFileName = m[0];
  const specFilePath = path.join(specFileName);
  if (!fs.existsSync(specFilePath)) {
    throw new Error(`Resolved spec file does not exist: ${specFilePath}`);
  }
  const sha1 = crypto.createHash('sha1').update(fs.readFileSync(specFilePath)).digest('hex');
  return { specFileName, specFilePath, sha1 };
}

export function defaultRefinementPath(wpId) {
  return path.join('.GOV', 'refinements', `${wpId}.md`);
}

export function isAsciiOnly(s) {
  return !/[^\x00-\x7F]/.test(s);
}

function getSingleField(content, label) {
  const re = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = content.match(re);
  return m ? m[1].trim() : '';
}

function hasHeading(content, heading) {
  const re = new RegExp(`^#{2,6}\\s+${heading}\\b`, 'mi');
  return re.test(content);
}

function extractFencedBlockAfterLabel(lines, label) {
  const labelIdx = lines.findIndex((l) => new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, 'i').test(l));
  if (labelIdx === -1) return { found: false, body: '' };

  let i = labelIdx + 1;
  while (i < lines.length && lines[i].trim() === '') i += 1;
  if (i >= lines.length) return { found: true, body: '' };

  const fenceStart = lines[i].trim();
  const fenceRe = /^```([a-z0-9_-]+)?\s*$/i;
  const m = fenceStart.match(fenceRe);
  if (!m) return { found: true, body: '' };

  const bodyLines = [];
  i += 1;
  for (; i < lines.length; i += 1) {
    if (lines[i].trim() === '```') break;
    bodyLines.push(lines[i]);
  }
  return { found: true, body: bodyLines.join('\n').trim() };
}

function extractFencedBlockAfterHeading(lines, heading) {
  const headingIdx = lines.findIndex((l) => new RegExp(`^#{2,6}\\s+${heading}\\b`, 'i').test(l));
  if (headingIdx === -1) return { found: false, body: '' };

  // Limit scan to the heading's section (until the next Markdown heading).
  const sectionStart = headingIdx + 1;
  let sectionEnd = lines.length;
  // NOTE: Headings inside fenced code blocks are not real section delimiters.
  // Without this, a fenced block that contains Markdown headings (common when
  // pasting verbatim spec text) would cause premature section termination.
  let inFence = false;
  for (let j = sectionStart; j < lines.length; j += 1) {
    const trimmed = (lines[j] || '').trim();
    if (/^```/.test(trimmed)) {
      inFence = !inFence;
      continue;
    }
    if (!inFence && /^#{1,6}\s+\S/.test(lines[j])) {
      sectionEnd = j;
      break;
    }
  }

  let i = sectionStart;
  while (i < sectionEnd && lines[i].trim() === '') i += 1;
  if (i >= sectionEnd) return { found: true, body: '' };

  // Find the first fence within the section.
  for (; i < sectionEnd; i += 1) {
    const fenceStart = lines[i].trim();
    const fenceRe = /^```([a-z0-9_-]+)?\s*$/i;
    const m = fenceStart.match(fenceRe);
    if (!m) continue;

    const bodyLines = [];
    i += 1;
    for (; i < sectionEnd; i += 1) {
      if (lines[i].trim() === '```') break;
      bodyLines.push(lines[i]);
    }
    return { found: true, body: bodyLines.join('\n').trim() };
  }

  return { found: true, body: '' };
}

function looksLikeNotApplicableBlock(s) {
  const v = (s || '').trim();
  if (!v) return true;
  return /^<not applicable(\s*;\s*ENRICHMENT_NEEDED\s*=\s*NO)?>\s*$/i.test(v);
}

function looksLikePlaceholderEnrichment(s) {
  const v = (s || '').trim();
  if (!v) return true;
  if (/^<paste/i.test(v)) return true;
  if (v.includes('<paste')) return true;
  return false;
}

function parseAnchors(content) {
  const lines = content.split('\n');
  const anchors = [];

  for (let i = 0; i < lines.length; i += 1) {
    if (!/^####\s+ANCHOR\b/i.test(lines[i])) continue;

    const sectionLines = [];
    for (let j = i + 1; j < lines.length; j += 1) {
      if (/^####\s+ANCHOR\b/i.test(lines[j])) break;
      sectionLines.push(lines[j]);
    }
    const section = sectionLines.join('\n');

    const specAnchor = (section.match(/^\s*-\s*SPEC_ANCHOR\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const startStr = (section.match(/^\s*-\s*CONTEXT_START_LINE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const endStr = (section.match(/^\s*-\s*CONTEXT_END_LINE\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
    const contextToken = (section.match(/^\s*-\s*CONTEXT_TOKEN\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';

    // Excerpt is a fenced block after "- EXCERPT_ASCII_ESCAPED:"
    const excerptLines = sectionLines;
    const excerpt = extractFencedBlockAfterLabel(excerptLines, 'EXCERPT_ASCII_ESCAPED').body;

    anchors.push({
      specAnchor,
      contextStartLine: startStr,
      contextEndLine: endStr,
      contextToken,
      excerpt,
    });
  }

  return anchors;
}

function isPlaceholderValue(s) {
  const v = (s || '').trim();
  if (!v) return true;
  if (v === 'PENDING') return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<paste/i.test(v)) return true;
  if (v === '<pending>') return true;
  return false;
}

function escapeRegExp(s) {
  return String(s || '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function isIsoDate(s) {
  return /^\d{4}-\d{2}-\d{2}$/.test(String(s || '').trim());
}

function parseIsoDateUtc(s) {
  const value = String(s || '').trim();
  if (!isIsoDate(value)) return null;
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function isVersionAtLeast(isoDate, minIsoDate) {
  if (!isIsoDate(isoDate) || !isIsoDate(minIsoDate)) return false;
  // ISO date strings are lexicographically comparable.
  return isoDate >= minIsoDate;
}

function isPositiveIntegerString(s) {
  return /^\d+$/.test(String(s || '').trim());
}

function normalizeCsv(value) {
  const raw = String(value || '').trim();
  if (!raw || /^NONE$/i.test(raw)) return [];
  return raw.split(',').map((s) => s.trim()).filter(Boolean);
}

function parsePipeRecord(item) {
  const record = {};
  for (const part of String(item || '').split('|')) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf(':');
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim().toUpperCase().replace(/\s+/g, '_');
    const value = trimmed.slice(idx + 1).trim();
    record[key] = value;
  }
  return record;
}

function parsePillarRubric(lines) {
  const pillars = [];
  const re = /^\s*-\s*PILLAR:\s*(.+?)\s*\|\s*STATUS:\s*(TOUCHED|NOT_TOUCHED|UNKNOWN)\s*\|\s*NOTES:\s*(.+?)\s*\|\s*STUB_WP_IDS:\s*(.+)\s*$/i;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    pillars.push({
      pillar: m[1].trim(),
      status: m[2].trim().toUpperCase(),
      notes: m[3].trim(),
      stubWpIdsRaw: m[4].trim(),
      stubWpIds: normalizeCsv(m[4]),
    });
  }
  return pillars;
}

function validateStubIds(rawValue, errors, label) {
  const raw = String(rawValue || '').trim();
  if (isPlaceholderValue(raw)) {
    errors.push(`${label} must be set (comma-separated WP-... IDs or NONE)`);
    return [];
  }
  if (/^NONE$/i.test(raw)) return [];

  const ids = normalizeCsv(raw);
  if (ids.length === 0) {
    errors.push(`${label} must be NONE or a comma-separated list of WP-... IDs`);
    return [];
  }

  for (const id of ids) {
    if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(id)) {
      errors.push(`${label} contains invalid WP id: ${id}`);
      continue;
    }
    const stubPath = path.join('.GOV', 'task_packets', 'stubs', `${id}.md`);
    if (!fs.existsSync(stubPath)) {
      errors.push(`Stub referenced in ${label} does not exist: ${stubPath.replace(/\\/g, '/')}`);
    }
  }
  return ids;
}

function validateBaseWpIds(rawValue, errors, label) {
  const raw = String(rawValue || '').trim();
  if (isPlaceholderValue(raw)) {
    errors.push(`${label} must be set (comma-separated Base WP IDs or NONE)`);
    return [];
  }
  if (/^NONE$/i.test(raw)) return [];

  const ids = normalizeCsv(raw);
  if (ids.length === 0) {
    errors.push(`${label} must be NONE or a comma-separated list of Base WP IDs`);
    return [];
  }

  for (const id of ids) {
    if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(id)) {
      errors.push(`${label} contains invalid Base WP id: ${id}`);
      continue;
    }
    if (/-v\d+$/i.test(id)) {
      errors.push(`${label} must use Base WP IDs only (no -vN revisions): ${id}`);
    }
  }
  return ids;
}

function extractIndentedListAfterLabel(lines, label) {
  const re = new RegExp(`^\\s*-\\s*${escapeRegExp(label)}\\s*:\\s*$`, 'i');
  const idx = lines.findIndex((l) => re.test(l));
  if (idx === -1) return { found: false, items: [] };

  const items = [];
  for (let i = idx + 1; i < lines.length; i += 1) {
    const line = lines[i] || '';
    if (/^#{1,6}\s+\S/.test(line)) break;
    if (/^\s*-\s+\S/.test(line) && !/^\s{2,}-\s+/.test(line)) break; // next top-level bullet starts
    const m = line.match(/^\s{2,}-\s+(.+)\s*$/);
    if (m) items.push(m[1].trim());
  }
  return { found: true, items };
}

function extractSectionLinesByHeading(lines, heading) {
  const headingRe = new RegExp(`^#{2,6}\\s+${escapeRegExp(heading)}\\b`, 'i');
  const headingIdx = lines.findIndex((l) => headingRe.test(l));
  if (headingIdx === -1) return { found: false, lines: [] };

  const sectionLines = [];
  let inFence = false;
  for (let i = headingIdx + 1; i < lines.length; i += 1) {
    const line = lines[i] || '';
    const trimmed = line.trim();
    if (/^```/.test(trimmed)) {
      inFence = !inFence;
      sectionLines.push(line);
      continue;
    }
    if (!inFence && /^#{1,6}\s+\S/.test(line)) break;
    sectionLines.push(line);
  }
  return { found: true, lines: sectionLines };
}

function extractAppendixJson(specContent, appendixId) {
  const beginNeedle = `<!-- HS_APPENDIX:BEGIN id=${appendixId}`;
  const endNeedle = `<!-- HS_APPENDIX:END id=${appendixId}`;

  const beginIdx = specContent.indexOf(beginNeedle);
  if (beginIdx === -1) {
    return { ok: false, error: `Missing appendix begin marker for ${appendixId}` };
  }

  const endIdx = specContent.indexOf(endNeedle, beginIdx);
  if (endIdx === -1) {
    return { ok: false, error: `Missing appendix end marker for ${appendixId}` };
  }

  const slice = specContent.slice(beginIdx, endIdx);
  const m = slice.match(/```json\s*\r?\n([\s\S]*?)\r?\n```/i);
  if (!m) {
    return { ok: false, error: `Missing JSON fenced block for appendix ${appendixId}` };
  }

  try {
    const json = JSON.parse(m[1]);
    return { ok: true, json };
  } catch (e) {
    return { ok: false, error: `Invalid JSON in appendix ${appendixId}: ${String(e?.message || e)}` };
  }
}

function readTaskBoardStatusMap() {
  if (!fs.existsSync(TASK_BOARD_PATH)) {
    return { ok: false, error: `Missing task board: ${TASK_BOARD_PATH.replace(/\\/g, '/')}` };
  }
  const content = fs.readFileSync(TASK_BOARD_PATH, 'utf8');
  const statuses = new Map();
  const pattern = /^-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[([^\]]+)\]/gm;
  let match = pattern.exec(content);
  while (match) {
    statuses.set(match[1].trim(), match[2].trim().toUpperCase());
    match = pattern.exec(content);
  }
  return { ok: true, statuses };
}

function extractMechanicalEngines(specContent) {
  const engines = [...specContent.matchAll(/#### Engine: ([^\n(]+).*?\n\n- \*\*Engine ID:\*\* `([^`]+)`/gs)]
    .map((m) => ({ title: m[1].trim(), id: m[2].trim() }))
    .filter((entry) => entry.id && entry.title);
  if (engines.length === 0) {
    return { ok: false, error: 'Failed to extract spec-grade mechanical engine set from Master Spec §11.8 / §6.3' };
  }
  return { ok: true, engines };
}

function parseMechanicalEngineRubric(lines) {
  const rows = [];
  const re = /^\s*-\s*ENGINE:\s*(.+?)\s*\|\s*ENGINE_ID:\s*([A-Za-z0-9._-]+)\s*\|\s*STATUS:\s*(TOUCHED|NOT_TOUCHED|UNKNOWN)\s*\|\s*NOTES:\s*(.+?)\s*\|\s*STUB_WP_IDS:\s*(.+)\s*$/i;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    rows.push({
      title: m[1].trim(),
      engineId: m[2].trim(),
      status: m[3].trim().toUpperCase(),
      notes: m[4].trim(),
      stubWpIdsRaw: m[5].trim(),
      stubWpIds: normalizeCsv(m[5]),
    });
  }
  return rows;
}

function parsePillarDecompositionRows(lines) {
  const rows = [];
  const re = /^\s*-\s*PILLAR:\s*(.+?)\s*\|\s*CAPABILITY_SLICE:\s*(.+?)\s*\|\s*SUBFEATURES:\s*(.+?)\s*\|\s*PRIMITIVES_FEATURES:\s*(.+?)\s*\|\s*MECHANICAL:\s*(.+?)\s*\|\s*ROI:\s*(HIGH|MEDIUM|LOW)\s*\|\s*RESOLUTION:\s*(IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW)\s*\|\s*STUB:\s*(.+?)\s*\|\s*NOTES:\s*(.+)\s*$/i;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    rows.push({
      pillar: m[1].trim(),
      capabilitySlice: m[2].trim(),
      subfeatures: m[3].trim(),
      primitivesFeaturesRaw: m[4].trim(),
      primitivesFeatures: normalizeCsv(m[4]),
      mechanicalRaw: m[5].trim(),
      mechanical: normalizeCsv(m[5]),
      roi: m[6].trim().toUpperCase(),
      resolution: m[7].trim().toUpperCase(),
      stubRaw: m[8].trim(),
      notes: m[9].trim(),
    });
  }
  return rows;
}

function parseExecutionRuntimeAlignmentRows(lines) {
  const rows = [];
  const re = /^\s*-\s*Capability:\s*(.+?)\s*\|\s*JobModel:\s*(AI_JOB|WORKFLOW|MECHANICAL_TOOL|UI_ACTION|NONE)\s*\|\s*Workflow:\s*(.+?)\s*\|\s*ToolSurface:\s*(UNIFIED_TOOL_SURFACE|MCP|COMMAND_CENTER|UI_ONLY|NONE)\s*\|\s*ModelExposure:\s*(LOCAL|CLOUD|BOTH|OPERATOR_ONLY)\s*\|\s*CommandCenter:\s*(VISIBLE|PLANNED|NONE)\s*\|\s*FlightRecorder:\s*(.+?)\s*\|\s*Locus:\s*(VISIBLE|PLANNED|NONE)\s*\|\s*StoragePosture:\s*(SQLITE_NOW_POSTGRES_READY|POSTGRES_ONLY|N\/A)\s*\|\s*Resolution:\s*(IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW)\s*\|\s*Stub:\s*(.+?)\s*\|\s*Notes:\s*(.+)\s*$/i;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    rows.push({
      capability: m[1].trim(),
      jobModel: m[2].trim().toUpperCase(),
      workflow: m[3].trim(),
      toolSurface: m[4].trim().toUpperCase(),
      modelExposure: m[5].trim().toUpperCase(),
      commandCenter: m[6].trim().toUpperCase(),
      flightRecorder: m[7].trim(),
      locus: m[8].trim().toUpperCase(),
      storagePosture: m[9].trim().toUpperCase(),
      resolution: m[10].trim().toUpperCase(),
      stubRaw: m[11].trim(),
      notes: m[12].trim(),
    });
  }
  return rows;
}

function parseForceMultiplierCandidates(lines) {
  const rows = [];
  const re = /^\s*-\s*Combo:\s*(.+?)\s*\|\s*Pillars:\s*(.+?)\s*\|\s*Mechanical:\s*(.+?)\s*\|\s*Primitives\/Features:\s*(.+?)\s*\|\s*Resolution:\s*(IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW)\s*\|\s*Stub:\s*(.+?)\s*\|\s*Notes:\s*(.+)\s*$/i;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    rows.push({
      combo: m[1].trim(),
      pillarsRaw: m[2].trim(),
      pillars: normalizeCsv(m[2]),
      mechanicalRaw: m[3].trim(),
      mechanical: normalizeCsv(m[3]),
      primitivesFeaturesRaw: m[4].trim(),
      primitivesFeatures: normalizeCsv(m[4]),
      resolution: m[5].trim().toUpperCase(),
      stubRaw: m[6].trim(),
      notes: m[7].trim(),
    });
  }
  return rows;
}

function parseGitHubProjectScoutRows(lines) {
  const list = extractIndentedListAfterLabel(lines, 'MATCHED_PROJECTS');
  if (!list.found) return { found: false, hasNone: false, rows: [] };
  if (list.items.length === 1 && /^NONE$/i.test(list.items[0])) {
    return { found: true, hasNone: true, rows: [] };
  }

  const rows = [];
  const re = /^Source:\s*(.+?)\s*\|\s*Repo:\s*([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)\s*\|\s*URL:\s*(https:\/\/github\.com\/\S+)\s*\|\s*Intent:\s*(SAME|ADJACENT|IMPLEMENTATION|UI_PATTERN|ARCH_PATTERN)\s*\|\s*Decision:\s*(ADOPT|ADAPT|REJECT|TRACK_ONLY)\s*\|\s*Impact:\s*(NONE|EXPAND_SCOPE|NEW_STUB|SPEC_UPDATE_NOW|UI_ENRICHMENT)\s*\|\s*Stub:\s*(.+?)\s*\|\s*Notes:\s*(.+)\s*$/i;
  for (const item of list.items) {
    const m = String(item || '').match(re);
    if (!m) continue;
    rows.push({
      source: m[1].trim(),
      repo: m[2].trim(),
      url: m[3].trim(),
      intent: m[4].trim().toUpperCase(),
      decision: m[5].trim().toUpperCase(),
      impact: m[6].trim().toUpperCase(),
      stubRaw: m[7].trim(),
      notes: m[8].trim(),
    });
  }
  return { found: true, hasNone: false, rows, rawItems: list.items };
}

function parseExistingCapabilityRows(lines, label) {
  const list = extractIndentedListAfterLabel(lines, label);
  if (!list.found) return { found: false, hasNone: false, rows: [] };
  if (list.items.length === 1 && /^NONE$/i.test(list.items[0])) {
    return { found: true, hasNone: true, rows: [] };
  }

  const rows = [];
  const re = /^Artifact:\s*(WP-[A-Za-z0-9][A-Za-z0-9-]*)\s*\|\s*BoardStatus:\s*([A-Z_]+)\s*\|\s*Intent:\s*(SAME|PARTIAL|DISTINCT)\s*\|\s*PrimitiveIndex:\s*(COVERED|MISSING|N\/A)\s*\|\s*Matrix:\s*(COVERED|MISSING|N\/A)\s*\|\s*UI:\s*(SAME|PARTIAL|NONE|N\/A)\s*\|\s*CodeReality:\s*(IMPLEMENTED|PARTIAL|NOT_PRESENT|N\/A)\s*\|\s*Resolution:\s*(REUSE_EXISTING|EXPAND_IN_THIS_WP|NEW_STUB|SPEC_UPDATE_NOW|KEEP_SEPARATE)\s*\|\s*Stub:\s*(.+?)\s*\|\s*Notes:\s*(.+)\s*$/i;
  for (const item of list.items) {
    const m = String(item || '').match(re);
    if (!m) continue;
    rows.push({
      artifact: m[1].trim(),
      boardStatus: m[2].trim().toUpperCase(),
      intent: m[3].trim().toUpperCase(),
      primitiveIndex: m[4].trim().toUpperCase(),
      matrix: m[5].trim().toUpperCase(),
      ui: m[6].trim().toUpperCase(),
      codeReality: m[7].trim().toUpperCase(),
      resolution: m[8].trim().toUpperCase(),
      stubRaw: m[9].trim(),
      notes: m[10].trim(),
    });
  }
  return { found: true, hasNone: false, rows, rawItems: list.items };
}

function parseCodeRealityEvidence(lines) {
  const list = extractIndentedListAfterLabel(lines, 'CODE_REALITY_EVIDENCE');
  if (!list.found) return { found: false, hasNone: false, rows: [] };
  if (list.items.length === 1 && /^NONE$/i.test(list.items[0])) {
    return { found: true, hasNone: true, rows: [] };
  }

  const rows = [];
  const re = /^Path:\s*(.+?)\s*\|\s*Artifact:\s*(WP-[A-Za-z0-9][A-Za-z0-9-]*|NONE)\s*\|\s*Covers:\s*(primitive|combo|ui-intent|execution)\s*\|\s*Verdict:\s*(IMPLEMENTED|PARTIAL|NOT_PRESENT)\s*\|\s*Notes:\s*(.+)\s*$/i;
  for (const item of list.items) {
    const m = String(item || '').match(re);
    if (!m) continue;
    rows.push({
      pathRaw: m[1].trim(),
      artifact: m[2].trim(),
      covers: m[3].trim().toLowerCase(),
      verdict: m[4].trim().toUpperCase(),
      notes: m[5].trim(),
    });
  }
  return { found: true, hasNone: false, rows, rawItems: list.items };
}

export function validateRefinementFile(refinementPath, { expectedWpId, requireSignature } = {}) {
  const errors = [];
  const parsed = {
    wpId: '',
    reviewStatus: '',
    signature: '',
    refinementFormatVersion: '',
    refinementEnforcementProfile: '',
    stubWpIdsRaw: '',
    stubWpIds: [],
    uiApplicable: '',
    uiVerdict: '',
    primitiveIndexAction: '',
    primitiveMatrixVerdict: '',
    pillarAlignmentVerdict: '',
    appendixMaintenanceVerdict: '',
    appendixSpecUpdateRequired: false,
    featureRegistryAction: '',
    uiGuidanceAction: '',
    interactionMatrixAction: '',
    researchCurrencyRequired: '',
    researchCurrencyVerdict: '',
    researchDepthVerdict: '',
    githubProjectScoutingVerdict: '',
    researchSources: [],
    researchSynthesis: [],
    githubProjectDecisions: [],
    primitivesTouched: [],
    mechanicalEnginesTouched: [],
    mechanicalEngineAlignmentVerdict: '',
    pillarsTouched: [],
    pillarsRequiringStubs: [],
    forceMultiplierVerdict: '',
    forceMultiplierResolutions: [],
    existingCapabilityAlignmentVerdict: '',
    matchedArtifactResolutions: [],
    codeRealitySummary: [],
    packetHydrationProfile: '',
    packetHydration: {
      requestor: '',
      agentId: '',
      riskTier: '',
      buildOrderDomain: '',
      buildOrderTechBlocker: '',
      buildOrderValueTier: '',
      buildOrderDependsOnRaw: '',
      buildOrderDependsOn: [],
      buildOrderBlocksRaw: '',
      buildOrderBlocks: [],
      specAnchorPrimary: '',
      what: '',
      why: '',
      inScopePaths: [],
      outOfScope: [],
      testPlan: '',
      doneMeans: [],
      filesToOpen: [],
      searchTerms: [],
      runCommands: '',
      riskMap: [],
    },
    uiSpec: {
      surfaces: [],
      controls: [],
      states: [],
      microcopy: [],
      accessibility: [],
    },
  };

  if (!fs.existsSync(refinementPath)) {
    errors.push(`Missing refinement file: ${refinementPath}`);
    return { ok: false, errors };
  }

  const content = fs.readFileSync(refinementPath, 'utf8');
  if (!isAsciiOnly(content)) {
    errors.push(`Refinement file contains non-ASCII bytes: ${refinementPath}`);
  }
  if (!hasHeading(content, 'TECHNICAL_REFINEMENT')) {
    errors.push('Refinement file missing TECHNICAL_REFINEMENT heading');
  }

  const wpId = getSingleField(content, 'WP_ID');
  parsed.wpId = wpId;
  if (expectedWpId && wpId !== expectedWpId) {
    errors.push(`WP_ID mismatch in refinement: expected ${expectedWpId}, got ${wpId || '<missing>'}`);
  }

  const refinementFormatVersion = getSingleField(content, 'REFINEMENT_FORMAT_VERSION');
  parsed.refinementFormatVersion = refinementFormatVersion;
  const isModernRefinement = isVersionAtLeast(refinementFormatVersion, '2026-03-06');
  const hasRuntimeAlignmentSections = isVersionAtLeast(refinementFormatVersion, '2026-03-08');
  if (refinementFormatVersion && !isIsoDate(refinementFormatVersion)) {
    errors.push('REFINEMENT_FORMAT_VERSION must be YYYY-MM-DD (ISO date)');
  }
  const refinementEnforcementProfile = getSingleField(content, 'REFINEMENT_ENFORCEMENT_PROFILE');
  parsed.refinementEnforcementProfile = refinementEnforcementProfile;
  const isHydratedResearchProfile = /^HYDRATED_RESEARCH_V1$/i.test(refinementEnforcementProfile || '');
  if (refinementEnforcementProfile && !isHydratedResearchProfile) {
    errors.push('REFINEMENT_ENFORCEMENT_PROFILE must be HYDRATED_RESEARCH_V1 when present');
  }

  // Resolve SPEC_CURRENT and validate resolved spec + sha1.
  let resolved = null;
  try {
    resolved = resolveSpecCurrent();
  } catch (e) {
    errors.push(String(e?.message || e));
  }
  if (resolved) {
    const resolvedLine = getSingleField(content, 'SPEC_TARGET_RESOLVED');
    const expectedResolvedLine = `.GOV/roles_shared/SPEC_CURRENT.md -> ${resolved.specFileName}`;
    if (resolvedLine !== expectedResolvedLine) {
      errors.push(`SPEC_TARGET_RESOLVED mismatch: expected "${expectedResolvedLine}", got "${resolvedLine || '<missing>'}"`);
    }

    const sha1Line = getSingleField(content, 'SPEC_TARGET_SHA1');
    if (!sha1Line || sha1Line.toLowerCase() !== resolved.sha1.toLowerCase()) {
      errors.push(`SPEC_TARGET_SHA1 mismatch: expected ${resolved.sha1}, got ${sha1Line || '<missing>'}`);
    }
  }

  // Required sections (protocol).
  const requiredSections = isModernRefinement
    ? [
        'GAPS_IDENTIFIED',
        'LANDSCAPE_SCAN',
        'FLIGHT_RECORDER_INTERACTION',
        'RED_TEAM_ADVISORY',
        'PRIMITIVES',
        'PRIMITIVE_INDEX',
        'PILLAR_ALIGNMENT',
        'PRIMITIVE_MATRIX',
        'UI_UX_RUBRIC',
        'ROADMAP_PHASE_SPLIT',
        'CLEARLY_COVERS',
        'ENRICHMENT',
        'SPEC_ANCHORS',
      ]
    : ['GAPS_IDENTIFIED', 'FLIGHT_RECORDER_INTERACTION', 'RED_TEAM_ADVISORY', 'PRIMITIVES'];

  if (isHydratedResearchProfile) {
    requiredSections.splice(2, 0, 'RESEARCH_CURRENCY');
    requiredSections.splice(3, 0, 'RESEARCH_DEPTH');
    requiredSections.splice(4, 0, 'GITHUB_PROJECT_SCOUTING');
    requiredSections.splice(6, 0, 'APPENDIX_MAINTENANCE');
    requiredSections.splice(7, 0, 'MECHANICAL_ENGINE_ALIGNMENT');
    if (hasRuntimeAlignmentSections) {
      requiredSections.splice(requiredSections.indexOf('PRIMITIVE_MATRIX'), 0, 'PILLAR_DECOMPOSITION');
      requiredSections.splice(requiredSections.indexOf('PRIMITIVE_MATRIX'), 0, 'EXECUTION_RUNTIME_ALIGNMENT');
    }
    requiredSections.splice(requiredSections.indexOf('UI_UX_RUBRIC'), 0, 'FORCE_MULTIPLIER_EXPANSION');
    requiredSections.splice(requiredSections.indexOf('UI_UX_RUBRIC'), 0, 'EXISTING_CAPABILITY_ALIGNMENT');
    requiredSections.splice(requiredSections.indexOf('CLEARLY_COVERS'), 0, 'PACKET_HYDRATION');
  }

  requiredSections.forEach((h) => {
    if (!hasHeading(content, h)) errors.push(`Missing required section heading: ${h}`);
  });

  const lines = content.split('\n');

  if (isModernRefinement) {
    const stubWpIdsRaw = getSingleField(content, 'STUB_WP_IDS');
    parsed.stubWpIdsRaw = stubWpIdsRaw;
    parsed.stubWpIds = validateStubIds(stubWpIdsRaw, errors, 'STUB_WP_IDS');
    if (isHydratedResearchProfile && hasRuntimeAlignmentSections && resolved?.specFileName) {
      const buildOrderPath = path.join('.GOV', 'roles_shared', 'BUILD_ORDER.md');
      try {
        const buildOrderContent = fs.readFileSync(buildOrderPath, 'utf8');
        const specTarget = (buildOrderContent.match(/^\s*-\s*SPEC_TARGET\s*:\s*(.+)\s*$/mi) || [])[1]?.trim() || '';
        if (specTarget !== resolved.specFileName) {
          errors.push(`BUILD_ORDER.md SPEC_TARGET mismatch: expected ${resolved.specFileName}, got ${specTarget || '<missing>'}`);
        }
        for (const stubId of parsed.stubWpIds) {
          if (!buildOrderContent.includes(stubId)) {
            errors.push(`BUILD_ORDER.md must include stub ${stubId} before the refinement gate can PASS`);
          }
        }
      } catch (e) {
        errors.push(`Could not read .GOV/roles_shared/BUILD_ORDER.md: ${String(e?.message || e)}`);
      }
    }

    // LANDSCAPE_SCAN minimums (enforced for modern refinements).
    const lsTimebox = getSingleField(content, 'TIMEBOX');
    const lsScope = getSingleField(content, 'SEARCH_SCOPE');
    const lsRefs = getSingleField(content, 'REFERENCES');
    const lsDecisions = getSingleField(content, 'DECISIONS (ADOPT/ADAPT/REJECT)');
    const lsLicense = getSingleField(content, 'LICENSE/IP_NOTES');
    const lsSpecImpact = getSingleField(content, 'SPEC_IMPACT');
    const lsSpecImpactReason = getSingleField(content, 'SPEC_IMPACT_REASON');

    if (isPlaceholderValue(lsTimebox)) errors.push('LANDSCAPE_SCAN TIMEBOX must be filled');
    if (isPlaceholderValue(lsScope)) errors.push('LANDSCAPE_SCAN SEARCH_SCOPE must be filled');
    if (isPlaceholderValue(lsRefs)) errors.push('LANDSCAPE_SCAN REFERENCES must be filled (or NONE + reason)');
    if (isPlaceholderValue(lsDecisions)) errors.push('LANDSCAPE_SCAN DECISIONS (ADOPT/ADAPT/REJECT) must be filled');
    if (isPlaceholderValue(lsLicense)) errors.push('LANDSCAPE_SCAN LICENSE/IP_NOTES must be filled (or NONE)');
    if (!/^(YES|NO)$/i.test(lsSpecImpact || '')) errors.push('LANDSCAPE_SCAN SPEC_IMPACT must be YES or NO');
    if (isPlaceholderValue(lsSpecImpactReason)) errors.push('LANDSCAPE_SCAN SPEC_IMPACT_REASON must be filled');

    if (isHydratedResearchProfile) {
      const researchRequired = getSingleField(content, 'RESEARCH_CURRENCY_REQUIRED');
      const researchReasonNo = getSingleField(content, 'RESEARCH_CURRENCY_REASON_NO');
      const sourceMaxAgeRaw = getSingleField(content, 'SOURCE_MAX_AGE_DAYS');
      const sourceLog = extractIndentedListAfterLabel(lines, 'SOURCE_LOG');
      const researchSynthesis = extractIndentedListAfterLabel(lines, 'RESEARCH_SYNTHESIS');
      const researchGaps = extractIndentedListAfterLabel(lines, 'RESEARCH_GAPS_TO_TRACK');
      const researchVerdict = getSingleField(content, 'RESEARCH_CURRENCY_VERDICT');
      const adoptPatterns = extractIndentedListAfterLabel(lines, 'ADOPT_PATTERNS');
      const adaptPatterns = extractIndentedListAfterLabel(lines, 'ADAPT_PATTERNS');
      const rejectPatterns = extractIndentedListAfterLabel(lines, 'REJECT_PATTERNS');
      const researchDepthVerdict = getSingleField(content, 'RESEARCH_DEPTH_VERDICT');
      const githubSearchQueries = extractIndentedListAfterLabel(lines, 'SEARCH_QUERIES');
      const githubProjectMatches = parseGitHubProjectScoutRows(lines);
      const githubProjectScoutingVerdict = getSingleField(content, 'GITHUB_PROJECT_SCOUTING_VERDICT');

      parsed.researchCurrencyRequired = (researchRequired || '').toUpperCase();
      parsed.researchCurrencyVerdict = (researchVerdict || '').toUpperCase();
      parsed.researchDepthVerdict = (researchDepthVerdict || '').toUpperCase();
      parsed.githubProjectScoutingVerdict = (githubProjectScoutingVerdict || '').toUpperCase();

      if (!/^(YES|NO)$/i.test(researchRequired || '')) {
        errors.push('RESEARCH_CURRENCY_REQUIRED must be YES or NO');
      }
      if (!/^(CURRENT|STALE|NOT_APPLICABLE)$/i.test(researchVerdict || '')) {
        errors.push('RESEARCH_CURRENCY_VERDICT must be CURRENT | STALE | NOT_APPLICABLE');
      }
      if (!/^(PASS|NOT_APPLICABLE)$/i.test(researchDepthVerdict || '')) {
        errors.push('RESEARCH_DEPTH_VERDICT must be PASS or NOT_APPLICABLE');
      }
      if (!/^(PASS|NOT_APPLICABLE)$/i.test(githubProjectScoutingVerdict || '')) {
        errors.push('GITHUB_PROJECT_SCOUTING_VERDICT must be PASS or NOT_APPLICABLE');
      }
      if (!researchSynthesis.found || researchSynthesis.items.length === 0 || researchSynthesis.items.some((s) => isPlaceholderValue(s))) {
        errors.push('RESEARCH_CURRENCY RESEARCH_SYNTHESIS must be filled (use NONE only if truly not applicable)');
      }
      if (!researchGaps.found || researchGaps.items.length === 0 || researchGaps.items.some((s) => isPlaceholderValue(s))) {
        errors.push('RESEARCH_CURRENCY RESEARCH_GAPS_TO_TRACK must be filled (use NONE if none)');
      }

      if (/^YES$/i.test(researchRequired || '')) {
        if (!isPositiveIntegerString(sourceMaxAgeRaw)) {
          errors.push('SOURCE_MAX_AGE_DAYS must be an integer when RESEARCH_CURRENCY_REQUIRED=YES');
        } else {
          const maxAgeDays = parseInt(sourceMaxAgeRaw, 10);
          if (maxAgeDays < 30 || maxAgeDays > 730) {
            errors.push('SOURCE_MAX_AGE_DAYS must be between 30 and 730');
          }
        }

        if (!sourceLog.found || sourceLog.items.length < 3) {
          errors.push('RESEARCH_CURRENCY SOURCE_LOG must include at least 3 sources when RESEARCH_CURRENCY_REQUIRED=YES');
        }

        const categories = new Set();
        let githubSourceCount = 0;
        let freshSources = 0;
        const maxAgeDays = isPositiveIntegerString(sourceMaxAgeRaw) ? parseInt(sourceMaxAgeRaw, 10) : null;
        const sourceTitles = new Set();
        const sourceByTitle = new Map();
        parsed.researchSources = [];

        for (const item of sourceLog.items) {
          if (/^NONE$/i.test(item)) {
            errors.push('RESEARCH_CURRENCY SOURCE_LOG cannot contain NONE when RESEARCH_CURRENCY_REQUIRED=YES');
            continue;
          }
          const record = parsePipeRecord(item);
          const source = record.SOURCE || '';
          const kind = (record.KIND || '').toUpperCase();
          const dateStr = record.DATE || '';
          const retrievedAt = record.RETRIEVED || '';
          const url = record.URL || '';
          const why = record.WHY || '';
          parsed.researchSources.push({ source, kind, date: dateStr, retrievedAt, url, why });
          if (source) sourceTitles.add(source);
          if (source) sourceByTitle.set(source, { kind, url });

          if (isPlaceholderValue(source)) errors.push(`RESEARCH_CURRENCY SOURCE_LOG entry missing Source: ${item}`);
          if (!/^(BIG_TECH|UNIVERSITY|PAPER|GITHUB|OSS_DOC)$/i.test(kind)) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG Kind must be BIG_TECH | UNIVERSITY | PAPER | GITHUB | OSS_DOC (got: ${kind || '<missing>'})`);
          } else if (kind === 'BIG_TECH') {
            categories.add('BIG_TECH');
          } else if (kind === 'UNIVERSITY' || kind === 'PAPER') {
            categories.add('UNIVERSITY_OR_PAPER');
          } else {
            categories.add('GITHUB_OR_OSS');
          }
          if (kind === 'GITHUB') githubSourceCount += 1;
          const sourceDate = parseIsoDateUtc(dateStr);
          if (!sourceDate) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG Date must be YYYY-MM-DD (got: ${dateStr || '<missing>'})`);
          } else {
            const ageDays = Math.floor((Date.now() - sourceDate.getTime()) / 86400000);
            if (ageDays < 0) errors.push(`RESEARCH_CURRENCY SOURCE_LOG Date cannot be in the future: ${dateStr}`);
            if (maxAgeDays !== null && ageDays >= 0 && ageDays <= maxAgeDays) freshSources += 1;
          }
          if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/.test(retrievedAt || '')) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG Retrieved must be RFC3339 UTC (got: ${retrievedAt || '<missing>'})`);
          } else {
            const retrievedDate = new Date(retrievedAt);
            if (Number.isNaN(retrievedDate.getTime()) || retrievedDate.getTime() > Date.now()) {
              errors.push(`RESEARCH_CURRENCY SOURCE_LOG Retrieved cannot be invalid or in the future: ${retrievedAt}`);
            }
          }
          if (!/^https:\/\/\S+$/i.test(url)) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG URL must be https://... (got: ${url || '<missing>'})`);
          }
          if (isPlaceholderValue(why)) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG Why must be filled for source: ${source || '<missing>'}`);
          }
        }

        if (!categories.has('BIG_TECH')) errors.push('RESEARCH_CURRENCY SOURCE_LOG must include at least one BIG_TECH source');
        if (!categories.has('UNIVERSITY_OR_PAPER')) errors.push('RESEARCH_CURRENCY SOURCE_LOG must include at least one UNIVERSITY or PAPER source');
        if (!categories.has('GITHUB_OR_OSS')) errors.push('RESEARCH_CURRENCY SOURCE_LOG must include at least one GITHUB or OSS_DOC source');
        if (maxAgeDays !== null && freshSources === 0) {
          errors.push('RESEARCH_CURRENCY SOURCE_LOG must include at least one source within SOURCE_MAX_AGE_DAYS');
        }
        if (researchSynthesis.items.filter((s) => !isPlaceholderValue(s) && !/^NONE$/i.test(s)).length === 0) {
          errors.push('RESEARCH_CURRENCY RESEARCH_SYNTHESIS must include at least one concrete insight');
        }
        const validateResearchPatternList = (label, listResult) => {
          if (!listResult.found || listResult.items.length === 0) {
            errors.push(`RESEARCH_DEPTH ${label} must include at least one entry when RESEARCH_CURRENCY_REQUIRED=YES`);
            return;
          }
          for (const item of listResult.items) {
            const record = parsePipeRecord(item);
            const source = record.SOURCE || '';
            const pattern = record.PATTERN || '';
            const whyText = record.WHY || '';
            if (!sourceTitles.has(source)) {
              errors.push(`RESEARCH_DEPTH ${label} must reference a Source from SOURCE_LOG (got: ${source || '<missing>'})`);
            }
            if (isPlaceholderValue(pattern)) errors.push(`RESEARCH_DEPTH ${label} Pattern must be filled`);
            if (isPlaceholderValue(whyText)) errors.push(`RESEARCH_DEPTH ${label} Why must be filled`);
          }
        };
        validateResearchPatternList('ADOPT_PATTERNS', adoptPatterns);
        validateResearchPatternList('ADAPT_PATTERNS', adaptPatterns);
        validateResearchPatternList('REJECT_PATTERNS', rejectPatterns);
        if (!/^PASS$/i.test(researchDepthVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=YES requires RESEARCH_DEPTH_VERDICT=PASS');
        }
        if (githubSourceCount === 0) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=YES requires at least one GITHUB source in SOURCE_LOG');
        }
        if (!githubSearchQueries.found || githubSearchQueries.items.filter((item) => !isPlaceholderValue(item) && !/^NONE$/i.test(item)).length === 0) {
          errors.push('GITHUB_PROJECT_SCOUTING SEARCH_QUERIES must include at least one concrete GitHub search angle when RESEARCH_CURRENCY_REQUIRED=YES');
        }
        if (!githubProjectMatches.found || githubProjectMatches.rows.length === 0) {
          errors.push('GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS must include at least one concrete GitHub project when RESEARCH_CURRENCY_REQUIRED=YES');
        } else if (githubProjectMatches.rows.length !== githubProjectMatches.rawItems.length) {
          errors.push('GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS contains malformed entries; each item must match the required Source|Repo|URL|Intent|Decision|Impact|Stub|Notes format');
        }
        parsed.githubProjectDecisions = [];
        for (const project of githubProjectMatches.rows) {
          const sourceMeta = sourceByTitle.get(project.source);
          if (!sourceMeta) {
            errors.push(`GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS references unknown Source: ${project.source}`);
          } else {
            if (sourceMeta.kind !== 'GITHUB') {
              errors.push(`GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS Source must be a GITHUB source from SOURCE_LOG (got ${sourceMeta.kind})`);
            }
            if (!/^https:\/\/github\.com\//i.test(sourceMeta.url || '')) {
              errors.push(`GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS Source ${project.source} must use a GitHub repository URL`);
            }
          }
          if (!project.url.toLowerCase().startsWith(`https://github.com/${project.repo.toLowerCase()}`)) {
            errors.push(`GITHUB_PROJECT_SCOUTING project URL must align with Repo ${project.repo}`);
          }
          if (isPlaceholderValue(project.notes)) {
            errors.push(`GITHUB_PROJECT_SCOUTING ${project.repo} Notes must be filled`);
          }
          if (project.impact === 'NEW_STUB') {
            const stubIds = validateStubIds(project.stubRaw, errors, `GITHUB_PROJECT_SCOUTING ${project.repo} Stub`);
            if (stubIds.length !== 1) {
              errors.push(`GITHUB_PROJECT_SCOUTING ${project.repo} with Impact=NEW_STUB must point to exactly one stub packet`);
            }
            for (const stubId of stubIds) {
              if (!parsed.stubWpIds.includes(stubId)) {
                errors.push(`Top-level STUB_WP_IDS must include GitHub-project-derived stub ${stubId}`);
              }
            }
          } else if (!/^NONE$/i.test(project.stubRaw || '')) {
            errors.push(`GITHUB_PROJECT_SCOUTING ${project.repo} must use Stub: NONE unless Impact=NEW_STUB`);
          }
          parsed.githubProjectDecisions.push(`${project.repo} -> ${project.decision} (${project.impact})`);
        }
        if (!/^PASS$/i.test(githubProjectScoutingVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=YES requires GITHUB_PROJECT_SCOUTING_VERDICT=PASS');
        }
        if (!/^CURRENT$/i.test(researchVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=YES requires RESEARCH_CURRENCY_VERDICT=CURRENT');
        }
      } else if (/^NO$/i.test(researchRequired || '')) {
        if (isPlaceholderValue(researchReasonNo)) {
          errors.push('RESEARCH_CURRENCY_REASON_NO is required when RESEARCH_CURRENCY_REQUIRED=NO');
        }
        if (!/^(N\/A|NONE)$/i.test(sourceMaxAgeRaw || '')) {
          errors.push('SOURCE_MAX_AGE_DAYS must be N/A or NONE when RESEARCH_CURRENCY_REQUIRED=NO');
        }
        if (!sourceLog.found || sourceLog.items.length === 0 || sourceLog.items.some((item) => !/^NONE$/i.test(item))) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires SOURCE_LOG to be a single NONE entry');
        }
        if (!/^NOT_APPLICABLE$/i.test(researchVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires RESEARCH_CURRENCY_VERDICT=NOT_APPLICABLE');
        }
        if ((adoptPatterns.items || []).some((item) => !/^NONE$/i.test(item)) || (adaptPatterns.items || []).some((item) => !/^NONE$/i.test(item)) || (rejectPatterns.items || []).some((item) => !/^NONE$/i.test(item))) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires ADOPT/ADAPT/REJECT pattern lists to be NONE');
        }
        if (!/^NOT_APPLICABLE$/i.test(researchDepthVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires RESEARCH_DEPTH_VERDICT=NOT_APPLICABLE');
        }
        if (!githubSearchQueries.found || githubSearchQueries.items.length === 0 || githubSearchQueries.items.some((item) => !/^NONE$/i.test(item))) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires GITHUB_PROJECT_SCOUTING SEARCH_QUERIES to be NONE');
        }
        if (!githubProjectMatches.found || !githubProjectMatches.hasNone) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires GITHUB_PROJECT_SCOUTING MATCHED_PROJECTS to be NONE');
        }
        if (!/^NOT_APPLICABLE$/i.test(githubProjectScoutingVerdict || '')) {
          errors.push('RESEARCH_CURRENCY_REQUIRED=NO requires GITHUB_PROJECT_SCOUTING_VERDICT=NOT_APPLICABLE');
        }
      }

      parsed.researchSynthesis = researchSynthesis.items.filter((s) => !/^NONE$/i.test(s));
    }

    // PRIMITIVES section minimums.
    const primTouched = extractIndentedListAfterLabel(lines, 'PRIMITIVES_TOUCHED (IDs)');
    const primNewOrUpdated = extractIndentedListAfterLabel(lines, 'PRIMITIVES_NEW_OR_UPDATED (IDs)');

    const primIds = new Set();
    const addPrimIdsFromList = (label, listResult, { requireSome } = {}) => {
      if (!listResult.found) {
        errors.push(`PRIMITIVES must include a ${label} list`);
        return;
      }
      const items = listResult.items.map((s) => (s || '').trim()).filter(Boolean);
      const hasNone = items.some((v) => /^NONE$/i.test(v));
      for (const item of items) {
        if (/^NONE$/i.test(item)) continue;
        if (/<fill|<pending>/i.test(item)) {
          errors.push(`${label} contains template placeholders; fill real PRIM-... IDs or write NONE`);
          continue;
        }
        if (!/^PRIM-[A-Za-z0-9][A-Za-z0-9_-]*$/i.test(item)) {
          errors.push(`${label} contains invalid primitive id (expected PRIM-...): ${item}`);
          continue;
        }
        primIds.add(item);
      }
      if (requireSome && primIds.size === 0 && !hasNone) {
        errors.push(`${label} must list one or more PRIM-... IDs, or NONE`);
      }
    };

    addPrimIdsFromList('PRIMITIVES_TOUCHED (IDs)', primTouched, { requireSome: true });
    addPrimIdsFromList('PRIMITIVES_NEW_OR_UPDATED (IDs)', primNewOrUpdated, { requireSome: false });
    parsed.primitivesTouched = Array.from(primIds);

    // PRIMITIVE_INDEX gate.
    const primIndexAction = getSingleField(content, 'PRIMITIVE_INDEX_ACTION');
    parsed.primitiveIndexAction = (primIndexAction || '').toUpperCase();
    if (!/^(UPDATED|NO_CHANGE)$/i.test(primIndexAction || '')) {
      errors.push('PRIMITIVE_INDEX_ACTION must be UPDATED or NO_CHANGE');
    }
    if (/^NO_CHANGE$/i.test(primIndexAction || '')) {
      const reason = getSingleField(content, 'PRIMITIVE_INDEX_REASON_NO_CHANGE');
      if (isPlaceholderValue(reason)) errors.push('PRIMITIVE_INDEX_REASON_NO_CHANGE is required when PRIMITIVE_INDEX_ACTION=NO_CHANGE');
    }

    // Enforce that any referenced PRIM-* IDs exist in the Spec primitive index/matrix (Appendix 12.4).
    // This makes "update the index first" mechanically enforceable.
    let specPrimitiveIds = null;
    let specImxEdgeIds = null;
    let specContent = null;
    if (resolved) {
      try {
        specContent = fs.readFileSync(resolved.specFilePath, 'utf8');
        const primTool = extractAppendixJson(specContent, 'HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX');
        if (!primTool.ok) {
          errors.push(primTool.error);
        } else {
          specPrimitiveIds = new Set(
            (primTool.json?.primitives || [])
              .map((p) => (p?.primitive_id || '').trim())
              .filter((v) => /^PRIM-/.test(v))
          );
        }

        const imx = extractAppendixJson(specContent, 'HS-APPX-INTERACTION-MATRIX');
        if (!imx.ok) {
          errors.push(imx.error);
        } else {
          specImxEdgeIds = new Set(
            (imx.json?.edges || [])
              .map((e) => (e?.edge_id || '').trim())
              .filter((v) => /^IMX-/.test(v))
          );
        }
      } catch (e) {
        errors.push(`Failed to read/parse spec appendices: ${String(e?.message || e)}`);
      }
    }

    if (specPrimitiveIds) {
      for (const primId of primIds) {
        if (!specPrimitiveIds.has(primId)) {
          errors.push(`Primitive ${primId} is referenced in refinement but is missing from Spec Appendix 12.4 (HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)`);
        }
      }
    }

    if (isHydratedResearchProfile) {
      const featureRegistryAction = getSingleField(content, 'FEATURE_REGISTRY_ACTION');
      const featureRegistryReason = getSingleField(content, 'FEATURE_REGISTRY_REASON_NO_CHANGE');
      const uiGuidanceAction = getSingleField(content, 'UI_GUIDANCE_ACTION');
      const uiGuidanceReason = getSingleField(content, 'UI_GUIDANCE_REASON');
      const interactionMatrixAction = getSingleField(content, 'INTERACTION_MATRIX_ACTION');
      const interactionMatrixReason = getSingleField(content, 'INTERACTION_MATRIX_REASON_NO_CHANGE');
      const appendixNotes = extractIndentedListAfterLabel(lines, 'APPENDIX_MAINTENANCE_NOTES');
      const appendixVerdict = getSingleField(content, 'APPENDIX_MAINTENANCE_VERDICT');

      parsed.featureRegistryAction = (featureRegistryAction || '').toUpperCase();
      parsed.uiGuidanceAction = (uiGuidanceAction || '').toUpperCase();
      parsed.interactionMatrixAction = (interactionMatrixAction || '').toUpperCase();
      parsed.appendixMaintenanceVerdict = (appendixVerdict || '').toUpperCase();
      parsed.appendixSpecUpdateRequired =
        parsed.featureRegistryAction === 'UPDATED'
        || parsed.uiGuidanceAction === 'UPDATED'
        || parsed.interactionMatrixAction === 'UPDATED'
        || parsed.primitiveIndexAction === 'UPDATED';

      if (!/^(UPDATED|NO_CHANGE)$/i.test(featureRegistryAction || '')) {
        errors.push('FEATURE_REGISTRY_ACTION must be UPDATED or NO_CHANGE');
      }
      if (/^NO_CHANGE$/i.test(featureRegistryAction || '') && isPlaceholderValue(featureRegistryReason)) {
        errors.push('FEATURE_REGISTRY_REASON_NO_CHANGE is required when FEATURE_REGISTRY_ACTION=NO_CHANGE');
      }

      if (!/^(UPDATED|NO_CHANGE|NOT_APPLICABLE)$/i.test(uiGuidanceAction || '')) {
        errors.push('UI_GUIDANCE_ACTION must be UPDATED | NO_CHANGE | NOT_APPLICABLE');
      }
      if (isPlaceholderValue(uiGuidanceReason)) {
        errors.push('UI_GUIDANCE_REASON must be filled');
      }

      if (!/^(UPDATED|NO_CHANGE)$/i.test(interactionMatrixAction || '')) {
        errors.push('INTERACTION_MATRIX_ACTION must be UPDATED or NO_CHANGE');
      }
      if (/^NO_CHANGE$/i.test(interactionMatrixAction || '') && isPlaceholderValue(interactionMatrixReason)) {
        errors.push('INTERACTION_MATRIX_REASON_NO_CHANGE is required when INTERACTION_MATRIX_ACTION=NO_CHANGE');
      }

      if (!appendixNotes.found || appendixNotes.items.filter((s) => !isPlaceholderValue(s)).length === 0) {
        errors.push('APPENDIX_MAINTENANCE_NOTES must include at least one concrete note');
      }
      if (!/^(OK|NEEDS_SPEC_UPDATE|NEEDS_STUBS)$/i.test(appendixVerdict || '')) {
        errors.push('APPENDIX_MAINTENANCE_VERDICT must be OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS');
      }
      if (parsed.appendixSpecUpdateRequired && parsed.appendixMaintenanceVerdict !== 'NEEDS_SPEC_UPDATE') {
        errors.push('Any appendix action marked UPDATED requires APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE');
      }
      if (!parsed.appendixSpecUpdateRequired && parsed.appendixMaintenanceVerdict === 'NEEDS_SPEC_UPDATE') {
        errors.push('APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE requires at least one appendix action to be UPDATED');
      }
      if (parsed.appendixSpecUpdateRequired && !/^YES$/i.test(lsSpecImpact || '')) {
        errors.push('Any appendix action marked UPDATED requires LANDSCAPE_SCAN SPEC_IMPACT=YES');
      }

      const githubProjectMatches = parseGitHubProjectScoutRows(lines);
      for (const project of githubProjectMatches.rows) {
        if (project.impact === 'SPEC_UPDATE_NOW' && !parsed.appendixSpecUpdateRequired) {
          errors.push(`GITHUB_PROJECT_SCOUTING ${project.repo} uses Impact=SPEC_UPDATE_NOW but appendix maintenance did not declare a spec update`);
        }
        if (project.impact === 'UI_ENRICHMENT' && parsed.uiGuidanceAction === 'NO_CHANGE') {
          errors.push(`GITHUB_PROJECT_SCOUTING ${project.repo} uses Impact=UI_ENRICHMENT, so UI_GUIDANCE_ACTION cannot remain NO_CHANGE`);
        }
      }
    }

    let engineRows = [];
    let specMechanicalEngines = [];
    let specMechanicalEngineIds = new Set();
    if (isHydratedResearchProfile) {
      if (!specContent) {
        errors.push('HYDRATED_RESEARCH_V1 requires the resolved Master Spec to be readable for mechanical engine extraction');
      } else {
        const engineCatalog = extractMechanicalEngines(specContent);
        if (!engineCatalog.ok) {
          errors.push(engineCatalog.error);
        } else {
          specMechanicalEngines = engineCatalog.engines;
          specMechanicalEngineIds = new Set(specMechanicalEngines.map((entry) => entry.id));
        }
      }

      engineRows = parseMechanicalEngineRubric(lines);
      for (const engine of specMechanicalEngines) {
        if (!engineRows.some((row) => row.engineId === engine.id)) {
          errors.push(`MECHANICAL_ENGINE_ALIGNMENT missing/invalid rubric line for engine: ${engine.title} (${engine.id})`);
        }
      }

      const engineStubUnion = new Set();
      const touchedEngineIds = [];
      for (const row of engineRows) {
        if (!specMechanicalEngineIds.has(row.engineId)) {
          errors.push(`MECHANICAL_ENGINE_ALIGNMENT references unknown spec engine id: ${row.engineId}`);
        } else {
          const expected = specMechanicalEngines.find((entry) => entry.id === row.engineId);
          if (expected && row.title.toUpperCase() !== expected.title.toUpperCase()) {
            errors.push(`MECHANICAL_ENGINE_ALIGNMENT engine title mismatch for ${row.engineId}: expected "${expected.title}", got "${row.title}"`);
          }
        }
        if (isPlaceholderValue(row.notes)) {
          errors.push(`MECHANICAL_ENGINE_ALIGNMENT ${row.engineId} NOTES must be filled`);
        }
        const rowStubIds = validateStubIds(row.stubWpIdsRaw, errors, `MECHANICAL_ENGINE_ALIGNMENT ${row.engineId} STUB_WP_IDS`);
        row.stubWpIds = rowStubIds;
        if (row.status === 'UNKNOWN' && rowStubIds.length === 0) {
          errors.push(`MECHANICAL_ENGINE_ALIGNMENT ${row.engineId} STATUS=UNKNOWN requires one or more STUB_WP_IDS`);
        }
        if (row.status === 'TOUCHED') {
          touchedEngineIds.push(row.engineId);
        }
        rowStubIds.forEach((id) => engineStubUnion.add(id));
      }
      parsed.mechanicalEnginesTouched = Array.from(new Set(touchedEngineIds));

      const engineVerdict = getSingleField(content, 'MECHANICAL_ENGINE_ALIGNMENT_VERDICT');
      parsed.mechanicalEngineAlignmentVerdict = (engineVerdict || '').toUpperCase();
      if (!/^(OK|NEEDS_STUBS|NEEDS_SPEC_UPDATE)$/i.test(engineVerdict || '')) {
        errors.push('MECHANICAL_ENGINE_ALIGNMENT_VERDICT must be OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE');
      }

      const engineNeedsStubs = engineRows.some((row) => row.status === 'UNKNOWN' || row.stubWpIds.length > 0);
      if (engineNeedsStubs && parsed.mechanicalEngineAlignmentVerdict === 'OK') {
        errors.push('MECHANICAL_ENGINE_ALIGNMENT rows with UNKNOWN or STUB_WP_IDS require MECHANICAL_ENGINE_ALIGNMENT_VERDICT=NEEDS_STUBS or NEEDS_SPEC_UPDATE');
      }
      if (!engineNeedsStubs && parsed.mechanicalEngineAlignmentVerdict === 'NEEDS_STUBS') {
        errors.push('MECHANICAL_ENGINE_ALIGNMENT_VERDICT=NEEDS_STUBS requires at least one engine row with STATUS=UNKNOWN or STUB_WP_IDS');
      }
      if (parsed.mechanicalEngineAlignmentVerdict === 'NEEDS_SPEC_UPDATE' && !parsed.appendixSpecUpdateRequired) {
        errors.push('MECHANICAL_ENGINE_ALIGNMENT_VERDICT=NEEDS_SPEC_UPDATE requires appendix maintenance to declare a spec update');
      }
      if (parsed.mechanicalEngineAlignmentVerdict === 'NEEDS_STUBS' && parsed.stubWpIds.length === 0) {
        errors.push('MECHANICAL_ENGINE_ALIGNMENT_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
      }
      for (const stubId of engineStubUnion) {
        if (!parsed.stubWpIds.includes(stubId)) {
          errors.push(`Top-level STUB_WP_IDS must include mechanical-engine-linked stub ${stubId}`);
        }
      }
    }

    // PILLAR_ALIGNMENT rubric lines.
    const pillars = [
      'Flight Recorder',
      'Calendar',
      'Monaco',
      'Word clone',
      'Excel clone',
      'Locus',
      'Loom',
      'Work packets (product, not repo)',
      'Task board (product, not repo)',
      'MicroTask',
      'Command Center',
      'Spec to prompt',
      'SQL to PostgreSQL shift readiness',
      'LLM-friendly data',
      'Stage',
      'Studio',
      'Atelier/Lens',
      'Skill distillation / LoRA',
      'ACE',
      'RAG',
    ];
    if (hasRuntimeAlignmentSections) {
      pillars.splice(pillars.indexOf('Spec to prompt'), 0, 'Execution / Job Runtime');
    }
    const pillarLookup = new Map(pillars.map((pillar) => [pillar.toUpperCase(), pillar]));
    for (const p of pillars) {
      const re = new RegExp(`^\\s*-\\s*PILLAR:\\s*${escapeRegExp(p)}\\s*\\|\\s*STATUS:\\s*(TOUCHED|NOT_TOUCHED|UNKNOWN)\\b`, 'i');
      if (!lines.some((l) => re.test(l))) {
        errors.push(`PILLAR_ALIGNMENT missing/invalid rubric line for pillar: ${p}`);
      }
    }
    const pillarRows = parsePillarRubric(lines);
    const pillarStubUnion = new Set();
    for (const row of pillarRows) {
      if (isPlaceholderValue(row.notes)) {
        errors.push(`PILLAR ${row.pillar} NOTES must be filled`);
      }
      const rowStubIds = validateStubIds(row.stubWpIdsRaw, errors, `PILLAR ${row.pillar} STUB_WP_IDS`);
      row.stubWpIds = rowStubIds;
      if (row.status === 'UNKNOWN' && rowStubIds.length === 0) {
        errors.push(`PILLAR ${row.pillar} STATUS=UNKNOWN requires one or more STUB_WP_IDS`);
      }
      rowStubIds.forEach((id) => pillarStubUnion.add(id));
    }
    parsed.pillarsTouched = pillarRows.filter((row) => row.status === 'TOUCHED').map((row) => row.pillar);
    parsed.pillarsRequiringStubs = pillarRows
      .filter((row) => row.stubWpIds.length > 0)
      .map((row) => `${row.pillar}: ${row.stubWpIds.join(', ')}`);
    for (const stubId of pillarStubUnion) {
      if (!parsed.stubWpIds.includes(stubId)) {
        errors.push(`Top-level STUB_WP_IDS must include pillar-linked stub ${stubId}`);
      }
    }

    const pillarVerdict = getSingleField(content, 'PILLAR_ALIGNMENT_VERDICT');
    parsed.pillarAlignmentVerdict = (pillarVerdict || '').toUpperCase();
    if (!/^(OK|NEEDS_SPEC_UPDATE|NEEDS_STUBS)$/i.test(pillarVerdict || '')) {
      errors.push('PILLAR_ALIGNMENT_VERDICT must be OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS');
    }
    if (/^NEEDS_STUBS$/i.test(pillarVerdict || '') && parsed.stubWpIds.length === 0) {
      errors.push('PILLAR_ALIGNMENT_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
    }

    if (isHydratedResearchProfile && hasRuntimeAlignmentSections) {
      const decompositionSection = extractSectionLinesByHeading(lines, 'PILLAR_DECOMPOSITION');
      const decompositionRows = parsePillarDecompositionRows(decompositionSection.lines);
      const decompositionVerdict = getSingleField(content, 'PILLAR_DECOMPOSITION_VERDICT');
      const decompositionStubUnion = new Set();
      const decompositionPillarsSeen = new Set();
      parsed.pillarDecompositionVerdict = (decompositionVerdict || '').toUpperCase();
      parsed.pillarDecompositionRows = [];

      if (decompositionRows.length === 0) {
        errors.push('PILLAR_DECOMPOSITION must include at least one concrete row');
      }
      if (!/^(OK|NEEDS_STUBS|NEEDS_SPEC_UPDATE)$/i.test(decompositionVerdict || '')) {
        errors.push('PILLAR_DECOMPOSITION_VERDICT must be OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE');
      }

      let decompositionHasStub = false;
      let decompositionHasSpecUpdate = false;
      for (const row of decompositionRows) {
        const canonicalPillar = pillarLookup.get(row.pillar.toUpperCase());
        if (!canonicalPillar) {
          errors.push(`PILLAR_DECOMPOSITION references unknown pillar: ${row.pillar}`);
          continue;
        }
        decompositionPillarsSeen.add(canonicalPillar);
        if (isPlaceholderValue(row.capabilitySlice)) {
          errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} CAPABILITY_SLICE must be filled`);
        }
        if (isPlaceholderValue(row.subfeatures)) {
          errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} SUBFEATURES must be filled`);
        }
        if (isPlaceholderValue(row.notes)) {
          errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} NOTES must be filled`);
        }
        for (const engineId of row.mechanical.filter((value) => !/^NONE$/i.test(value))) {
          if (!specMechanicalEngineIds.has(engineId)) {
            errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} references unknown mechanical engine: ${engineId}`);
          }
        }

        let stubId = 'NONE';
        if (row.resolution === 'NEW_STUB') {
          decompositionHasStub = true;
          const stubIds = validateStubIds(row.stubRaw, errors, `PILLAR_DECOMPOSITION ${canonicalPillar} STUB`);
          if (stubIds.length !== 1) {
            errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} Resolution=NEW_STUB must point to exactly one stub packet`);
          }
          stubId = stubIds[0] || 'NONE';
          stubIds.forEach((id) => decompositionStubUnion.add(id));
        } else if (!/^NONE$/i.test(row.stubRaw || '')) {
          errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} must use STUB: NONE unless RESOLUTION=NEW_STUB`);
        }

        if (row.resolution === 'SPEC_UPDATE_NOW') {
          decompositionHasSpecUpdate = true;
          if (!parsed.appendixSpecUpdateRequired) {
            errors.push(`PILLAR_DECOMPOSITION ${canonicalPillar} uses RESOLUTION=SPEC_UPDATE_NOW but appendix maintenance did not declare a spec update`);
          }
        }

        parsed.pillarDecompositionRows.push(
          `PILLAR: ${canonicalPillar} | CAPABILITY_SLICE: ${row.capabilitySlice} | SUBFEATURES: ${row.subfeatures} | PRIMITIVES_FEATURES: ${row.primitivesFeaturesRaw} | MECHANICAL: ${row.mechanicalRaw} | ROI: ${row.roi} | RESOLUTION: ${row.resolution} | STUB: ${stubId} | NOTES: ${row.notes}`
        );
      }

      for (const pillar of parsed.pillarsTouched) {
        if (!decompositionPillarsSeen.has(pillar)) {
          errors.push(`PILLAR_DECOMPOSITION must include at least one row for touched pillar: ${pillar}`);
        }
      }
      for (const stubId of decompositionStubUnion) {
        if (!parsed.stubWpIds.includes(stubId)) {
          errors.push(`Top-level STUB_WP_IDS must include pillar-decomposition-linked stub ${stubId}`);
        }
      }

      if (decompositionHasSpecUpdate && parsed.pillarDecompositionVerdict !== 'NEEDS_SPEC_UPDATE') {
        errors.push('PILLAR_DECOMPOSITION_VERDICT must be NEEDS_SPEC_UPDATE when any row resolves to SPEC_UPDATE_NOW');
      } else if (!decompositionHasSpecUpdate && decompositionHasStub && parsed.pillarDecompositionVerdict !== 'NEEDS_STUBS') {
        errors.push('PILLAR_DECOMPOSITION_VERDICT must be NEEDS_STUBS when any row resolves to NEW_STUB and none resolve to SPEC_UPDATE_NOW');
      } else if (!decompositionHasSpecUpdate && !decompositionHasStub && parsed.pillarDecompositionVerdict !== 'OK') {
        errors.push('PILLAR_DECOMPOSITION_VERDICT must be OK when all rows resolve IN_THIS_WP');
      }

      const runtimeSection = extractSectionLinesByHeading(lines, 'EXECUTION_RUNTIME_ALIGNMENT');
      const runtimeRows = parseExecutionRuntimeAlignmentRows(runtimeSection.lines);
      const runtimeVerdict = getSingleField(content, 'EXECUTION_RUNTIME_ALIGNMENT_VERDICT');
      const runtimeStubUnion = new Set();
      parsed.executionRuntimeAlignmentVerdict = (runtimeVerdict || '').toUpperCase();
      parsed.executionRuntimeAlignmentRows = [];

      if (runtimeRows.length === 0) {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT must include at least one concrete row');
      }
      if (!/^(OK|NEEDS_STUBS|NEEDS_SPEC_UPDATE)$/i.test(runtimeVerdict || '')) {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT_VERDICT must be OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE');
      }

      let runtimeHasStub = false;
      let runtimeHasSpecUpdate = false;
      for (const row of runtimeRows) {
        if (isPlaceholderValue(row.capability)) {
          errors.push('EXECUTION_RUNTIME_ALIGNMENT Capability must be filled');
        }
        if (isPlaceholderValue(row.workflow)) {
          errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} Workflow must be filled`);
        }
        if (isPlaceholderValue(row.notes)) {
          errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} Notes must be filled`);
        }
        if (isPlaceholderValue(row.flightRecorder)) {
          errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} FlightRecorder must be filled (use NONE if not applicable)`);
        }

        let stubId = 'NONE';
        if (row.resolution === 'NEW_STUB') {
          runtimeHasStub = true;
          const stubIds = validateStubIds(row.stubRaw, errors, `EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} Stub`);
          if (stubIds.length !== 1) {
            errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} Resolution=NEW_STUB must point to exactly one stub packet`);
          }
          stubId = stubIds[0] || 'NONE';
          stubIds.forEach((id) => runtimeStubUnion.add(id));
        } else if (!/^NONE$/i.test(row.stubRaw || '')) {
          errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} must use Stub: NONE unless Resolution=NEW_STUB`);
        }

        if (row.resolution === 'SPEC_UPDATE_NOW') {
          runtimeHasSpecUpdate = true;
          if (!parsed.appendixSpecUpdateRequired) {
            errors.push(`EXECUTION_RUNTIME_ALIGNMENT ${row.capability || '<missing>'} uses Resolution=SPEC_UPDATE_NOW but appendix maintenance did not declare a spec update`);
          }
        }

        parsed.executionRuntimeAlignmentRows.push(
          `Capability: ${row.capability} | JobModel: ${row.jobModel} | Workflow: ${row.workflow} | ToolSurface: ${row.toolSurface} | ModelExposure: ${row.modelExposure} | CommandCenter: ${row.commandCenter} | FlightRecorder: ${row.flightRecorder} | Locus: ${row.locus} | StoragePosture: ${row.storagePosture} | Resolution: ${row.resolution} | Stub: ${stubId} | Notes: ${row.notes}`
        );
      }
      for (const stubId of runtimeStubUnion) {
        if (!parsed.stubWpIds.includes(stubId)) {
          errors.push(`Top-level STUB_WP_IDS must include execution-runtime-linked stub ${stubId}`);
        }
      }
      if (runtimeHasSpecUpdate && parsed.executionRuntimeAlignmentVerdict !== 'NEEDS_SPEC_UPDATE') {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT_VERDICT must be NEEDS_SPEC_UPDATE when any row resolves to SPEC_UPDATE_NOW');
      } else if (!runtimeHasSpecUpdate && runtimeHasStub && parsed.executionRuntimeAlignmentVerdict !== 'NEEDS_STUBS') {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT_VERDICT must be NEEDS_STUBS when any row resolves to NEW_STUB and none resolve to SPEC_UPDATE_NOW');
      } else if (!runtimeHasSpecUpdate && !runtimeHasStub && parsed.executionRuntimeAlignmentVerdict !== 'OK') {
        errors.push('EXECUTION_RUNTIME_ALIGNMENT_VERDICT must be OK when all rows resolve IN_THIS_WP');
      }
    }

    // PRIMITIVE_MATRIX minimums.
    const matrixTimebox = getSingleField(content, 'MATRIX_SCAN_TIMEBOX');
    if (isPlaceholderValue(matrixTimebox)) errors.push('PRIMITIVE_MATRIX MATRIX_SCAN_TIMEBOX must be filled');

    const matrixVerdict = getSingleField(content, 'PRIMITIVE_MATRIX_VERDICT');
    parsed.primitiveMatrixVerdict = (matrixVerdict || '').toUpperCase();
    if (!/^(OK|NEEDS_STUBS|NONE_FOUND)$/i.test(matrixVerdict || '')) {
      errors.push('PRIMITIVE_MATRIX_VERDICT must be OK | NEEDS_STUBS | NONE_FOUND');
    }

    const imxIdsRaw = getSingleField(content, 'IMX_EDGE_IDS_ADDED_OR_UPDATED');
    if (isPlaceholderValue(imxIdsRaw)) {
      errors.push('IMX_EDGE_IDS_ADDED_OR_UPDATED must be set (comma-separated IMX-... IDs or NONE)');
    }
    const imxIds = /^NONE$/i.test(imxIdsRaw || '')
      ? []
      : (imxIdsRaw || '').split(',').map((s) => s.trim()).filter(Boolean);

    if (!/^NONE_FOUND$/i.test(matrixVerdict || '') && imxIds.length === 0) {
      errors.push('PRIMITIVE_MATRIX requires IMX_EDGE_IDS_ADDED_OR_UPDATED to list one or more IMX-... IDs (or set PRIMITIVE_MATRIX_VERDICT=NONE_FOUND)');
    }
    if (/^NONE_FOUND$/i.test(matrixVerdict || '') && !/^NONE$/i.test(imxIdsRaw || '')) {
      errors.push('PRIMITIVE_MATRIX_VERDICT=NONE_FOUND requires IMX_EDGE_IDS_ADDED_OR_UPDATED=NONE');
    }

    for (const id of imxIds) {
      if (!/^IMX-\d{3,}$/i.test(id)) {
        errors.push(`IMX_EDGE_IDS_ADDED_OR_UPDATED contains invalid IMX edge id: ${id}`);
        continue;
      }
      if (specImxEdgeIds && !specImxEdgeIds.has(id)) {
        errors.push(`IMX edge id ${id} is not present in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX)`);
      }
    }

    const matrixEdges = lines.filter((l) => /^\s*-\s*Edge:\s*/i.test(l));
    if (!/^NONE_FOUND$/i.test(matrixVerdict || '') && matrixEdges.length === 0) {
      errors.push('PRIMITIVE_MATRIX must include at least one "- Edge: ..." entry (or set PRIMITIVE_MATRIX_VERDICT=NONE_FOUND with reason)');
    }
    if (/^NONE_FOUND$/i.test(matrixVerdict || '')) {
      const r = getSingleField(content, 'PRIMITIVE_MATRIX_REASON');
      if (isPlaceholderValue(r)) errors.push('PRIMITIVE_MATRIX_REASON is required when PRIMITIVE_MATRIX_VERDICT=NONE_FOUND');
    }
    if (/^NEEDS_STUBS$/i.test(matrixVerdict || '') && parsed.stubWpIds.length === 0) {
      errors.push('PRIMITIVE_MATRIX_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
    }
    if (isHydratedResearchProfile) {
      if (imxIds.length > 0 && parsed.interactionMatrixAction !== 'UPDATED') {
        errors.push('INTERACTION_MATRIX_ACTION must be UPDATED when IMX_EDGE_IDS_ADDED_OR_UPDATED lists one or more edges');
      }
      if (imxIds.length === 0 && parsed.interactionMatrixAction === 'UPDATED') {
        errors.push('INTERACTION_MATRIX_ACTION=UPDATED requires one or more IMX_EDGE_IDS_ADDED_OR_UPDATED');
      }

      const comboPressureMode = getSingleField(content, 'COMBO_PRESSURE_MODE');
      const forceMultiplierReason = getSingleField(content, 'FORCE_MULTIPLIER_REASON');
      const forceMultiplierVerdict = getSingleField(content, 'FORCE_MULTIPLIER_VERDICT');
      const candidates = parseForceMultiplierCandidates(lines);
      const touchedPillars = new Set(parsed.pillarsTouched);
      const touchedEngines = new Set(parsed.mechanicalEnginesTouched);
      const forceMultiplierStubUnion = new Set();
      const candidatePillarsSeen = new Set();
      const candidateEnginesSeen = new Set();
      const requiredCandidateCount = Math.max(3, Math.min(12, touchedPillars.size + touchedEngines.size));

      parsed.forceMultiplierVerdict = (forceMultiplierVerdict || '').toUpperCase();
      if (!/^AUTO$/i.test(comboPressureMode || '')) {
        errors.push('FORCE_MULTIPLIER_EXPANSION COMBO_PRESSURE_MODE must be AUTO');
      }
      if (!/^(OK|NEEDS_STUBS|NEEDS_SPEC_UPDATE)$/i.test(forceMultiplierVerdict || '')) {
        errors.push('FORCE_MULTIPLIER_VERDICT must be OK | NEEDS_STUBS | NEEDS_SPEC_UPDATE');
      }
      if (isPlaceholderValue(forceMultiplierReason)) {
        errors.push('FORCE_MULTIPLIER_REASON must be filled');
      }
      if (candidates.length < requiredCandidateCount) {
        errors.push(`FORCE_MULTIPLIER_EXPANSION must include at least ${requiredCandidateCount} candidates for the touched pillar/mechanical-engine surface area`);
      }

      let hasNewStubResolution = false;
      let hasSpecUpdateResolution = false;
      parsed.forceMultiplierResolutions = [];

      for (const candidate of candidates) {
        const pillarRefs = candidate.pillars.filter((value) => !/^NONE$/i.test(value));
        const mechanicalRefs = candidate.mechanical.filter((value) => !/^NONE$/i.test(value));
        const primitiveFeatureRefs = candidate.primitivesFeatures.filter((value) => !/^NONE$/i.test(value));

        if (isPlaceholderValue(candidate.combo)) {
          errors.push('FORCE_MULTIPLIER_EXPANSION candidate Combo must be filled');
        }
        if (isPlaceholderValue(candidate.notes)) {
          errors.push(`FORCE_MULTIPLIER_EXPANSION candidate "${candidate.combo || '<missing>'}" Notes must be filled`);
        }
        if (pillarRefs.length === 0 && mechanicalRefs.length === 0 && primitiveFeatureRefs.length === 0) {
          errors.push(`FORCE_MULTIPLIER_EXPANSION candidate "${candidate.combo || '<missing>'}" must reference at least one pillar, mechanical engine, or primitive/feature`);
        }

        for (const pillar of pillarRefs) {
          const canonicalPillar = pillarLookup.get(pillar.toUpperCase());
          if (!canonicalPillar) {
            errors.push(`FORCE_MULTIPLIER_EXPANSION candidate "${candidate.combo || '<missing>'}" references unknown pillar: ${pillar}`);
          } else {
            candidatePillarsSeen.add(canonicalPillar);
          }
        }
        for (const engineId of mechanicalRefs) {
          if (!specMechanicalEngineIds.has(engineId)) {
            errors.push(`FORCE_MULTIPLIER_EXPANSION candidate "${candidate.combo || '<missing>'}" references unknown mechanical engine: ${engineId}`);
          } else {
            candidateEnginesSeen.add(engineId);
          }
        }

        let candidateStubId = 'NONE';
        if (candidate.resolution === 'NEW_STUB') {
          hasNewStubResolution = true;
          const stubIds = validateStubIds(candidate.stubRaw, errors, `FORCE_MULTIPLIER candidate "${candidate.combo || '<missing>'}" Stub`);
          if (stubIds.length !== 1) {
            errors.push(`FORCE_MULTIPLIER candidate "${candidate.combo || '<missing>'}" with Resolution=NEW_STUB must point to exactly one stub packet`);
          }
          candidateStubId = stubIds[0] || 'NONE';
          stubIds.forEach((id) => forceMultiplierStubUnion.add(id));
        } else if (!/^NONE$/i.test(candidate.stubRaw || '')) {
          errors.push(`FORCE_MULTIPLIER candidate "${candidate.combo || '<missing>'}" must use Stub: NONE unless Resolution=NEW_STUB`);
        }

        if (candidate.resolution === 'SPEC_UPDATE_NOW') {
          hasSpecUpdateResolution = true;
          if (!parsed.appendixSpecUpdateRequired) {
            errors.push(`FORCE_MULTIPLIER candidate "${candidate.combo || '<missing>'}" uses Resolution=SPEC_UPDATE_NOW but appendix maintenance did not declare a spec update`);
          }
        }

        parsed.forceMultiplierResolutions.push(`${candidate.combo} -> ${candidate.resolution} (stub: ${candidateStubId})`);
      }

      for (const pillar of touchedPillars) {
        if (!candidatePillarsSeen.has(pillar)) {
          errors.push(`FORCE_MULTIPLIER_EXPANSION must include at least one candidate for touched pillar: ${pillar}`);
        }
      }
      for (const engineId of touchedEngines) {
        if (!candidateEnginesSeen.has(engineId)) {
          errors.push(`FORCE_MULTIPLIER_EXPANSION must include at least one candidate for touched mechanical engine: ${engineId}`);
        }
      }
      for (const stubId of forceMultiplierStubUnion) {
        if (!parsed.stubWpIds.includes(stubId)) {
          errors.push(`Top-level STUB_WP_IDS must include force-multiplier-linked stub ${stubId}`);
        }
      }

      if (hasSpecUpdateResolution && parsed.forceMultiplierVerdict !== 'NEEDS_SPEC_UPDATE') {
        errors.push('FORCE_MULTIPLIER_VERDICT must be NEEDS_SPEC_UPDATE when any candidate resolves to SPEC_UPDATE_NOW');
      } else if (!hasSpecUpdateResolution && hasNewStubResolution && parsed.forceMultiplierVerdict !== 'NEEDS_STUBS') {
        errors.push('FORCE_MULTIPLIER_VERDICT must be NEEDS_STUBS when any candidate resolves to NEW_STUB and none resolve to SPEC_UPDATE_NOW');
      } else if (!hasSpecUpdateResolution && !hasNewStubResolution && parsed.forceMultiplierVerdict !== 'OK') {
        errors.push('FORCE_MULTIPLIER_VERDICT must be OK when all candidates resolve IN_THIS_WP');
      }
      if (parsed.forceMultiplierVerdict === 'NEEDS_STUBS' && parsed.stubWpIds.length === 0) {
        errors.push('FORCE_MULTIPLIER_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
      }
      if (parsed.forceMultiplierVerdict === 'NEEDS_SPEC_UPDATE' && !parsed.appendixSpecUpdateRequired) {
        errors.push('FORCE_MULTIPLIER_VERDICT=NEEDS_SPEC_UPDATE requires appendix maintenance to declare a spec update');
      }
    }

    if (isHydratedResearchProfile) {
      const scanScope = getSingleField(content, 'SCAN_SCOPE');
      const currentUiApplicable = getSingleField(content, 'UI_UX_APPLICABLE');
      const stubMatches = parseExistingCapabilityRows(lines, 'MATCHED_STUBS');
      const activeMatches = parseExistingCapabilityRows(lines, 'MATCHED_ACTIVE_PACKETS');
      const completedMatches = parseExistingCapabilityRows(lines, 'MATCHED_COMPLETED_PACKETS');
      const codeRealityEvidence = parseCodeRealityEvidence(lines);
      const alignmentVerdict = getSingleField(content, 'EXISTING_CAPABILITY_ALIGNMENT_VERDICT');
      const alignmentReason = getSingleField(content, 'EXISTING_CAPABILITY_ALIGNMENT_REASON');
      const taskBoardMapResult = readTaskBoardStatusMap();
      const taskBoardStatuses = taskBoardMapResult.ok ? taskBoardMapResult.statuses : new Map();
      const needsEvidenceArtifacts = new Set();
      const allArtifacts = new Set();
      const allRows = [];

      parsed.existingCapabilityAlignmentVerdict = (alignmentVerdict || '').toUpperCase();

      if (isPlaceholderValue(scanScope)) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT SCAN_SCOPE must be filled');
      }
      if (!/^(OK|REUSE_EXISTING|NEEDS_SCOPE_EXPANSION|NEEDS_STUBS|NEEDS_SPEC_UPDATE)$/i.test(alignmentVerdict || '')) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT_VERDICT must be OK | REUSE_EXISTING | NEEDS_SCOPE_EXPANSION | NEEDS_STUBS | NEEDS_SPEC_UPDATE');
      }
      if (isPlaceholderValue(alignmentReason)) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT_REASON must be filled');
      }
      if (!taskBoardMapResult.ok) {
        errors.push(taskBoardMapResult.error);
      }

      const matchGroups = [
        { label: 'MATCHED_STUBS', parsedMatches: stubMatches, allowedStatuses: new Set(['STUB']), expectsStubFile: true, expectsOfficialPacket: false, codeRealityAllowed: new Set(['N/A']) },
        { label: 'MATCHED_ACTIVE_PACKETS', parsedMatches: activeMatches, allowedStatuses: new Set(['READY_FOR_DEV', 'IN_PROGRESS', 'BLOCKED']), expectsStubFile: false, expectsOfficialPacket: true, codeRealityAllowed: new Set(['PARTIAL', 'NOT_PRESENT', 'N/A']) },
        { label: 'MATCHED_COMPLETED_PACKETS', parsedMatches: completedMatches, allowedStatuses: new Set(['VALIDATED', 'OUTDATED_ONLY', 'FAIL', 'SUPERSEDED']), expectsStubFile: false, expectsOfficialPacket: true, codeRealityAllowed: new Set(['IMPLEMENTED', 'PARTIAL', 'NOT_PRESENT']) },
      ];

      for (const group of matchGroups) {
        if (!group.parsedMatches.found) {
          errors.push(`EXISTING_CAPABILITY_ALIGNMENT must include ${group.label} (use NONE if there are no matches)`);
          continue;
        }
        if (!group.parsedMatches.hasNone && group.parsedMatches.rows.length !== group.parsedMatches.rawItems.length) {
          errors.push(`${group.label} contains malformed entries; each item must match the required Artifact|BoardStatus|Intent|PrimitiveIndex|Matrix|UI|CodeReality|Resolution|Stub|Notes format`);
        }
        for (const row of group.parsedMatches.rows) {
          allRows.push(row);
          allArtifacts.add(row.artifact);
          if (!group.allowedStatuses.has(row.boardStatus)) {
            errors.push(`${group.label} ${row.artifact} has invalid BoardStatus ${row.boardStatus}`);
          }
          if (taskBoardStatuses.size > 0) {
            const actualStatus = taskBoardStatuses.get(row.artifact);
            if (!actualStatus) {
              errors.push(`${group.label} references ${row.artifact} but it is missing from ${TASK_BOARD_PATH.replace(/\\/g, '/')}`);
            } else if (actualStatus !== row.boardStatus) {
              errors.push(`${group.label} ${row.artifact} BoardStatus drifted from TASK_BOARD: expected ${actualStatus}, got ${row.boardStatus}`);
            }
          }
          if (group.expectsStubFile) {
            const stubPath = path.join('.GOV', 'task_packets', 'stubs', `${row.artifact}.md`);
            if (!fs.existsSync(stubPath)) {
              errors.push(`${group.label} ${row.artifact} is missing stub file ${stubPath.replace(/\\/g, '/')}`);
            }
          }
          if (group.expectsOfficialPacket) {
            const packetPath = path.join('.GOV', 'task_packets', `${row.artifact}.md`);
            if (!fs.existsSync(packetPath)) {
              errors.push(`${group.label} ${row.artifact} is missing official task packet ${packetPath.replace(/\\/g, '/')}`);
            }
          }
          if (!group.codeRealityAllowed.has(row.codeReality)) {
            errors.push(`${group.label} ${row.artifact} has invalid CodeReality ${row.codeReality} for that artifact class`);
          }
          if (isPlaceholderValue(row.notes)) {
            errors.push(`${group.label} ${row.artifact} Notes must be filled`);
          }

          if (row.resolution === 'NEW_STUB') {
            const stubIds = validateStubIds(row.stubRaw, errors, `${group.label} ${row.artifact} Stub`);
            if (stubIds.length !== 1) {
              errors.push(`${group.label} ${row.artifact} with Resolution=NEW_STUB must point to exactly one stub packet`);
            }
            for (const stubId of stubIds) {
              if (!parsed.stubWpIds.includes(stubId)) {
                errors.push(`Top-level STUB_WP_IDS must include reuse-alignment stub ${stubId}`);
              }
            }
          } else if (!/^NONE$/i.test(row.stubRaw || '')) {
            errors.push(`${group.label} ${row.artifact} must use Stub: NONE unless Resolution=NEW_STUB`);
          }

          if (row.resolution === 'SPEC_UPDATE_NOW' && !parsed.appendixSpecUpdateRequired) {
            errors.push(`${group.label} ${row.artifact} uses Resolution=SPEC_UPDATE_NOW but appendix maintenance did not declare a spec update`);
          }
          if (row.resolution === 'REUSE_EXISTING') {
            if (row.intent === 'DISTINCT') {
              errors.push(`${group.label} ${row.artifact} cannot use Resolution=REUSE_EXISTING with Intent=DISTINCT`);
            }
            if (row.primitiveIndex === 'MISSING') {
              errors.push(`${group.label} ${row.artifact} cannot use Resolution=REUSE_EXISTING when PrimitiveIndex=MISSING`);
            }
            if (row.matrix === 'MISSING') {
              errors.push(`${group.label} ${row.artifact} cannot use Resolution=REUSE_EXISTING when Matrix=MISSING`);
            }
            if (row.ui === 'PARTIAL' || row.ui === 'NONE') {
              errors.push(`${group.label} ${row.artifact} cannot use Resolution=REUSE_EXISTING when UI is PARTIAL or NONE`);
            }
            if (group.label === 'MATCHED_COMPLETED_PACKETS' && row.codeReality !== 'IMPLEMENTED') {
              errors.push(`${group.label} ${row.artifact} requires CodeReality=IMPLEMENTED for Resolution=REUSE_EXISTING`);
            }
          }
          if (row.resolution === 'KEEP_SEPARATE' && row.intent === 'SAME') {
            errors.push(`${group.label} ${row.artifact} cannot use Resolution=KEEP_SEPARATE with Intent=SAME`);
          }
          if ((group.label === 'MATCHED_STUBS' || group.label === 'MATCHED_ACTIVE_PACKETS') && row.intent === 'SAME' && row.resolution === 'NEW_STUB') {
            errors.push(`${group.label} ${row.artifact} cannot create a NEW_STUB when the same-intent capability already has a tracked governance artifact`);
          }
          if (
            group.label === 'MATCHED_COMPLETED_PACKETS'
            && row.intent === 'SAME'
            && row.codeReality === 'IMPLEMENTED'
            && row.primitiveIndex === 'COVERED'
            && row.matrix !== 'MISSING'
            && row.ui !== 'PARTIAL'
            && row.ui !== 'NONE'
            && row.resolution !== 'REUSE_EXISTING'
          ) {
            errors.push(`${group.label} ${row.artifact} already covers the same intent in code/spec/UI; Resolution must be REUSE_EXISTING`);
          }
          if (group.label === 'MATCHED_COMPLETED_PACKETS' && (row.intent === 'SAME' || row.resolution === 'REUSE_EXISTING')) {
            needsEvidenceArtifacts.add(row.artifact);
          }
        }
      }

      const evidenceRows = codeRealityEvidence.rows || [];
      const evidenceByArtifact = new Map();
      if (!codeRealityEvidence.found) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT must include CODE_REALITY_EVIDENCE (use NONE if there is no applicable code evidence)');
      } else if (!codeRealityEvidence.hasNone && evidenceRows.length !== codeRealityEvidence.rawItems.length) {
        errors.push('CODE_REALITY_EVIDENCE contains malformed entries; each item must match the required Path|Artifact|Covers|Verdict|Notes format');
      }

      for (const row of evidenceRows) {
        const evidencePath = path.normalize(row.pathRaw);
        if (isPlaceholderValue(row.pathRaw)) {
          errors.push('CODE_REALITY_EVIDENCE Path must be filled');
        } else if (!fs.existsSync(evidencePath)) {
          errors.push(`CODE_REALITY_EVIDENCE path does not exist: ${row.pathRaw.replace(/\\/g, '/')}`);
        }
        if (row.artifact !== 'NONE' && !allArtifacts.has(row.artifact)) {
          errors.push(`CODE_REALITY_EVIDENCE references unknown Artifact: ${row.artifact}`);
        }
        if (isPlaceholderValue(row.notes)) {
          errors.push(`CODE_REALITY_EVIDENCE ${row.pathRaw} Notes must be filled`);
        }
        if (row.artifact !== 'NONE') {
          const existing = evidenceByArtifact.get(row.artifact) || [];
          existing.push(row);
          evidenceByArtifact.set(row.artifact, existing);
        }
        parsed.codeRealitySummary.push(`${row.pathRaw} -> ${row.verdict} (${row.artifact})`);
      }

      for (const artifact of needsEvidenceArtifacts) {
        const evidence = evidenceByArtifact.get(artifact) || [];
        if (evidence.length === 0) {
          errors.push(`MATCHED_COMPLETED_PACKETS ${artifact} requires CODE_REALITY_EVIDENCE because it claims SAME intent or REUSE_EXISTING`);
        } else if (!evidence.some((row) => row.verdict === 'IMPLEMENTED')) {
          errors.push(`MATCHED_COMPLETED_PACKETS ${artifact} requires at least one CODE_REALITY_EVIDENCE entry with Verdict=IMPLEMENTED`);
        }
      }

      const anyPrimitiveMissing = allRows.some((row) => row.primitiveIndex === 'MISSING' && row.resolution !== 'KEEP_SEPARATE');
      const anyMatrixMissing = allRows.some((row) => row.matrix === 'MISSING' && row.resolution !== 'KEEP_SEPARATE');
      const anyUiMissing = allRows.some((row) => (row.ui === 'PARTIAL' || row.ui === 'NONE') && row.resolution !== 'KEEP_SEPARATE');
      if (anyPrimitiveMissing && parsed.primitiveIndexAction !== 'UPDATED') {
        errors.push('Existing capability alignment found PrimitiveIndex=MISSING on a non-separated match; PRIMITIVE_INDEX_ACTION must be UPDATED');
      }
      if (anyMatrixMissing && parsed.interactionMatrixAction !== 'UPDATED') {
        errors.push('Existing capability alignment found Matrix=MISSING on a non-separated match; INTERACTION_MATRIX_ACTION must be UPDATED');
      }
      if (anyUiMissing && /^YES$/i.test(currentUiApplicable || '') && parsed.uiGuidanceAction === 'NO_CHANGE') {
        errors.push('Existing capability alignment found missing/partial same-intent UI coverage; UI_GUIDANCE_ACTION cannot stay NO_CHANGE when UI_UX_APPLICABLE=YES');
      }

      parsed.matchedArtifactResolutions = allRows.map((row) => `${row.artifact} -> ${row.resolution}`);

      const derivedAlignmentVerdict =
        allRows.some((row) => row.resolution === 'SPEC_UPDATE_NOW') ? 'NEEDS_SPEC_UPDATE'
          : allRows.some((row) => row.resolution === 'NEW_STUB') ? 'NEEDS_STUBS'
            : allRows.some((row) => row.resolution === 'EXPAND_IN_THIS_WP') ? 'NEEDS_SCOPE_EXPANSION'
              : allRows.some((row) => row.resolution === 'REUSE_EXISTING') ? 'REUSE_EXISTING'
                : 'OK';

      if (parsed.existingCapabilityAlignmentVerdict !== derivedAlignmentVerdict) {
        errors.push(`EXISTING_CAPABILITY_ALIGNMENT_VERDICT must be ${derivedAlignmentVerdict} based on the listed artifact resolutions`);
      }
      if (parsed.existingCapabilityAlignmentVerdict === 'NEEDS_STUBS' && parsed.stubWpIds.length === 0) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
      }
      if (parsed.existingCapabilityAlignmentVerdict === 'NEEDS_SPEC_UPDATE' && !parsed.appendixSpecUpdateRequired) {
        errors.push('EXISTING_CAPABILITY_ALIGNMENT_VERDICT=NEEDS_SPEC_UPDATE requires appendix maintenance to declare a spec update');
      }
    }

    // UI_UX_RUBRIC minimums.
    const uiApplicable = getSingleField(content, 'UI_UX_APPLICABLE');
    const uiVerdict = getSingleField(content, 'UI_UX_VERDICT');
    parsed.uiApplicable = (uiApplicable || '').toUpperCase();
    parsed.uiVerdict = (uiVerdict || '').toUpperCase();
    if (!/^(YES|NO)$/i.test(uiApplicable || '')) errors.push('UI_UX_APPLICABLE must be YES or NO');
    if (/^NO$/i.test(uiApplicable || '')) {
      const r = getSingleField(content, 'UI_UX_REASON_NO');
      if (isPlaceholderValue(r)) errors.push('UI_UX_REASON_NO is required when UI_UX_APPLICABLE=NO');
    } else if (/^YES$/i.test(uiApplicable || '')) {
      const surfaces = extractIndentedListAfterLabel(lines, 'UI_SURFACES');
      const controls = extractIndentedListAfterLabel(lines, 'UI_CONTROLS (buttons/dropdowns/inputs)');
      const states = extractIndentedListAfterLabel(lines, 'UI_STATES (empty/loading/error)');
      const microcopy = extractIndentedListAfterLabel(lines, 'UI_MICROCOPY_NOTES (labels, helper text, hover explainers)');
      const accessibility = extractIndentedListAfterLabel(lines, 'UI_ACCESSIBILITY_NOTES');

      parsed.uiSpec.surfaces = surfaces.items.filter((s) => !/^NONE$/i.test(s));
      parsed.uiSpec.controls = controls.items.filter((s) => !/^NONE$/i.test(s));
      parsed.uiSpec.states = states.items.filter((s) => !/^NONE$/i.test(s));
      parsed.uiSpec.microcopy = microcopy.items.filter((s) => !/^NONE$/i.test(s));
      parsed.uiSpec.accessibility = accessibility.items.filter((s) => !/^NONE$/i.test(s));

      if (!surfaces.found || surfaces.items.filter((s) => !isPlaceholderValue(s)).length === 0) {
        errors.push('UI_UX_RUBRIC UI_SURFACES must include at least one concrete surface');
      }
      if (!controls.found || controls.items.filter((s) => !isPlaceholderValue(s)).length === 0) {
        errors.push('UI_UX_RUBRIC UI_CONTROLS must include at least one concrete control');
      } else {
        const anyTooltip = controls.items.some((s) => /\bTooltip:\b/i.test(s) && !/Tooltip:\s*<fill/i.test(s));
        if (!anyTooltip) errors.push('UI_UX_RUBRIC UI_CONTROLS entries must include concrete Tooltip: text');
      }
      if (!states.found || states.items.length === 0) {
        errors.push('UI_UX_RUBRIC UI_STATES must be filled (use NONE only if truly not applicable)');
      }
      if (!microcopy.found || microcopy.items.length === 0) {
        errors.push('UI_UX_RUBRIC UI_MICROCOPY_NOTES must be filled (use NONE only if truly not applicable)');
      }
      if (!accessibility.found || accessibility.items.length === 0) {
        errors.push('UI_UX_RUBRIC UI_ACCESSIBILITY_NOTES must be filled');
      }
      if (!/^(OK|NEEDS_STUBS|UNKNOWN)$/i.test(uiVerdict || '')) {
        errors.push('UI_UX_VERDICT must be OK | NEEDS_STUBS | UNKNOWN');
      }
    }
    if ((/^NEEDS_STUBS$/i.test(uiVerdict || '') || /^UNKNOWN$/i.test(uiVerdict || '')) && parsed.stubWpIds.length === 0) {
      errors.push('UI_UX_VERDICT requiring follow-up (NEEDS_STUBS|UNKNOWN) requires top-level STUB_WP_IDS to list one or more stub packets');
    }
    if (isHydratedResearchProfile && /^YES$/i.test(uiApplicable || '') && parsed.uiGuidanceAction === 'NOT_APPLICABLE') {
      errors.push('UI_GUIDANCE_ACTION cannot be NOT_APPLICABLE when UI_UX_APPLICABLE=YES');
    }

    if (isHydratedResearchProfile) {
      const hydration = parsed.packetHydration;
      parsed.packetHydrationProfile = getSingleField(content, 'PACKET_HYDRATION_PROFILE');
      if (!/^HYDRATED_RESEARCH_V1$/i.test(parsed.packetHydrationProfile || '')) {
        errors.push('PACKET_HYDRATION_PROFILE must be HYDRATED_RESEARCH_V1');
      }

      hydration.requestor = getSingleField(content, 'REQUESTOR');
      hydration.agentId = getSingleField(content, 'AGENT_ID');
      hydration.riskTier = getSingleField(content, 'RISK_TIER');
      hydration.buildOrderDomain = getSingleField(content, 'BUILD_ORDER_DOMAIN');
      hydration.buildOrderTechBlocker = getSingleField(content, 'BUILD_ORDER_TECH_BLOCKER');
      hydration.buildOrderValueTier = getSingleField(content, 'BUILD_ORDER_VALUE_TIER');
      hydration.buildOrderDependsOnRaw = getSingleField(content, 'BUILD_ORDER_DEPENDS_ON');
      hydration.buildOrderBlocksRaw = getSingleField(content, 'BUILD_ORDER_BLOCKS');
      hydration.buildOrderDependsOn = validateBaseWpIds(hydration.buildOrderDependsOnRaw, errors, 'PACKET_HYDRATION BUILD_ORDER_DEPENDS_ON');
      hydration.buildOrderBlocks = validateBaseWpIds(hydration.buildOrderBlocksRaw, errors, 'PACKET_HYDRATION BUILD_ORDER_BLOCKS');
      hydration.specAnchorPrimary = getSingleField(content, 'SPEC_ANCHOR_PRIMARY');
      hydration.what = getSingleField(content, 'WHAT');
      hydration.why = getSingleField(content, 'WHY');
      hydration.inScopePaths = extractIndentedListAfterLabel(lines, 'IN_SCOPE_PATHS').items.filter((s) => !/^NONE$/i.test(s));
      hydration.outOfScope = extractIndentedListAfterLabel(lines, 'OUT_OF_SCOPE').items.filter((s) => !/^NONE$/i.test(s));
      hydration.testPlan = extractFencedBlockAfterLabel(lines, 'TEST_PLAN').body;
      hydration.doneMeans = extractIndentedListAfterLabel(lines, 'DONE_MEANS').items.filter((s) => !/^NONE$/i.test(s));
      hydration.filesToOpen = extractIndentedListAfterLabel(lines, 'FILES_TO_OPEN').items.filter((s) => !/^NONE$/i.test(s));
      hydration.searchTerms = extractIndentedListAfterLabel(lines, 'SEARCH_TERMS').items.filter((s) => !/^NONE$/i.test(s));
      hydration.runCommands = extractFencedBlockAfterLabel(lines, 'RUN_COMMANDS').body;
      hydration.riskMap = extractIndentedListAfterLabel(lines, 'RISK_MAP').items.filter((s) => !/^NONE$/i.test(s));

      if (isPlaceholderValue(hydration.requestor)) errors.push('PACKET_HYDRATION REQUESTOR must be filled');
      if (isPlaceholderValue(hydration.agentId)) errors.push('PACKET_HYDRATION AGENT_ID must be filled');
      if (!/^(LOW|MEDIUM|HIGH)$/i.test(hydration.riskTier || '')) errors.push('PACKET_HYDRATION RISK_TIER must be LOW | MEDIUM | HIGH');
      if (!/^(BACKEND|FRONTEND|GOV|CROSS_BOUNDARY)$/i.test(hydration.buildOrderDomain || '')) errors.push('PACKET_HYDRATION BUILD_ORDER_DOMAIN must be BACKEND | FRONTEND | GOV | CROSS_BOUNDARY');
      if (!/^(YES|NO)$/i.test(hydration.buildOrderTechBlocker || '')) errors.push('PACKET_HYDRATION BUILD_ORDER_TECH_BLOCKER must be YES or NO');
      if (!/^(LOW|MEDIUM|HIGH)$/i.test(hydration.buildOrderValueTier || '')) errors.push('PACKET_HYDRATION BUILD_ORDER_VALUE_TIER must be LOW | MEDIUM | HIGH');
      if (isPlaceholderValue(hydration.specAnchorPrimary)) errors.push('PACKET_HYDRATION SPEC_ANCHOR_PRIMARY must be filled');
      if (isPlaceholderValue(hydration.what)) errors.push('PACKET_HYDRATION WHAT must be filled');
      if (isPlaceholderValue(hydration.why)) errors.push('PACKET_HYDRATION WHY must be filled');
      if (hydration.inScopePaths.length === 0) errors.push('PACKET_HYDRATION IN_SCOPE_PATHS must list one or more concrete paths');
      if (hydration.outOfScope.length === 0) errors.push('PACKET_HYDRATION OUT_OF_SCOPE must list one or more concrete exclusions');
      if (isPlaceholderValue(hydration.testPlan)) errors.push('PACKET_HYDRATION TEST_PLAN must contain a fenced command block');
      if (hydration.doneMeans.filter((s) => !isPlaceholderValue(s)).length === 0) errors.push('PACKET_HYDRATION DONE_MEANS must list one or more measurable criteria');
      if (hydration.filesToOpen.filter((s) => !isPlaceholderValue(s)).length === 0) errors.push('PACKET_HYDRATION FILES_TO_OPEN must list one or more files');
      if (hydration.searchTerms.filter((s) => !isPlaceholderValue(s)).length === 0) errors.push('PACKET_HYDRATION SEARCH_TERMS must list one or more concrete search terms');
      if (isPlaceholderValue(hydration.runCommands)) errors.push('PACKET_HYDRATION RUN_COMMANDS must contain a fenced command block');
      if (hydration.riskMap.filter((s) => !isPlaceholderValue(s)).length === 0) errors.push('PACKET_HYDRATION RISK_MAP must list one or more concrete risk mappings');
    }

    if (/^NEEDS_STUBS$/i.test(parsed.appendixMaintenanceVerdict || '') && parsed.stubWpIds.length === 0) {
      errors.push('APPENDIX_MAINTENANCE_VERDICT=NEEDS_STUBS requires top-level STUB_WP_IDS to list one or more stub packets');
    }
  }

  // Clearly covers / enrichment fields must be filled before signature.
  const clearlyVerdict = getSingleField(content, 'CLEARLY_COVERS_VERDICT');
  if (!/^(PASS|FAIL)$/i.test(clearlyVerdict || '')) {
    errors.push('CLEARLY_COVERS_VERDICT must be PASS or FAIL (no PENDING placeholders)');
  }
  const clearlyReason = getSingleField(content, 'CLEARLY_COVERS_REASON');
  if (isPlaceholderValue(clearlyReason)) {
    errors.push('CLEARLY_COVERS_REASON must be filled (no placeholders)');
  }

  const enrichmentNeeded = getSingleField(content, 'ENRICHMENT_NEEDED');
  if (!/^(YES|NO)$/i.test(enrichmentNeeded || '')) {
    errors.push('ENRICHMENT_NEEDED must be YES or NO (no PENDING placeholders)');
  }

  // Deterministic cross-field consistency: "clearly covers" vs "enrichment needed"
  if (/^PASS$/i.test(clearlyVerdict) && /^YES$/i.test(enrichmentNeeded) && !parsed.appendixSpecUpdateRequired) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=PASS requires ENRICHMENT_NEEDED=NO unless appendix/spec-update maintenance is required');
  }
  if (/^FAIL$/i.test(clearlyVerdict) && /^NO$/i.test(enrichmentNeeded)) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=FAIL requires ENRICHMENT_NEEDED=YES');
  }
  if (parsed.appendixSpecUpdateRequired && !/^YES$/i.test(enrichmentNeeded || '')) {
    errors.push('Appendix actions marked UPDATED require ENRICHMENT_NEEDED=YES so packet creation stays blocked until the new spec version exists');
  }
  if (!parsed.appendixSpecUpdateRequired && /^NEEDS_SPEC_UPDATE$/i.test(parsed.appendixMaintenanceVerdict || '') && !/^YES$/i.test(enrichmentNeeded || '')) {
    errors.push('APPENDIX_MAINTENANCE_VERDICT=NEEDS_SPEC_UPDATE requires ENRICHMENT_NEEDED=YES');
  }

  // Optional, but if present it must be consistent (prevents "PASS but ambiguous" procedure failures).
  const ambiguityFoundLinePresent = /^\s*-\s*AMBIGUITY_FOUND\s*:/mi.test(content);
  const ambiguityFound = ambiguityFoundLinePresent ? getSingleField(content, 'AMBIGUITY_FOUND') : '';
  if (ambiguityFoundLinePresent && !/^(YES|NO)$/i.test(ambiguityFound || '')) {
    errors.push('AMBIGUITY_FOUND must be YES or NO (no PENDING placeholders)');
  }
  if (/^YES$/i.test(ambiguityFound)) {
    if (!/^FAIL$/i.test(clearlyVerdict)) {
      errors.push('AMBIGUITY_FOUND=YES requires CLEARLY_COVERS_VERDICT=FAIL');
    }
    if (!/^YES$/i.test(enrichmentNeeded)) {
      errors.push('AMBIGUITY_FOUND=YES requires ENRICHMENT_NEEDED=YES');
    }
  }

  // Proposed spec enrichment block: enforce consistency when present.
  const proposedViaLabelPrimary = extractFencedBlockAfterLabel(lines, 'PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)');
  const proposedViaLabelSecondary = extractFencedBlockAfterLabel(lines, 'PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates)');
  const proposedViaLabel = proposedViaLabelPrimary.found ? proposedViaLabelPrimary : proposedViaLabelSecondary;
  const proposedViaHeading = extractFencedBlockAfterHeading(lines, 'PROPOSED_SPEC_ENRICHMENT');
  const proposedFound = proposedViaLabel.found || proposedViaHeading.found;
  const proposedBody = (proposedViaLabel.found ? proposedViaLabel.body : '') || (proposedViaHeading.found ? proposedViaHeading.body : '') || '';

  if (/^NO$/i.test(enrichmentNeeded)) {
    const reasonNo = getSingleField(content, 'REASON_NO_ENRICHMENT');
    if (isPlaceholderValue(reasonNo)) {
      errors.push('REASON_NO_ENRICHMENT is required when ENRICHMENT_NEEDED=NO');
    }

    if (proposedFound && !looksLikeNotApplicableBlock(proposedBody)) {
      errors.push('PROPOSED_SPEC_ENRICHMENT must be "<not applicable; ENRICHMENT_NEEDED=NO>" when ENRICHMENT_NEEDED=NO');
    }
  } else if (/^YES$/i.test(enrichmentNeeded)) {
    if (!proposedFound || looksLikeNotApplicableBlock(proposedBody) || looksLikePlaceholderEnrichment(proposedBody)) {
      errors.push('PROPOSED_SPEC_ENRICHMENT must contain full verbatim Markdown when ENRICHMENT_NEEDED=YES');
    }
  }

  // Anchors: must exist and be filled + token-in-window.
  const anchors = parseAnchors(content);
  if (anchors.length === 0) {
    errors.push('SPEC_ANCHORS missing: include one or more ANCHOR sections');
  }

  let specLines = null;
  if (resolved) {
    specLines = fs.readFileSync(resolved.specFilePath, 'utf8').split('\n');
  }

  anchors.forEach((a, idx) => {
    const n = idx + 1;
    if (isPlaceholderValue(a.specAnchor)) errors.push(`ANCHOR ${n}: SPEC_ANCHOR must be filled`);
    if (isPlaceholderValue(a.contextToken)) errors.push(`ANCHOR ${n}: CONTEXT_TOKEN must be filled`);

    const startNum = parseInt(a.contextStartLine, 10);
    const endNum = parseInt(a.contextEndLine, 10);
    if (Number.isNaN(startNum) || Number.isNaN(endNum) || startNum < 1 || endNum < startNum) {
      errors.push(`ANCHOR ${n}: CONTEXT_START_LINE/CONTEXT_END_LINE must be integers with start>=1 and end>=start`);
    } else if (specLines) {
      if (endNum > specLines.length) {
        errors.push(`ANCHOR ${n}: CONTEXT_END_LINE out of range (spec has ${specLines.length} lines)`);
      } else {
        const window = specLines.slice(startNum - 1, endNum).join('\n');
        if (!window.includes(a.contextToken)) {
          errors.push(`ANCHOR ${n}: CONTEXT_TOKEN not found within spec line window [${startNum}, ${endNum}]`);
        }
      }
    }

    if (isPlaceholderValue(a.excerpt)) errors.push(`ANCHOR ${n}: EXCERPT_ASCII_ESCAPED must be filled`);
  });
  // Explicit user approval evidence line (deterministic).
  // Enforced for modern refinements when requireSignature=true.
  if (requireSignature && isModernRefinement) {
    const approvalEvidence = getSingleField(content, 'USER_APPROVAL_EVIDENCE');
    if (isPlaceholderValue(approvalEvidence)) {
      errors.push('USER_APPROVAL_EVIDENCE must be set (not <pending>) before signature/packet creation');
    } else {
      const expected = 'APPROVE REFINEMENT ' + wpId;
      if (approvalEvidence !== expected) {
        errors.push('USER_APPROVAL_EVIDENCE must equal ' + expected);
      }
    }
  }

  const reviewStatus = getSingleField(content, 'USER_REVIEW_STATUS');
  const signature = getSingleField(content, 'USER_SIGNATURE');
  parsed.reviewStatus = reviewStatus;
  parsed.signature = signature;
  if (requireSignature) {
    if (!/^(APPROVED)$/i.test(reviewStatus || '')) {
      errors.push('USER_REVIEW_STATUS must be APPROVED before task packet creation');
    }
    if (!signature || signature === '<pending>') {
      errors.push('USER_SIGNATURE must be set (not <pending>) before task packet creation');
    }
  }

  return { ok: errors.length === 0, errors, parsed };
}
