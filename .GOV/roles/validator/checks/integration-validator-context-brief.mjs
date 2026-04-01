#!/usr/bin/env node

import fs from "node:fs";
import {
  currentGitContext,
  loadPacket,
  packetExists,
  packetPath,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { REPO_ROOT, repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import {
  buildIntegrationValidatorContextBrief,
  formatIntegrationValidatorContextBrief,
} from "../scripts/lib/integration-validator-context-brief-lib.mjs";

function usage() {
  console.error("Usage: node .GOV/roles/validator/checks/integration-validator-context-brief.mjs WP-{ID} [--json]");
  process.exit(1);
}

function parseArgs(argv) {
  const wpId = String(argv[0] || "").trim();
  if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) usage();

  let json = false;
  for (let index = 1; index < argv.length; index += 1) {
    const token = String(argv[index] || "").trim();
    if (token === "--json") {
      json = true;
      continue;
    }
    usage();
  }

  return { wpId, json };
}

const parsed = parseArgs(process.argv.slice(2));
if (!packetExists(parsed.wpId)) {
  console.error(`[INTEGRATION_VALIDATOR_CONTEXT_BRIEF] Task packet not found: ${packetPath(parsed.wpId)}`);
  process.exit(1);
}

const gateStatePath = resolveValidatorGatePath(parsed.wpId);
let gateState = {};
if (fs.existsSync(repoPathAbs(gateStatePath))) {
  gateState = JSON.parse(fs.readFileSync(repoPathAbs(gateStatePath), "utf8"));
}

const gitContext = currentGitContext();
const brief = buildIntegrationValidatorContextBrief({
  repoRoot: gitContext.topLevel || REPO_ROOT,
  wpId: parsed.wpId,
  packetContent: loadPacket(parsed.wpId),
  gitContext,
  gateState,
  committedEvidence: gateState?.committed_validation_evidence?.[parsed.wpId] || null,
  gateStatePath,
});

if (parsed.json) {
  process.stdout.write(`${JSON.stringify(brief, null, 2)}\n`);
} else {
  process.stdout.write(formatIntegrationValidatorContextBrief(brief));
}
