import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

const SPEC_CURRENT_PATH = path.join('docs', 'SPEC_CURRENT.md');

export function resolveSpecCurrent() {
  if (!fs.existsSync(SPEC_CURRENT_PATH)) {
    throw new Error(`Missing ${SPEC_CURRENT_PATH}`);
  }
  const specCurrent = fs.readFileSync(SPEC_CURRENT_PATH, 'utf8');
  const m = specCurrent.match(/Handshake_Master_Spec_v[0-9.]+\.md/);
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
  return path.join('docs', 'refinements', `${wpId}.md`);
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

  // Resolve SPEC_CURRENT and validate resolved spec + sha1.
  let resolved = null;
  try {
    resolved = resolveSpecCurrent();
  } catch (e) {
    errors.push(String(e?.message || e));
  }
  if (resolved) {
    const resolvedLine = getSingleField(content, 'SPEC_TARGET_RESOLVED');
    const expectedResolvedLine = `docs/SPEC_CURRENT.md -> ${resolved.specFileName}`;
    if (resolvedLine !== expectedResolvedLine) {
      errors.push(`SPEC_TARGET_RESOLVED mismatch: expected "${expectedResolvedLine}", got "${resolvedLine || '<missing>'}"`);
    }

    const sha1Line = getSingleField(content, 'SPEC_TARGET_SHA1');
    if (!sha1Line || sha1Line.toLowerCase() !== resolved.sha1.toLowerCase()) {
      errors.push(`SPEC_TARGET_SHA1 mismatch: expected ${resolved.sha1}, got ${sha1Line || '<missing>'}`);
    }
  }

  // Required sections (protocol).
  ['GAPS_IDENTIFIED', 'FLIGHT_RECORDER_INTERACTION', 'RED_TEAM_ADVISORY', 'PRIMITIVES'].forEach((h) => {
    if (!hasHeading(content, h)) errors.push(`Missing required section heading: ${h}`);
  });

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
  } else if (/^NO$/i.test(enrichmentNeeded)) {
    const reasonNo = getSingleField(content, 'REASON_NO_ENRICHMENT');
    if (isPlaceholderValue(reasonNo)) {
      errors.push('REASON_NO_ENRICHMENT is required when ENRICHMENT_NEEDED=NO');
    }
  } else if (/^YES$/i.test(enrichmentNeeded)) {
    const lines = content.split('\n');
    const block = extractFencedBlockAfterLabel(lines, 'PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)');
    // If label parsing fails due to label mismatch, fallback: look for heading then ```md block.
    let proposed = '';
    if (block.found) proposed = block.body;
    if (!proposed) {
      const idx = lines.findIndex((l) => /^####\s+PROPOSED_SPEC_ENRICHMENT\b/i.test(l));
      if (idx !== -1) {
        const after = lines.slice(idx + 1);
        const fenceIdx = after.findIndex((l) => l.trim().startsWith('```'));
        if (fenceIdx !== -1) {
          const b = [];
          for (let i = fenceIdx + 1; i < after.length; i += 1) {
            if (after[i].trim() === '```') break;
            b.push(after[i]);
          }
          proposed = b.join('\n').trim();
        }
      }
    }
    if (!proposed || /^<paste/i.test(proposed) || proposed.includes('<paste')) {
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
