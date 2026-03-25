#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { loadPacket } from "../scripts/lib/role-resume-utils.mjs";
import { GOV_ROOT_REPO_REL } from "../scripts/lib/runtime-paths.mjs";
import { evaluateWpDeclaredTopology } from "../scripts/lib/wp-declared-topology-lib.mjs";

function usage() {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/checks/wp-declared-topology-check.mjs WP-{ID}`);
  process.exit(1);
}

function repoRoot() {
  return execFileSync("git", ["rev-parse", "--show-toplevel"], {
    stdio: ["ignore", "pipe", "ignore"],
    encoding: "utf8",
  }).trim();
}

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) usage();

const packetContent = loadPacket(wpId);
const evaluation = evaluateWpDeclaredTopology({
  repoRoot: repoRoot(),
  wpId,
  packetContent,
});

const prefix = evaluation.ok ? "PASS" : "FAIL";
console.log(`[WP_DECLARED_TOPOLOGY_CHECK] ${prefix}: ${wpId}`);
console.log(`  - coder_branch=${evaluation.topology.coderBranch}`);
console.log(`  - coder_worktree=${evaluation.topology.coderWorktreeDir}`);
console.log(`  - wp_validator_worktree=${evaluation.topology.wpValidatorWorktreeDir}`);
console.log(`  - integration_validator_worktree=${evaluation.topology.integrationWorktreeDir}`);
console.log(`  - related_worktrees=${evaluation.relatedWorktrees.length}`);
console.log(`  - undeclared_worktrees=${evaluation.undeclaredWorktrees.length}`);
for (const issue of evaluation.issues) {
  console.log(`  - issue=${issue}`);
}

process.exit(evaluation.ok ? 0 : 1);
