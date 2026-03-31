#!/usr/bin/env node

import { buildActiveLaneBrief, formatActiveLaneBrief } from "../scripts/session/active-lane-brief-lib.mjs";
import { REPO_ROOT } from "../scripts/lib/runtime-paths.mjs";

function usage() {
  console.error("Usage: node .GOV/roles_shared/checks/active-lane-brief.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> WP-{ID} [--json]");
  process.exit(1);
}

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const json = process.argv.slice(4).includes("--json");

if (!role || !wpId || !/^WP-/.test(wpId)) usage();

const brief = buildActiveLaneBrief({
  repoRoot: REPO_ROOT,
  role,
  wpId,
});

if (json) {
  process.stdout.write(`${JSON.stringify(brief, null, 2)}\n`);
} else {
  process.stdout.write(formatActiveLaneBrief(brief));
}
