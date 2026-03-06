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

function isVersionAtLeast(isoDate, minIsoDate) {
  if (!isIsoDate(isoDate) || !isIsoDate(minIsoDate)) return false;
  // ISO date strings are lexicographically comparable.
  return isoDate >= minIsoDate;
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
  if (expectedWpId && wpId !== expectedWpId) {
    errors.push(`WP_ID mismatch in refinement: expected ${expectedWpId}, got ${wpId || '<missing>'}`);
  }

  const refinementFormatVersion = getSingleField(content, 'REFINEMENT_FORMAT_VERSION');
  const isModernRefinement = isVersionAtLeast(refinementFormatVersion, '2026-03-06');
  if (refinementFormatVersion && !isIsoDate(refinementFormatVersion)) {
    errors.push('REFINEMENT_FORMAT_VERSION must be YYYY-MM-DD (ISO date)');
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

  requiredSections.forEach((h) => {
    if (!hasHeading(content, h)) errors.push(`Missing required section heading: ${h}`);
  });

  if (isModernRefinement) {
    const stubWpIdsRaw = getSingleField(content, 'STUB_WP_IDS');
    if (isPlaceholderValue(stubWpIdsRaw)) {
      errors.push('STUB_WP_IDS must be set (comma-separated WP-... IDs or NONE)');
    } else if (!/^NONE$/i.test(stubWpIdsRaw)) {
      const ids = stubWpIdsRaw.split(',').map((s) => s.trim()).filter(Boolean);
      if (ids.length === 0) {
        errors.push('STUB_WP_IDS must be NONE or a comma-separated list of WP-... IDs');
      } else {
        for (const id of ids) {
          if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(id)) {
            errors.push(`STUB_WP_IDS contains invalid WP id: ${id}`);
            continue;
          }
          const stubPath = path.join('.GOV', 'task_packets', 'stubs', `${id}.md`);
          if (!fs.existsSync(stubPath)) {
            errors.push(`Stub referenced in STUB_WP_IDS does not exist: ${stubPath.replace(/\\/g, '/')}`);
          }
        }
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

    // PRIMITIVES section minimums.
    const lines = content.split('\n');
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

    // PRIMITIVE_INDEX gate.
    const primIndexAction = getSingleField(content, 'PRIMITIVE_INDEX_ACTION');
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
    const pillarVerdict = getSingleField(content, 'PILLAR_ALIGNMENT_VERDICT');
    if (!/^(OK|NEEDS_SPEC_UPDATE|NEEDS_STUBS)$/i.test(pillarVerdict || '')) {
      errors.push('PILLAR_ALIGNMENT_VERDICT must be OK | NEEDS_SPEC_UPDATE | NEEDS_STUBS');
    }

    // PRIMITIVE_MATRIX minimums.
    const matrixTimebox = getSingleField(content, 'MATRIX_SCAN_TIMEBOX');
    if (isPlaceholderValue(matrixTimebox)) errors.push('PRIMITIVE_MATRIX MATRIX_SCAN_TIMEBOX must be filled');

    const matrixVerdict = getSingleField(content, 'PRIMITIVE_MATRIX_VERDICT');
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

    // UI_UX_RUBRIC minimums.
    const uiApplicable = getSingleField(content, 'UI_UX_APPLICABLE');
    if (!/^(YES|NO)$/i.test(uiApplicable || '')) errors.push('UI_UX_APPLICABLE must be YES or NO');
    if (/^NO$/i.test(uiApplicable || '')) {
      const r = getSingleField(content, 'UI_UX_REASON_NO');
      if (isPlaceholderValue(r)) errors.push('UI_UX_REASON_NO is required when UI_UX_APPLICABLE=NO');
    } else if (/^YES$/i.test(uiApplicable || '')) {
      const surfaces = extractIndentedListAfterLabel(lines, 'UI_SURFACES');
      if (!surfaces.found || surfaces.items.filter((s) => !isPlaceholderValue(s)).length === 0) {
        errors.push('UI_UX_RUBRIC UI_SURFACES must include at least one concrete surface');
      }
      const controls = extractIndentedListAfterLabel(lines, 'UI_CONTROLS (buttons/dropdowns/inputs)');
      if (!controls.found || controls.items.filter((s) => !isPlaceholderValue(s)).length === 0) {
        errors.push('UI_UX_RUBRIC UI_CONTROLS must include at least one concrete control');
      } else {
        const anyTooltip = controls.items.some((s) => /\bTooltip:\b/i.test(s) && !/Tooltip:\s*<fill/i.test(s));
        if (!anyTooltip) errors.push('UI_UX_RUBRIC UI_CONTROLS entries must include concrete Tooltip: text');
      }
      const uiVerdict = getSingleField(content, 'UI_UX_VERDICT');
      if (!/^(OK|NEEDS_STUBS|UNKNOWN)$/i.test(uiVerdict || '')) {
        errors.push('UI_UX_VERDICT must be OK | NEEDS_STUBS | UNKNOWN');
      }
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
  if (/^PASS$/i.test(clearlyVerdict) && /^YES$/i.test(enrichmentNeeded)) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=PASS requires ENRICHMENT_NEEDED=NO');
  }
  if (/^FAIL$/i.test(clearlyVerdict) && /^NO$/i.test(enrichmentNeeded)) {
    errors.push('Inconsistent refinement: CLEARLY_COVERS_VERDICT=FAIL requires ENRICHMENT_NEEDED=YES');
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
  const lines = content.split('\n');
  const proposedViaLabel = extractFencedBlockAfterLabel(lines, 'PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)');
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
  if (requireSignature) {
    if (!/^(APPROVED)$/i.test(reviewStatus || '')) {
      errors.push('USER_REVIEW_STATUS must be APPROVED before task packet creation');
    }
    if (!signature || signature === '<pending>') {
      errors.push('USER_SIGNATURE must be set (not <pending>) before task packet creation');
    }
  }

  return { ok: errors.length === 0, errors, parsed: { wpId, reviewStatus, signature } };
}
