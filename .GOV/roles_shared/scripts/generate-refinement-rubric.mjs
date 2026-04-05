#!/usr/bin/env node
/**
 * RGF-91: Auto-generate the exhaustive pillar and engine rubric lines for a refinement file.
 *
 * Usage: node generate-refinement-rubric.mjs [--pillars] [--engines] [--both]
 *
 * Outputs the rubric lines to stdout so the orchestrator can paste them into the refinement.
 * The orchestrator only needs to fill STATUS and NOTES per line.
 */

import fs from 'fs';
import path from 'path';
import { GOV_ROOT_REPO_REL, repoPathAbs } from './lib/runtime-paths.mjs';
import { resolveSpecCurrent } from '../checks/refinement-check.mjs';

const KNOWN_PILLARS = [
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
  'Front End Memory System',
  'Execution / Job Runtime',
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

function extractMechanicalEngines(specContent) {
  const engines = [...specContent.matchAll(/#### Engine: ([^\n(]+).*?\n\n- \*\*Engine ID:\*\* `([^`]+)`/gs)]
    .map((m) => ({ title: m[1].trim(), id: m[2].trim() }))
    .filter((entry) => entry.id && entry.title);
  return engines;
}

function generatePillarRubric() {
  const lines = [
    '### PILLAR_ALIGNMENT (Handshake pillars cross-check)',
    '- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.',
    '- Required rubric lines (one per pillar; do not delete lines, fill values):',
  ];
  for (const pillar of KNOWN_PILLARS) {
    lines.push(`  - PILLAR: ${pillar} | STATUS: NOT_TOUCHED | NOTES: <fill> | STUB_WP_IDS: NONE`);
  }
  lines.push('- PILLAR_ALIGNMENT_VERDICT: OK');
  return lines.join('\n');
}

function generateEngineRubric(specContent) {
  const engines = extractMechanicalEngines(specContent);
  if (engines.length === 0) {
    console.error('[ERROR] Could not extract mechanical engines from spec. Check §11.8 / §6.3.');
    process.exit(1);
  }
  const lines = [
    '### MECHANICAL_ENGINE_ALIGNMENT (spec-grade 22-engine set; treat each as a stand-alone feature surface)',
    '- Rule: inspect the spec-grade mechanical engine set in Master Spec 11.8 / 6.3 as first-class force multipliers. Do not treat these engines as a vague implementation bag. If UNKNOWN, create stubs instead of guessing.',
    '- Required rubric lines (one per engine; do not delete lines, fill values):',
  ];
  for (const engine of engines) {
    lines.push(`  - ENGINE: ${engine.title} | ENGINE_ID: ${engine.id} | STATUS: NOT_TOUCHED | NOTES: <fill> | STUB_WP_IDS: NONE`);
  }
  lines.push('- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK');
  return lines.join('\n');
}

const args = process.argv.slice(2);
const wantPillars = args.includes('--pillars') || args.includes('--both') || args.length === 0;
const wantEngines = args.includes('--engines') || args.includes('--both') || args.length === 0;

let specContent = '';
if (wantEngines) {
  try {
    const { specFilePath } = resolveSpecCurrent();
    specContent = fs.readFileSync(repoPathAbs(specFilePath), 'utf8');
  } catch (e) {
    console.error(`[ERROR] ${e.message}`);
    process.exit(1);
  }
}

if (wantEngines) {
  console.log(generateEngineRubric(specContent));
  console.log('');
}
if (wantPillars) {
  console.log(generatePillarRubric());
}
