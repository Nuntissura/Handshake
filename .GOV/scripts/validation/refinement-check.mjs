import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

const SPEC_CURRENT_PATH = path.join('.GOV', 'roles_shared', 'SPEC_CURRENT.md');

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
    researchSources: [],
    researchSynthesis: [],
    primitivesTouched: [],
    pillarsTouched: [],
    pillarsRequiringStubs: [],
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
    requiredSections.splice(6, 0, 'APPENDIX_MAINTENANCE');
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

      parsed.researchCurrencyRequired = (researchRequired || '').toUpperCase();
      parsed.researchCurrencyVerdict = (researchVerdict || '').toUpperCase();

      if (!/^(YES|NO)$/i.test(researchRequired || '')) {
        errors.push('RESEARCH_CURRENCY_REQUIRED must be YES or NO');
      }
      if (!/^(CURRENT|STALE|NOT_APPLICABLE)$/i.test(researchVerdict || '')) {
        errors.push('RESEARCH_CURRENCY_VERDICT must be CURRENT | STALE | NOT_APPLICABLE');
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
        let freshSources = 0;
        const maxAgeDays = isPositiveIntegerString(sourceMaxAgeRaw) ? parseInt(sourceMaxAgeRaw, 10) : null;
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
          const url = record.URL || '';
          const why = record.WHY || '';
          parsed.researchSources.push({ source, kind, date: dateStr, url, why });

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
          const sourceDate = parseIsoDateUtc(dateStr);
          if (!sourceDate) {
            errors.push(`RESEARCH_CURRENCY SOURCE_LOG Date must be YYYY-MM-DD (got: ${dateStr || '<missing>'})`);
          } else {
            const ageDays = Math.floor((Date.now() - sourceDate.getTime()) / 86400000);
            if (ageDays < 0) errors.push(`RESEARCH_CURRENCY SOURCE_LOG Date cannot be in the future: ${dateStr}`);
            if (maxAgeDays !== null && ageDays >= 0 && ageDays <= maxAgeDays) freshSources += 1;
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
    if (resolved) {
      try {
        const specContent = fs.readFileSync(resolved.specFilePath, 'utf8');
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
